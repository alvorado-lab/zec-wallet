# Spec — the sync-server picker (SRV-1a; phase-3 P3-13)

**Status:** Shipped — 2026-09-14 (S276: drafted; commit 1 of phase-3 §6 —
the core types, catalog, door, aux row and the resolution at open — commit
2/3 — the probe, the switch, the bridge and its regen — the UI and the review
fold landed the same day; S277: the device walk on the Pixel and the two
gate-3 rows, each with a watched mutant — §8 carries both; phase-3 §6
carries the walk's record and its two UX observations, unranked).
**Amended 2026-10-01 by ADR-0568** (maintainer S314/S315): the SDK ships no
gated server and no key; a host offers its own servers, keyed or not, through
`sync_servers`; a user's custom server may carry a key, which the SDK stores.
The sections that changed say so inline (D1, D4, D5, D6, §1.4, §2, §3.3, §3.4, §4, §5,
§6, §7, §8).
Written from phase-1 §4s's four answered questions and the founder's S274
course correction 2 (the picker half ships for 0.0.1; automatic failover is
its own later row).
**Implements:** the founder's 2026-09-12 rulings in
`docs/plan/production-readiness-phase-1.md` §4s (ruling 1: a custom endpoint is
SMALL — validate that it answers, tell the user they are trusting it; ruling 2:
the gated server's key SHIPS in the app — superseded by ADR-0568, the host's
build carries it; ruling 3, failover, is OUT of this spec
by the S274 correction) · Development Principle 5 (metadata: every server-bound
field answers "does the relay need this?") · Principle 6 (honest degradation:
the "switch servers" copy names an action the UI now offers) · Principle 7
(every byte from the network is hostile — a user-typed URL is network input)
· Architecture invariant 1 (Rust owns all state — the choice lives in the aux
DB, not in a Dart preference).
**Relates to:** `wallet-sdk.md` §2.3 (config — **this spec adds one field**,
`sync_servers`, see §0), §3.1 (`open` — the persisted choice now wins over the
config endpoint), §3.3 (the Dart surface — four handle methods and one free
function), §5.4 (NEVER-log — the key and the custom host), §6.3 (the
consuming-handle session-swap contract, the `rescan_from` shape this reuses);
`ironwood-nu63-support.md` §6.2 (the consensus verdict and the grace anchor
are PER WALLET — a switch does not touch them); ADR-0534 (the rescan preserves
the aux tables — the choice row rides that list); ADR-0013 (the host is told in
types what the SDK does).
**Does NOT own:** automatic failover between offered servers
(§4s ruling 3 — its precondition is measured in §1.4 and it is a later row);
Tor per server; a third predefined server; any relay of the key; the Ironwood
completing-height drift §1.4 records (phase-1 §4b owed row 7's oracle owns it).

---

## 0. Consistency audit (operating principle #4)

Read against `wallet-sdk.md`, `ironwood-nu63-support.md`, the ADRs, phase-1
§4s and the copy in `wallet_en.arb`. One source of truth per predicate; the
sentences below are the ones this spec changes or retires.

| # | Where | What it says today | What this spec does |
|---|---|---|---|
| A1 | `wallet-sdk.md` §2.3 `WalletConfig` | `endpoint` is ONE required string; no list, no choice | Adds `sync_servers: Vec<SyncServer>` (the OFFERED list, may be empty). `endpoint` stays REQUIRED and becomes the DEFAULT — what a wallet with no choice uses, and what a choice that can no longer be honoured falls back to. §2.3 gets a two-line patch pointing here; the field list there is otherwise unchanged |
| A2 | `wallet-sdk.md` §3.1 `open` | "`NetworkMismatch` if `cfg.network` disagrees with the stored manifest" — the endpoint is whatever the config says | A persisted choice WINS over `cfg.endpoint` at open (§1.2 D2). §3.1 gets one sentence and a pointer |
| A3 | `wallet_en.arb` `@walletStallEndpoint` description | "the server is not user-switchable yet, so it must not promise a switch" | The premise is retired. The COPY does not change (a refused dial still cannot say whose fault it is); the description's parenthetical is rewritten to name the picker and keep the hedge for its real reason |
| A4 | phase-1 §4s "Out until contracted: custom URLs typed by the user" | written before ruling 1 | Superseded by ruling 1 in the same section; this spec is the contract. No edit to §4s (it is the record) |
| A5 | the project's private operations notes (not in the repository) | "`LightServerEndpoint` is a single host-supplied URL" | Stale once this ships; the notes are the founder's, noted in the start-here, not edited by this spec |
| A6 | `sync_status_sheet.dart` / `wallet_providers.dart` `walletEndpointHostProvider` | the Server row is wired from the host's `WalletConfig.endpointUrl` "so the display and the actual connection can never drift" | The no-drift RULE stands; its SOURCE moves: the row reads the SESSION's `syncServerStatus().effective` (the connection's own truth, in Rust). The config-derived override stays only as the pre-session value |
| A7 | `sync_controller.rs` streak doc: "an endpoint gets a fresh streak budget per LAUNCH" | one endpoint per wallet life | A switch builds a NEW controller, so a new server starts at zero — the phase-3 P3-13 row's "reset the streak per endpoint". The doc gains "or per switch" |
| A8 | `wallet.rs` `hash_convention_reversed` doc: "a `controller_over`/`injected` over a live `Inner` — that site must clear this latch" | names the test seam as the only site | The switch is the SHIPPED site; it clears the latch by construction (a new `Inner` is born `false`). The doc names the switch |

Nothing else in the corpus carries a second copy of these predicates.

---

## 1. Design decisions

### 1.1 The problem

The wallet talks to exactly one lightwalletd, named by the host at open.
Every honest next-step string the SDK ships for a sick server —
`walletStallEndpointMisbehaving`, `walletSyncExplainUpToDateDegraded`,
`walletSyncEndpointBehind`, `walletStallBirthdayInFuture`, the GRACE-1 clock and
blocks endings — tells the user to **switch servers**, and no surface offers it.
The founder's ask (S265, with the Seeker's sync sheet in hand): pick the server
from a predefined list, or enter your own; the SDK exposes the list and the
current choice; the UI offers the switch. And Phase 4's exit — a clean-room
install that sends a transaction — is not a proof against one hardcoded server
that may be down that hour.

### 1.2 Decisions, each with the alternatives it beat

**D1 — Who owns what.** The SDK owns the SHAPE (`SyncServer`,
`SyncServerChoice`, `SyncServerStatus`), the VALIDATION (every URL through
`LightServerEndpoint::new`, every list through one door), the SWITCH and the
PERSISTENCE. The host supplies the OFFERED list — its contents — and each
gated entry's key. The user supplies a custom URL, and optionally a key for it
(ADR-0568). The SDK also EXPORTS the reference catalog
(`SyncServerCatalog::reference`) — public servers only since ADR-0568 — which a
host extends with servers of its own.
*Rejected:* (a) a constant table inside the SDK with the key baked in — the
SDK publishes to pub.dev, and a key in package source is public on pub.dev,
which is worse than "extractable from an APK" (ruling 2 accepted the latter,
not the former); (b) the host owns everything and re-provisions with a new
`WalletConfig` — every host would rebuild the persistence and the "which
server am I really on" truth, and invariant 1 says that truth is Rust's.

**D2 — The persisted choice wins at open.** `WalletConfig.endpoint` is the
DEFAULT. Once the user has chosen, the aux row decides which server the wallet
opens against; the config endpoint is what a wallet with no row uses, and what
a row that cannot be honoured (the host stopped offering that id, or the row
is unreadable) falls back to — VISIBLY, through `SyncServerStatus.fallback`,
never silently.
*Rejected:* the config always wins and the picker is session-only — a choice
that dies with the process is the "switch servers" remedy failing on the next
launch, exactly when the user needs it.

**D3 — A switch is a consuming session swap, the `rescan_from` shape.** Stop
and join the loop, quiesce to sole ownership of `Inner`, write the one aux row,
rebuild `Inner` over the SAME database with the new endpoint and auth, return
a new `Wallet`; the bridge replaces the handle's inner in place (as
`rescan_from` does), and the UI swaps the session (as `rescanActiveWallet`
does) so every live stream re-subscribes. No disk change but the row. No
rescan.
*Rejected:* (a) make `Inner.endpoint` mutable under a lock — it is documented
lock-free-immutable, the engine builds its client from it per pass, and the
`SyncController`'s status channel and rewinding streak would stay bound to the
old server; (b) close and re-open with a new config — D1's second rejection.

What a fresh `Inner` buys, deliberately: a NEW `SyncController` (the
rewinding streak is a judgement about A server; a new server starts at zero —
the P3-13 row's "reset the streak per endpoint"), `hash_convention_reversed`
born `false` (§4s consequence 3 — the latch reset — for free), a fresh dialer
per pass. What the switch CARRIES across, because a host that does not swap
sessions must see no discontinuity: `seq` (the monotonic stream stamp — a
restart to 0 would let a stale event beat a fresh snapshot), the
`ProposalRegistry` (its counter starts at 1 per registry; a restart could
alias a Dart-held id onto a fresh token — a money bug; the live tokens are
anchored on the wallet's own DB, which the switch does not touch), the
`incoming_tx` watch sender (subscribers stay attached), `incoming_fault_floor`,
`unparseable_txids`, `ephemeral_backoff`, `reclaim_in_flight`, `tor_posture`
(the fell-back latch AND ADR-0552's patience clock in one object — carry the
POSTURE across a switch, never a copied bool, or every server switch silently
re-arms a fresh minute of insistence)
(a per-wallet Tor fact), `dial_counters` (FR-37 — the counts run "since the
wallet opened", and a switch is not a reopen), the `tor_state_tx` watch sender
(stage S1 `truth`, the `incoming_tx` reason: a host renders its transport chip
from `watchTorState`, and a stream that ended here would leave that chip on
the OLD session's word while the new session fell back — the assembler
re-publishes the rebuilt session's truth into it), `created_at`,
`freshly_generated`, `seed`,
`seed_port`, `clock`, `db`, `aux_db`, `cache`, `lock`. NOT carried: `sync`,
`hash_convention_reversed`, and the swap `OnceLock` (re-initialised, the
rescan precedent — the reference host's swap activation re-fires on the
session swap; a direct-handle host calls `enableNearSwap` again, which its
doc will say).

**D4 — What a switch leaves alone (Q-S3).** The consensus verdict and the
grace anchor (`consensus_verdict`: per wallet; INC-006's guard judges at the
highest ATTESTED height, so a lower-tip server cannot lower it), the sync
stamp, the ever-synced flag, the creation stamp, the recorded-heights ledger
(`root_bind`), the block cache, the scan queue, the outbox. A switch to a
server BEHIND the wallet's own scanned height reads `EndpointBehind` on its
first pass (T0-1c-R2's floor reads the wallet's own height) — honest, and the
picker is now the remedy that copy names. A server whose completing heights
differ from the ledger is `EndpointMisbehaving` on the pools it differs on —
BIND-1-R's rules, unchanged. The two servers §1.4 measured — `zec.rocks` and
the maintainer's gated server, which since ADR-0568 a host offers, not the
catalog — were identical on every pool, so switching between them could not
trip the bind then. The SDK makes no such claim about other offered servers
or a custom one: the bind judges each.

**D5 — The custom endpoint is SMALL (ruling 1).** It passes the SAME
money-path floor as a predefined entry — TLS via `LightServerEndpoint` (http
only to loopback), chain identity via the one existing predicate
`endpoint_network_matches` BEFORE anything is believed, `NetworkMismatch`
refuses — plus ONE reachability probe (`GetLightdInfo`) before the switch,
and copy that says plainly the user is trusting that server. No Tor per
server, nothing else. *Amended by ADR-0568:* the user MAY give a key for it — a
header name and a value, through the same `EndpointAuth` door as a host's key;
the probe and every request to that server carry it. A key on a plaintext
(loopback `http://`) custom URL is refused: any app on the device can bind a
loopback port.

**D6 — A gated server's key is the HOST's, never the SDK's (ADR-0568,
superseding ruling 2's built-in key).** The SDK ships no gated server and no
key. A host that offers one puts it in `sync_servers` with its key, which it
supplies from its own build (a `--dart-define`, the swap JWT precedent); with
no key it leaves the entry out, so the list is honest about what works.
*History:* from 2026-09-18 to 2026-10-01 the reference UI carried a built-in
key for the maintainer's gated server ("just insert builtin key"); ADR-0568
removed it before publication, because a key in a published package is a key
for anyone.
**The residual, unchanged:** a revoked or rotated key surfaces as
`Stalled { EndpointUnreachable }` ("check your connection") rather than "this
server refused you", because `transport_err` maps every server-sent status to
the unreachable stall.
A `SyncServer` naming an auth header without a value is refused at the config
door, never offered unauthenticated (the S259 `EndpointAuth` rule — "supplying
one without the other is a typed refusal" — applied to the list). The SDK
NEVER persists a HOST's key: the row stores the entry's id, and the host
re-supplies the key at every open. A USER's custom key is the one key the SDK
stores (ADR-0568: only the user knows it). It lives in the choice row,
SQLCipher-sealed and bound to the URL it was saved for. It is erased when the
user picks anything else, and dies at wipe.

**D7 — No automatic failover.** Founder S274. Ruling 3's premise is recorded
in §4s, its precondition is measured in §1.4, and it is its own later row —
it must construct a new `SyncController` per endpoint, which D3 already does
for a user switch, so the picker is the failover's building block, not a
detour from it.

**D8 — Persistence.** One singleton row in the aux DB, `sync_server_choice`,
beside the other per-wallet settings; rides a rescan UNCLEARED (the choice is
about the user's trust, not the chain view — the `consensus_stamp` precedent,
not `sync_stamp`'s); dies with the wallet at wipe.

**D9 — A switch requires a successful probe, predefined or custom.** One
rule. Offline, the switch is refused with the unreachable copy — honest,
and the sync loop keeps retrying the current server on its own. *Rejected:*
"switch anyway" for predefined entries — it would let a user leave a working
server for one the wallet cannot reach, and the stall that follows would read
as the new server's fault.

### 1.3 Trade-offs

A switch costs the in-flight pass (cancelled at its next checkpoint, at most
one batch — the `QUIESCE_MAX` band documented on `rescan_from`), forgives the
rewinding streak for the NEW server, and re-initialises the swap service. The
reference host already session-swaps for a rescan; a third-party host using
the handle directly gets the carried counters and only needs to re-subscribe
its status stream (its doc says so). A user whose chosen id disappears from a
later host release falls back to the default with a banner, not silently.

### 1.4 The measurement this design rests on

Founder S269: probe both servers' completing heights before designing
anything that points the wallet at servers we do not run. Done, twice:

- **2026-09-10** (`docs/handoff/devops-lightwalletd-request.md` row 2;
  captures under `docs/plan/probes/ironwood-subtree-roots-probe.*`):
  Sapling 1128 / Orchard 769 / Ironwood 4 roots, root hashes and completing
  block hashes byte-identical between `zec.rocks` and the maintainer's own
  gated lightwalletd (no longer named here: ADR-0568). Ironwood completing
  heights then: 3451206, 3463000, 3467168, 3475149.
- **2026-09-14** (this session; script and raw output under
  `docs/plan/probes/lightwalletd-completing-heights.*`, `grpcurl` over
  `zec.rocks`'s own reflection protoset): both at tip 3,482,905,
  `chainName=main`, branch `37a5165b`;
  Sapling 1128 / Orchard 769 / Ironwood 4 roots; the completing heights and
  the SHA-256 of every pool's concatenated root+completing hashes IDENTICAL
  between the two servers. Ironwood completing heights today:
  **3451206, 3468125, 3471053, 3475149.**

**The finding beside the answer:** Ironwood indexes 1 and 2 MOVED between the
two dates — by 5,125 and 3,885 blocks, upward — on BOTH servers alike. That
is the class the RED-BY-RULING plant `a_rewind_does_not_license_a_move_no_reorg_could_produce`
describes and phase-1 §4b owed row 7 owns (an oracle above the newest bundled
row). It is not this spec's to fix. It is recorded here so nobody reads
"identical" as "stable": the two servers agree with EACH OTHER at every
sample, which is all the picker needs — a switch between them shows the bind
the same numbers.

---

## 2. Domain types (Rust core; `sdk/zec-wallet-core/src/sync_server.rs`, new)

```rust
/// A server the host OFFERS. Validated at the config door; immutable after.
#[derive(Clone)]
pub struct SyncServer {
    /// Stable, host-chosen id — what the aux row stores. `[a-z0-9-]`,
    /// 1..=SYNC_SERVER_ID_MAX_BYTES. Never shown; the label is.
    pub id: SyncServerId,
    /// Display name. Printable, 1..=SYNC_SERVER_LABEL_MAX_BYTES.
    pub label: String,
    /// The same validator as `WalletConfig.endpoint` — https, or http to
    /// loopback only; no userinfo; no path.
    pub endpoint: LightServerEndpoint,
    /// A gated entry's key (the HOST's — ADR-0568). Its TYPE is the §5.4
    /// guard: `Debug` prints `<redacted>`, the wire marks it sensitive. `None`
    /// for every public server.
    pub auth: Option<EndpointAuth>,
}
// `Debug` is hand-written to print id, label and host — never `auth`.

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyncServerId(String);

/// What the user asked for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SyncServerChoice {
    /// One of the offered entries, by id.
    Predefined(SyncServerId),
    /// A URL the user typed, with the key the user gave for it, if any
    /// (ADR-0568). Already validated (the newtypes are the proof); a key on a
    /// plaintext URL is refused at the door (`InvalidEndpointAuth`).
    Custom { endpoint: LightServerEndpoint, key: Option<EndpointAuth> },
    /// Clear the choice: back to `WalletConfig.endpoint`.
    Default,
}

/// What the wallet is actually talking to, and why.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyncServerStatus {
    /// The endpoint every client this wallet builds dials right now.
    pub effective: LightServerEndpoint,
    /// `WalletConfig.endpoint` — shown as "App default" when it is not among
    /// the offered entries.
    pub default: LightServerEndpoint,
    /// The persisted choice. `None` = no row = the default is in use by
    /// absence, not by fallback.
    pub choice: Option<SyncServerChoice>,
    /// `Some` = the row exists but could not be honoured; `effective` is then
    /// `default`. Never silent (Principle 10).
    pub fallback: Option<SyncServerFallback>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SyncServerFallback {
    /// The row names an id this host no longer offers.
    ChoiceNotOffered { id: SyncServerId },
    /// The row is present but malformed (a truncated cell, an unknown kind,
    /// a URL the validator refuses). The wallet is usable; the choice is not.
    ChoiceUnreadable,
}

/// The probe's answer — a fact about REACHABILITY and IDENTITY, never a
/// verdict about honesty (the sync guards judge that, pass by pass).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyncServerProbe {
    /// The server's claimed tip. UNTRUSTED; a bare height, loggable (§5.4).
    pub tip: BlockHeight,
}
```

Config: `WalletConfig.sync_servers: Vec<SyncServer>` — the OFFERED list; may
be empty (then the picker offers the default and a custom entry). The door
(`WalletConfig` validation, both the core `open`/`create` paths and the
bridge's `validate_wallet_config`) refuses: more than `SYNC_SERVERS_MAX`
entries; duplicate ids; an id or label outside its bound or charset; an
`auth` whose header the transport cannot send (`EndpointAuth::new` already
does). Refusal is the existing `WalletError::InvalidEndpoint { reason }`
(`RW-CFG-001`) — ONE config-door variant for every endpoint-shaped refusal,
the reason string discriminating. (Amended S276 at the build, from a planned
`InvalidSyncServers`: a new core variant needs the bridge's regenerated
encoder in the same commit, and the list door ships a commit before the
bridge does; one door variant is also the smaller API.)

Catalog: `SyncServerCatalog::reference(network) -> Result<Vec<SyncServer>,
WalletError>` — PUBLIC servers only (ADR-0568; until 2026-10-01 it also named
the maintainer's gated server, keyed by the caller):

| network | entries |
|---|---|
| Main | `zec-rocks` — "zec.rocks" — `https://zec.rocks:443`, no auth |
| Test | `zec-rocks-testnet` — "zec.rocks (testnet)" — `https://testnet.zec.rocks:443`, no auth |

A host that offers servers of its own — gated or not — appends them:
`[...referenceSyncServers(network: n), SyncServer(id:, label:, url:,
authHeader:, authValue:)]`. One list, one door (ADR-0568 rejected a second
`extraSyncServers` field).

The catalog's URLs are the ONE place the reference servers are named in Rust;
`wallet_config.dart`'s `referenceMainnetEndpoint`/`referenceTestnetEndpoint`
are retired in favour of reading the catalog (one source of truth).

Persistence (`sync_server_choice`, aux DB, `ensure_table` in
`db::ensure_aux_tables`, name added to `AUX_TABLES_PRESERVED`):

```sql
CREATE TABLE IF NOT EXISTS sync_server_choice (
    singleton          INTEGER PRIMARY KEY CHECK (singleton = 1),
    kind               TEXT    NOT NULL CHECK (kind IN ('predefined', 'custom')),
    value              TEXT    NOT NULL,   -- the id, or the URL
    chosen_at_unix_secs INTEGER NOT NULL   -- observability only, never a money input
);
-- ADR-0568: added by an idempotent ALTER TABLE … ADD COLUMN (guarded by
-- pragma_table_info) on every open — never in the CREATE, so a new and an
-- upgraded database have one column order (the rescan copy's signature check).
ALTER TABLE sync_server_choice ADD COLUMN auth_header TEXT; -- a USER's custom key: header
ALTER TABLE sync_server_choice ADD COLUMN auth_value  TEXT; -- … value
ALTER TABLE sync_server_choice ADD COLUMN auth_url    TEXT; -- the URL the key was saved for
```

`Default` DELETEs the row. A `custom` choice with a key writes all three key
columns; every other write sets them NULL, so leaving a keyed server erases its
key (the aux connection runs `PRAGMA secure_delete = ON`). The read is bounded
(`value` longer than `SYNC_SERVER_URL_MAX_BYTES`, a header longer than
`SYNC_SERVER_AUTH_HEADER_MAX_BYTES`, a value longer than
`ENDPOINT_AUTH_VALUE_MAX_BYTES`) and never fails the open. These read as
`ChoiceUnreadable`, logged at `warn`, with the default in use under the
DEFAULT's auth:
- a half key;
- a key on a `predefined` row;
- a key the door refuses;
- an `auth_url` that is not `value` (an older build rewrote the URL and left
  the key; the key is never lent to another server).

A HOST's key is never written: the row stores the entry's id, and the host
re-supplies the key at every open.

Constants (`constants.rs`, each with its why in the source):

| name | value | why this value |
|---|---|---|
| `SYNC_SERVERS_MAX` | 8 | a picker is a short list; eight is more than any host has asked for and bounds the config's allocation from a hostile-looking host DTO |
| `SYNC_SERVER_ID_MAX_BYTES` | 32 | ids are slugs; 32 holds `example-gated-mainnet-fallback` with room, and bounds the aux cell |
| `SYNC_SERVER_AUTH_HEADER_MAX_BYTES` | 64 | ADR-0568: a header name a user types; real gating headers are under 32 bytes, and the bound caps the aux cell and the bridge input before the validator sees it |
| `SYNC_SERVER_LABEL_MAX_BYTES` | 64 | a label fits one row on a 320 dp screen at 2× text scale below this |
| `SYNC_SERVER_URL_MAX_BYTES` | 2048 | the classic browser URL bound; a hostname is ≤ 253 bytes, so anything near this is not a server — and it caps what a text field can hand the validator |
| `SYNC_SERVER_PROBE_TIMEOUT_SECS` | 15 | above the measured cold TLS+h2 connect of both servers (≤ 0.35 s) by two orders, below the point a user gives up on a spinner; distinct from the sync client's own unary timeout, which is a scan budget |

Lifecycle: `LifecyclePhase::SwitchingServer` (new) — the transient phase a
concurrent `snapshot`/`propose` observes as `WalletBusy { SwitchingServer }`;
mirrored on the bridge `LifecyclePhase`. A NEW variant on a `#[non_exhaustive]`
enum: the bridge mirror's `bridge_enums_cover_core_variants` row turns red
until it is added.

Errors (`WalletError`, mirrored on `WalletErrorKind`): the existing
`InvalidEndpoint { reason }` (the list door, see above), `SyncServerNotOffered`
(a `Predefined` id the current config does not carry), `SyncServerUnreachable`
(the probe could not dial or timed out — the transport class; distinct from
the sync loop's `StallReason`, which is a status, not an error), and the
existing `NetworkMismatch` (the probe's identity check failed — the same
variant `open` uses for a manifest disagreement, because it is the same fact:
this server is not this wallet's network). ADR-0568 adds
`InvalidEndpointAuth { reason }`: every `EndpointAuth::new` refusal, a host's or
a user's, so a picker can tell a bad key from a bad URL by KIND, never by text.
It covers a header over the bound, any `grpc-`-prefixed header, and a key on a
plaintext custom URL. Codes: `RW-SRV-001..002`. The two
NEW variants and the new `LifecyclePhase` land in the SAME commit as the
bridge mirror and its regen (the generated encoder matches every variant
exhaustively, so a core variant without its regen does not compile the
bridge).

---

## 3. Interface design

### 3.1 Inbound — the core (`Wallet`)

```rust
/// The offered list, exactly as the config carried it (validated).
pub fn sync_servers(&self) -> &[SyncServer];

/// Which server the wallet dials, which choice produced it, and whether a
/// fallback is in force. Lock-free (immutable after open/switch).
pub fn sync_server_status(&self) -> SyncServerStatus;

/// Dial `choice` with THIS wallet's TorPolicy (a fresh isolation key — the
/// probe circuit is not the sync circuit) and ask `GetLightdInfo` under
/// SYNC_SERVER_PROBE_TIMEOUT_SECS. Refuses typed: `SyncServerNotOffered`,
/// `SyncServerUnreachable`, `NetworkMismatch` (via `endpoint_network_matches`,
/// the ONE predicate). Never switches. `Default` probes the config endpoint.
pub async fn probe_sync_server(&self, choice: &SyncServerChoice)
    -> Result<SyncServerProbe, WalletError>;

/// Probe, then switch. CONSUMES the handle (the `rescan_from` contract): on
/// success the returned `Wallet` REPLACES it (call `start_sync`); on a probe
/// refusal NOTHING changed and the handle is returned inside the error's
/// carrier — see §6 — so the host keeps its wallet; on a fault past the
/// stop-join the handle is gone and the host re-opens (the row was written
/// before the rebuild, so the re-open honours the choice).
pub async fn switch_sync_server(self, choice: SyncServerChoice)
    -> Result<Wallet, SwitchRefused>;
```

`SwitchRefused` is `{ wallet: Option<Wallet>, error: WalletError }` — `Some`
on every refusal BEFORE the stop-join (the probe, an id not offered, a phase
that is not `Open`), `None` past it. The bridge folds this back into
"replace inner on success / leave the old inner on a pre-stop refusal / handle
closed on a post-stop fault", which is what the Dart caller needs and what
`rescan_from` could not offer (its refusals also came after teardown). The
order inside: (1) gate `require_open`; (2) resolve the choice against
`sync_servers()` (or the default) — `SyncServerNotOffered`; (3) probe —
`SyncServerUnreachable` / `NetworkMismatch`; (4) `sync.stop().await`;
(5) `lifecycle.transition(SwitchingServer)`; (6) quiesce to sole ownership
(`QUIESCE_MAX`, `WalletBusy { SwitchingServer }` past it); (7) write or
delete the row on the solely-owned aux connection; (8) rebuild `Inner`
through `from_open`'s assembler with the carried fields of D3 and the new
`endpoint`/`endpoint_auth`; (9) return. No in-flight-send fence: the data DB
is untouched, so the witness-inversion class `rescan_from` fences against
cannot arise (a named test says so).

At `open` (and `create`'s reopen paths): after `store::open`, read the row;
resolve it against `cfg.sync_servers` and `cfg.endpoint`; the resolved
endpoint and auth go onto `OpenConfig` — `Inner.endpoint` is the EFFECTIVE
server from the first pass. `cfg.endpoint`/`cfg.endpoint_auth` are retained
on `Inner` as `default_endpoint`/`default_auth` for `Default` and for
`SyncServerStatus.default`.

### 3.2 Outbound — SDK-internal

No new port. The probe rides `LightwalletdClient::connect` (the same
`TorPolicy`/`NetDialer` resolution as every client — under `Required` with an
unreachable runtime it dials NOTHING, the existing `stall_on_failure` plan)
and `ChainOracle::server_identity` (the same `GetLightdInfo` mapping
`provision` uses). The row rides the aux SQLCipher connection like
`ever_synced`/`consensus_stamp`. Would this work with a different transport
or driver? Yes — the probe never names tonic or the dialer; it names the
policy.

### 3.3 The bridge (`sdk/zec_wallet/rust/src/api/`)

`config.rs`: `WalletConfig.sync_servers: Vec<SyncServer>` with the DTO
`SyncServer { id: String, label: String, url: String, auth_header:
Option<String>, auth_value: Option<String> }` — `auth_value` carries the §5.4
framing `endpoint_auth_value` already has (Dart cannot zeroize; prefer a
dart-define; one key for every install). Free function
`reference_sync_servers(network: Network) -> Result<Vec<SyncServer>,
WalletApiError>` (ADR-0568 removed its `gated_key`).
`wallet.rs`: `sync_servers()`, `sync_server_status() -> SyncServerStatus`,
`probe_sync_server(choice) -> SyncServerProbe`, `switch_sync_server(&mut
self, choice)` (in-place replace; a pre-stop refusal leaves `inner` as it
was; a post-stop fault leaves the handle closed — the `rescan_from` doc's
recover-by-reopen story, narrowed). Mirrors: `SyncServerChoice`
(`predefined(id)` / `custom(url, key)` / `default`). The custom key is a PLAIN
struct, `SyncServerKey { header: String, value: String }` (ADR-0568). FRB
generates no `toString` for a plain struct, unlike a freezed enum variant, so
`'$choice'` can never print the value. On every value the SDK RETURNS (a
status), `key.value` is the empty string: the header names the saved key, and
the value never leaves Rust. Other mirrors: `SyncServerStatus`
(`effectiveUrl`, `defaultUrl`, `choice`, `fallback`), `SyncServerFallback`,
`SyncServerProbe { tip: u32 }`, the new `LifecyclePhase` and
`WalletErrorKind` variants. `bridge_enums_cover_core_variants` and
`public_enums_non_exhaustive` police the mirrors; a new bridge row pins that
`SyncServer`'s Dart-visible `Debug`/`toString` never carries `auth_value`.
The bindings are regenerated with `just wallet-bridge-gen` (14 min, ALONE).

### 3.4 The UI (`sdk/zec_wallet_ui`)

- `WalletSession` (the port) gains `syncServers()`, `syncServerStatus()`,
  `probeSyncServer(SyncServerChoice)`; `FakeWalletSession` implements them
  with an injectable probe outcome.
- `WalletProvisioner` gains `switchSyncServer(SyncServerChoice) ->
  Future<WalletSession>`: a session swap over the same handle, the
  `rescanFrom` shape. `OnboardingController.switchSyncServer` mirrors
  `rescanActiveWallet` — same `identityEpoch` (the same wallet's life), kind
  carried, recover-by-reopen on a post-stop fault, single-flighted with
  rescan and delete (one `_mutationInFlight` latch, or the three existing
  ones checked together — the implementer records which).
- `walletEndpointHostProvider` is re-derived: a `walletSyncServerStatusProvider`
  (a `FutureProvider` keyed on the session, so a swap refreshes it) supplies
  the host of `effective`; the config-derived override stays as the value
  before a session exists. The no-drift rule (S151) is kept by reading the
  connection's own truth.
- The sync sheet's Server row becomes a tappable row (44 × 44 dp target,
  chevron, semantics "Server, zec.rocks, button") opening `SyncServerSheet`.
- `SyncServerSheet` (new, `features/wallet/sync_server_sheet.dart`): the
  offered entries as radio rows (label + host; the current one marked "In
  use"); "App default" as a row when the default is not among them; "Custom
  server…" expanding a URL field (`https://host:port` hint, inline validation
  through `validateWalletConfig`'s endpoint rule — the SDK's, not a Dart
  regex), a "Check server" action, then "Use this server". *ADR-0568:* under the
  URL, "Access key (optional)" and "Key header". The key field is SHOWN by
  default with a Hide toggle (*S15, dated correction:* an obscured field is
  secure entry to iOS, which put its Passwords bar over it on the iPhone walk;
  the key is a bearer access handle, shown only while typed and never filled
  back from the store, accepted on the maintainer's S315 ruling that it is not a
  secret — screenshots and shoulder-surfing see it while typed). Its context
  menu offers Paste only, so a typed key is never copied to the clipboard.
  Whichever way it is toggled it has no suggestions, no autocorrect, no IME
  learning and no autofill hints. A key needs a header. A
  bad key or header shows its own copy, by the `invalidEndpointAuth` KIND. Any
  edit to the URL, the key or the header un-verifies the checked server. After a
  switch the key field is cleared. A current keyed custom server shows "Key
  saved". Before the first
  use of ANY non-current server: the in-flight notice, whose body names the
  state the switch interrupts (S278 fold of the S277 walk's two observations):
  while connecting or scanning, "Switching restarts the sync in progress. Your
  balance and history stay. Funds may show as pending until the new server's
  scan catches up."; at any up-to-date state, "Switching reconnects to the new
  server. Your balance and history stay."; an undelivered or idle status takes
  the scanning copy (the cautious claim). Before the
  first CHECK of a custom host — the probe is the first packet that reaches
  it, and reveals the IP unless Tor is on — the trust notice (§8 gate 8 lists
  the copy), once per (host, key present) per sheet. With a key, it adds "Your
  key identifies you to this server. It can link your payments to your wallet,
  even over Tor." Cancel sends nothing (moved from the
  switch to the probe by the S276 crypto audit). A `fallback`
  renders a banner at the top of both the sheet and the sync sheet's Server
  row ("The server you chose isn't offered by this app any more — using
  zec.rocks").
- States, all defined (gate 2): idle · probing (spinner on the row, the
  actions disabled) · unreachable (typed copy + Retry + "Check the address")
  · wrong network · invalid URL (inline, under the field) · switching (the
  sheet stays up with a progress row; the sync badge shows the fresh
  session's `Connecting`) · switched (the sheet closes; the Server row shows
  the new host) · failed-recovered (the rescan controller's copy pattern:
  "Couldn't switch — still using zec.rocks").

---

## 4. Security

- **The custom URL is network input typed by a user.** Bounded FIRST
  (`SYNC_SERVER_URL_MAX_BYTES`, in the bridge and again in the core), then
  `LightServerEndpoint::new` (https or loopback http; no userinfo; no path).
  A rejected URL never reaches a dial, a log, or the aux row.
- **`GetLightdInfo` is hostile.** Only `chain_name` and
  `sapling_activation_height` are compared (through `endpoint_network_matches`)
  and only `block_height` is surfaced, as a `BlockHeight`. `vendor`, `version`
  and every other string are dropped at the boundary — the picker does not
  need them and would otherwise render server-chosen text.
- **A hostile custom server** can lie, withhold, or serve a foreign chain
  view. It cannot move funds (spends need the seed; it sees ciphertext and
  broadcasts). What it can do is exactly what the sync guards already judge
  pass by pass — the recorded-heights bind, the consensus verdict at the
  highest attested height (INC-006), the rewind/direction floors, the
  `Withheld`/`Behind`/`Misbehaving` reports — and each reaches the user with
  the honest copy and the remedy this spec builds. The trust notice names the
  PRIVACY consequence the guards cannot: the server learns the wallet's IP
  (unless Tor), its birthday range, the transparent addresses it polls
  (`GetAddressUtxos` — the most wallet-identifying item, even under Tor), the
  txids it fetches for enhancement, and its broadcasts (widened from three
  items to five by the S276 security review).
- **A switch onto the server already in use** (`LightServerEndpoint::same_server`
  — scheme, host, and the port with the default filled in) writes the row and
  updates the status WITHOUT rebuilding the session: a rebuild births a fresh
  `SyncController`, whose rewinding streak is a judgement about that server,
  and a host or a user typing the current server's URL must not be able to
  forgive a misbehaving-server report at zero cost (the S272 stop/start rule's
  sibling; the S276 security review's MEDIUM). And the URL bound lives in the
  ONE validator, `LightServerEndpoint::new`, not only at the bridge — so no
  endpoint can exist that the aux row's bounded read-back would refuse; the
  switch still checks, after the rebuild, that the session dials the probed
  server and refuses LOUD (`resolution_drift`) if not — and on that refusal
  CLOSES the rebuilt wallet and drops the handle (`SwitchRefused { wallet:
  None, StoreCorrupt }`), the post-stop-fault shape the host already handles
  by re-opening on the row: a KEPT handle under that error would have sent
  the host's re-open racing a live handle for the single-writer lock (the
  S276 code review's MAJOR).
  *ADR-0568:* "already in use" means the same server AND the same key: a
  choice that changes only the key rebuilds the session. Otherwise the old
  clients would keep sending the old key. The post-rebuild check compares the
  key as well as the endpoint. Accepted cost: on a server that ignores the
  header, a junk key forgives the streak in one switch. The guards still judge
  every pass.
- **Host keys.** `EndpointAuth` throughout the core (§5.4 by type: redacted
  `Debug`, sensitive on the wire, zeroized). It crosses the bridge as
  `auth_value` with the documented Dart residency caveat. A host's key is
  NEVER persisted by the SDK and never logged. A `SyncServer` that names a
  header without a value is refused at the door (D6).
- **User keys** (ADR-0568) are stored in the choice row and bound to their
  URL by `auth_url`. The rules:
  - **Never returned.** A status carries the header and an empty value.
  - **Never logged.**
  - **Never lent elsewhere.** A key never reaches another server: a fallback
    dials the default under the DEFAULT's auth, and a row whose `auth_url` no
    longer matches is unreadable.
  - **Bounded and checked at the door.** The header is bounded
    (`SYNC_SERVER_AUTH_HEADER_MAX_BYTES`) and refused if it has a `grpc-`
    prefix; the value is bounded (`ENDPOINT_AUTH_VALUE_MAX_BYTES`). Both checks
    run at the bridge and again at the read. A key is refused over plaintext.
  - **It can identify the user.** A key the user brings breaks the
    one-value-for-every-install rule. It can link every request to that
    server, including broadcasts on a fresh circuit, to one account, and the
    trust notice says so.
  - **Residency, stated honestly.**
    - *Zeroized:* our copies (the core's `EndpointAuth`, the stored-key read,
      `Resolved.auth`, the status's cloned choice, the session's
      `endpoint_auth`, each client's interceptor and channel recipe, the
      probe's included). The bridge wraps the value before its first check,
      and the read copies it only after its bound check.
    - *Not zeroized:* the Dart text controller and every Dart `SyncServerKey`
      built from it, the inbound call buffer on both sides of the bridge,
      SQLite's bound parameter, decrypted page cache and column text, the
      `HeaderValue` check copy, and hyper's buffers.
    - *On disk:* never plaintext (SQLCipher). The aux connection runs
      `secure_delete = ON` (the row can overflow a page). After a switch,
      the WAL is truncated, so the old key is zeroed in the file. If a
      reader blocks the truncation, it completes at close. Copy-on-write
      filesystems and flash may keep old ciphertext blocks below the file;
      the real erasure at wipe is the DB key's destruction.
- **The aux row** is SQLCipher-sealed like every aux table and dies at wipe.
- **Crypto:** no primitive touched. ADR-0568's stored user key brought
  crypto audit into the review set for that change: erasure, the equality
  comparison, and residency.

---

## 5. Privacy & metadata

- **What leaves the device, to whom.** The probe: one `GetLightdInfo` to the
  probed server — no wallet data. After a switch: the whole sync stream moves
  to the chosen server (birthday range via the first block request; the
  broadcasts; the tx-enhancement fetches — the same exposure `wallet-sdk.md`
  §5.1 documents today, now to a server the user chose). The previous server
  sees nothing further. No parallel dialing, no failover traffic (D7).
- **Traffic pattern.** No new timer, no background probe. A probe is a user
  tap; a switch is one stop-join and one fresh loop start.
- **Logging.** One span, `wallet.sync_server`, fields `outcome` (`probe_ok`
  / `switched` / a typed error code / `fallback_not_offered` /
  `fallback_unreadable`), `kind` (`predefined` / `custom` / `default`) and,
  for `predefined`, `id` (public config). NEVER the custom host or URL (the
  S151 rule on the sheet — an endpoint host is display data, shown but not
  logged — applies with more force to one the user typed), never a key —
  neither a host's nor a user's, neither its value nor its header name
  (ADR-0568) — never the probe's vendor strings. The tip is a bare height and MAY be
  logged.

---

## 6. Error handling & degradation

| case | typed | recoverable | what the user sees |
|---|---|---|---|
| URL fails the validator | `InvalidEndpoint { reason }` | yes | inline under the field, the reason in words ("must start with https://", "no username or password in the address", "no path after the host") |
| URL over the bound | `InvalidEndpoint { "too long" }` | yes | inline |
| a key or header the door refuses, or a key on a plaintext URL (ADR-0568) | `InvalidEndpointAuth { reason }` | yes | inline under the key fields: "This key or header can't be used" |
| a stored key row that cannot be honoured (half, oversized, refused, URL mismatch) | `SyncServerStatus.fallback = ChoiceUnreadable` (not an error) | yes: pick again | the existing unreadable banner; the default dials under the default's auth |
| probe: dial fails / times out | `SyncServerUnreachable` | yes | "Couldn't reach this server. Check the address, or try again." + Retry — never "check your connection" alone (the server may be the down side; the `walletStallEndpoint` hedge) |
| probe: wrong network | `NetworkMismatch` | no (that server, this wallet) | "This server is on a different Zcash network." |
| `Predefined` id not offered | `SyncServerNotOffered` | host bug | the row is not rendered, so unreachable from the sheet; a direct-handle host gets the typed error |
| a concurrent op during the switch | `WalletBusy { SwitchingServer }` | yes, retry | the sheet's progress row; the money surfaces keep their last values (the rescan swap's contract) |
| fault after the stop-join (quiesce timeout, an aux write fault) | the underlying error; the handle is closed | yes: re-open | "Couldn't switch — still using <host>" after the recover-by-reopen; a `DiskFull` on the row write says so (the rescan's needs-space copy) |
| the choice cannot be honoured at open | `SyncServerStatus.fallback` (not an error) | yes: pick again | the banner on the Server row and the sheet |
| offline | the probe fails → `SyncServerUnreachable` | yes | the unreachable copy; the sync loop keeps retrying the CURRENT server (queued sends stay queued — nothing about a switch attempt touches the outbox) |
| first pass on the new server: behind / degraded / misbehaving / birthday-in-future | the existing `SyncStatus` arms | as today | the existing honest copy — whose "switch servers" now points at a real action |

No silent arm: every refusal is typed and surfaced; every fallback is a
field on the status the sheet renders.

---

## 7. Performance

The probe is one unary RPC under 15 s; measured cold connect ≤ 0.35 s to
both servers §1.4 measured. A switch costs the in-flight pass's cancel
(≤ one batch; the `rescan_from` band) plus the quiesce poll (25 ms steps,
30 s max) — no DB rebuild, no rescan, no re-download: the block cache and the
scan queue are the wallet's, not the server's. The aux read at open is one
indexed singleton `SELECT`. No new timers; nothing runs on a schedule.

---

## 8. Testing strategy — the named-test contract

Every row lands with a watched mutant in `evals/mutants.tsv` (the cited line
is the one the watch printed). Files: `sync_server.rs` (new), `config.rs`,
`wallet.rs`, `db.rs`, `provision.rs`, `sync_controller.rs`; the bridge's
`tests/extraction_policy.rs`; `zec_wallet_ui/test/`.

**Gate 1 — security**

- `sync_server_debug_never_prints_the_key` — `SyncServer`'s `Debug` carries
  id, label, host; the key's bytes appear nowhere.
- `the_dart_sync_server_surface_never_prints_the_key` (extraction policy) —
  no `toString` in the bridge's generated `config.dart`/`config.freezed.dart`
  renders `authValue` (added at the S276 fold: the row above had claimed the
  Dart half without a test).
- `an_over_long_url_is_refused_at_the_one_validator` (config.rs) — the byte
  bound is the FIRST check inside `LightServerEndpoint::new`, the one
  validator every URL passes (renamed at the S276 fold from a planned
  before-the-validator door: one validator, one order).
- `probe_isolation_key_is_prefixed_unique_and_distinct_from_sync_send_and_detect`
  — the probe circuit is neither the sync circuit nor a broadcast nor an
  ephemeral-detect one: its own prefix, a random token per probe, no other
  isolation key a prefix of it.
- `probe_drops_every_lightd_info_field_but_the_three_it_compares_and_the_tip`
  — NO TEST, by construction (recorded at the build): the probe reads
  `ServerIdentity`, the `ChainOracle` seam's DTO, which carries exactly the
  three compared fields and the tip and nothing else — vendor and version
  never cross that boundary (`provision.rs`), and `SyncServerProbe` carries
  the tip alone. A test would assert a struct's field list.
- `the_row_never_stores_the_key` — after a switch to a HOST-keyed entry, the
  aux row holds the id and NULL key columns (re-pointed by ADR-0568: a host's
  key is still never stored).
- `probe_rides_the_wallets_tor_policy` — `Required` with an unreachable
  runtime: the fake dialer sees ZERO dials and the probe is
  `Sync { stall: TorUnavailable }` — Tor's word, never the server's (the
  S276 security review's LOW: "check the address" would blame a server
  nothing reached; the sheet renders the badge's Tor copy).

**Gate 2 — UX (widget tests, `sync_server_sheet_test.dart`)**

- `sync_server_sheet_lists_offered_servers_and_marks_the_one_in_use`.
- `the_app_default_row_appears_only_when_the_default_is_not_offered`.
- `an_invalid_custom_url_is_refused_inline_with_the_sdks_reason`.
- `a_failed_probe_renders_the_unreachable_copy_with_retry`.
- `a_wrong_network_probe_renders_the_network_copy`.
- `the_trust_notice_precedes_the_first_use_of_a_custom_server`.
- `the_in_flight_notice_precedes_any_switch`.
- `the_in_flight_notice_names_the_state_it_interrupts` (added at the S278
  fold: the reconnect copy at an up-to-date status, the restart-and-pending
  copy at a scanning one, the same sheet, status pushed between the taps).
- `a_successful_switch_swaps_the_session_and_the_server_row_reads_the_new_host`.
- `a_refused_switch_renders_the_typed_copy_and_changes_nothing` (added at the
  build: the pre-swap refusal at the sheet — the typed copy, no re-open, the
  in-use marker unmoved).
- `the_sync_sheets_server_row_is_a_button_that_opens_the_picker` (added at the
  build: the Server row's semantics label, its 44 dp height, and the tap that
  opens the picker).
- `the_fallback_banner_renders_on_the_sheet_and_the_server_row`.
- `sync_server_sheet_targets_are_44px_and_labelled` (the
  `appearance_screen_switches_theme_with_44px_semantic_targets` pattern).

**Gate 3 — common sense (the worst case walked)**

- `a_switch_onto_a_server_behind_the_wallets_own_height_reads_endpoint_behind_on_the_first_pass`
  — and the sheet's remedy is the picker, so the loop has an exit. **LANDED
  S277** (`wallet.rs`, beside the switch rows): a real scan to `H` on the
  first session, `switched_via_honest_oracle` (the probe is identity, not
  height — the switch is NOT refused), then `controller_over` on the handle it
  returns with a server `REORG_MAX_BLOCKS + 1` below `H`: `EndpointBehind`
  naming `H − REORG_MAX_BLOCKS`, the first session's stamp intact, no range
  asked. Watched: `sync::tip_reference`'s `max` reverted to the row alone —
  red at the status assertion with its one-session sibling.
- `a_switch_between_the_two_reference_servers_shows_the_bind_the_same_recorded_heights`
  — both fakes serving the same roots; no `RecordedHeight` refusal. **LANDED
  S277**, amended at the build: the roots are the generator's bind-consistent
  testnet sequence, NOT a transcription of the 2026-09-14 measurement — §1.4's
  record holds Ironwood completing HEIGHTS and per-pool digests, never the
  root hashes a wire `SubtreeRoot` needs, and two of those heights had MOVED
  since 09-10 (the plant's class), so a fixture on them would red for the
  plant's reason, not the switch's. The birthday sits at `H + 1` (nothing to
  scan) so the scanned-count oracle abstains and the switched session's
  re-serve is judged by the record alone: `UpToDate` on both sessions, the
  Sapling/Orchard `subtree_end_height` columns equal before and after (the
  testnet bundle proves no Ironwood subtree — that record is empty, and equal).
  Watched: `root_bind::check_recorded_heights`'s bound made always-refuse —
  red at the switched session's pass, three existing bind rows beside it.

**Gate 4 — architecture (one source of truth)**

- `sync_servers_config_refuses_duplicate_ids_oversize_lists_and_unprintable_labels`.
- `a_sync_server_naming_a_header_without_a_value_is_refused_at_the_door`.
- `the_reference_catalog_names_public_servers_only` (ADR-0568; replaces
  `reference_catalog_names_two_servers_on_mainnet_and_one_on_testnet` and
  `a_gated_catalog_entry_without_its_key_is_not_offered`, removed with the
  gated entry).
- `probe_refuses_a_foreign_chain_through_the_one_identity_predicate` — the
  mutant flips `endpoint_network_matches`; both this row and provision's
  network-match rows go red together, which is the proof there is one
  predicate.
- `the_dart_reference_endpoints_match_the_catalog` (a core policy row over
  `wallet_config.dart`: its two default constants equal the catalog's entries
  byte for byte — amended from "read from the catalog": the Dart builder stays
  pure for the host-VM tests, so it carries a copy and the row pins the copy;
  the picker marks the row in use by URL equality, which is why the copy
  gained its `:443`).
- `aux_tables_preserved_matches_what_migrate_creates` (existing) — turns red
  until `sync_server_choice` is in both lists.

**Gate 5 — observability**

- `a_switch_emits_the_sync_server_span_with_kind_and_outcome_and_never_a_custom_host`
  — `tracing_guard` capture; the custom host string is absent from every
  event.
- `a_url_value_trips_the_scan_where_a_catalog_id_is_clean` (tracing_guard.rs)
  — `://` is a FORBIDDEN token (added at the S276 fold): a rendered endpoint
  under any name, `server_id` included, trips the scan; a catalog id is
  clean.

**Gate 6 — edge cases (each with its behaviour)**

- `a_persisted_choice_wins_over_the_config_endpoint_at_open` (the shipped
  path, `store::open` → `from_open`) and `resolve_honours_a_present_row_over_the_default`
  (the pure layer, where the mutant watches in seconds).
- `a_choice_the_host_no_longer_offers_falls_back_to_the_default_visibly`.
- `an_unreadable_choice_row_reads_as_fallback_and_never_fails_the_open`
  (a truncated cell, an unknown kind, a URL the validator refuses — three
  cases in one row with named sub-asserts).
- `sync_server_choice_survives_a_rescan` (`db::copy_aux_tables` copies it;
  the rescan re-opens on the chosen server).
- `switch_default_deletes_the_row_and_returns_to_the_config_endpoint`.
- `a_switch_stops_and_joins_the_loop_before_the_row_is_written` — FOLDED into
  the gate-5 row at the build: both are one tracing capture of one switch,
  and the ordering assertion (`loop_stop` before `switched`) lives there.
- `a_switch_keeps_seq_and_the_proposal_counter_monotone` — a proposal made
  before the switch is still `send`-able after it; the next `seq` is greater.
- `a_switch_starts_a_fresh_streak_and_a_clear_hash_convention_latch`.
- `a_switch_leaves_the_verdict_grace_anchor_stamps_ledger_and_outbox_untouched`.
- `a_switch_onto_an_unreachable_server_changes_nothing_and_returns_the_wallet`
  (the `SwitchRefused { wallet: Some(_) }` arm).
- `a_switch_needs_no_in_flight_send_fence` — an in-flight row present, the
  switch succeeds, the row is intact after.
- `a_concurrent_snapshot_during_a_switch_is_wallet_busy_switching_server`.
- `a_switch_to_the_current_server_writes_the_row_and_keeps_the_session_and_its_streak`
  — the implementer's first call ("not special-cased, rebuild anyway") was
  REVERSED by the S276 security review: a rebuild forgives the rewinding
  streak, so a switch onto the server in use (by `same_server`) writes the
  row, updates the status and keeps the `Inner` — the same controller, the
  same streak. Also `an_over_long_url_is_refused_at_the_one_validator` and
  `same_server_ignores_the_default_port_and_host_case` (config.rs), and the
  `probe_rides_the_wallets_tor_policy` row now pins that a Tor-required
  runtime that is down refuses as `Sync { TorUnavailable }`, never as the
  server's fault (the review's LOW; the UI's
  `a_tor_down_probe_renders_the_tor_copy_not_the_server_copy` renders it).
- Deferred, ticketed: a switch while a swap deposit is being tracked (the
  swap service re-initialises; the in-flight swap rows are aux and survive;
  tracking resumes on the host's re-enable) — covered by the existing
  `swap_record_store` rows plus one new `a_switch_leaves_in_flight_swap_rows_intact`.

**ADR-0568 — the user's key (each with a watched mutant; the list and the
casualties are `docs/plan/srv-key-the-sdk-ships-no-gated-server.md` §3)**

- **Core:**
  - `a_custom_choice_with_a_key_round_trips_the_aux_row`
  - `leaving_a_keyed_custom_server_erases_its_key`
  - `a_malformed_key_row_reads_unreadable`
  - `an_older_build_rewriting_the_url_never_lends_the_key`
  - `an_existing_choice_table_gains_the_key_columns_on_open`
  - `a_rescan_carries_the_keyed_custom_choice`
  - `the_probe_and_the_switch_send_the_custom_key`
  - `a_reopen_honours_the_keyed_custom_choice`
  - `a_switch_onto_the_same_server_with_a_new_key_rebuilds`
  - `an_identical_choice_onto_the_current_server_still_keeps_the_session`
    (the counterpart; it widens Gate 6's
    `a_switch_to_the_current_server_writes_the_row_and_keeps_the_session_and_its_streak`
    to a keyed custom choice)
  - `a_same_url_default_with_another_key_is_a_switch`
  - `a_transport_refused_keyed_custom_never_lends_its_key_to_the_default`
  - `a_key_on_a_plaintext_custom_url_is_refused`
  - `endpoint_auth_refuses_any_grpc_prefix_and_an_oversized_header`
  - `endpoint_auth_equality_is_by_header_and_value`
  - `a_stored_choice_debug_never_prints_the_key`
  - `the_aux_connection_zeroes_an_erased_key` (the erase on file bytes, with a
    control that finds the marker without `secure_delete`)
  - `a_keyed_connection_zeroes_freed_pages_by_sqlciphers_default` (measured at
    the build: SQLCipher's keyed-handle default)
  - `the_aux_connection_runs_secure_delete` (the production aux handle)
  - `leaving_a_keyed_server_folds_the_wal_at_the_switch`
  - the gate-5 `tracing_guard` row widened to a keyed custom switch
- **Bridge:**
  - `custom_choice_key_is_bounded_and_typed`
  - `the_status_never_returns_a_custom_key_value`
- **UI:**
  - `the_sheet_sends_the_key_with_the_custom_probe_and_switch`
  - `editing_the_key_unverifies_the_custom_server`
  - `a_key_needs_a_header`
  - `a_bad_key_shows_the_key_copy_not_the_url_copy`
  - `the_trust_notice_names_identification_and_re_asks_when_a_key_is_added`
  - `a_keyed_current_custom_server_shows_key_saved`
  - `a_custom_choice_never_prints_its_key`
  - `the_key_field_never_enables_suggestions_or_learning`

**Gate 7 — no magic numbers**

- `sync_server_constants_are_named_and_bounded` — the six constants (ADR-0568
  added `SYNC_SERVER_AUTH_HEADER_MAX_BYTES`)
  asserted at their boundaries, the probe timeout above the measured connect.

**Gate 8 — i18n (16 ARB files; machine copy per the founder's S272 waiver;
`flutter gen-l10n` + the `l10n-drift-check` leg)**

New keys, English source: `walletSyncServerSheetTitle` "Sync server" ·
`walletSyncServerInUse` "In use" · `walletSyncServerAppDefault` "App default" ·
`walletSyncServerCustom` "Custom server…" · `walletSyncServerCustomHint`
"https://host:port" · `walletSyncServerCheck` "Check server" ·
`walletSyncServerUse` "Use this server" · `walletSyncServerChecking`
"Checking…" · `walletSyncServerSwitching` "Switching…" ·
`walletSyncServerUnreachable` "Couldn't reach this server. Check the address —
and if it's right, either this server isn't answering or your app can't reach
it right now. Try again, or pick another server." — SPLIT at stage S1 `copy`
into this (the user TYPED the address) and `walletSyncServerUnreachableOffered`
"Couldn't reach this server. The wallet can't tell whether this server isn't
answering or your app can't reach it right now. Pick another server, or try
again later." (the address came from the app's own list): `probe_oracle` maps
everything that is not a FAILED private dial onto this one kind, and since
stage S1 a private path that accepts a dial and then carries nothing no longer
reports `TorUnavailable`, so a censored path and a wedged server arrive here as
the same error — "check the address" is the right first step only for the
reader who typed one · `walletSyncServerWrongNetwork` "This server is on a
different Zcash network." · `walletSyncServerInvalidUrl` "That doesn't look like a server address. Use
https://host:port." — ONE key for every door reason (amended at the build:
per-reason keys would have meant matching the SDK's reason string across the
FFI, the gotcha the bridge rules forbid; the reason is never echoed) ·
`walletSyncServerNotOffered`, `walletSyncServerBusy` (the two refusals the
kind switch also renders) · `walletSyncServerContinue`, `walletSyncServerCancel`,
`walletSyncServerTrustTitle` (the two notice dialogs' chrome) ·
`walletSyncServerFallbackUnreadable` "The remembered server choice couldn't be
read. Using {host}." ·
`walletSyncServerSwitchNotice` "Switching restarts the sync in progress. Your
balance and history stay. Funds may show as pending until the new server's
scan catches up." (the connecting/scanning body) ·
`walletSyncServerSwitchNoticeAtTip` "Switching reconnects to the new server.
Your balance and history stay." (the up-to-date body; S278) ·
`walletSyncServerTrustNotice` "You're trusting
this server to report your balance and history and to relay your payments. It
will see your IP address unless Tor is on, roughly when your wallet was
created, and the transactions you send." · `walletSyncServerFallbackNotOffered`
"The server you chose isn't offered by this app any more. Using {host}." ·
`walletSyncServerSwitchFailedRecovered` "Couldn't switch — still using
{host}." · `walletSyncServerRowSemantics` "Server, {host}, opens the server
picker". And the description edit of A3. *ADR-0568 adds:*
`walletSyncServerKeyLabel` "Access key (optional)" ·
`walletSyncServerKeyHeaderLabel` "Key header" · `walletSyncServerKeyHeaderNeeded`
"Enter the header your server expects" · `walletSyncServerKeyInvalid` "This key
or header can't be used" · `walletSyncServerKeySaved` "Key saved" ·
`walletSyncServerKeyShow` / `walletSyncServerKeyHide` (the toggle's semantics) ·
`walletSyncServerTrustNoticeKey` "Your key identifies you to this server. It can
link your payments to your wallet, even over Tor."

- `every_locale_carries_the_sync_server_keys` (the
  `every_locale_carries_the_three_reworded_keys` pattern).

**Split:** unit (core rows above), bridge policy (`extraction_policy.rs`),
widget (`flutter test` — the WHOLE suite with its total count is the exit
criterion, never a subset), and one device walk on the Pixel (dump → tap →
verify-dump → assert) before the mini-batch's review: open the sheet, switch
zec.rocks → the maintainer's gated server, watch the pass reach tip, switch back. No E2E
money movement is needed — the switch touches no funds. **RUN S277
(2026-09-14)** on the Pixel 10 Pro, HEAD debug build, screenshot-and-tap (on
this app `uiautomator` publishes no Flutter node — the wallet log's S238
correction): both switches landed, each logged
`wallet.sync_server outcome="switched"` with kind and id only, the switched
session reached tip on the gated server and again on zec.rocks, the gated
entry was offered because the key rode the build. (Since ADR-0568 that entry
is the HOST's, not the reference catalog's; the walk is re-run as a custom
server with a key.) Record and two UX observations:
phase-3 §6 "The device walk".

---

## 9. Multi-platform

A switch is user-initiated in the foreground; nothing changes about
background behaviour on any platform (iOS's background limits already end a
pass; a fresh loop start on resume is the existing contract). The key
define is per build on every platform; a desktop build without it simply
offers one predefined server. Loopback `http://` custom endpoints work on
desktop (a local lightwalletd) and are refused for a remote host everywhere.

---

## 10. Multi-device & sync

The choice is a DEVICE preference about trust, not wallet identity: it does
not sync, it does not ride the seed, and two devices holding the same wallet
may trust different servers with no conflict. The data model assumes nothing
about one user = one device: the row is per wallet DB, and a second device's
row is its own.

---

## 11. Question register

| # | question | answer |
|---|---|---|
| Q1 | should a switch between the two predefined servers require a probe? | Yes — D9, one rule. The founder may relax it for predefined entries; then a failed probe becomes a warning, not a refusal, and the test `a_switch_onto_an_unreachable_server_changes_nothing_and_returns_the_wallet` narrows to custom |
| Q2 | should `sync_servers` be REQUIRED on the config (no empty list)? | No — a third-party host with one server of its own should not have to name it twice. Empty = the default plus custom |
| Q3 | does the reference app keep `ZEC_ENDPOINT`? | Yes, as the DEFAULT override it is today; the catalog is the list. A `ZEC_ENDPOINT` not in the list shows as "App default" |
| Q4 | the fresh-streak decision: should a user switch inherit the streak? | No — a streak is a judgement about one server; inheriting it would let server A's misbehaviour brand server B. Recorded so the failover row can revisit it with the same reasoning |
