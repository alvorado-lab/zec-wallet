//! The sync-server picker (`docs/specs/sync-server-picker.md`, phase-3 P3-13):
//! the types a host OFFERS servers with, the reference catalog, the one aux row
//! that remembers the user's choice, and the pure resolution that turns a row
//! plus an offered list into the endpoint the wallet actually dials.
//!
//! **Who owns what (spec D1).** The SDK owns the SHAPE ([`SyncServer`],
//! [`SyncServerChoice`], [`SyncServerStatus`]), the VALIDATION (every URL
//! through [`LightServerEndpoint::new`], every list through
//! [`validate_sync_servers`]), the SWITCH (`Wallet::switch_sync_server`) and the
//! PERSISTENCE (the [`TABLE`] row). The host supplies the offered list's
//! CONTENTS and each gated entry's key; the user supplies a custom URL and,
//! optionally, a key for it (ADR-0568). The reference catalog
//! ([`SyncServerCatalog::reference`]) is exported — public servers only — and a
//! host appends servers of its own to it.
//!
//! **The persisted choice WINS at open (spec D2).** `WalletConfig.endpoint` is
//! the DEFAULT — what a wallet with no row uses, and what a row that cannot be
//! honoured falls back to, VISIBLY through [`SyncServerStatus::fallback`]. The
//! resolution is [`resolve`], a pure function, so its every arm is a unit test
//! and its mutants watch in seconds; `Wallet::from_open` wires it.
//!
//! **A HOST's key never lands here (spec D6).** A `Predefined` row stores the
//! entry's ID, never its auth; the host re-supplies the key at every open. A
//! `Custom` row stores the URL, which the validator has already refused
//! userinfo on, and — the ONE key the SDK stores (ADR-0568: only the user
//! knows it) — the user's key for that server, bound to the URL it was saved
//! for (`auth_url`), erased by every other write, never returned, never
//! logged. The row rides a rescan UNCLEARED (`db::AUX_TABLES_PRESERVED`,
//! the `consensus_stamp` posture — the choice is about the user's trust, not
//! the chain view) and dies with the wallet at wipe.
//!
//! **Storage.** Same discipline as [`crate::ever_synced`]: pure SQL over the aux
//! SQLCipher connection to the ONE sealed `wallet.db`, created idempotently in
//! `db::migrate` (`ensure_aux_tables`), single-statement writes (`RESERVED`
//! atomically — the aux-write deadlock-freedom invariant holds). The READ is
//! infallible by design: a wrong server cannot move funds (the sync guards
//! judge a lying server pass by pass), so a malformed row degrades to the
//! DEFAULT with `fallback = ChoiceUnreadable` rather than failing the open.

use rusqlite::{Connection, OptionalExtension};

use crate::config::{EndpointAuth, LightServerEndpoint};
use crate::constants::{
    ENDPOINT_AUTH_VALUE_MAX_BYTES, SYNC_SERVER_AUTH_HEADER_MAX_BYTES, SYNC_SERVER_ID_MAX_BYTES,
    SYNC_SERVER_LABEL_MAX_BYTES, SYNC_SERVER_URL_MAX_BYTES, SYNC_SERVERS_MAX,
};
use crate::db::map_aux_err;
use crate::error::WalletError;
use crate::money::Network;

/// A stable, host-chosen identifier for an offered server — what the aux row
/// stores. `[a-z0-9-]`, `1..=SYNC_SERVER_ID_MAX_BYTES`. Never shown to a user;
/// the label is.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SyncServerId(String);

impl SyncServerId {
    /// Validate an id: lowercase ASCII letters, digits and `-`, bounded.
    /// The charset is deliberately narrow — the id is a slug that lands in an
    /// aux cell and a log line, never a display string.
    pub fn new(id: impl Into<String>) -> Result<Self, WalletError> {
        let id = id.into();
        if id.is_empty() {
            return Err(WalletError::InvalidEndpoint {
                reason: "sync server id is empty",
            });
        }
        if id.len() > SYNC_SERVER_ID_MAX_BYTES {
            return Err(WalletError::InvalidEndpoint {
                reason: "sync server id is longer than SYNC_SERVER_ID_MAX_BYTES",
            });
        }
        if !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(WalletError::InvalidEndpoint {
                reason: "sync server id must be lowercase ascii letters, digits or '-'",
            });
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A server the host OFFERS. Validated at construction; immutable after.
///
/// `Debug` is hand-written: it prints the id, the label and the endpoint,
/// NEVER `auth` (whose own `Debug` already redacts the value — this type does
/// not even name the header, so a config dump says nothing about the key at
/// all). No `PartialEq`: comparing two servers would compare secrets.
#[derive(Clone)]
pub struct SyncServer {
    id: SyncServerId,
    label: String,
    endpoint: LightServerEndpoint,
    auth: Option<EndpointAuth>,
}

impl SyncServer {
    /// Build an offered entry. The label is bounded and must be printable —
    /// it is rendered verbatim in the picker, so a control character here is
    /// a display bug at best and a spoof at worst.
    pub fn new(
        id: SyncServerId,
        label: impl Into<String>,
        endpoint: LightServerEndpoint,
        auth: Option<EndpointAuth>,
    ) -> Result<Self, WalletError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(WalletError::InvalidEndpoint {
                reason: "sync server label is empty",
            });
        }
        if label.len() > SYNC_SERVER_LABEL_MAX_BYTES {
            return Err(WalletError::InvalidEndpoint {
                reason: "sync server label is longer than SYNC_SERVER_LABEL_MAX_BYTES",
            });
        }
        if label.chars().any(char::is_control) {
            return Err(WalletError::InvalidEndpoint {
                reason: "sync server label carries a control character",
            });
        }
        Ok(Self {
            id,
            label,
            endpoint,
            auth,
        })
    }

    pub fn id(&self) -> &SyncServerId {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn endpoint(&self) -> &LightServerEndpoint {
        &self.endpoint
    }

    /// Whether this entry sends an auth header. The VALUE is never exposed;
    /// the transport reads it through `EndpointAuth::with_value`.
    pub fn is_gated(&self) -> bool {
        self.auth.is_some()
    }

    /// The auth header's NAME (`x-zcash-rpc-key`), if the entry is gated —
    /// public config, the same thing `EndpointAuth`'s `Debug` prints. Never
    /// the value.
    pub fn auth_header(&self) -> Option<&str> {
        self.auth.as_ref().map(|a| a.header().as_str())
    }

    /// The entry's auth, for the client the switch and the probe build. The
    /// VALUE is read only through `EndpointAuth::with_value`, in the transport.
    pub(crate) fn auth(&self) -> Option<&EndpointAuth> {
        self.auth.as_ref()
    }
}

impl core::fmt::Debug for SyncServer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SyncServer")
            .field("id", &self.id.as_str())
            .field("label", &self.label)
            .field("endpoint", &self.endpoint.as_str())
            .field("gated", &self.auth.is_some())
            .finish()
    }
}

