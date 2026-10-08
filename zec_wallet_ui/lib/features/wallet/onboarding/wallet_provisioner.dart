import 'package:zec_wallet/zec_wallet.dart';

import '../wallet_session.dart';

/// Where a rescan starts (#317) — a sealed INTENT, not a bare number, so the
/// rebuilding banner can say honestly WHICH recovery the user chose and the
/// adapter can thread each arm through its own conversion.
sealed class RescanTarget {
  const RescanTarget();
}

/// The whole chain: the SDK floors to Sapling activation — slowest, most
/// thorough, never skips anything (the "Scan all history" button).
class RescanAllHistory extends RescanTarget {
  const RescanAllHistory();
}

/// From an explicit user-picked earliest date. NEVER clamped in either
/// direction: a pick BELOW the wallet's floor is the post-restore recovery
/// path (the floor may itself be a too-recent restore estimate), and a pick
/// above it is the user's informed speed trade-off (the sheet's copy states
/// that older funds stay hidden until a deeper rescan).
class RescanFromTime extends RescanTarget {
  const RescanFromTime(this.earliestTime);

  final DateTime earliestTime;
}

/// From the wallet's OWN scan floor (its birthday height) — the sheet's DEFAULT
/// for every wallet whose floor is readable (#317). This is the one
/// always-money-SAFE default: rescan is a DESTRUCTIVE, LOWER-ONLY rebuild, so a
/// start ABOVE the current floor would HIDE every note between the old floor and
/// the start (the SDK's LOWER-ONLY contract, `rescan_from`), and the floor is
/// the highest start that hides nothing. Being the wallet's birthday it also
/// never wastes a scan on empty pre-wallet blocks. It does NOT claim to recover
/// the user's every fund — a too-high restore birthday can leave real deposits
/// BELOW this floor; those are recovered by going lower (Pick-a-date / Scan-all),
/// never by this default. (When the floor is UNKNOWN — no account yet, or a
/// faulted read — the sheet defaults to [RescanAllHistory] instead, the only
/// safe "unknown floor" choice.)
class RescanFromWalletBirthday extends RescanTarget {
  const RescanFromWalletBirthday(this.floorHeight);

  /// The wallet's birthday height, read via `WalletSession.birthdayHeight()`
  /// by the sheet that constructs this.
  final int floorHeight;
}

/// The provisioning + backup-read seam used by ONBOARDING only — separate from
/// the sync-focused [WalletSession] port (design invariant 2 — hexagonal).
///
/// WHY its own port, not a few more methods on [WalletSession]:
///  1. It carries the SANCTIONED OUTBOUND key crossing ([revealMnemonic], spec
///     §3.3) that the everyday sync/balance UI must NEVER touch. Keeping it off
///     [WalletSession] means a screen that only renders the balance cannot even
///     reach the recovery words — the boundary is structural, not a convention.
///  2. The whole onboarding state machine ([OnboardingController]) must be
///     host-VM unit-testable with NO native library and NO device: a real FRB
///     `WalletHandle` is opaque and cannot be constructed in `flutter test`. The
///     controller depends on THIS interface and tests inject a fake; the
///     production adapter (`FrbWalletProvisioner`, wrapping a real `WalletHandle`
///     + a path_provider `dbDir` + the sealed-keychain `WalletConfig`) lands
///     with the on-device onboarding slice and overrides
///     [walletProvisionerProvider]. Rust stays the single source of truth
///     (invariant 1); every method forwards straight to the core.
///
/// LIFECYCLE: an implementation OWNS the underlying wallet handle across
/// [createGenerated]/[open] — [revealMnemonic] reads the SAME handle and the
/// returned [WalletSession] wraps it. Exactly one wallet is provisioned per
/// instance. (The deterministic handle close-on-teardown rides the FFI adapter,
/// not this slice — the controller tracks onboarding PHASE, not handle
/// ownership.)
///
/// BOUNDEDNESS CONTRACT (unstable-network reliability): [createGenerated] /
/// [open] do LOCAL work only — provisioning the SDK wallet is disk + keychain
/// (the single-writer lock, SQLCipher open, seed seal/unseal); it does NOT touch
/// the network, so the controller can `await` it without a network timeout. The
/// first-sync round-trip to lightwalletd (e.g. resolving a null birthday to the
/// current tip) is the SYNC engine's job and surfaces as a `SyncStatus.stalled`
/// EVENT on the live stream — never as a hang here. An implementation MUST still
/// resolve in BOUNDED time (the impl owns the bound — a wedged keychain/fsync
/// must time out to a typed [WalletErrorKind.keystoreUnavailable] so the
/// onboarding flow lands on a retryable failure, not a permanent spinner) and
/// MUST NOT block on the network. The controller relies on this contract instead
/// of imposing its own timeout (which would fight the SDK's).
///
abstract interface class WalletProvisioner {
  /// Whether a wallet is already provisioned at the host data dir (a prior
  /// [createGenerated] completed). Drives the boot fork: no wallet → offer
  /// create; a wallet → open it and gate on backup confirmation.
  Future<bool> walletExists();

