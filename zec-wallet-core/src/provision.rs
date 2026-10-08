//! §3.2f — live account provisioning over the BUNDLED-frontier anchor (the
//! W3-inc-2c-iv-c crypto-change review). The bridge that makes inc-2c-iii's offline
//! `import_account` seam LIVE: resolve a birthday height from the endpoint's tip
//! (or the host's `config.birthday`), then anchor it with the TRUSTED bundled
//! commitment-tree frontier — NEVER a live `get_tree_state`.
//!
//! **The trust model (crypto audit 2026-06-14, §3.2f):** the backend does NOT
//! self-verify a birthday frontier (it trusts lightwalletd for tree state), and a
//! live frontier is cryptographically unverifiable at a recent height. So the
//! birthday frontier comes from the signed-binary bundle ([`checkpoints::
//! bundled_treestate`]) — the M2 live-frontier threat is ELIMINATED, not
//! mitigated. This module STRUCTURALLY cannot consult a live frontier: the
//! [`ChainOracle`] seam exposes only the tip + the network identity — there is no
//! tree-state method on it by design (`provision_never_provisions_from_a_live_frontier`).
//!
//! The endpoint reads here are CHEAP correctness guards (honestly scoped, §3.2f):
//! the `get_lightd_info` network-match (host/manifest-misconfig defense) and the
//! tip clamp (config-typo defense) — NOT adversary defense (every server-reported
//! field is server-controlled; the bundled frontier is what defeats a liar).
//!
//! **Since `ironwood-nu63-support.md` §3.1 the same `get_lightd_info` read also
//! feeds the consensus-staleness verdict** ([`crate::consensus`]), which is a
//! different axis with a different posture: the chain guard here answers "are we
//! on the network the host configured", the verdict answers "does this build
//! implement the rules that network is running". [`endpoint_identity`] does the
//! one fetch and hands the identity on; the verdict itself is computed and
//! persisted by the sync pass, not here.

use async_trait::async_trait;
use zcash_client_backend::data_api::AccountBirthday;

use crate::account::birthday_from_treestate;
use crate::checkpoints;
use crate::constants::NEW_WALLET_BIRTHDAY_LAG_BLOCKS;
use crate::error::WalletError;
use crate::money::{BlockHeight, Network};
use crate::net::grpc::{GrpcError, LightwalletdClient, transport_err};

/// What provisioning needs from the (untrusted) endpoint, §3.2f. A testability
/// seam mirroring iv-a's `CompactBlockSource`: the crate ships only the
/// lightwalletd CLIENT codegen (no server), so the provisioning logic is unit
/// tested through a fake oracle, no live network. **By design it exposes only the
/// tip + the network identity — NO tree-state method** — so `resolve_birthday`
/// cannot anchor the birthday on a live frontier even by accident.
#[async_trait]
pub(crate) trait ChainOracle {
    /// Best-chain tip height (the birthday-lag input + the above-tip clamp).
    async fn tip_height(&mut self) -> Result<u64, GrpcError>;
    /// The endpoint's self-reported network identity (the network-match inputs).
    async fn server_identity(&mut self) -> Result<ServerIdentity, GrpcError>;
}

/// The endpoint's self-reported network identity (the §3.2f network-match inputs,
/// extracted from `get_lightd_info` — the upstream DTO stays behind the seam).
///
/// EXTENDED for `ironwood-nu63-support.md` §1.2: the two historical constants
/// stay (they still catch a cross-network misconfig, which is all they ever
/// could catch), and the two fields that actually MOVE across a network upgrade
/// are added. We had been fetching both on every handshake and discarding them
/// while the SDK signed six weeks of transactions the network was guaranteed to
/// reject.
pub(crate) struct ServerIdentity {
    pub(crate) chain_name: String,
    pub(crate) sapling_activation_height: u64,
    /// lightwalletd reports this as a hex STRING; parsed at the boundary by
    /// [`crate::consensus::parse_branch_id`]. `None` when absent, empty or
    /// unparseable — an old or non-conforming server. `None` is NEVER read as
    /// "fine": see `consensus::ConsensusCompatibility::Unknown` and spec §6.3.
    pub(crate) consensus_branch_id: Option<u32>,
    /// The endpoint's claimed best-chain tip. UNTRUSTED, and used only as one
    /// half of the two-height judgement in `consensus::consensus_compatibility`
    /// — an endpoint that lies about it can only ever earn itself more
    /// scrutiny, never less (spec §1.3).
    pub(crate) block_height: u64,
}

#[async_trait]
impl ChainOracle for LightwalletdClient {
    async fn tip_height(&mut self) -> Result<u64, GrpcError> {
        Ok(self.get_latest_block().await?.height)
    }

    async fn server_identity(&mut self) -> Result<ServerIdentity, GrpcError> {
        let info = self.get_lightd_info().await?;
        Ok(ServerIdentity {
            // §4.6: the hex string is hostile input; a total parse, never a
            // panic, and an unparseable value degrades to `None` (= "we could
            // not check"), never to a fabricated agreement.
            consensus_branch_id: crate::consensus::parse_branch_id(&info.consensus_branch_id),
            block_height: info.block_height,
            chain_name: info.chain_name,
            sapling_activation_height: info.sapling_activation_height,
        })
    }
}

/// Does the endpoint's self-reported identity match `network`? (§3.2f network-match
/// — a CHEAP correctness guard against host/manifest misconfig, NOT adversary
/// defense: a malicious server lies about both fields. Matches on BOTH the
/// `chain_name` label AND the numeric `sapling_activation_height`, which ties to
/// our bundle's activation anchor — an honest server gets both right.)
pub(crate) fn endpoint_network_matches(network: Network, id: &ServerIdentity) -> bool {
    id.chain_name == checkpoints::chain_name(network)
        && id.sapling_activation_height == u64::from(checkpoints::activation_height(network))
}

