//! FR-33 — the FRB boundary's OWN wire-contract version, readable BY NAME.
//!
//! The Dart package and this native library can be paired from different
//! builds (a pub.dev package over a prebuilt library, a host that builds the
//! native half on its own schedule). flutter_rust_bridge's one pairing check,
//! the generated content hash, did not move across eight DTO-changing commits
//! (FR-33's measurement), so a skewed pair initialised cleanly and decoded a
//! `TorRuntimeKind` one tag off — "Tor active", in the protected tone, for a path
//! that may have declared itself exposed.
//!
//! The fix is the discipline the C boundary already has
//! (`ZW_NET_DIALER_ABI_VERSION`), applied to the FRB boundary:
//!
//! * [`BRIDGE_ABI_VERSION`] names the wire contract `frb_generated.rs` encodes.
//!   What FORCES it to move is the crate's build script
//!   (`build.rs::bridge_abi_gate`): the generated codec's fingerprint is
//!   recorded against each version in the append-only [`BRIDGE_ABI_HISTORY`],
//!   and any build over a regen that changed the wire code FAILS until the next
//!   version is appended — the file moves, the version must move. A build, not a
//!   test, because no test runs at commit time on this track (the FR-33 review's
//!   MEDIUM); the pre-commit `sdk-clippy` leg builds.
//! * [`zec_wallet_bridge_abi_version`] is a plain C export OUTSIDE `crate::api`,
//!   so FRB never wraps it. Dart resolves it by NAME: a version query sent
//!   THROUGH the bridge would be dispatched by FRB's numeric function id, and on
//!   a mismatched pair those ids may name different functions.
//! * The Dart side (`lib/src/bridge_abi.dart`, `lib/src/rust_lib.dart`) holds
//!   `kBridgeAbiVersion` and checks it in `RustLib.init` BEFORE the generated
//!   init runs — so a mismatched pair exchanges no bridge message at all. The
//!   build script also binds the Dart constant to this one.
//!
//! Unsafe posture: the module denies `unsafe`; the one allowance is the export
//! attribute on the one function (no `unsafe` block exists here).

#![deny(unsafe_code)]

/// The bridge's wire-contract version. Bump it — and `kBridgeAbiVersion` in
/// `lib/src/bridge_abi.dart`, and append the new row to [`BRIDGE_ABI_HISTORY`]
/// — whenever `frb_generated.rs`'s wire code changes. The crate's BUILD SCRIPT
/// refuses a build that did not (`build.rs::bridge_abi_gate`), and prints the
/// row to append.
pub const BRIDGE_ABI_VERSION: u32 = 11;