  /// Create a BRAND-NEW wallet: the SDK generates a fresh 24-word seed in Rust
  /// (`OsRng`) and seals it under the device keychain — the seed NEVER crosses
  /// the bridge. Returns the live [WalletSession]. The caller MUST drive the
  /// recovery-phrase backup ([revealMnemonic]) and confirm it before the wallet
  /// is presented as deposit-ready (money-safety; the [OnboardingController]
  /// gate). Throws (typed) if a wallet already exists here — call [walletExists]
  /// first; the non-clobber is the contract, never a silent overwrite.
  Future<WalletSession> createGenerated();

  /// Open the EXISTING provisioned wallet at the host data dir. Throws (typed)
  /// for not-found / another-instance-open / network-mismatch / device-locked /
  /// an interrupted create — open does NOT repair a remnant (it throws
  /// `provisioningIncomplete`); CREATE resumes the repair from the sealed seed,
  /// which is why the boot fork routes a remnant (`walletExists() == false`) to
  /// create, never here.
  Future<WalletSession> open();

  /// RESTORE an existing wallet from its BIP39 recovery phrase — the ONE
  /// sanctioned INBOUND key crossing (spec §3.3; counterpart to the outbound
  /// [revealMnemonic]). [mnemonicWords] are the recovery words in order.
  ///
  /// THE LOWERCASE CONTRACT (a HARD money-reliability requirement): the words
  /// MUST already be lowercased + trimmed — the SDK keeps the audited `bip39`
  /// parser whole and does NOT case-fold, so a mis-cased/space-padded word is
  /// rejected as a typed `invalidMnemonic` carrying its INDEX and a CORRECT
  /// backup would fail to restore. The [OnboardingController] enforces this at
  /// its `startRestore` chokepoint via `normalizeMnemonicInput`; this method
  /// trusts that contract (it does not re-normalize — re-casing key material is
  /// the controller's single responsibility, kept off the device adapter).
  ///
  /// [approximateCreationTime] is the OPTIONAL wallet-creation time used to floor
  /// the restore scan (faster than a full history scan). The adapter converts it
  /// to a birthday HEIGHT via the SDK's conservative `estimateBirthday` and the
  /// configured network, then rides it on `config.birthdayHeight` (the ONE source
  /// of truth — create/open read the same field). `null` means "I don't know":
  /// the scan floors to Sapling activation — a full, slower, but money-SAFE scan
  /// that never silently skips older funds (NEVER ~tip).
  ///
  /// LOCAL-ONLY, same BOUNDEDNESS CONTRACT as [createGenerated]: BIP39
  /// validation + seed seal + SQLCipher provision are disk + keychain; the first
  /// chain round-trip is the sync engine's job. NON-CLOBBER: throws (typed)
  /// `walletAlreadyExists` over a completed wallet (the sealed seed is NEVER
  /// overwritten) and `seedMismatch` if a DIFFERENT phrase is restored over an
  /// interrupted-create remnant — both are structural in the SDK, so the boot
  /// fork (restore offered only from Welcome, i.e. `walletExists() == false`)
  /// rarely sees them.
  ///
  /// KEY RESIDUE (§10): the inbound word-list copy lives in Dart memory (which
  /// cannot be zeroized) only until Rust owns it as a `SeedSource`; the same
  /// minimal, documented residue as the outbound reveal — minimised, not
  /// eliminated. No BIP39 passphrase ("25th word") is exposed here: the
  /// reference app restores standard phrases (matching its passphrase-free
  /// create path); a passphrase wallet is an expert case, deferred
  /// (manager-flagged).
  Future<WalletSession> restore(
    List<String> mnemonicWords, {
    DateTime? approximateCreationTime,
  });