/// Resolve the fully-OFFLINE birthday for the create-time EAGER account import
/// (FR-24, #359, crypto-change review) — or `None` when the job must stay with
/// the lazy first-sync [`resolve_birthday`] below. Eager arms:
///
/// - **Configured birthday at/below the bundle's tree-state tail** (create OR
///   restore): the height is HOST-asserted and the anchor is signed-binary
///   data — no network claim is involved, so importing it offline changes no
///   trust relationship. A configured birthday ABOVE the tail stays lazy so
///   the `BirthdayInFuture` tip clamp (the config-typo guard, which NEEDS a
///   live tip) still fires at first sync.
/// - **A FRESHLY-GENERATED create (`Generate` / host-attested
///   `FreshRawBytes`) with the CLAMPED creation stamp** (§3.2f #356-F4, read
///   back from the store — the write-clamp already bounds clock-ahead skew at
///   the creating binary's bundle tail): `estimate_birthday(created_at)`
///   floors conservatively, and a freshly-minted seed cannot have chain
///   history before its stamp — the exact money argument the lazy path
///   already relies on, minus the (unused) live tip. (For `FreshRawBytes`
///   that premise is the HOST's attestation — the §3.2f lying-host contract.)
/// - Everything else — a restore with NO birthday (activation full scan is a
///   first-sync decision) or a stampless fresh remnant — stays lazy.
///
/// Failure posture is the CALLER's (create fails loud on the fresh arm;
/// the restore arm degrades to lazy) — this fn only errors on the fail-closed
/// corrupt-binary cases (`StoreCorrupt` empty bundle / undecodable tree state),
/// mirroring [`resolve_birthday`].
pub(crate) fn resolve_offline_birthday(
    network: Network,
    configured: Option<BlockHeight>,
    created_at: Option<u64>,
    freshly_generated: bool,
) -> Result<Option<AccountBirthday>, WalletError> {
    let tail = checkpoints::treestate_table(network)
        .last()
        .map(|row| row.0)
        .unwrap_or(0);
    let requested: u32 = match (configured, created_at) {
        (Some(h), _) if h.value() <= tail => h.value(),
        (Some(_), _) => return Ok(None), // above tail → lazy (the tip clamp's job)
        (None, Some(at)) if freshly_generated => {
            checkpoints::estimate_birthday(network, at).value()
        }
        _ => return Ok(None), // no-birthday restore / stampless — lazy
    };
    let treestate = checkpoints::bundled_treestate(network, BlockHeight::new(requested))
        .ok_or(WalletError::StoreCorrupt)?;
    // An undecodable bundled tree state is the same fail-closed corrupt-binary
    // case the lazy resolver (later in this file) maps: StoreCorrupt.
    birthday_from_treestate(treestate)
        .map(Some)
        .map_err(|_| WalletError::StoreCorrupt)
}

/// #397 (spec §3.7 D2): the WATCH-ONLY create's offline birthday — ALWAYS
/// eager, never lazy. A watch-only wallet CANNOT defer its account import to
/// the lazy first-sync resolver: that path re-derives from a seed the wallet
/// structurally lacks (and `acquire_seed` refuses `WatchOnly`), and the UFVK
/// exists nowhere at rest until the import lands it in the engine's accounts
/// table. So the requested height is FLOORED to the bundle's tree-state tail
/// when it lies above it — a pure over-scan in the (tail‥birthday) gap
/// (bundle tails are release-fresh, modern spacing 2,500 blocks), never a
/// missed note; the floor also conservatively absorbs an above-tip typo the
/// offline path cannot tip-clamp. Same fail-closed corrupt-binary posture as
/// [`resolve_offline_birthday`].
pub(crate) fn resolve_watch_only_birthday(
    network: Network,
    requested: BlockHeight,
) -> Result<AccountBirthday, WalletError> {
    let tail = checkpoints::treestate_table(network)
        .last()
        .map(|row| row.0)
        .unwrap_or(0);
    let floored = requested.value().min(tail);
    let treestate = checkpoints::bundled_treestate(network, BlockHeight::new(floored))
        .ok_or(WalletError::StoreCorrupt)?;
    birthday_from_treestate(treestate).map_err(|_| WalletError::StoreCorrupt)
}

/// Fetch the endpoint's identity, applying the wrong-CHAIN guard, and hand the
/// identity back for the caller's consensus judgement.
///
/// The chain guard (typed `NetworkMismatch`, `RW-STORE-002`) is honestly NOT
/// adversary defense — a lying endpoint passes it, and the bundled anchor is
/// the real trust root. It is the typed misconfig door, and it is a DIFFERENT
/// axis from consensus staleness: the chain you are on versus the rules that
/// chain is running. One fetch answers both; two codes report them.
///
/// And two SURFACES, which must not be confused (T0-1c). A wrong chain is a
/// server answering with data this wallet cannot use — the content tier, so it
/// renders `Stalled { EndpointMisbehaving }` through `sync_controller::stall_for`:
/// "switch servers", the only remedy there is. A consensus `Unsupported` /
/// foreign-branch verdict is the OTHER axis and the other class: the server is
/// fine and the BUILD is stale, which is `SyncStatus::UpToDateLimited` ("update
/// the app"), published rather than stalled.
pub(crate) async fn endpoint_identity<C: ChainOracle + ?Sized>(
    client: &mut C,
    network: Network,
) -> Result<ServerIdentity, WalletError> {
    let id = client.server_identity().await.map_err(transport_err)?;
    if !endpoint_network_matches(network, &id) {
        return Err(WalletError::NetworkMismatch);
    }
    Ok(id)
}

/// The wrong-chain endpoint guard (CS-M1) as a bare check — the
/// provisioning caller that needs no verdict.
pub(crate) async fn verify_endpoint_network<C: ChainOracle + ?Sized>(
    client: &mut C,
    network: Network,
) -> Result<(), WalletError> {
    endpoint_identity(client, network).await?;
    Ok(())
}