/// Validate an OFFERED list at the config door: bounded, ids unique. Every
/// entry was already validated by its own constructor.
pub fn validate_sync_servers(list: &[SyncServer]) -> Result<(), WalletError> {
    if list.len() > SYNC_SERVERS_MAX {
        return Err(WalletError::InvalidEndpoint {
            reason: "more sync servers offered than SYNC_SERVERS_MAX",
        });
    }
    for (i, a) in list.iter().enumerate() {
        if list[..i].iter().any(|b| b.id == a.id) {
            return Err(WalletError::InvalidEndpoint {
                reason: "two offered sync servers share an id",
            });
        }
    }
    Ok(())
}

/// What the user asked for.
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SyncServerChoice {
    /// One of the offered entries, by id.
    Predefined(SyncServerId),
    /// A URL the user typed, with the key the user gave for it, if any
    /// (ADR-0568). Already validated — the newtypes are the proof. A key on a
    /// plaintext URL is refused where a choice enters the wallet
    /// ([`check_custom_key`], asked by the probe and the switch) and where a
    /// stored one is read back ([`resolve`]). `Debug` prints the key's header,
    /// never its value (`EndpointAuth`'s own redaction).
    Custom {
        endpoint: LightServerEndpoint,
        key: Option<EndpointAuth>,
    },
    /// Clear the choice: back to `WalletConfig.endpoint`.
    Default,
}

/// The ONE rule for a custom server's key (ADR-0568): a key never rides a
/// plaintext (`http://` loopback) URL — any app on the device can bind a
/// loopback port and read it. The documented host-key loopback exemption
/// (`EndpointAuth::new`'s doc) is a developer's config, not a user's key.
pub(crate) fn check_custom_key(
    endpoint: &LightServerEndpoint,
    key: Option<&EndpointAuth>,
) -> Result<(), WalletError> {
    if key.is_some() && endpoint.is_plaintext() {
        return Err(WalletError::InvalidEndpointAuth {
            reason: "a key cannot be sent to an unencrypted (http://) server",
        });
    }
    Ok(())
}

/// Why a persisted choice is NOT the endpoint in use. Never silent
/// (Principle 10): the status carries it and the picker renders it.
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SyncServerFallback {
    /// The row names an id this host no longer offers.
    ChoiceNotOffered { id: SyncServerId },
    /// The row is present but malformed (a truncated cell, an unknown kind, a
    /// URL the validator refuses). The wallet is usable; the choice is not.
    ChoiceUnreadable,
    /// The row is a readable CUSTOM choice that the wallet's transport policy
    /// cannot carry: a plaintext (`http://` loopback) URL persisted under
    /// `TorPolicy::Off` met a host-dialer policy at this open (FR-29 spec
    /// §6.1 E12). Plaintext rides only the SDK's own direct dialer; through a
    /// host transport it would ship raw gRPC, so the default is dialed and the
    /// picker says why. The choice is kept, so switching back under `Off`
    /// honours it again — never a permanent stall, never plaintext.
    ChoiceRefusedByTransport,
}

/// The reachability probe's answer — a fact about REACHABILITY and IDENTITY
/// (the server dialed under the wallet's own `TorPolicy`, answered, and
/// claims this wallet's network), never a verdict about honesty: the sync
/// guards judge that pass by pass.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyncServerProbe {
    /// The server's claimed best-chain tip. UNTRUSTED; a bare height,
    /// loggable (§5.4). Only `GetLightdInfo`'s three compared fields and
    /// this one survive the boundary — vendor and version strings are dropped.
    pub tip: crate::money::BlockHeight,
}

/// What the wallet is actually talking to, and why.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyncServerStatus {
    /// The endpoint every client this wallet builds dials right now.
    pub effective: LightServerEndpoint,
    /// `WalletConfig.endpoint` — rendered as "App default" when it is not
    /// among the offered entries.
    pub default: LightServerEndpoint,
    /// The persisted choice. `None` = no row = the default is in use by
    /// absence, not by fallback.
    pub choice: Option<SyncServerChoice>,
    /// `Some` = the row exists but could not be honoured; `effective` is then
    /// `default`.
    pub fallback: Option<SyncServerFallback>,
}

// ── The reference catalog ───────────────────────────────────────────────────

/// One reference entry, as the catalog names it — the ONE place the reference
/// servers' URLs live in Rust (the Dart reference config reads them through
/// the bridge rather than carrying its own literals). Every reference entry is
/// a PUBLIC server: the SDK ships no gated server and no key (ADR-0568).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReferenceSyncServer {
    pub id: &'static str,
    pub label: &'static str,
    pub url: &'static str,
}

/// The public mainnet reference server. Until ADR-0568 the catalog also named
/// the maintainer's own gated lightwalletd (phase-1 §4s Q-S1), keyed by its
/// caller; that entry is now the HOST's to offer, through
/// `WalletConfig.sync_servers`, with a key the host's build supplies.
pub const REFERENCE_SYNC_SERVERS_MAINNET: &[ReferenceSyncServer] = &[ReferenceSyncServer {
    id: "zec-rocks",
    label: "zec.rocks",
    url: "https://zec.rocks:443",
}];

/// The testnet rail (the same operator as the mainnet reference).
pub const REFERENCE_SYNC_SERVERS_TESTNET: &[ReferenceSyncServer] = &[ReferenceSyncServer {
    id: "zec-rocks-testnet",
    label: "zec.rocks (testnet)",
    url: "https://testnet.zec.rocks:443",
}];

/// The reference catalog.
pub struct SyncServerCatalog;

impl SyncServerCatalog {
    /// The reference entries for `network`, as plain descriptors (no key).
    pub fn entries(network: Network) -> &'static [ReferenceSyncServer] {
        // Exhaustive on purpose (the security review's INFO): a third network
        // must get its own list, never mainnet's by fall-through.
        match network {
            Network::Main => REFERENCE_SYNC_SERVERS_MAINNET,
            Network::Test => REFERENCE_SYNC_SERVERS_TESTNET,
        }
    }

    /// The reference list as offered entries — public servers, no key
    /// (ADR-0568). A host that offers servers of its own, gated or not,
    /// appends them to this list in `WalletConfig.sync_servers`.
    pub fn reference(network: Network) -> Result<Vec<SyncServer>, WalletError> {
        Self::entries(network)
            .iter()
            .map(|e| {
                SyncServer::new(
                    SyncServerId::new(e.id)?,
                    e.label,
                    LightServerEndpoint::new(e.url)?,
                    None,
                )
            })
            .collect()
    }
}

// ── The aux row ─────────────────────────────────────────────────────────────

/// The single-row table name — referenced by `db::AUX_TABLES_PRESERVED`
/// (ADR-0534; rides a rescan UNCLEARED).
pub(crate) const TABLE: &str = "sync_server_choice";

const KIND_PREDEFINED: &str = "predefined";
const KIND_CUSTOM: &str = "custom";