  /// Create a WATCH-ONLY wallet from an exported unified full viewing key
  /// (#397 §3.7 D2 / ADR-0538 — the consumer half of [exportUfvk]). [ufvk] is
  /// the standard `uview1…` / `uviewtest1…` string; garbage, a truncated
  /// artifact, or a WRONG-network key throw a typed
  /// [WalletErrorKind.invalidViewingKey] (a well-formed cross-network key is
  /// [WalletErrorKind.networkMismatch]) BEFORE any store side effect — the
  /// controller renders it as a fixable input fault, exactly like a bad
  /// mnemonic word. NOT a spend-key crossing: a UFVK is viewing capability
  /// (total history visibility, no spend, no seed), so a watch-only wallet
  /// has NOTHING to back up and goes straight to Active (no
  /// backup-confirmation step).
  ///
  /// [birthdayHeight] is REQUIRED (unlike [restore]'s optional creation time):
  /// a watch-only import has no lazy seed-path, so the account is imported
  /// eagerly at this floor. Derive it from the user's picked date via
  /// [estimateBirthdayHeight]. Too-high hides older history (the same contract
  /// as a seed restore); too-low only scans longer.
  ///
  /// Same BOUNDEDNESS + NON-CLOBBER contract as [restore]. A crash-window
  /// account-less remnant CONVERGES on a re-run with the same artifact; a
  /// DIFFERENT artifact over an existing watch-only store throws
  /// `walletAlreadyExists`.
  Future<WalletSession> createWatchOnly(
    String ufvk, {
    required int birthdayHeight,
  });