/// Resolve the wallet's birthday [`AccountBirthday`] for `network` from the
/// endpoint's tip (or the host-supplied `configured` height), anchored by the
/// TRUSTED bundled frontier (§3.2f). NEVER reads a live `get_tree_state` — the
/// frontier comes from the signed binary.
///
/// Steps: (1) network-match (`NetworkMismatch` on a wrong-chain endpoint, before
/// any side effect); (2) resolve the requested height — `configured` if the host
/// gave one (above the tip REFERENCE — the endpoint's tip, or the bundle's
/// newest row when the tip is below it (T0-1c-R2 M3) — ⇒ `BirthdayInFuture`);
/// else, with NO host birthday, the
/// default depends on the CREATION STAMP (`created_at`, §3.2f #356-F4) and
/// `freshly_generated`: a stamped wallet (the stamp is written ONLY on a
/// freshly-generated create — `SeedSource::Generate`, or the host-attested
/// `FreshRawBytes` whose §3.2f lying-host contract carries the trust shift —
/// so its presence proves the seed was minted at ~that wall-clock
/// — even across a reopen, where `freshly_generated` is unknowable)
/// ⇒ `min(tip − NEW_WALLET_BIRTHDAY_LAG_BLOCKS, estimate_birthday(created_at))`,
/// bounding the birthday at ~creation time no matter how long the wallet stayed
/// offline before this LAZY first sync (the F4 silent-fund-loss shape: a deposit
/// received between an offline create and a first sync days later sat below a
/// tip−lag birthday resolved on the sync day); a stamp-less brand-new wallet
/// (pre-F4 belt) ⇒ `tip − NEW_WALLET_BIRTHDAY_LAG_BLOCKS`; a RESTORE / imported
/// seed (possible history) ⇒ Sapling ACTIVATION (a full scan, the §1.7 "skip ⇒
/// activation" contract) — NEVER ~tip, which would silently skip the user's
/// entire history (the §2.3 too-high-birthday silent-fund-loss class). The stamp
/// only ever LOWERS the fresh default: a clock-behind device just scans wider,
/// and a clock-ahead one was capped at WRITE to the creating binary's own
/// bundle-tail time (`checkpoints::newest_checkpoint_time` — so even a FUTURE
/// binary's fresher bundle can never estimate the stamp above a height the
/// creating binary already knew about, ≤ the tip at create ≤ any deposit's
/// height) — money-safe in both skew directions. (3) pick the newest
/// bundled frontier `< requested` (birthday `≤ requested` — never skip funds)
/// and decode it. A transport failure surfaces typed `Sync { stall }`; an
/// empty/undecodable bundle is the fail-closed corrupt-binary case
/// (`StoreCorrupt`).
pub(crate) async fn resolve_birthday<C: ChainOracle + ?Sized>(
    client: &mut C,
    network: Network,
    configured: Option<BlockHeight>,
    freshly_generated: bool,
    created_at: Option<u64>,
) -> Result<AccountBirthday, WalletError> {
    // (1) Network-match — reject a wrong-chain endpoint BEFORE any provisioning
    // write (a host/manifest misconfig; honestly NOT adversary defense, §3.2f).
    verify_endpoint_network(client, network).await?;

    // (2) Resolve + clamp the requested birthday height. The tip is always fetched
    // (it bounds the configured value AND defaults a new wallet's birthday).
    let tip_u64 = client.tip_height().await.map_err(transport_err)?;
    // A height beyond u32 is a garbage/hostile response (consensus heights are
    // u32): the identical lie `sync::fetch_tip` refuses, and it gets the identical
    // answer — the content-tier class, `EndpointMisbehaving`, through the class
    // helper so a grep for it finds this site (T0-1c, §4j row 2; the hand-spelled
    // `EndpointUnreachable` that stood here said "check your connection" for a
    // server that had answered).
    let tip = u32::try_from(tip_u64).map_err(|_| crate::sync::endpoint_unusable())?;
    // T0-1c-R2 (§4n Q-G3, M3): this is the THIRD endpoint height a pass reads,
    // and — since R2 — it is graded through the SAME grade `sync::fetch_tip`'s
    // is (`sync::tip_standing`, with no scanned height: an unprovisioned wallet
    // has none, so the bundle's newest row is the whole reference), and a tip
    // below the row is CLAMPED UP TO IT before the birthday resolves. §4k-R
    // decision 5 left this height ungraded because "it only ever LOWERS a
    // birthday — money-safe by direction"; the wrap (§4m #3) found what that
    // sentence did not say: the lowering is a PERMANENT write. `import_account`
    // is durable, so a lying-low first tip on a fresh install floored the
    // birthday toward activation and bought a ~3 M-block scan that survived
    // switching servers (a recoverable scan-DoS, never fund loss — and the
    // from-below clamp this sentence used to owe to iv-d). The clamp is
    // money-safe by construction: the row is a checkpoint the chain provably
    // reached before this binary shipped, a fresh wallet is created after it
    // ships, so every deposit it can ever receive sits at or above the real
    // tip at create ≥ the row > `row − lag`; and a configured birthday at or
    // below the row is a height the chain provably reached, so a server that
    // says otherwise is behind, not the birthday in the future. Bought: the
    // birthday a behind server provisions equals the one a current server
    // gives (never activation), and T0-1c's population — a configured birthday
    // above a stuck validator's tip — provisions instead of stalling. Lost: a
    // configured birthday ABOVE the row against a behind server is still
    // `BirthdayInFuture` (the chain may or may not have reached it; the
    // stall's own reading, `StallReason::BirthdayInFuture`, says so), and the
    // behind warn fires here as well as at `fetch_tip` on the same pass.
    //
    // T0-1c-R3 (§4n-R decision 1): provisioning is only HALF of continuing the
    // pass, and the clamp is KEPT rather than replaced by REFUSE. A clamped
    // birthday is above a behind tip by construction, and upstream
    // `update_chain_tip` cannot take a birthday above `tip + 1`
    // (`ScanRange::from_parts` asserts) — the R2 ruling measured the clamp
    // ALONE panicking the pass on exactly the remnant it was written for
    // (INC-024's class). The other half is `sync::birthday_beyond_tip` in
    // `Wallet::sync_once`: the same pass's `fetch_tip` grades the tip, the guard
    // returns the pass before `record_chain_tip`, and the surface reads
    // `SyncStatus::EndpointBehind` with nothing scanned. The two are ONE
    // mechanism; a reader changing either must read the other. What the clamp
    // buys over REFUSE, what it costs, and where the birthday it writes sits
    // (the anchor ceiling — the same height FR-24 writes with no server at all)
    // are recorded at `birthday_beyond_tip`.
    let tip_reference = match crate::sync::tip_standing(network, tip, None) {
        crate::sync::TipStanding::AtOrAboveBundle => tip,
        crate::sync::TipStanding::BehindBundle { newest_known } => newest_known,
    };
    let requested = requested_birthday_height(
        network,
        tip_reference,
        configured,
        freshly_generated,
        created_at,
    )?;

    // (3) Anchor on the TRUSTED bundled frontier (never a live get_tree_state) →
    // decode to a birthday. An empty/undecodable bundle is a corrupt binary.
    let treestate = checkpoints::bundled_treestate(network, BlockHeight::new(requested))
        .ok_or(WalletError::StoreCorrupt)?;
    birthday_from_treestate(treestate).map_err(|_| WalletError::StoreCorrupt)
}