/// Create the row's table (idempotent; runs in `db::migrate` on every
/// provision AND open, so existing wallets gain it with no schema bump — a
/// pre-picker wallet has no row, which reads as "the default, by absence").
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sync_server_choice (
             singleton           INTEGER PRIMARY KEY CHECK (singleton = 1),
             kind                TEXT    NOT NULL CHECK (kind IN ('predefined', 'custom')),
             value               TEXT    NOT NULL,
             chosen_at_unix_secs INTEGER NOT NULL
         );",
    )
    .map_err(map_aux_err)?;
    ensure_key_columns(conn)
}

/// The user's custom-server key columns (ADR-0568), added by an idempotent
/// `ALTER TABLE … ADD COLUMN` — NEVER in the `CREATE` above, so a fresh and an
/// upgraded database converge to ONE column order (`copy_aux_tables` maps by
/// ordinal behind `assert_columns_match`). Appended in this fixed order; a
/// future column goes after `auth_url`. Not a TOCTOU hazard: `migrate` runs
/// under the exclusive single-opener lock (the `intent_store` precedent).
///
/// - `auth_header`, `auth_value`: the user's key for a `custom` row;
/// - `auth_url`: the URL the key was saved FOR. An older build rewriting the
///   row's URL leaves the key behind; the read refuses a key whose `auth_url`
///   is not the row's URL, so the key is never lent to another server.
fn ensure_key_columns(conn: &Connection) -> Result<(), WalletError> {
    let existing: std::collections::HashSet<String> = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(sync_server_choice)")
            .map_err(map_aux_err)?;
        let names = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(map_aux_err)?;
        names.collect::<Result<_, _>>().map_err(map_aux_err)?
    };
    for column in ["auth_header", "auth_value", "auth_url"] {
        if !existing.contains(column) {
            conn.execute_batch(&format!(
                "ALTER TABLE sync_server_choice ADD COLUMN {column} TEXT"
            ))
            .map_err(map_aux_err)?;
        }
    }
    Ok(())
}

const UPSERT: &str = "INSERT INTO sync_server_choice
         (singleton, kind, value, chosen_at_unix_secs, auth_header, auth_value, auth_url)
     VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(singleton) DO UPDATE SET
         kind = excluded.kind,
         value = excluded.value,
         chosen_at_unix_secs = excluded.chosen_at_unix_secs,
         auth_header = excluded.auth_header,
         auth_value = excluded.auth_value,
         auth_url = excluded.auth_url";

/// Persist `choice`: `Predefined` stores the ID (never a HOST's key), `Custom`
/// the URL and — when the user gave one — the user's key with the URL it is
/// bound to; `Default` DELETES the row. EVERY write sets every key column, so
/// any choice but a keyed custom one ERASES a stored key (the aux connection
/// runs `secure_delete`, `db::ensure_secure_delete`). One statement each —
/// crash-atomic; a kill leaves the old row or the new one, never a torn cell.
/// The key's value is bound BORROWED inside `with_value` — never copied, never
/// formatted into SQL text. `at_unix_secs` is observability only, never read
/// as a money input. Production caller: `Wallet::switch_sync_server`, on the
/// solely-owned aux connection.
pub(crate) fn write(
    conn: &Connection,
    choice: &SyncServerChoice,
    at_unix_secs: u64,
) -> Result<(), WalletError> {
    let at = i64::try_from(at_unix_secs).unwrap_or(i64::MAX);
    let none: Option<&str> = None;
    let written = match choice {
        SyncServerChoice::Predefined(id) => conn.execute(
            UPSERT,
            rusqlite::params![KIND_PREDEFINED, id.as_str(), at, none, none, none],
        ),
        SyncServerChoice::Custom {
            endpoint,
            key: None,
        } => conn.execute(
            UPSERT,
            rusqlite::params![KIND_CUSTOM, endpoint.as_str(), at, none, none, none],
        ),
        SyncServerChoice::Custom {
            endpoint,
            key: Some(key),
        } => key.with_value(|value| {
            conn.execute(
                UPSERT,
                rusqlite::params![
                    KIND_CUSTOM,
                    endpoint.as_str(),
                    at,
                    key.header_name(),
                    value,
                    endpoint.as_str()
                ],
            )
        }),
        SyncServerChoice::Default => return clear(conn),
        // `#[non_exhaustive]` in the public API; inside the crate every variant
        // is named above, so this arm is unreachable in a lockstep build.
        #[allow(unreachable_patterns)]
        _ => return clear(conn),
    };
    written.map_err(map_aux_err)?;
    Ok(())
}

/// Delete the row (the `Default` choice). Idempotent.
pub(crate) fn clear(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM sync_server_choice", [])
        .map_err(map_aux_err)?;
    Ok(())
}

/// The row as read back — RAW, before resolution against the offered list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum StoredChoice {
    /// No row: the default, by absence.
    Absent,
    /// A `predefined` row: the stored id string (validated in [`resolve`]).
    Predefined(String),
    /// A `custom` row: the stored URL string (validated in [`resolve`]) and
    /// the user's key, ALREADY an `EndpointAuth` (built inside [`read`], so no
    /// owned plaintext copy of the value outlives the read — and `Debug` here
    /// prints the header, never the value).
    Custom {
        url: String,
        key: Option<EndpointAuth>,
    },
    /// A row is present and cannot be read as either kind (an unknown kind,
    /// a cell over its bound, a half key, a key on a `predefined` row, a key
    /// bound to another URL, a key the door refuses, a read fault). Resolves
    /// to the default — under the DEFAULT's auth — with `ChoiceUnreadable`.
    Unreadable,
}

/// One TEXT-or-NULL cell, bounded BEFORE anything is copied (§4.6).
/// `Some(None)` = NULL; `Some(Some(s))` = text within `max` bytes; `None` =
/// unreadable (over the bound, not UTF-8, or another storage type).
fn bounded_text(v: rusqlite::types::ValueRef<'_>, max: usize) -> Option<Option<&str>> {
    match v {
        rusqlite::types::ValueRef::Null => Some(None),
        rusqlite::types::ValueRef::Text(b) if b.len() <= max => {
            std::str::from_utf8(b).ok().map(Some)
        }
        _ => None,
    }
}

/// The longest `kind` cell the writer produces (`predefined`), with room.
const KIND_MAX_BYTES: usize = 16;