  /// Rescan the wallet from an EARLIER birthday to recover funds an over-high
  /// restore birthday skipped (ADR-0534 — the in-app fix for the ADR-0533
  /// "balance reads zero, sync says done" gap). [target] chooses the floor:
  /// [RescanFromTime] is converted to a birthday HEIGHT via the SDK's
  /// CONSERVATIVE `estimateBirthday` (never past the time → never skips
  /// notes) and the configured network, exactly as [restore] threads its
  /// birthday; [RescanFromWalletBirthday] passes the wallet's own floor
  /// height verbatim (the sheet's default — see the type doc);
  /// [RescanAllHistory] scans the FULL history (floors to Sapling activation
  /// — slower, but money-SAFE; it never silently skips older funds, NEVER
  /// ~tip). NO key material crosses — the seed stays sealed in the keychain;
  /// the only thing passed is a bare height.
  ///
  /// Returns a FRESH [WalletSession] over the rebuilt wallet on success. The
  /// caller MUST swap it into the active gate ([OnboardingController]): the
  /// rebuilt data DB is account-less + EMPTY until the next sync repopulates it,
  /// and the live-sync graph keys off session IDENTITY — a fresh session is what
  /// makes sync re-subscribe from the lower birthday and the balance/history
  /// re-read. Durable money state (queued sends, the refund-address index,
  /// in-flight swap detection) is PRESERVED across the rebuild.
  ///
  /// THROWS (typed `WalletApiError`) on ANY rescan FAULT and LEAVES THE HANDLE
  /// CLOSED (a `StateError` for the precondition violation of calling this before
  /// a create/open — a programming error, not a fault) — ADR-0534
  /// guarantees the seed seal + a COMPLETE data DB are intact on every error
  /// path, so the caller RECOVERS by calling [open] again. Which DB that is
  /// depends on where the fault hit (#379 docs truth): before the rebuild's
  /// atomic rename it is the PRIOR wallet, unchanged; on the post-rename fault
  /// arms (dir-fsync / cache reset / re-open) it is the REBUILT lower-birthday
  /// wallet, whose balance/history repopulate via the auto-restarted sync. NO
  /// funds are lost on either arm, and the error kind does not distinguish
  /// them. The [OnboardingController] owns that recover-by-reopen routing (it
  /// owns the onboarding state), which is why this method does not re-open
  /// itself.
  ///
  /// LOCAL-ONLY (disk + keychain; stops + joins sync first, then rebuilds the
  /// data DB under the single-writer lock — bounded, no network). The first chain
  /// round-trip is the sync engine's job and surfaces as a `SyncStatus` event,
  /// never a hang here. Valid only after a [createGenerated]/[open]/[restore] on
  /// this instance (the same handle ownership the port documents above).
  Future<WalletSession> rescanFrom(RescanTarget target);

  /// Switch the active wallet onto [choice] and REMEMBER it (the picker,
  /// P3-13 — `sync-server-picker.md` D3): the SDK probes the server, stops and
  /// joins the sync loop, writes the choice, and rebuilds the session over the
  /// SAME database — no rescan, no re-download; funds, history, queued sends
  /// and the sync verdict are untouched. Returns a FRESH session over the same
  /// handle, the [rescanFrom] shape, so the live-sync graph (keyed on session
  /// identity) rebuilds and re-subscribes; sync auto-restarts on the new
  /// server.
  ///
  /// A refusal BEFORE the swap (`syncServerUnreachable`, `networkMismatch`,
  /// `syncServerNotOffered`, `invalidEndpoint`, a `walletBusy` in any phase
  /// but `switchingServer`) leaves the handle OPEN and everything as it was —
  /// the typed `WalletApiError` propagates and the caller keeps its session.
  /// A fault PAST the stop-join leaves the handle CLOSED (the `rescanFrom`
  /// contract): the [OnboardingController] owns the recover-by-reopen. Valid
  /// only after a [createGenerated]/[open]/[restore] on this instance.
  Future<WalletSession> switchSyncServer(SyncServerChoice choice);

  /// The SDK's conservative DateTime→height estimate for THIS wallet's
  /// network (#317) — the same floor [restore] and [RescanFromTime] convert
  /// through, exposed so the rescan sheet can show an honest "about N blocks
  /// to scan" size cue for a picked date BEFORE the user commits. Synchronous
  /// + local (bundled checkpoints; no network, no key material). Display
  /// guidance only — the binding conversion stays inside [rescanFrom].
  int estimateBirthdayHeight(DateTime time);

  /// The current wallet's BIP39 recovery words, in index order — the ONE
  /// sanctioned OUTBOUND key crossing (spec §3.3). Valid only after a
  /// [createGenerated]/[open] on this instance. The words ARE the whole secret:
  /// the caller shows them ONCE on a FLAG_SECURE screen, never logs / persists /
  /// screenshots / transmits them. Dart memory cannot be zeroized — the
  /// documented §10 residue; the Rust side wipes its own copies when this
  /// returns. Throws [WalletErrorKind.noMnemonic] for a raw-seed wallet (the
  /// host-supplied-seed path; its recovery is the host's own master phrase,
  /// never a wallet-local phrase).
  Future<List<String>> revealMnemonic();