/// The height the birthday is REQUESTED at (step (2) of [`resolve_birthday`]),
/// pure — no IO, no clock — over `tip_reference`, the endpoint's tip already
/// clamped up to the bundle's newest row (T0-1c-R2 M3; the caller does the
/// clamp through `sync::tip_standing`). Factored out so the arithmetic below
/// can be driven at heights the clamp makes unreachable end to end (every
/// network's row sits far above `activation + lag`, and the anchor ceiling in
/// `checkpoints::bundled_treestate` flattens every request above Nu6.3's
/// activation onto one row, so two requests near the tail resolve to the same
/// birthday). The resolved birthday is always ≤ the request.
///
/// A configured height above `tip_reference` ⇒ `BirthdayInFuture`. No host
/// birthday: a FRESH wallet (no history) caps the requested height at
/// `tip_reference − lag`, further bounded by the creation stamp when one exists
/// (#356-F4): provisioning is LAZY (first sync), so tip − lag ALONE was
/// resolved on the first-SYNC day — a deposit received between an offline
/// create and that sync sat below the birthday (the silent-fund-loss shape).
/// `estimate_birthday(created_at)` floors to the bracketing bundled checkpoint
/// (never overshoots); a clock-ahead device was capped at WRITE to the creating
/// binary's bundle-tail time (tail ≤ height at binary release ≤ tip at create ≤
/// any deposit's height — and a later binary's fresher bundle cannot lift a cap
/// enforced on the stored value) and a clock-behind one just scans wider — the
/// min can only LOWER the birthday, never raise it. The stamp's PRESENCE alone
/// proves a freshly-generated create on this device (Generate or host-attested
/// FreshRawBytes — never written on restore, and a restore-shaped repair CLEARS
/// an inherited one), so a stamped account-less REOPEN — where
/// `freshly_generated` is unknowable and used to floor to activation — takes
/// the same bounded-from-creation default instead of a multi-year full scan. A
/// RESTORE / imported seed (possible history, no stamp) must NOT default to
/// ~tip — that silently skips the user's whole history (§2.3) — so it floors to
/// activation (a full scan, §1.7), the conservative money-safe default; the
/// host supplies `config.birthday` (or `estimate_birthday`) to start later.
/// Floor: no shielded notes exist below Sapling activation (reachable for a
/// configured height; the fresh default's `tip_reference − lag` sits above it on
/// every network the clamp serves).
pub(crate) fn requested_birthday_height(
    network: Network,
    tip_reference: u32,
    configured: Option<BlockHeight>,
    freshly_generated: bool,
    created_at: Option<u64>,
) -> Result<u32, WalletError> {
    let activation = checkpoints::activation_height(network);
    let fresh_default = tip_reference.saturating_sub(NEW_WALLET_BIRTHDAY_LAG_BLOCKS);
    let requested = match (configured, created_at) {
        (Some(h), _) if h.value() > tip_reference => return Err(WalletError::BirthdayInFuture),
        (Some(h), _) => h.value(),
        (None, Some(at)) => fresh_default.min(checkpoints::estimate_birthday(network, at).value()),
        (None, None) if freshly_generated => fresh_default, // stamp-less pre-F4 belt
        (None, None) => activation, // restore with no birthday ⇒ full scan (§1.7), never ~tip
    };
    Ok(requested.max(activation))
}