/// Read the row. INFALLIBLE by design (see the module doc): a fault reads as
/// [`StoredChoice::Unreadable`], logged at `warn` with no value (§5.4 — the
/// value may be a custom host, or a key).
///
/// Every cell is read BORROWED (`get_ref`) and bounded before a byte is
/// copied; the key's value is copied exactly once, straight into the
/// `Zeroizing` an `EndpointAuth` owns (crypto audit H3). ONE query, so a
/// mistyped column stays `Unreadable` through the one error path.
pub(crate) fn read(conn: &Connection) -> StoredChoice {
    let row: Result<Option<StoredChoice>, rusqlite::Error> = conn
        .query_row(
            "SELECT kind, value, auth_header, auth_value, auth_url
             FROM sync_server_choice WHERE singleton = 1",
            [],
            |r| {
                let kind = bounded_text(r.get_ref(0)?, KIND_MAX_BYTES);
                let value = bounded_text(r.get_ref(1)?, SYNC_SERVER_URL_MAX_BYTES);
                let header = bounded_text(r.get_ref(2)?, SYNC_SERVER_AUTH_HEADER_MAX_BYTES);
                let secret = bounded_text(r.get_ref(3)?, ENDPOINT_AUTH_VALUE_MAX_BYTES);
                let bound_url = bounded_text(r.get_ref(4)?, SYNC_SERVER_URL_MAX_BYTES);
                Ok(decode_row(kind, value, header, secret, bound_url))
            },
        )
        .optional();
    let stored = match row {
        Ok(None) => StoredChoice::Absent,
        Ok(Some(stored)) => stored,
        Err(_) => StoredChoice::Unreadable,
    };
    if matches!(stored, StoredChoice::Unreadable) {
        tracing::warn!(
            target: "zec_wallet_core",
            outcome = "fallback_unreadable",
            "wallet.sync_server"
        );
    }
    stored
}

/// The row's cells, already bounded, as a [`StoredChoice`]. Pure over borrowed
/// cells (`bounded_text`'s shape), so every refusal is a plain unit test.
fn decode_row(
    kind: Option<Option<&str>>,
    value: Option<Option<&str>>,
    header: Option<Option<&str>>,
    secret: Option<Option<&str>>,
    bound_url: Option<Option<&str>>,
) -> StoredChoice {
    let (Some(Some(kind)), Some(Some(value)), Some(header), Some(secret), Some(bound_url)) =
        (kind, value, header, secret, bound_url)
    else {
        return StoredChoice::Unreadable;
    };
    match (kind, header, secret, bound_url) {
        (KIND_PREDEFINED, None, None, None) => StoredChoice::Predefined(value.to_owned()),
        // A key on a predefined row: our writer never puts one there.
        (KIND_PREDEFINED, ..) => StoredChoice::Unreadable,
        (KIND_CUSTOM, None, None, None) => StoredChoice::Custom {
            url: value.to_owned(),
            key: None,
        },
        // The key is bound to THIS row's URL, or it is not lent at all.
        (KIND_CUSTOM, Some(header), Some(secret), Some(bound_url)) if bound_url == value => {
            match EndpointAuth::from_zeroizing(header, zeroize::Zeroizing::new(secret.to_owned())) {
                Ok(key) => StoredChoice::Custom {
                    url: value.to_owned(),
                    key: Some(key),
                },
                Err(_) => StoredChoice::Unreadable,
            }
        }
        // A half key, a key bound to another URL, an unknown kind.
        _ => StoredChoice::Unreadable,
    }
}

// ── Resolution ──────────────────────────────────────────────────────────────

/// The endpoint the wallet dials, and the status that explains it.
#[derive(Clone, Debug)]
pub(crate) struct Resolved {
    pub(crate) effective: LightServerEndpoint,
    pub(crate) auth: Option<EndpointAuth>,
    pub(crate) choice: Option<SyncServerChoice>,
    pub(crate) fallback: Option<SyncServerFallback>,
}