  /// Export this wallet's UNIFIED FULL VIEWING KEY — the ONE sanctioned UFVK
  /// egress (#397 §3.7 D1 / ADR-0538; the outbound sibling of [revealMnemonic],
  /// gated by the reference UI at the SAME backup-phrase bar). Returns the
  /// standard `uview1…` / `uviewtest1…` string. Valid only after a
  /// create/open/restore/createWatchOnly on this instance (works at every
  /// custody tier including watch-only — re-export is an identity operation).
  ///
  /// **HOST CONTRACT (LOUD):** the returned string grants TOTAL HISTORY
  /// VISIBILITY — every incoming AND outgoing payment, past and future — to
  /// anyone who holds it; it cannot spend and cannot reach the seed. The
  /// export screen states this (the D9 warning copy) and gates behind a
  /// deliberate re-auth. Unlike a seed a UFVK is PUBLIC-once-shared, so it is
  /// copyable + QR-able (the divergence from the backup screen); still never
  /// logged (§5.4). Bounded like [revealMnemonic].
  Future<String> exportUfvk();

  /// The PRODUCTION per-tier custody disclosure (FR-14 H1) for THIS wallet — the
  /// honest "are my keys hardware-backed / what does delete do" answer a host
  /// renders BEFORE [deleteWallet] and as an ambient custody badge. Reads ONLY
  /// the measured vault tier (no seed, no unseal, NO key material). Bounded like
  /// the other local steps. A `tier == "none"` (headless desktop) or an
  /// `EraseAssurance.bestEffort` tier is HONEST best-effort — the host MUST
  /// disclose the flash-recovery residual. No tier is permanent erasure
  /// (ADR-0571).
  Future<CustodyDisclosure> custodyDisclosure();

  /// Delete this wallet (FR-14): the "delete wallet" / "reset" primitive.
  /// CLOSES the live handle first (a wipe refuses while an instance holds the
  /// single-writer lock — `WalletOpen`), THEN deletes the keychain wrap key (so
  /// this key store can no longer open the on-disk seals or the DB ciphertext;
  /// [custodyDisclosure] says how strongly, ADR-0571) and removes the data
  /// directory. The seed NEVER crosses the bridge.
  ///
  /// THROWS (typed `WalletApiError`) on a wipe FAULT and may LEAVE THE HANDLE
  /// CLOSED — but the keychain-first ordering means a fault deletes NOTHING (the
  /// seals + files are intact), so the caller RECOVERS the still-usable wallet by
  /// calling [open] again (the [OnboardingController] owns that recover-by-reopen
  /// routing, exactly as it does for [rescanFrom]). Bounded (a wedged keychain
  /// surfaces as the typed retryable timeout, never a permanent spinner). After a
  /// SUCCESSFUL shred this instance no longer owns a wallet — the caller resets
  /// onboarding to Welcome.
  Future<void> deleteWallet();

  /// FORCE-complete a [deleteWallet] for a wallet that cannot be opened and whose
  /// custody key is ALREADY GONE — the #251 escape out of a non-retryable
  /// `OnboardingFailed` (`needsRecovery`). The plain [deleteWallet] fails CLOSED
  /// (`keystoreInconsistent`) when its verify-real-sever guard finds no key to
  /// sever; this SKIPS that guard and removes the unreadable remnant's files. It
  /// MUST be called DELIBERATELY — only AFTER a plain [deleteWallet] reported
  /// `keystoreInconsistent`, with the device UNLOCKED (the SDK `wipe_force`
  /// contract: forcing past the guard could otherwise delete the wrong files). The
  /// seed is NOT on the device — funds are recovered from the recovery phrase the
  /// user then enters — so removing the remnant loses no money. THROWS on a genuine
  /// filesystem fault (the caller re-probes to a still-escapable failure, never a
  /// dead-end). After success this instance owns no wallet — the caller routes to
  /// the restore form.
  Future<void> forceDeleteWallet();
}