/// Test-only scripted [`ChainOracle`] — no network. Shared by `provision`'s own
/// tests and the `wallet` end-to-end test (the crate ships only the client
/// codegen, so the provisioning path is driven through this fake).
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A scripted endpoint. Each field can be set to drive a path or inject a
    /// transport fault on the next call.
    pub(crate) struct FakeOracle {
        pub(crate) chain_name: String,
        pub(crate) sapling_activation: u64,
        pub(crate) tip: u64,
        /// The endpoint's claimed consensus branch (`ironwood-nu63-support.md`
        /// §2). `honest()` reports what OUR compiled params compute at `tip`,
        /// so an ordinary provisioning test is `Current`; a staleness test sets
        /// this to something else.
        pub(crate) consensus_branch_id: Option<u32>,
        pub(crate) identity_err: Option<GrpcError>,
        pub(crate) tip_err: Option<GrpcError>,
    }

    impl FakeOracle {
        /// An honest endpoint for `network` reporting `tip` — including a
        /// consensus branch that AGREES with our compiled params at that
        /// height, which is what "honest" has to mean now that the branch is
        /// part of the identity.
        pub(crate) fn honest(network: Network, tip: u64) -> Self {
            use zcash_protocol::consensus::{BlockHeight as ConsensusHeight, BranchId};
            let branch = BranchId::for_height(
                &network.consensus(),
                ConsensusHeight::from_u32(u32::try_from(tip).unwrap_or(u32::MAX)),
            );
            Self {
                chain_name: checkpoints::chain_name(network).to_owned(),
                sapling_activation: u64::from(checkpoints::activation_height(network)),
                tip,
                consensus_branch_id: Some(u32::from(branch)),
                identity_err: None,
                tip_err: None,
            }
        }
    }

    #[async_trait]
    impl ChainOracle for FakeOracle {
        async fn tip_height(&mut self) -> Result<u64, GrpcError> {
            match self.tip_err.take() {
                Some(e) => Err(e),
                None => Ok(self.tip),
            }
        }
        async fn server_identity(&mut self) -> Result<ServerIdentity, GrpcError> {
            match self.identity_err.take() {
                Some(e) => Err(e),
                None => Ok(ServerIdentity {
                    chain_name: self.chain_name.clone(),
                    sapling_activation_height: self.sapling_activation,
                    // The fake reports the branch our COMPILED params expect at
                    // its tip, so an ordinary provisioning test is `Current` and
                    // is not accidentally testing the refusal path. Tests that
                    // want staleness set it explicitly.
                    consensus_branch_id: self.consensus_branch_id,
                    block_height: self.tip,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::FakeOracle;
    use super::*;
    use crate::state::StallReason;

    /// The frontier `resolve_birthday` anchored on must come from the BUNDLE for
    /// the resolved height — independent of any live tree-state (there is none).
    fn expected_bundled_height(network: Network, requested: u32) -> u32 {
        let ts = checkpoints::bundled_treestate(network, BlockHeight::new(requested))
            .expect("a bundled row");
        u32::try_from(ts.height).expect("height fits u32")
    }

    #[tokio::test]
    async fn provision_rejects_network_mismatched_endpoint() {
        // §8: a mainnet wallet pointed at a testnet-identifying endpoint is
        // NetworkMismatch BEFORE any provisioning (the cheap correctness guard).
        let mut oracle = FakeOracle {
            chain_name: "test".to_owned(),
            sapling_activation: u64::from(checkpoints::activation_height(Network::Test)),
            tip: 3_000_000,
            // Deliberately the branch a HONEST endpoint would report: the
            // wrong-chain guard must fire on its own inputs, not be rescued by
            // a coincidentally-wrong branch (the two axes stay separable).
            consensus_branch_id: None,
            identity_err: None,
            tip_err: None,
        };
        let r = resolve_birthday(&mut oracle, Network::Main, None, true, None).await;
        assert!(matches!(r, Err(WalletError::NetworkMismatch)), "got {r:?}");
    }

    #[tokio::test]
    async fn resolve_birthday_new_wallet_uses_tip_lag_and_bundle() {
        // §8: with no configured birthday, the requested height is tip − lag, and
        // the birthday is the bundled frontier for it (NEVER a live frontier).
        // The fixture tip sits ABOVE the bundle's newest row (T0-1c-R2 M3: a tip
        // below it is clamped up to the row before the lag is taken, so only a
        // tip above the row is the reference this row's formula names).
        for net in [Network::Main, Network::Test] {
            let tip: u64 = u64::from(crate::root_bind::newest_bundled_height(net)) + 100_000;
            let mut oracle = FakeOracle::honest(net, tip);
            let b = resolve_birthday(&mut oracle, net, None, true, None)
                .await
                .expect("resolves");
            let requested =
                u32::try_from(tip).expect("test tip fits u32") - NEW_WALLET_BIRTHDAY_LAG_BLOCKS;
            assert_eq!(
                u32::try_from(u64::from(b.height())).unwrap(),
                expected_bundled_height(net, requested) + 1,
                "{net:?}: birthday = bundled-frontier height + 1 for tip−lag"
            );
            // never above the requested height (the money property)
            assert!(u64::from(b.height()) <= u64::from(requested));
        }
    }

    #[tokio::test]
    async fn resolve_birthday_honors_configured_height() {
        // §8: a host-supplied birthday picks the newest bundled frontier below it.
        let net = Network::Main;
        let configured = 2_000_000u32;
        let mut oracle = FakeOracle::honest(net, 3_300_000);
        let b = resolve_birthday(
            &mut oracle,
            net,
            Some(BlockHeight::new(configured)),
            true,
            None,
        )
        .await
        .expect("resolves");
        assert_eq!(
            u32::try_from(u64::from(b.height())).unwrap(),
            expected_bundled_height(net, configured) + 1,
            "birthday = bundled-frontier height + 1 for the configured height"
        );
        assert!(
            u64::from(b.height()) <= u64::from(configured),
            "never above configured"
        );
    }

    #[tokio::test]
    async fn resolve_birthday_stamped_fresh_wallet_floors_at_creation_time() {
        // §8 (#356-F4, the headline pin): a STAMPED fresh wallet whose first
        // sync runs long after create resolves at estimate(created_at), NOT at
        // the first-sync day's tip − lag — the deposit-before-first-sync
        // window is inside the scan. Mid-slice created_at, tip far above.
        for net in [Network::Main, Network::Test] {
            let created_at = 1_700_000_000u64; // Nov 2023 — mid-slice both nets
            let tip: u64 = 3_300_000;
            let tip_lag = u32::try_from(tip).unwrap() - NEW_WALLET_BIRTHDAY_LAG_BLOCKS;
            let estimate = checkpoints::estimate_birthday(net, created_at).value();
            assert!(
                estimate < tip_lag,
                "{net:?}: precondition — the stamp must be the binding term"
            );
            let mut oracle = FakeOracle::honest(net, tip);
            let b = resolve_birthday(&mut oracle, net, None, true, Some(created_at))
                .await
                .expect("resolves");
            assert_eq!(
                u32::try_from(u64::from(b.height())).unwrap(),
                expected_bundled_height(net, estimate) + 1,
                "{net:?}: birthday = bundled frontier + 1 for estimate(created_at)"
            );
            // The money property: at or below ~creation, never the sync-day tip.
            assert!(u64::from(b.height()) <= u64::from(estimate));
        }
    }

    #[test]
    fn resolve_birthday_stamp_never_raises_above_tip_lag() {
        // §8 (#356-F4): the stamp is a one-direction bound — when tip − lag is
        // LOWER than the estimate (a stamp at the write-clamp maximum, a tip
        // just above the bundle tail), the min keeps tip − lag: the stamp can
        // only ever WIDEN the scan relative to the unstamped fresh default.
        //
        // Driven at the REQUEST (`requested_birthday_height`) since T0-1c-R2: a
        // tip below the row is clamped up to it, so the only tips at which
        // `tip − lag < estimate` can hold sit inside `[row, row + lag)` — and
        // there the anchor ceiling (`bundled_treestate`) resolves both terms to
        // the same row, so the resolved birthday cannot tell the branches apart.
        // The request can.
        let net = Network::Main;
        let created_at = checkpoints::newest_checkpoint_time(net); // clamp max
        let row = crate::root_bind::newest_bundled_height(net);
        let tip_reference = row + NEW_WALLET_BIRTHDAY_LAG_BLOCKS / 2;
        let tip_lag = tip_reference - NEW_WALLET_BIRTHDAY_LAG_BLOCKS;
        assert!(
            checkpoints::estimate_birthday(net, created_at).value() > tip_lag,
            "precondition — the estimate must be the larger term"
        );
        let requested = requested_birthday_height(net, tip_reference, None, true, Some(created_at))
            .expect("resolves");
        assert_eq!(
            requested, tip_lag,
            "min picks tip − lag — identical to the unstamped fresh default"
        );
        assert_eq!(
            requested_birthday_height(net, tip_reference, None, true, None).expect("resolves"),
            tip_lag,
            "…which is what the unstamped fresh default requests"
        );
    }

    #[tokio::test]
    async fn resolve_birthday_stamped_reopen_avoids_the_activation_full_scan() {
        // §8 (#356-F4): `freshly_generated` is unknowable across a reopen
        // (`open` passes false), but the stamp's PRESENCE alone proves a
        // Generate-create on this device — an account-less reopen provisions
        // bounded-from-creation instead of the multi-year activation scan,
        // while a stamp-LESS reopen keeps the conservative activation floor.
        let net = Network::Main;
        let created_at = 1_700_000_000u64;
        let tip: u64 = 3_300_000;
        let estimate = checkpoints::estimate_birthday(net, created_at).value();

        let mut stamped = FakeOracle::honest(net, tip);
        let b = resolve_birthday(&mut stamped, net, None, false, Some(created_at))
            .await
            .expect("resolves");
        assert_eq!(
            u32::try_from(u64::from(b.height())).unwrap(),
            expected_bundled_height(net, estimate) + 1,
            "a stamped reopen takes the creation floor, not activation"
        );

        let activation = checkpoints::activation_height(net);
        let mut unstamped = FakeOracle::honest(net, tip);
        let b = resolve_birthday(&mut unstamped, net, None, false, None)
            .await
            .expect("resolves");
        assert_eq!(
            u32::try_from(u64::from(b.height())).unwrap(),
            activation + 1,
            "a stamp-less reopen keeps the §1.7 activation full-scan floor"
        );
    }

    #[tokio::test]
    async fn resolve_birthday_configured_wins_over_stamp() {
        // §8 (#356-F4): `config.birthday` is the host/user's explicit trusted
        // input — it takes precedence over the stamp in BOTH directions
        // (including a configured height BELOW the estimate: a restore-style
        // deep scan the user asked for must never be silently narrowed).
        let net = Network::Main;
        let configured = 2_000_000u32;
        let created_at = checkpoints::newest_checkpoint_time(net); // estimates far above
        let mut oracle = FakeOracle::honest(net, 3_300_000);
        let b = resolve_birthday(
            &mut oracle,
            net,
            Some(BlockHeight::new(configured)),
            true,
            Some(created_at),
        )
        .await
        .expect("resolves");
        assert_eq!(
            u32::try_from(u64::from(b.height())).unwrap(),
            expected_bundled_height(net, configured) + 1,
            "configured wins — the stamp is ignored"
        );
    }

    #[tokio::test]
    async fn resolve_birthday_zero_stamp_floors_to_activation() {
        // §8 (#356-F4): the broken-clock arm — a pre-epoch clock (or the
        // corrupt-bundle write clamp) stamps 0; the estimate degrades to the
        // activation row ⇒ a full scan. Safe-but-slow, never a fabricated
        // recent birthday.
        let net = Network::Test;
        let activation = checkpoints::activation_height(net);
        let mut oracle = FakeOracle::honest(net, 3_300_000);
        let b = resolve_birthday(&mut oracle, net, None, true, Some(0))
            .await
            .expect("resolves");
        assert_eq!(
            u32::try_from(u64::from(b.height())).unwrap(),
            activation + 1,
            "a zero stamp is the activation full scan, never ~tip"
        );
    }

    #[tokio::test]
    async fn resolve_birthday_above_tip_is_birthday_in_future() {
        // §8: a configured birthday above the live tip is rejected (config typo) —
        // but `configured == tip` exactly is VALID (the `>` fence-post, not `>=`).
        // The tip sits ABOVE the bundle's newest row, so it is the reference
        // (T0-1c-R2 M3: below the row, the row is — see
        // `a_configured_birthday_at_or_below_the_row_is_never_in_the_future`).
        let net = Network::Main;
        let tip = crate::root_bind::newest_bundled_height(net) + 1_000;
        let mut oracle = FakeOracle::honest(net, u64::from(tip));
        let r = resolve_birthday(
            &mut oracle,
            net,
            Some(BlockHeight::new(tip + 1)),
            true,
            None,
        )
        .await;
        assert!(matches!(r, Err(WalletError::BirthdayInFuture)), "got {r:?}");
        // the exact-tip boundary resolves (a regression to `>=` would break here)
        let mut at_tip = FakeOracle::honest(net, u64::from(tip));
        let r = resolve_birthday(&mut at_tip, net, Some(BlockHeight::new(tip)), true, None).await;
        assert!(
            r.is_ok(),
            "configured == tip is valid, not BirthdayInFuture: {r:?}"
        );
    }

    #[tokio::test]
    async fn resolve_birthday_floors_below_activation() {
        // §8: a configured birthday below activation floors to the activation row
        // (no shielded notes below). birthday = activation + 1.
        let net = Network::Main;
        let act = checkpoints::activation_height(net);
        let mut oracle = FakeOracle::honest(net, 3_000_000);
        let b = resolve_birthday(
            &mut oracle,
            net,
            Some(BlockHeight::new(act - 5)),
            true,
            None,
        )
        .await
        .expect("resolves");
        assert_eq!(u32::try_from(u64::from(b.height())).unwrap(), act + 1);
    }

    #[tokio::test]
    async fn resolve_birthday_restore_without_birthday_floors_to_activation() {
        // §8 (THE money fix, 3-lens): a RESTORE (freshly_generated=false) with NO
        // host birthday must floor to ACTIVATION (a full scan, §1.7) — NEVER ~tip,
        // which would silently skip the user's whole history (§2.3). Contrast with
        // a FRESH wallet (=true), which legitimately starts at tip−lag.
        for net in [Network::Main, Network::Test] {
            let act = checkpoints::activation_height(net);
            let tip: u64 = 3_300_000;

            // RESTORE + None ⇒ activation (birthday = activation + 1), NOT tip−lag
            let mut restore = FakeOracle::honest(net, tip);
            let b = resolve_birthday(&mut restore, net, None, false, None)
                .await
                .expect("resolves");
            assert_eq!(
                u32::try_from(u64::from(b.height())).unwrap(),
                act + 1,
                "{net:?}: restore w/o birthday scans from activation, never ~tip"
            );

            // FRESH + None ⇒ tip−lag (far above activation) — the contrast that
            // proves the restore floor is the freshly_generated flag, not a constant
            let mut fresh = FakeOracle::honest(net, tip);
            let bf = resolve_birthday(&mut fresh, net, None, true, None)
                .await
                .expect("resolves");
            assert!(
                u32::try_from(u64::from(bf.height())).unwrap() > act + 1,
                "{net:?}: a fresh wallet starts near tip, not activation"
            );
        }
    }

    #[tokio::test]
    async fn resolve_birthday_tip_far_above_bundle_tail_floors_to_the_anchor_ceiling() {
        // §8 (mobile 3-lens — the stale-bundle case the code comments name): a live
        // tip far above our NEWEST usable bundled checkpoint resolves to that row
        // (birthday = row + 1), a bounded rescan from there — never a silent wrong
        // anchor and never a fund skip.
        //
        // AMENDED 2026-09-06 (`ironwood-nu63-support.md` §1.5): the resolved row is
        // the newest AT OR BELOW THE ANCHOR CEILING, not the raw bundle tail. Above
        // the newest activation this build knows, the chain state can contain a
        // value pool we cannot model, and what we import there is immutable — so a
        // build anchors only where it can model the rules. With the current pin the
        // ceiling is below the bundle tail, which is exactly the Ironwood case.
        for net in [Network::Main, Network::Test] {
            let ceiling = crate::consensus::newest_known_activation(&net.consensus())
                .map_or(u32::MAX, u32::from);
            let expected_row = checkpoints::treestate_table(net)
                .iter()
                .map(|&(h, ..)| h)
                .rfind(|&h| h <= ceiling)
                .expect("a row at or below the ceiling");
            let tail = checkpoints::treestate_table(net)
                .last()
                .expect("non-empty")
                .0;
            let mut oracle = FakeOracle::honest(net, u64::from(tail) + 5_000_000);
            let b = resolve_birthday(&mut oracle, net, None, true, None)
                .await
                .expect("resolves");
            assert_eq!(
                u32::try_from(u64::from(b.height())).unwrap(),
                expected_row + 1,
                "{net:?}: a tip far above the bundle floors to the newest row the \
                 ceiling allows"
            );
            // Non-vacuity: with the current pin the ceiling must actually BITE on
            // mainnet, or this test would pass while asserting nothing new.
            if net == Network::Main {
                assert!(
                    expected_row < tail,
                    "the mainnet ceiling no longer bites — the bundle tail is now \
                     within modelled rules, so re-check whether the 13 Ironwood-window \
                     rows still need excluding"
                );
            }
        }
    }

    #[tokio::test]
    async fn provision_surfaces_transport_failure_typed() {
        // §8 (unstable-network): an unreachable endpoint surfaces typed
        // Sync { stall } carrying the policy reason — never a silent hang/panic.
        let net = Network::Main;
        let mut oracle = FakeOracle::honest(net, 3_000_000);
        oracle.identity_err = Some(GrpcError::Transport {
            stall: StallReason::TorUnavailable,
        });
        let r = resolve_birthday(&mut oracle, net, None, true, None).await;
        assert!(
            matches!(
                r,
                Err(WalletError::Sync {
                    stall: StallReason::TorUnavailable
                })
            ),
            "got {r:?}"
        );

        // a Timeout on the identity call preserves its stall reason too
        let mut t = FakeOracle::honest(net, 3_000_000);
        t.identity_err = Some(GrpcError::Timeout {
            stall: StallReason::EndpointUnreachable,
        });
        assert!(
            matches!(
                resolve_birthday(&mut t, net, None, true, None).await,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointUnreachable
                })
            ),
            "Timeout preserves stall"
        );

        // a fault on the SECOND call (tip) — and the Status arm → EndpointUnreachable
        let mut s = FakeOracle::honest(net, 3_000_000);
        s.tip_err = Some(GrpcError::Status { code: 14 }); // 14 = UNAVAILABLE
        assert!(
            matches!(
                resolve_birthday(&mut s, net, None, true, None).await,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointUnreachable
                })
            ),
            "tip-call Status fault → Sync{{EndpointUnreachable}}"
        );
    }

    /// T0-1c (§4j row 2; the unit guard behind E5): the provisioning tip's `u32`
    /// collapse says what `sync::fetch_tip` says for the identical lie —
    /// `EndpointMisbehaving`, "switch servers" — never the dead-link "check your
    /// connection" it used to spell out by hand (two answers to one question).
    /// The identity check runs first and PASSES here: the fake is an honest
    /// mainnet server whose one wrong answer is the tip, so the stall is the tip
    /// arm's and not the chain guard's (IT-10).
    #[tokio::test]
    async fn resolve_birthday_tip_beyond_u32_is_endpoint_misbehaving() {
        let net = Network::Main;
        let mut oracle = FakeOracle::honest(net, u64::from(u32::MAX) + 1);
        let r = resolve_birthday(&mut oracle, net, None, true, None).await;
        assert!(
            matches!(
                r,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "a tip beyond u32 is the content-tier stall, got {r:?}"
        );
    }

    #[tokio::test]
    async fn provision_never_provisions_from_a_live_frontier() {
        // §8 THE trust-model guard. The STRUCTURAL impossibility is the real
        // guarantee — `ChainOracle` exposes no `get_tree_state`, so a poisoned-
        // frontier fake is literally unconstructable. This test verifies the
        // *consequence*: the anchored birthday is purely bundle-derived — it equals
        // the bundled frontier for the resolved height, a value computable with ZERO
        // endpoint tree-state input — so no live frontier (poisoned or not) can move it.
        // T0-1c-R2 M3 sharpens the thesis: the tip is BELOW the bundle's row, so
        // not even the endpoint's tip enters the request — the row does.
        for net in [Network::Main, Network::Test] {
            let tip: u64 = 2_500_000;
            let row = crate::root_bind::newest_bundled_height(net);
            assert!(
                tip < u64::from(row),
                "{net:?}: the fixture tip sits below the row"
            );
            let mut oracle = FakeOracle::honest(net, tip);
            let b = resolve_birthday(&mut oracle, net, None, true, None)
                .await
                .expect("resolves");
            let requested = row - NEW_WALLET_BIRTHDAY_LAG_BLOCKS;
            assert_eq!(
                u32::try_from(u64::from(b.height())).unwrap(),
                expected_bundled_height(net, requested) + 1,
                "{net:?}: birthday is bundle-derived — from the row, not from any live \
                 frontier and not from a below-row tip"
            );
        }
    }

    #[test]
    fn resolve_birthday_lag_below_activation_floors_to_activation() {
        // §8 (gate-7 compound floor): a new wallet whose `tip − lag` lands at/below
        // Sapling activation floors to the activation row, never below — the
        // lag-vs-activation interaction (no configured birthday). Driven at the
        // REQUEST since T0-1c-R2: a tip that low is clamped up to the bundle's
        // row before the lag is taken, so end to end this arm of the floor is
        // unreachable on any network whose row exceeds `activation + lag` (both);
        // the arithmetic stays, and the configured-height arm still reaches it
        // (`resolve_birthday_floors_below_activation`).
        for net in [Network::Main, Network::Test] {
            let act = checkpoints::activation_height(net);
            // a reference chosen so reference − lag == activation exactly
            let reference = act + NEW_WALLET_BIRTHDAY_LAG_BLOCKS;
            assert_eq!(
                requested_birthday_height(net, reference, None, true, None).expect("resolves"),
                act,
                "{net:?}: reference − lag at activation requests activation"
            );
            assert_eq!(
                requested_birthday_height(net, reference - 1, None, true, None).expect("resolves"),
                act,
                "{net:?}: …and one below it floors to activation, never below"
            );
        }
    }

    /// T0-1c-R2 (§4n G3, M3) — a lying-low provisioning tip cannot floor the
    /// birthday toward activation: for a FRESH wallet with no host birthday, a
    /// tip far below the bundle's newest row resolves the SAME birthday a
    /// current endpoint at the row does — far above activation — and the
    /// request is `row − lag`, computable from the compiled bundle alone.
    /// (G3's "≥ row − lag" on the birthday itself is not satisfiable on either
    /// network: `bundled_treestate` caps every anchor at Nu6.3's activation —
    /// 3,428,143 mainnet / 4,134,000 testnet — which sits below `row − lag`;
    /// the birthday a CURRENT server gives is the ceiling row + 1, and that is
    /// what this row pins.) Mutant: the clamp removed (the reference reverted to
    /// the raw tip) — the lying-low birthday falls to the bundled row near the
    /// tip, and both assertions red.
    #[tokio::test]
    async fn a_lying_low_provisioning_tip_cannot_floor_the_birthday_below_what_the_row_gives() {
        for net in [Network::Main, Network::Test] {
            let row = crate::root_bind::newest_bundled_height(net);
            let activation = checkpoints::activation_height(net);
            // A tip a validator stuck years back would report: just past the
            // activation-plus-lag floor, ~3 M blocks below the row.
            let lying_low = u64::from(activation) + u64::from(NEW_WALLET_BIRTHDAY_LAG_BLOCKS) + 5;
            let mut liar = FakeOracle::honest(net, lying_low);
            let from_liar = resolve_birthday(&mut liar, net, None, true, None)
                .await
                .expect("a behind server still provisions (REPORT AND CONTINUE)");
            let mut current = FakeOracle::honest(net, u64::from(row));
            let from_current = resolve_birthday(&mut current, net, None, true, None)
                .await
                .expect("resolves");
            assert_eq!(
                from_liar.height(),
                from_current.height(),
                "{net:?}: the lying-low tip provisions the birthday a current server gives"
            );
            assert!(
                u32::from(from_liar.height()) > activation + 100_000,
                "{net:?}: never a birthday at activation (got {})",
                u32::from(from_liar.height())
            );
            assert_eq!(
                requested_birthday_height(net, row, None, true, None).expect("resolves"),
                row - NEW_WALLET_BIRTHDAY_LAG_BLOCKS,
                "{net:?}: the request is row − lag, a function of the compiled bundle"
            );
        }
    }

    /// T0-1c-R2 (§4n G6's population, M3 + §4m #13) — a configured birthday at
    /// or below the bundle's newest row is a height the chain provably reached,
    /// so a server whose tip is below it is BEHIND, not the birthday in the
    /// future: provisioning proceeds (and the pass then reports the standing).
    /// Above the row, against the same behind server, `BirthdayInFuture` stands
    /// — the chain may or may not have reached it. Mutant: the clamp removed
    /// (the first arm red: `BirthdayInFuture` for a height below the row).
    #[tokio::test]
    async fn a_configured_birthday_at_or_below_the_row_is_never_in_the_future() {
        for net in [Network::Main, Network::Test] {
            let row = crate::root_bind::newest_bundled_height(net);
            let behind_tip = u64::from(row) - 100_000;
            let mut behind = FakeOracle::honest(net, behind_tip);
            let r =
                resolve_birthday(&mut behind, net, Some(BlockHeight::new(row)), true, None).await;
            assert!(
                r.is_ok(),
                "{net:?}: a birthday AT the row against a behind server provisions, got {r:?}"
            );
            let mut behind = FakeOracle::honest(net, behind_tip);
            let r = resolve_birthday(
                &mut behind,
                net,
                Some(BlockHeight::new(row + 1)),
                true,
                None,
            )
            .await;
            assert!(
                matches!(r, Err(WalletError::BirthdayInFuture)),
                "{net:?}: one above the row against a behind server is still in the future, \
                 got {r:?}"
            );
        }
    }

    #[test]
    fn resolve_offline_birthday_decision_table() {
        // §8 (FR-24, pin): the OFFLINE eager-import resolver's full
        // decision table over the REAL mainnet bundle — which (configured,
        // stamp, freshly_generated) shapes import at create and which stay
        // with the lazy first-sync resolver. No oracle anywhere: every Some
        // arm is computable from the signed binary alone.
        let net = Network::Main;
        let tail = checkpoints::treestate_table(net)
            .last()
            .expect("non-empty")
            .0;
        let act = checkpoints::activation_height(net);

        // configured ≤ tail ⇒ Some, for ANY seed source (restore shape here);
        // the money property: birthday ≤ configured + 1 (bundled row + 1,
        // never above the host-asserted height).
        let configured = 2_000_000u32;
        assert!(configured <= tail, "precondition: mid-slice configured");
        let b = resolve_offline_birthday(net, Some(BlockHeight::new(configured)), None, false)
            .expect("resolves")
            .expect("configured-≤-tail imports eagerly");
        let got = u32::try_from(u64::from(b.height())).unwrap();
        assert_eq!(got, expected_bundled_height(net, configured) + 1);
        assert!(got <= configured + 1, "birthday never above configured + 1");

        // configured > tail ⇒ None (lazy — the BirthdayInFuture tip clamp's
        // job), EVEN for a stamped Generate: configured precedence is total.
        let r = resolve_offline_birthday(
            net,
            Some(BlockHeight::new(tail + 1)),
            Some(1_700_000_000),
            true,
        )
        .expect("resolves");
        assert!(
            r.is_none(),
            "above-tail configured stays lazy, even Generate"
        );

        // configured BELOW activation ⇒ Some at the activation-row floor
        // (birthday = activation + 1) — the floor edge, never below.
        let b = resolve_offline_birthday(net, Some(BlockHeight::new(act - 5)), None, false)
            .expect("resolves")
            .expect("below-activation configured floors, still eager");
        assert_eq!(u32::try_from(u64::from(b.height())).unwrap(), act + 1);

        // no-birthday non-generate ⇒ None (the activation-full-scan decision
        // belongs to first sync) — with or without a surviving stamp.
        for created_at in [None, Some(1_700_000_000)] {
            let r = resolve_offline_birthday(net, None, created_at, false).expect("resolves");
            assert!(r.is_none(), "no-birthday non-generate stays lazy");
        }

        // stampless Generate (created_at None — a pre-F4 remnant) ⇒ None.
        let r = resolve_offline_birthday(net, None, None, true).expect("resolves");
        assert!(r.is_none(), "stampless Generate stays lazy");

        // Generate + stamp ⇒ Some at the estimate_birthday(stamp)-derived
        // height (bundled row below the estimate, + 1).
        let created_at = 1_700_000_000u64; // Nov 2023 — mid-slice
        let estimate = checkpoints::estimate_birthday(net, created_at).value();
        let b = resolve_offline_birthday(net, None, Some(created_at), true)
            .expect("resolves")
            .expect("stamped Generate imports eagerly");
        let got = u32::try_from(u64::from(b.height())).unwrap();
        assert_eq!(got, expected_bundled_height(net, estimate) + 1);
        assert!(
            u64::from(b.height()) <= u64::from(estimate),
            "never above the creation estimate"
        );
    }

    #[test]
    fn endpoint_network_match_guards_chain_and_activation() {
        // §3.2f network-match (cheap correctness guard): an honest endpoint's
        // (chain_name, sapling_activation_height) matches; either field wrong is a
        // mismatch (the host/manifest-misconfig case).
        let id = |chain: &str, act: u64| ServerIdentity {
            chain_name: chain.to_owned(),
            sapling_activation_height: act,
            // Irrelevant to THIS guard, and that is the point: the chain guard
            // and the consensus-staleness predicate are different axes
            // (`ironwood-nu63-support.md` §2).
            consensus_branch_id: None,
            block_height: 0,
        };
        for net in [Network::Main, Network::Test] {
            let act = u64::from(checkpoints::activation_height(net));
            let name = checkpoints::chain_name(net);
            assert!(
                endpoint_network_matches(net, &id(name, act)),
                "honest match"
            );
            assert!(
                !endpoint_network_matches(net, &id("wrong", act)),
                "wrong chain_name"
            );
            assert!(
                !endpoint_network_matches(net, &id(name, act + 1)),
                "wrong sapling activation"
            );
        }
        // a mainnet wallet pointed at a testnet endpoint (the real misconfig)
        let test_act = u64::from(checkpoints::activation_height(Network::Test));
        assert!(!endpoint_network_matches(
            Network::Main,
            &id(checkpoints::chain_name(Network::Test), test_act)
        ));
    }
}