/// Turn a stored row plus the offered list into the endpoint to dial. PURE.
///
/// - no row ⇒ the default, by absence;
/// - a `predefined` id that is offered ⇒ that entry (its auth rides along);
/// - a `predefined` id that is NOT offered ⇒ the default,
///   `ChoiceNotOffered { id }` — the choice is kept so the picker can say
///   which server it was;
/// - a `custom` URL the validator accepts ⇒ that URL, with the user's key if
///   the row carries one (ADR-0568) — UNLESS it is plaintext and
///   `plaintext_admitted` is false (a host-dialer policy, FR-29 spec §6.1 E12)
///   ⇒ the default, `ChoiceRefusedByTransport`, the choice kept;
/// - a `custom` row with a key on a plaintext URL (the writer never produces
///   one: [`check_custom_key`]) ⇒ the default, `ChoiceUnreadable`;
/// - anything else ⇒ the default, `ChoiceUnreadable`.
///
/// Every fallback dials the default under the DEFAULT's auth — a stored custom
/// key is never lent to another server.
///
/// `plaintext_admitted` is [`crate::config::plaintext_admitted`] evaluated on
/// the wallet's policy — the door's ONE predicate, asked here for the aux
/// row the door never sees. A bool rather than the policy itself keeps this
/// function pure over plain data.
pub(crate) fn resolve(
    stored: StoredChoice,
    offered: &[SyncServer],
    default: &LightServerEndpoint,
    default_auth: Option<&EndpointAuth>,
    plaintext_admitted: bool,
) -> Resolved {
    let fall_back = |choice: Option<SyncServerChoice>, fallback: SyncServerFallback| Resolved {
        effective: default.clone(),
        auth: default_auth.cloned(),
        choice,
        fallback: Some(fallback),
    };
    match stored {
        StoredChoice::Absent => Resolved {
            effective: default.clone(),
            auth: default_auth.cloned(),
            choice: None,
            fallback: None,
        },
        StoredChoice::Predefined(raw) => match SyncServerId::new(raw) {
            Ok(id) => match offered.iter().find(|s| s.id == id) {
                Some(server) => Resolved {
                    effective: server.endpoint.clone(),
                    auth: server.auth.clone(),
                    choice: Some(SyncServerChoice::Predefined(id)),
                    fallback: None,
                },
                None => fall_back(
                    Some(SyncServerChoice::Predefined(id.clone())),
                    SyncServerFallback::ChoiceNotOffered { id },
                ),
            },
            Err(_) => fall_back(None, SyncServerFallback::ChoiceUnreadable),
        },
        StoredChoice::Custom { url, key } => match LightServerEndpoint::new(url) {
            Ok(endpoint) if check_custom_key(&endpoint, key.as_ref()).is_err() => {
                fall_back(None, SyncServerFallback::ChoiceUnreadable)
            }
            // E12: readable, but the transport cannot carry it — the choice is
            // kept (the picker names it; `Off` honours it again), the default
            // is dialed, and the status says why. Keyless by the arm above.
            Ok(endpoint) if endpoint.is_plaintext() && !plaintext_admitted => fall_back(
                Some(SyncServerChoice::Custom { endpoint, key }),
                SyncServerFallback::ChoiceRefusedByTransport,
            ),
            Ok(endpoint) => Resolved {
                effective: endpoint.clone(),
                auth: key.clone(),
                choice: Some(SyncServerChoice::Custom { endpoint, key }),
                fallback: None,
            },
            Err(_) => fall_back(None, SyncServerFallback::ChoiceUnreadable),
        },
        StoredChoice::Unreadable => fall_back(None, SyncServerFallback::ChoiceUnreadable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&c).expect("ensure");
        c
    }

    fn endpoint(url: &str) -> LightServerEndpoint {
        LightServerEndpoint::new(url).expect("endpoint")
    }

    fn server(id: &str, url: &str, auth: Option<EndpointAuth>) -> SyncServer {
        SyncServer::new(SyncServerId::new(id).expect("id"), id, endpoint(url), auth)
            .expect("server")
    }

    fn key() -> EndpointAuth {
        EndpointAuth::new("x-zcash-rpc-key", "the-key-value").expect("auth")
    }

    /// A user's key for their own server (ADR-0568) — a different header and
    /// value from the host's [`key`], so a test can tell which one rode.
    fn user_key() -> EndpointAuth {
        EndpointAuth::new("x-api-key", "users-own-key-7c1e").expect("auth")
    }

    fn custom(url: &str) -> SyncServerChoice {
        SyncServerChoice::Custom {
            endpoint: endpoint(url),
            key: None,
        }
    }

    fn stored_custom(url: &str) -> StoredChoice {
        StoredChoice::Custom {
            url: url.to_owned(),
            key: None,
        }
    }

    /// The key columns of the row, read RAW (no decoding).
    fn raw_key_columns(c: &Connection) -> (Option<String>, Option<String>, Option<String>) {
        c.query_row(
            "SELECT auth_header, auth_value, auth_url FROM sync_server_choice WHERE singleton = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("row")
    }

    /// The pre-FR-29 four-input shape for the rows below, every one of which
    /// resolves under `TorPolicy::Off` (plaintext admitted). Shadows the glob
    /// import on purpose (an explicit item wins over `use super::*`); the E12
    /// row calls `super::resolve` with the policy bit spelled out.
    fn resolve(
        stored: StoredChoice,
        offered: &[SyncServer],
        default: &LightServerEndpoint,
        default_auth: Option<&EndpointAuth>,
    ) -> Resolved {
        super::resolve(stored, offered, default, default_auth, true)
    }

    /// FR-29 spec §6.1 E12 (T21, the pure layer): a readable CUSTOM choice that
    /// is plaintext (`http://` loopback — persisted under `Off`, where it is the
    /// development configuration) meets a policy that admits no plaintext (a
    /// host dialer): the default is dialed, the fallback is
    /// `ChoiceRefusedByTransport`, and the choice is KEPT so the picker can
    /// name it and `Off` honours it again — the control below. A plaintext
    /// choice under a non-Off policy must never resolve to the plaintext
    /// endpoint (raw gRPC through the host's transport) and must never stall.
    #[test]
    fn a_persisted_plaintext_choice_under_a_host_dialer_falls_back_visibly() {
        let default = endpoint("https://default.example:443");
        let loopback = "http://127.0.0.1:9067";
        let refused = super::resolve(stored_custom(loopback), &[], &default, None, false);
        assert_eq!(
            refused.effective, default,
            "the plaintext choice is never dialed through a host transport"
        );
        assert_eq!(
            refused.fallback,
            Some(SyncServerFallback::ChoiceRefusedByTransport)
        );
        assert_eq!(
            refused.choice,
            Some(custom(loopback)),
            "the choice is kept — readable, just not carriable"
        );
        assert!(refused.auth.is_none(), "no default auth was configured");
        // The control: the SAME row under `Off` is honoured (the stage 0 rule
        // admits plaintext only there).
        let honoured = super::resolve(stored_custom(loopback), &[], &default, None, true);
        assert_eq!(honoured.effective.as_str(), loopback);
        assert_eq!(honoured.fallback, None);
        // And an `https` custom choice is untouched by the policy bit.
        let tls = super::resolve(
            stored_custom("https://mine.example:443"),
            &[],
            &default,
            None,
            false,
        );
        assert_eq!(tls.effective.as_str(), "https://mine.example:443");
        assert_eq!(tls.fallback, None);
    }

    #[test]
    fn the_reference_catalog_names_public_servers_only() {
        // ADR-0568: the SDK ships no gated server and no key. Mainnet is
        // exactly zec.rocks; testnet its rail; no entry on either is gated.
        let main = SyncServerCatalog::reference(Network::Main).expect("catalog");
        let ids: Vec<&str> = main.iter().map(|s| s.id().as_str()).collect();
        assert_eq!(ids, ["zec-rocks"]);
        assert_eq!(main[0].endpoint().as_str(), "https://zec.rocks:443");
        let test = SyncServerCatalog::reference(Network::Test).expect("catalog");
        let ids: Vec<&str> = test.iter().map(|s| s.id().as_str()).collect();
        assert_eq!(ids, ["zec-rocks-testnet"]);
        assert_eq!(test[0].endpoint().as_str(), "https://testnet.zec.rocks:443");
        assert!(
            main.iter().chain(test.iter()).all(|s| !s.is_gated()),
            "a reference entry carries no key"
        );
    }

    #[test]
    fn sync_servers_config_refuses_duplicate_ids_oversize_lists_and_unprintable_labels() {
        // Duplicate ids.
        let dup = vec![
            server("a", "https://a.example:443", None),
            server("a", "https://b.example:443", None),
        ];
        assert!(matches!(
            validate_sync_servers(&dup),
            Err(WalletError::InvalidEndpoint { reason }) if reason.contains("share an id")
        ));
        // Oversize list.
        let many: Vec<SyncServer> = (0..=SYNC_SERVERS_MAX)
            .map(|i| server(&format!("s{i}"), "https://a.example:443", None))
            .collect();
        assert!(matches!(
            validate_sync_servers(&many),
            Err(WalletError::InvalidEndpoint { reason }) if reason.contains("SYNC_SERVERS_MAX")
        ));
        // A full list of distinct ids is fine.
        let full: Vec<SyncServer> = (0..SYNC_SERVERS_MAX)
            .map(|i| server(&format!("s{i}"), "https://a.example:443", None))
            .collect();
        validate_sync_servers(&full).expect("a full distinct list is accepted");
        // Unprintable / empty / oversize labels, and ids outside the charset.
        let id = SyncServerId::new("ok").expect("id");
        for bad in [
            "",
            "   ",
            "a\u{0007}b",
            &"x".repeat(SYNC_SERVER_LABEL_MAX_BYTES + 1),
        ] {
            assert!(
                SyncServer::new(id.clone(), bad, endpoint("https://a.example:443"), None).is_err(),
                "label {bad:?} must be refused"
            );
        }
        for bad in [
            "",
            "Upper",
            "with space",
            "dot.ted",
            &"a".repeat(SYNC_SERVER_ID_MAX_BYTES + 1),
        ] {
            assert!(
                SyncServerId::new(bad).is_err(),
                "id {bad:?} must be refused"
            );
        }
        SyncServerId::new("zec-rocks-2").expect("a slug is accepted");
    }

    #[test]
    fn sync_server_debug_never_prints_the_key() {
        // Gate 1: a config dump names the id, the label and the host, and says
        // `gated: true` — the key's bytes appear nowhere, nor its header. The
        // id is NOT the word "gated" (the crypto audit's LOW: it made the
        // positive assert vacuous).
        let s = server("entry-one", "https://g.example:443", Some(key()));
        let dbg = format!("{s:?}");
        assert!(dbg.contains("entry-one"), "{dbg}");
        assert!(dbg.contains("gated: true"), "the field, not the id: {dbg}");
        assert!(dbg.contains("https://g.example:443"), "{dbg}");
        assert!(!dbg.contains("the-key-value"), "the key leaked: {dbg}");
        assert!(!dbg.contains("x-zcash-rpc-key"), "the header leaked: {dbg}");
        let public = server("entry-two", "https://p.example:443", None);
        assert!(format!("{public:?}").contains("gated: false"));
    }

    #[test]
    fn a_predefined_row_stores_the_id_only() {
        // Spec D6 at the row: a `Predefined` choice structurally has no auth —
        // the id, the kind, the stamp, and NULL in every key column (those
        // hold only a USER's custom key, ADR-0568). The wallet-level
        // `the_row_never_stores_the_key` in wallet.rs is the falsifiable half:
        // a switch onto a host-keyed entry, every aux table scanned for the
        // key's bytes.
        let c = conn();
        let id = SyncServerId::new("example-gated").expect("id");
        write(&c, &SyncServerChoice::Predefined(id), 1_760_000_000).expect("write");
        let (kind, value): (String, String) = c
            .query_row(
                "SELECT kind, value FROM sync_server_choice WHERE singleton = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("row");
        assert_eq!(kind, "predefined");
        assert_eq!(value, "example-gated");
        assert_eq!(
            raw_key_columns(&c),
            (None, None, None),
            "a predefined row carries no key"
        );
    }

    #[test]
    fn a_custom_choice_with_a_key_round_trips_the_aux_row() {
        // ADR-0568: the user's key is stored with the URL it is bound to, and
        // reads back as the same `EndpointAuth` — which `resolve` then sends.
        let c = conn();
        let url = "https://mine.example:443";
        write(
            &c,
            &SyncServerChoice::Custom {
                endpoint: endpoint(url),
                key: Some(user_key()),
            },
            1,
        )
        .expect("write");
        assert_eq!(
            raw_key_columns(&c),
            (
                Some("x-api-key".to_owned()),
                Some("users-own-key-7c1e".to_owned()),
                Some(url.to_owned())
            ),
            "the key and the URL it is bound to"
        );
        let stored = read(&c);
        assert_eq!(
            stored,
            StoredChoice::Custom {
                url: url.to_owned(),
                key: Some(user_key())
            }
        );
        let r = resolve(
            stored,
            &[],
            &endpoint("https://default.example:443"),
            Some(&key()),
        );
        assert_eq!(r.effective.as_str(), url);
        assert_eq!(
            r.auth,
            Some(user_key()),
            "the user's key rides, not the default's"
        );
        assert_eq!(r.fallback, None);
    }

    #[test]
    fn leaving_a_keyed_custom_server_erases_its_key() {
        // ADR-0568: every write but a keyed custom one sets the key columns
        // NULL — a predefined choice, a keyless custom one (even the SAME
        // URL), and Default (which deletes the row).
        let keyed = || SyncServerChoice::Custom {
            endpoint: endpoint("https://mine.example:443"),
            key: Some(user_key()),
        };
        for leave in [
            SyncServerChoice::Predefined(SyncServerId::new("zec-rocks").expect("id")),
            custom("https://mine.example:443"),
            custom("https://other.example:443"),
        ] {
            let c = conn();
            write(&c, &keyed(), 1).expect("keyed");
            write(&c, &leave, 2).expect("leave");
            assert_eq!(
                raw_key_columns(&c),
                (None, None, None),
                "the key survived leaving for {leave:?}"
            );
        }
        let c = conn();
        write(&c, &keyed(), 1).expect("keyed");
        write(&c, &SyncServerChoice::Default, 2).expect("default");
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM sync_server_choice", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 0, "Default deletes the row, key and all");
    }

    #[test]
    fn a_malformed_key_row_reads_unreadable() {
        // ADR-0568: a row our writer never writes reads `ChoiceUnreadable` — never keyless, never lent.
        type Opt<'a> = Option<&'a str>;
        type Row<'a> = (&'a str, &'a str, Opt<'a>, Opt<'a>, Opt<'a>);
        let url = "https://mine.example:443";
        let over_value = "k".repeat(ENDPOINT_AUTH_VALUE_MAX_BYTES + 1);
        let over_header = "x".repeat(SYNC_SERVER_AUTH_HEADER_MAX_BYTES + 1);
        let cases: [Row<'_>; 7] = [
            (
                "a header only",
                "custom",
                Some("x-api-key"),
                None,
                Some(url),
            ),
            ("a value only", "custom", None, Some("v"), Some(url)),
            (
                "an oversized value",
                "custom",
                Some("x-api-key"),
                Some(&over_value),
                Some(url),
            ),
            (
                "an oversized header",
                "custom",
                Some(&over_header),
                Some("v"),
                Some(url),
            ),
            (
                "a refused header",
                "custom",
                Some("grpc-key"),
                Some("v"),
                Some(url),
            ),
            (
                "a key with no bound url",
                "custom",
                Some("x-api-key"),
                Some("v"),
                None,
            ),
            (
                "a pair on a predefined row",
                "predefined",
                Some("x-api-key"),
                Some("v"),
                None,
            ),
        ];
        let default = endpoint("https://default.example:443");
        for (what, kind, header, value, bound) in cases {
            let c = conn();
            let row_value = if kind == "predefined" {
                "zec-rocks"
            } else {
                url
            };
            c.execute(
                "INSERT INTO sync_server_choice
                     (singleton, kind, value, chosen_at_unix_secs, auth_header, auth_value, auth_url)
                 VALUES (1, ?1, ?2, 0, ?3, ?4, ?5)",
                rusqlite::params![kind, row_value, header, value, bound],
            )
            .expect("seed");
            assert_eq!(read(&c), StoredChoice::Unreadable, "{what}");
            let r = resolve(read(&c), &[], &default, Some(&key()));
            assert_eq!(r.effective, default, "{what}");
            assert_eq!(r.auth, Some(key()), "{what}: the default's own auth");
            assert_eq!(
                r.fallback,
                Some(SyncServerFallback::ChoiceUnreadable),
                "{what}"
            );
        }
        // A key column of another storage type (a BLOB) is unreadable through
        // the same single path.
        let c = conn();
        c.execute(
            "INSERT INTO sync_server_choice
                 (singleton, kind, value, chosen_at_unix_secs, auth_header, auth_value, auth_url)
             VALUES (1, 'custom', ?1, 0, 'x-api-key', X'6b6579', ?1)",
            [url],
        )
        .expect("seed");
        assert_eq!(read(&c), StoredChoice::Unreadable, "a BLOB value");
    }

    #[test]
    fn an_older_build_rewriting_the_url_never_lends_the_key() {
        // Security review M1: a build from before ADR-0568 upserts only the
        // four columns it knows, so it rewrites the URL and LEAVES the key.
        // The key is bound to the URL it was saved for, so the new build reads
        // the row as unreadable rather than sending the key to the new server.
        let c = conn();
        write(
            &c,
            &SyncServerChoice::Custom {
                endpoint: endpoint("https://mine.example:443"),
                key: Some(user_key()),
            },
            1,
        )
        .expect("keyed");
        // The older build's exact statement.
        c.execute(
            "INSERT INTO sync_server_choice (singleton, kind, value, chosen_at_unix_secs)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(singleton) DO UPDATE SET
                 kind = excluded.kind,
                 value = excluded.value,
                 chosen_at_unix_secs = excluded.chosen_at_unix_secs",
            rusqlite::params!["custom", "https://elsewhere.example:443", 2],
        )
        .expect("old-build upsert");
        assert_eq!(read(&c), StoredChoice::Unreadable);
        let r = resolve(
            read(&c),
            &[],
            &endpoint("https://default.example:443"),
            None,
        );
        assert!(
            r.auth.is_none(),
            "the user's key was lent to another server"
        );
        assert_eq!(r.fallback, Some(SyncServerFallback::ChoiceUnreadable));
        // The control: the same old-build statement onto the SAME URL keeps
        // the binding, so the key still reads (nothing was lent anywhere).
        let c = conn();
        write(
            &c,
            &SyncServerChoice::Custom {
                endpoint: endpoint("https://mine.example:443"),
                key: Some(user_key()),
            },
            1,
        )
        .expect("keyed");
        c.execute(
            "UPDATE sync_server_choice SET chosen_at_unix_secs = 2 WHERE singleton = 1",
            [],
        )
        .expect("touch");
        assert!(matches!(
            read(&c),
            StoredChoice::Custom { key: Some(_), .. }
        ));
    }

    #[test]
    fn an_existing_choice_table_gains_the_key_columns_on_open() {
        // A wallet from before ADR-0568: the four-column table holding a
        // custom row. `ensure_table` (every open) adds the three columns; the
        // row reads as before; a keyed write lands; and the upgraded table has
        // the SAME columns, in the same order and type, as a fresh one — the
        // rescan copy's `assert_columns_match` depends on it.
        let old = Connection::open_in_memory().expect("db");
        old.execute_batch(
            "CREATE TABLE sync_server_choice (
                 singleton           INTEGER PRIMARY KEY CHECK (singleton = 1),
                 kind                TEXT    NOT NULL CHECK (kind IN ('predefined', 'custom')),
                 value               TEXT    NOT NULL,
                 chosen_at_unix_secs INTEGER NOT NULL
             );",
        )
        .expect("old table");
        // The URL is a PARAMETER, not inside the multi-line SQL: the policy
        // scanner strips `//` comments line by line, and a `https://` on a
        // continuation line of a string reads to it as a comment.
        old.execute(
            "INSERT INTO sync_server_choice VALUES (1, 'custom', ?1, 0)",
            ["https://mine.example:443"],
        )
        .expect("old row");
        ensure_table(&old).expect("upgrade");
        ensure_table(&old).expect("idempotent");
        assert_eq!(read(&old), stored_custom("https://mine.example:443"));
        write(
            &old,
            &SyncServerChoice::Custom {
                endpoint: endpoint("https://mine.example:443"),
                key: Some(user_key()),
            },
            1,
        )
        .expect("keyed write");
        assert!(matches!(
            read(&old),
            StoredChoice::Custom { key: Some(_), .. }
        ));
        let columns = |c: &Connection| -> Vec<(String, String)> {
            let mut s = c
                .prepare(
                    "SELECT name, type FROM pragma_table_info('sync_server_choice') ORDER BY cid",
                )
                .expect("pragma");
            s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .expect("rows")
                .collect::<Result<_, _>>()
                .expect("collect")
        };
        assert_eq!(columns(&old), columns(&conn()), "upgraded ≠ fresh");
    }

    #[test]
    fn a_key_on_a_plaintext_custom_url_is_refused() {
        // Security review L4: any app on the device can bind a loopback port.
        let loopback = endpoint("http://127.0.0.1:9067");
        assert!(matches!(
            check_custom_key(&loopback, Some(&user_key())),
            Err(WalletError::InvalidEndpointAuth { .. })
        ));
        check_custom_key(&loopback, None).expect("a keyless loopback choice stays legal");
        check_custom_key(&endpoint("https://mine.example:443"), Some(&user_key()))
            .expect("a key over TLS is fine");
        // A stored row in that shape (never written by us) is unreadable.
        let r = resolve(
            StoredChoice::Custom {
                url: "http://127.0.0.1:9067".to_owned(),
                key: Some(user_key()),
            },
            &[],
            &endpoint("https://default.example:443"),
            None,
        );
        assert_eq!(r.fallback, Some(SyncServerFallback::ChoiceUnreadable));
        assert!(r.auth.is_none());
    }

    #[test]
    fn a_transport_refused_keyed_custom_never_lends_its_key_to_the_default() {
        // E12 with a key cannot arise (a key on plaintext is unreadable,
        // above); every OTHER fallback keeps the default's own auth. Pinned at
        // the not-offered arm, the one a keyed row could meet after a host
        // change: the default dials under `default_auth`, never the user's.
        let default = endpoint("https://default.example:443");
        let r = resolve(
            StoredChoice::Predefined("gone".to_owned()),
            &[],
            &default,
            Some(&key()),
        );
        assert_eq!(r.auth, Some(key()));
        let r = resolve(
            StoredChoice::Custom {
                url: "http://127.0.0.1:9067".to_owned(),
                key: Some(user_key()),
            },
            &[],
            &default,
            Some(&key()),
        );
        assert_eq!(r.effective, default);
        assert_eq!(
            r.auth,
            Some(key()),
            "the default's auth, never the user's key"
        );
    }

    #[test]
    fn a_stored_choice_debug_never_prints_the_key() {
        // Crypto audit H3: `StoredChoice` and `SyncServerChoice` derive
        // `Debug`; the key inside is an `EndpointAuth`, whose `Debug` redacts.
        let stored = StoredChoice::Custom {
            url: "https://mine.example:443".to_owned(),
            key: Some(user_key()),
        };
        let choice = SyncServerChoice::Custom {
            endpoint: endpoint("https://mine.example:443"),
            key: Some(user_key()),
        };
        for rendered in [format!("{stored:?}"), format!("{choice:?}")] {
            assert!(
                !rendered.contains("users-own-key-7c1e"),
                "leaked: {rendered}"
            );
            assert!(rendered.contains("redacted"), "{rendered}");
        }
    }

    #[test]
    fn sync_server_choice_row_round_trips_and_default_deletes_it() {
        let c = conn();
        assert_eq!(read(&c), StoredChoice::Absent);
        let id = SyncServerId::new("zec-rocks").expect("id");
        write(&c, &SyncServerChoice::Predefined(id), 1).expect("write");
        assert_eq!(read(&c), StoredChoice::Predefined("zec-rocks".to_owned()));
        // A later choice REPLACES (one row, latest wins).
        write(&c, &custom("https://mine.example:443"), 2).expect("write custom");
        assert_eq!(read(&c), stored_custom("https://mine.example:443"));
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM sync_server_choice", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 1);
        // Default deletes; a second Default is a no-op.
        write(&c, &SyncServerChoice::Default, 3).expect("default");
        assert_eq!(read(&c), StoredChoice::Absent);
        write(&c, &SyncServerChoice::Default, 4).expect("default again");
        assert_eq!(read(&c), StoredChoice::Absent);
        // ensure_table never clobbers a row.
        write(&c, &custom("https://x.example:443"), 5).expect("write");
        ensure_table(&c).expect("re-ensure");
        assert_eq!(read(&c), stored_custom("https://x.example:443"));
    }

    #[test]
    fn an_unreadable_choice_row_reads_as_fallback_and_never_fails_the_open() {
        // Three malformed shapes, one outcome: Unreadable — and `resolve`
        // then lands on the default with `ChoiceUnreadable`, never an error.
        let default = endpoint("https://default.example:443");
        // (1) an unknown kind — the CHECK constraint refuses the insert, so
        //     the shape reaches the reader only through a table without the
        //     constraint (an older or tampered schema); build that table.
        let c = Connection::open_in_memory().expect("db");
        c.execute_batch(
            "CREATE TABLE sync_server_choice (singleton INTEGER PRIMARY KEY, kind TEXT, value TEXT, chosen_at_unix_secs INTEGER);
             INSERT INTO sync_server_choice VALUES (1, 'weird', 'x', 0);",
        )
        .expect("seed");
        assert_eq!(read(&c), StoredChoice::Unreadable);
        // (2) a cell over the URL bound.
        let c = conn();
        c.execute(
            "INSERT INTO sync_server_choice (singleton, kind, value, chosen_at_unix_secs)
             VALUES (1, 'custom', ?1, 0)",
            [&"h".repeat(SYNC_SERVER_URL_MAX_BYTES + 1)],
        )
        .expect("seed");
        assert_eq!(read(&c), StoredChoice::Unreadable);
        // (3) a missing table (a read fault).
        let c = Connection::open_in_memory().expect("db");
        assert_eq!(read(&c), StoredChoice::Unreadable);
        // (4) a custom URL the validator refuses (userinfo) — readable as a
        //     row, unreadable as a choice.
        let r = resolve(
            stored_custom("https://user:pw@h.example:443"),
            &[],
            &default,
            None,
        );
        assert_eq!(r.effective, default);
        assert_eq!(r.fallback, Some(SyncServerFallback::ChoiceUnreadable));
        assert_eq!(r.choice, None);
        let r = resolve(StoredChoice::Unreadable, &[], &default, None);
        assert_eq!(r.effective, default);
        assert_eq!(r.fallback, Some(SyncServerFallback::ChoiceUnreadable));
    }

    #[test]
    fn resolve_honours_a_present_row_over_the_default() {
        // Spec D2 at the pure layer: the row wins; absence is the default by
        // absence (no fallback); the auth rides with the chosen entry.
        let default = endpoint("https://default.example:443");
        let default_auth = key();
        let offered = vec![
            server("zec-rocks", "https://zec.rocks:443", None),
            server(
                "example-gated",
                "https://lightwalletd.example.com:443",
                Some(key()),
            ),
        ];
        let r = resolve(
            StoredChoice::Absent,
            &offered,
            &default,
            Some(&default_auth),
        );
        assert_eq!(r.effective, default);
        assert!(
            r.auth.is_some(),
            "the default's auth rides with the default"
        );
        assert_eq!(r.choice, None);
        assert_eq!(r.fallback, None);

        let r = resolve(
            StoredChoice::Predefined("example-gated".to_owned()),
            &offered,
            &default,
            None,
        );
        assert_eq!(r.effective.as_str(), "https://lightwalletd.example.com:443");
        assert!(r.auth.is_some(), "the gated entry's auth rides with it");
        assert_eq!(
            r.choice,
            Some(SyncServerChoice::Predefined(
                SyncServerId::new("example-gated").expect("id")
            ))
        );
        assert_eq!(r.fallback, None);

        let r = resolve(
            stored_custom("https://mine.example:443"),
            &offered,
            &default,
            Some(&default_auth),
        );
        assert_eq!(r.effective.as_str(), "https://mine.example:443");
        assert!(
            r.auth.is_none(),
            "a custom server never inherits the default's key"
        );
        assert_eq!(r.choice, Some(custom("https://mine.example:443")));
        assert_eq!(r.fallback, None);
    }

    #[test]
    fn a_choice_the_host_no_longer_offers_falls_back_to_the_default_visibly() {
        let default = endpoint("https://default.example:443");
        let offered = vec![server("zec-rocks", "https://zec.rocks:443", None)];
        let r = resolve(
            StoredChoice::Predefined("gone-server".to_owned()),
            &offered,
            &default,
            None,
        );
        assert_eq!(r.effective, default);
        let gone = SyncServerId::new("gone-server").expect("id");
        assert_eq!(
            r.choice,
            Some(SyncServerChoice::Predefined(gone.clone())),
            "the choice is KEPT so the picker can name the server that is gone"
        );
        assert_eq!(
            r.fallback,
            Some(SyncServerFallback::ChoiceNotOffered { id: gone })
        );
        // An EMPTY offered list with a predefined row: same arm.
        let r = resolve(
            StoredChoice::Predefined("zec-rocks".to_owned()),
            &[],
            &default,
            None,
        );
        assert!(matches!(
            r.fallback,
            Some(SyncServerFallback::ChoiceNotOffered { .. })
        ));
    }

    // Gate 7 asks for exactly this: an assertion on a constant, so that
    // moving the constant past its boundary reds a NAMED test. The lint
    // objects that the condition is constant; that is the point.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn sync_server_constants_are_named_and_bounded() {
        // Gate 7: the six constants at their boundaries.
        assert!(
            SYNC_SERVERS_MAX >= 2,
            "the reference catalog plus at least one host server"
        );
        assert!(SYNC_SERVERS_MAX <= 16, "a picker is a short list");
        assert!("example-gated-mainnet-fallback".len() <= SYNC_SERVER_ID_MAX_BYTES);
        // ADR-0568: real gating header names fit, and the bound caps the cell.
        assert!("x-zcash-rpc-key".len() <= SYNC_SERVER_AUTH_HEADER_MAX_BYTES);
        assert!(SYNC_SERVER_AUTH_HEADER_MAX_BYTES <= 128);
        assert!("zec.rocks (testnet)".len() <= SYNC_SERVER_LABEL_MAX_BYTES);
        assert!(SYNC_SERVER_URL_MAX_BYTES >= 253 + "https://".len() + ":65535".len());
        assert!(
            crate::constants::SYNC_SERVER_PROBE_TIMEOUT_SECS >= 2,
            "above the measured cold connect (≤ 0.35 s) by an order of magnitude"
        );
        assert!(
            crate::constants::SYNC_SERVER_PROBE_TIMEOUT_SECS <= 30,
            "below the point a user gives up on a spinner"
        );
    }
}