/// Every wire contract the bridge has had: `(version, fingerprint of
/// frb_generated.rs)`, read by `build.rs::bridge_abi_gate` — keep one row per
/// line in this exact shape. APPEND-ONLY, like the frozen derivation labels: a
/// row is never edited in place (that would re-point a released version at a
/// different contract); a changed codec — including one reverted to an older
/// codec — gets the NEXT version.
const BRIDGE_ABI_HISTORY: &[(u32, u64)] = &[
    // 1 — FR-33, (2026-09-18): the first recorded contract — the codec as
    // of FR-5 C1 (ABI v3 host dialer, `BuiltIn` removed) under the renamed
    // `ZecWalletRustLib` entry class, in its canonical (`cargo fmt -p
    // zec_wallet`) form, as `wallet-bridge-verify` pins it.
    (1, 0x48d9_183d_7dfb_001a),
    // 2 — FR-35, (2026-09-19): the host-switched device log. Two sync
    // functions (`set_device_log`, `device_log_level`) and one enum
    // (`DeviceLogLevel { Off, Errors, Detailed }`) join the surface; every
    // function id after them in the generated dispatch table moves, which is
    // exactly the skew this version exists to refuse at init.
    (2, 0x3261_9c38_76d2_2893),
    // 3 — stage S1 `truth`, (2026-09-19): the private path tells the
    // truth. One `TorState` variant (`Unanswered { runtime }` — ready, and
    // nothing has come back for a minute), two verbs (`watch_tor_state`, the
    // live state stream; `dial_counts`, dials by arm × outcome) and the DTO
    // they carry (`DialCounts { private, clearnet: ArmCounts }`). A Dart half
    // generated before this decodes the new variant one tag off, which is the
    // skew FR-33 exists to refuse.
    (3, 0xc617_9e19_b657_2210),
    // 4 — stage S8, (2026-09-21): payment identity and durable retry.
    // One `SwapErrorKind` variant (`QuoteTermsDiffer`, RW-SWAP-016 — the
    // caller's DTO differs from the SDK's issued record), inserted before
    // `Unknown` so `Unknown`'s wire tag moved; one enum (`DeliveryState`:
    // persisted / retryPending / accepted / confirmed / unknown), one field
    // (`TxSummary.delivery`) and one verb (`delivery_state`) — the four
    // delivery states a host reads per transaction (ADR-0555, ADR-0556).
    (4, 0xa1c8_3740_a115_8ba8),
    // 5 — stage S2 build B, (2026-09-23): the host's lifecycle. One verb
    // (`sync_for`, the caller-bounded pass) and its DTO (`BoundedSync {
    // scanned_to, tip, finished, resubmitted }`); one `WalletErrorKind`
    // variant (`SyncRunning`, RW-SYNC-005); two fields
    // (`TxSummary.expiry_height`, the height a created send's unknown outcome
    // resolves at; `SendProposal.single_recipient_zat`, FR-46's signed total
    // to one recipient — ADR-0562).
    (5, 0x0df4_c493_73d5_1974),
    // 6 — stage S5 `sink`, (2026-09-28): the device log a host receives.
    // One verb (`watch_device_log`, the host's stream of the SDK's own
    // device-log lines), its DTO (`DeviceLogLine { seq, severity, tag, text }`)
    // and one enum (`DeviceLogSeverity { Error, Warn, Info }`). The verb is
    // appended to the dispatch table as the new highest id (74); no earlier
    // id moves.
    (6, 0x009c_d3ea_7f9e_6e46),
    // 7 — stage S16 `bridge`, (2026-09-28): the duress force-sever
    // (FR-53). One verb (`WalletHandle::sever_custody`, its deadline in whole
    // milliseconds), its DTO (`SeverReport { severed, holder, files }`) and
    // five enums (`SeverOutcome`, `UnprovenReason`, `NotSeveredCause`,
    // `HolderSeen`, `FilesOutcome`, each with an `Unknown` arm). The verb
    // sorts into the middle of the handle's methods (id 38), so every
    // function id after it moves — the skew this version exists to refuse.
    // Also `sever_answer_grace_ms` (the overrun's slack), and the overrun's
    // own answer, `NotSeveredCause::StillRunning` + `FilesOutcome::StillInUse`.
    // This row's hash was RE-RECORDED at the S16 bridge diff-review fold,
    // BEFORE version 7 ever left the stage branch (no build carried the first
    // hash). The never-edit rule protects a version a host may have shipped;
    // no host had this one. Re-recorded, not bumped to 8: the ruling.
    (7, 0x1703_f81f_9d4a_dbbf),
    // 8 — R10, (2026-09-30): a local stall names the right remedy. One
    // `StallReason` variant (`StorageUnavailable` — a busy or I/O-faulted
    // store, "retrying", never "restore"), inserted before `Unknown` so
    // `Unknown`'s wire tag moved.
    (8, 0x172f_624c_c7b8_3c64),
    // 9 — SRV-KEY (ADR-0568), (2026-10-01): the SDK ships no gated server; a
    // user's server may carry a key. `reference_sync_servers` loses its
    // `gated_key` argument; `SyncServerChoice::Custom` gains `key:
    // Option<SyncServerKey>` (a new DTO, `SyncServerKey { header, value }`);
    // one `WalletErrorKind` variant (`InvalidEndpointAuth`, RW-CFG-004),
    // inserted beside the RW-CFG family so every later tag moved.
    (9, 0x7a6d_1816_955f_cc18),
    // 10 — the 2026-10-05 review's fixes (2026-10-06): `CustodyDisclosure.
    // crypto_erases_on_delete: bool` becomes `erase_assurance: EraseAssurance`
    // (a new enum, `HardwareKeyDeleted | BestEffort | Unknown`; ADR-0571 — a
    // deleted key is no longer reported as permanently unrecoverable), and one
    // `WalletErrorKind` variant (`BroadcastJitterTooLong`, RW-CFG-005, F01),
    // inserted beside the RW-CFG family so every later tag moved.
    (10, 0x7263_2a9b_b7b4_f4d8),
    // 11 — the 2026-10-07 external review's swap fix: one `ProviderProtocolReason`
    // variant (`RequestEchoMismatch`, the quote door's echo check), inserted
    // before the `Unknown` arm, so `Unknown`'s tag moved.
    (11, 0xd785_26d3_9ea0_c8ca),
];

// The one check the language can make itself: the version is the history's
// last. The fingerprint and the Dart constant are the build script's (a const
// panic cannot print the row to append).
const _: () = assert!(BRIDGE_ABI_HISTORY[BRIDGE_ABI_HISTORY.len() - 1].0 == BRIDGE_ABI_VERSION);

/// The bridge's wire-contract version, for the Dart package to compare with the
/// one it was generated against before it sends a single bridge message. A C
/// export resolved by NAME (never through FRB's function ids). Pure: no
/// argument, no state, cannot fail.
#[allow(unsafe_code)] // the `no_mangle` export attribute only
#[unsafe(no_mangle)]
pub extern "C" fn zec_wallet_bridge_abi_version() -> u32 {
    BRIDGE_ABI_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The export answers the constant (pinned so the symbol's existence and
    /// value are part of the suite, not only of the device walk).
    #[test]
    fn the_c_export_reports_the_bridge_abi_version() {
        const { assert!(BRIDGE_ABI_VERSION >= 1) };
        assert_eq!(zec_wallet_bridge_abi_version(), BRIDGE_ABI_VERSION);
    }
}
