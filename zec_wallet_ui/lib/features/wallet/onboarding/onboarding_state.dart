import 'package:zec_wallet/zec_wallet.dart';

import '../wallet_session.dart';

/// The onboarding state machine's states (spec §3.2g iii-B). A sealed family so
/// the screen layer renders each phase with an exhaustive `switch` (no default
/// blind spot) and the money-safety gate is a single, checkable predicate:
/// a deposit-capable [WalletSession] is reachable ONLY through
/// [OnboardingActive] — i.e. ONLY after the recovery-phrase backup is
/// confirmed-and-persisted.
///
/// Holds NO recovery words: the words are fetched on demand by the (ephemeral)
/// backup screen via `WalletProvisioner.revealMnemonic` and dropped when it
/// disposes — never parked in this long-lived controller state (key-residue
/// minimization; the §10 Dart exposure is kept as small as possible).
sealed class OnboardingState {
  const OnboardingState();
}

/// Boot probe in flight (is a wallet on disk? open it; is backup confirmed?).
/// The transient initial state; the UI shows a neutral spinner.
class OnboardingLoading extends OnboardingState {
  const OnboardingLoading();
}

/// No provisioner/store wired in this build — onboarding cannot run. The
/// production default until the on-device FFI adapter lands; the wallet surface
/// renders its honest not-set-up state (no deposit invited).
class OnboardingUnavailable extends OnboardingState {
  const OnboardingUnavailable();
}

/// No wallet on disk → offer to create one OR restore from a recovery phrase.
/// The entry point of both provisioning flows.
class OnboardingWelcome extends OnboardingState {
  const OnboardingWelcome();
}

/// `createGenerated` in flight (seed generated + sealed in Rust). Transient.
class OnboardingGenerating extends OnboardingState {
  const OnboardingGenerating();
}

/// The restore words-entry screen is showing (entered from [OnboardingWelcome]
/// via "Restore from a recovery phrase"). Carries an optional inline [fault] so a
/// FIXABLE input problem — a mistyped word, a too-recent birthday — returns the
/// user to THIS form with an honest message, never a full-screen failure that
/// would discard the phrase they typed.
///
/// Holds NO recovery words (the §10 discipline, identical to the rest of this
/// family): the entered phrase lives ONLY in the restore screen's ephemeral
/// `TextEditingController`, dropped when that screen unmounts — never parked in
/// this long-lived controller state.
class OnboardingRestoreInput extends OnboardingState {
  const OnboardingRestoreInput({this.fault, this.invalidWordIndex});

  /// The fixable input fault to surface inline, or `null` on first entry.
  final RestoreInputFault? fault;

  /// The 1-based word position to point the user at — set ONLY for
  /// [RestoreInputFault.invalidWord] AND only when the SDK reported an index
  /// (a checksum failure has no single offending word, so this stays `null`).
  final int? invalidWordIndex;
}

/// `restore` in flight (BIP39 validation + seed seal in Rust). Transient. The
/// restore screen is rendered for this state too (not a separate busy view) so
/// the typed phrase in its controller survives a return to
/// [OnboardingRestoreInput] on a fixable fault — the user never retypes 24 words
/// after a single-word typo.
class OnboardingRestoring extends OnboardingState {
  const OnboardingRestoring();
}

/// The WATCH-ONLY import screen is showing (entered from [OnboardingWelcome]
/// via "Watch a wallet"; #397 §3.7 D5). The user pastes/scans a unified full
/// viewing key + picks the wallet's creation date. Carries an optional inline
/// [fault] so a FIXABLE input problem (a malformed key, a wrong-network key, a
/// too-recent date) returns to THIS form with an honest message, never a
/// full-screen failure — exactly like [OnboardingRestoreInput].
///
/// Holds NO viewing key: the pasted string lives ONLY in the input screen's
/// ephemeral `TextEditingController` (a UFVK is public, but the smallest-scope
/// discipline still applies) — never parked in this long-lived controller state.
class OnboardingWatchOnlyInput extends OnboardingState {
  const OnboardingWatchOnlyInput({this.fault});

  /// The fixable input fault to surface inline, or `null` on first entry.
  final WatchOnlyInputFault? fault;
}

/// `createWatchOnly` in flight (UFVK decode + account import in Rust).
/// Transient. The watch-only input screen is rendered for this state too so the
/// pasted key + picked date survive a return to [OnboardingWatchOnlyInput] on a
/// fixable fault — the user never re-pastes after a wrong-network typo.
class OnboardingCreatingWatchOnly extends OnboardingState {
  const OnboardingCreatingWatchOnly();
}

/// The wallet EXISTS but the recovery-phrase backup is NOT yet confirmed — the
/// money-safety holding state. NOT deposit-ready: the wallet surface stays
/// not-set-up, and a crash/restart here RE-ENTERS this state (never
/// [OnboardingActive]). The backup screen reveals the words here.
class OnboardingAwaitingBackup extends OnboardingState {
  const OnboardingAwaitingBackup(this.session, {this.hasPriorActivity = false});

  /// The live wallet, held so the active state can adopt it on confirm without
  /// re-opening. NOT exposed to the wallet surface until [OnboardingActive].
  final WalletSession session;

  /// Whether this wallet shows signs of a PRIOR LIFE (it has synced, or holds
  /// a balance) — set by the boot probe's local read, `false` for a
  /// this-session create/restore. The forced-backup "Start over" escape
  /// (#356-F2) is suppressed when `true` (security HIGH): a wallet can
  /// only land back here WITH history if its confirmed flag was lost (a
  /// silently-reset prefs file over an intact wallet DB), and this screen
  /// renders no balance — so without this bit, a FUNDED wallet would offer a
  /// fresh-create-flavored delete while looking exactly like an empty one.
  /// Fail-safe direction: an unreadable probe sets `true` (the escape hides;
  /// the screen degrades to the pre-F2 posture — re-confirm the backup —
  /// which is annoying, never dangerous).
  final bool hasPriorActivity;
}

/// Persisting the user's backup confirmation. Transient; on success →
/// [OnboardingActive], on a persist failure → back to [OnboardingAwaitingBackup]
/// (the gate stays closed — never deposit-ready without a durable confirm).
class OnboardingConfirming extends OnboardingState {
  const OnboardingConfirming(this.session, {this.hasPriorActivity = false});

  final WalletSession session;

  /// Threaded from [OnboardingAwaitingBackup] so a persist-failure revert
  /// restores the same posture (and the screen's Start-over visibility never
  /// flickers across the confirm round-trip).
  final bool hasPriorActivity;
}

/// Backup confirmed-and-persisted → the wallet is deposit-ready and the sync UI
/// is live. The ONLY state that exposes a [WalletSession] to the wallet surface.
class OnboardingActive extends OnboardingState {
  const OnboardingActive(
    this.session, {
    required this.identityEpoch,
    required this.isWatchOnly,
  });

  /// GATE DISCIPLINE (the money-safety SSOT): `walletSessionProvider` is the
  /// ONE reader of this field. The `session` on [OnboardingAwaitingBackup] /
  /// [OnboardingConfirming] is threaded forward only, never exposed. Do NOT add
  /// a second reader of any pre-active session — the deposit gate's whole
  /// correctness is "a session reaches the UI iff state is OnboardingActive".
  /// The ONE sanctioned exception (#356-F2): the boot probe's bounded,
  /// controller-internal `snapshot()` read that computes
  /// [OnboardingAwaitingBackup.hasPriorActivity] — local, side-effect-free,
  /// and never surfaces the session or its data to any UI. It is not
  /// precedent for a reader that does.
  final WalletSession session;

  /// The WALLET-LIFE identity this active session belongs to (#381 (c)).
  /// [OnboardingController] assigns a fresh value every time a DIFFERENT
  /// wallet becomes active (boot open, create-confirm, restore) and keeps
  /// the SAME value across a rescan's session swap / fault-recovery reopen
  /// (same wallet, new handle). `walletIdentityProvider` derives the money
  /// surface's retention key from this, so last-known balance/list values
  /// survive a same-wallet swap but can NEVER carry across a delete →
  /// create/restore into the next wallet's first frames. The value is only
  /// ever compared for equality — never persisted, never logged with money
  /// context; it counts wallet LIVES this process, which is not sensitive.
  final int identityEpoch;

  /// Whether this active wallet is WATCH-ONLY (#397 §3.7 D4/D5) — the immutable
  /// wallet KIND, captured at the moment the session became active: a watch-only
  /// import ⇒ `true`; a create/restore (a seed wallet) ⇒ `false`; a boot open ⇒
  /// a bounded once-read of [WalletSession.isWatchOnly]; a rescan swap carries
  /// the prior value forward (a data-DB rebuild can never change a wallet's
  /// kind). [isWatchOnlyProvider] reads THIS synchronously, so the chrome
  /// (hidden Send/Swap/Shield + the "Watch-only" badge + the Security screen's
  /// export-instead-of-backup arm) renders correctly on the FIRST frame after a
  /// wallet mounts — never a flash of spend affordances while an async kind-read
  /// settles (the converged HIGH). Fail-safe direction is `false` (the
  /// full-spend chrome): a spurious `false` only shows an affordance the SDK
  /// then refuses typed ([WalletErrorKind.watchOnly]), never a wrong spend.
  final bool isWatchOnly;
}

/// Provisioning/open failed. Carries an honest, ACTIONABLE category (never a
/// raw code; design invariant 6) so the screen shows a next step + the right
/// recovery affordance.
class OnboardingFailed extends OnboardingState {
  const OnboardingFailed(this.kind);

  final OnboardingFailureKind kind;
}

/// Honest, actionable onboarding failure categories — the axis the UI keys on
/// (Retry vs a different recovery path). Coarse on purpose: the stable error
/// CODE rides logs, never the UI, and no error PAYLOAD is read so nothing
/// sensitive (address/amount/seed) can leak (§5.4).
enum OnboardingFailureKind {
  /// Device locked / keychain transiently unreachable — "unlock your device
  /// and try again". Retryable.
  deviceLocked,

  /// Another instance already holds this wallet open — close it elsewhere,
  /// then retry. Retryable.
  alreadyOpen,

  /// The wallet's key was destroyed (biometric/PIN reset) or its storage/seal
  /// is damaged — recovery is the recovery phrase (restore), NOT a retry.
  needsRecovery,

  /// Out of disk space — free some and try again. Retryable.
  storageFull,

  /// No device key vault at all (no Keystore / no keyring). The wallet won't
  /// custody a seed without one (fail-closed). Permanent for this device.
  noVault,

  /// Couldn't reach the network during setup — check the connection, retry.
  network,

  /// Provisioning was interrupted (a create that didn't finish) — the next step
  /// is to try again, which re-probes and RESUMES the repair via create (open
  /// cannot repair a remnant). Retryable; distinct from `needsRecovery` (the
  /// seed is intact and resumable, not destroyed).
  interruptedSetup,

  /// The HOST app supplied an invalid wallet configuration (e.g. a rejected
  /// data directory, RW-CFG) — a developer error, not a device or user
  /// condition. NON-retryable: the same config fails the same way every time,
  /// so a Retry here is an infinite lie-loop (security review N1). The
  /// screen shows the honest "report this to the app's developer" copy and —
  /// uniquely — NO action button (every available action would deceive).
  configuration,

  /// Anything else (or an unmapped/forward-compat kind). Retryable; the real
  /// stable code rides logs.
  unknown,
}

extension OnboardingFailureKindX on OnboardingFailureKind {
  /// Whether the next step is "try again" (transient) versus a different
  /// recovery path (restore, or a different device). Drives whether the screen
  /// offers a Retry action.
  bool get isRetryable => switch (this) {
    OnboardingFailureKind.deviceLocked => true,
    OnboardingFailureKind.alreadyOpen => true,
    OnboardingFailureKind.storageFull => true,
    OnboardingFailureKind.network => true,
    OnboardingFailureKind.interruptedSetup => true,
    OnboardingFailureKind.unknown => true,
    OnboardingFailureKind.needsRecovery => false,
    OnboardingFailureKind.noVault => false,
    OnboardingFailureKind.configuration => false,
  };
}

/// Map a provisioning/open failure to an honest onboarding category. Reads the
/// typed FRB [WalletApiError.kind] only — never its payload — so no
/// address/amount/seed can leak (§5.4); an unmapped or non-FRB error is
/// `unknown` (retryable; the real code rides logs). Pure + total, so it is
/// unit-tested at its boundary without a device.
OnboardingFailureKind classifyOnboardingFailure(Object error) {
  if (error is WalletApiError) {
    return switch (error.kind) {
      WalletErrorKind_KeystoreUnavailable() =>
        OnboardingFailureKind.deviceLocked,
      // Another writer holds the wallet file — the same condition
      // `WalletAlreadyOpen` names, arriving from SQLite rather than from the
      // SDK's own guard. T0-7 (§4aa): a `SQLITE_BUSY` past the busy timeout
      // during the FIRST OPEN of a new or seed-restored wallet used to reach
      // here as `StoreCorrupt` and render the red "restore from your recovery
      // phrase"; the core now classifies it, and it must land on a RETRYABLE
      // kind that names the next step. `alreadyOpen` is the honest existing one
      // — "another instance holds this wallet, close it and retry" — so no new
      // string is minted for a condition the copy already covers.
      WalletErrorKind_WalletAlreadyOpen() ||
      WalletErrorKind_StoreBusy() => OnboardingFailureKind.alreadyOpen,
      WalletErrorKind_VaultAbsent() => OnboardingFailureKind.noVault,
      WalletErrorKind_DiskFull() => OnboardingFailureKind.storageFull,
      // Any sync stall during provisioning is treated as a transient network
      // condition. If a future StallReason needs different routing (e.g. a
      // non-network local fault), split this arm on the inner reason.
      WalletErrorKind_Sync() => OnboardingFailureKind.network,
      // A damaged seal/wrap, a destroyed key, or corrupt storage all mean the
      // on-disk wallet can't be opened here — the honest next step is restore
      // from the recovery phrase, not a retry.
      WalletErrorKind_KeystoreInconsistent() ||
      WalletErrorKind_SealInvalid() ||
      WalletErrorKind_SealVersionUnsupported() ||
      WalletErrorKind_WrapArtifactInvalid() ||
      WalletErrorKind_WrapVersionUnsupported() ||
      WalletErrorKind_StoreCorrupt() => OnboardingFailureKind.needsRecovery,
      // An interrupted create: open CANNOT repair a remnant (it throws this),
      // create resumes it from the sealed seed. Route to a retryable
      // "interrupted setup" — a retry re-probes, walletExists reports false for
      // a remnant, and the create path resumes the repair. Unreachable on the
      // normal boot path now (the probe routes remnants to create), but pinned
      // so it never falls through to `unknown` and a misleading message.
      WalletErrorKind_ProvisioningIncomplete() =>
        OnboardingFailureKind.interruptedSetup,
      // A raw IO failure during the probe/open (e.g. the degraded-filesystem
      // case): the SDK THROWS this (never `Ok(false)`), so it reaches
      // OnboardingFailed here, never OnboardingWelcome — the money-routing pin.
      // Unclassified but retryable; the stable RW-IO code rides logs.
      WalletErrorKind_Io() => OnboardingFailureKind.unknown,
      // A rejected dbDir (RW-CFG-003, #324) is a HOST programming error, not a
      // user/device condition — for every UI-package consumer the
      // walletOnboardingOverrides wiring belt already threw a loud
      // ArgumentError long before onboarding could run, so this arm is
      // defense-in-depth for a manually-wired host. It MUST NOT coalesce to
      // the retryable `unknown` (its own kind doc says non-retryable): a
      // manually-wired host would get an infinite retry-fail loop with
      // generic copy (security review N1). The dedicated non-retryable
      // arm tells the truth — a config error only the developer can fix.
      WalletErrorKind_InvalidDbDir() => OnboardingFailureKind.configuration,
      // A jitter window above the SDK's ceiling (RW-CFG-005): the same kind of
      // host configuration bug, the same non-retryable arm.
      WalletErrorKind_BroadcastJitterTooLong() =>
        OnboardingFailureKind.configuration,
      // Everything else (incl. networkMismatch / notFound — near-unreachable on
      // the reference app's fixed mainnet config, and a config error the user
      // can't act on) coalesces to the retryable unknown; the stable code rides
      // logs.
      _ => OnboardingFailureKind.unknown,
    };
  }
  return OnboardingFailureKind.unknown;
}

/// A FIXABLE restore-input fault — the user corrects the phrase (or birthday)
/// and retries on the SAME screen. Distinct axis from [OnboardingFailureKind]
/// (which is for provisioning failures needing a device fix or a different
/// recovery path): a fault here keeps the user on the restore form with the
/// phrase they typed intact. Coarse on purpose — no error payload is read beyond
/// the word INDEX (never the word itself — §5.4).
enum RestoreInputFault {
  /// A word isn't a BIP39 word, or the phrase checksum fails — check the words,
  /// capitalization, and typos, then try again. The 1-based index of the
  /// offending word rides [OnboardingRestoreInput.invalidWordIndex] when the SDK
  /// pinned one (a checksum failure has none).
  invalidWord,

  /// A DIFFERENT phrase was supplied over an interrupted-create remnant of
  /// another wallet (the constant-time seed compare rejected it) — retry with
  /// the correct phrase. Near-unreachable from a clean Welcome entry; honest if
  /// it ever surfaces.
  seedMismatch,

  /// A completed wallet already exists on this device — the next step is to go
  /// back and open it, not restore over it (the sealed seed is never clobbered).
  /// Near-unreachable from Welcome (the boot fork routes an existing wallet to
  /// open); defensive.
  alreadyExists,

  /// The chosen wallet-creation date is too recent — its birthday height is
  /// above the chain tip. Pick an earlier date or leave it blank for a full,
  /// money-safe scan.
  birthdayTooRecent,
}

/// A FIXABLE watch-only-import fault — the user corrects the viewing key (or
/// date) and retries on the SAME screen (#397 §3.7 D2). Distinct axis from
/// [RestoreInputFault]: no word index (a UFVK is one atomic string), and it
/// adds the wrong-network arm the SDK reports distinctly from garbage. No error
/// payload is read (§5.4 — the offending string is never echoed).
enum WatchOnlyInputFault {
  /// The pasted string isn't a valid unified full viewing key (garbage,
  /// truncated, or the wrong encoding) — check it and paste again.
  invalidViewingKey,

  /// A WELL-FORMED viewing key for the WRONG network (a mainnet `uview1…` on a
  /// testnet wallet, or vice versa) — it cannot import here.
  networkMismatch,

  /// A wallet already exists on this device — go back and open it, not import
  /// over it. Near-unreachable from Welcome (the boot fork routes an existing
  /// wallet to open); defensive.
  alreadyExists,

  /// The chosen creation date is too recent (its birthday is above the tip) —
  /// pick an earlier date.
  birthdayTooRecent,
}

/// Map a watch-only-import failure to a FIXABLE input fault, or `null` when it
/// is a provisioning failure for the generic [OnboardingFailed] screen. Reads
/// the typed [WalletApiError.kind] only (never a payload — §5.4). Pure + total.
WatchOnlyInputFault? classifyWatchOnlyInputFault(Object error) {
  if (error is! WalletApiError) return null;
  return switch (error.kind) {
    WalletErrorKind_InvalidViewingKey() =>
      WatchOnlyInputFault.invalidViewingKey,
    WalletErrorKind_NetworkMismatch() => WatchOnlyInputFault.networkMismatch,
    WalletErrorKind_WalletAlreadyExists() => WatchOnlyInputFault.alreadyExists,
    WalletErrorKind_BirthdayInFuture() => WatchOnlyInputFault.birthdayTooRecent,
    // Not input-fixable (device locked / no vault / disk full / corrupt …) →
    // let classifyOnboardingFailure route it to the generic screen.
    _ => null,
  };
}

/// Map a restore failure to a FIXABLE input fault (+ the 1-based word index when
/// known), or `null` when it is a provisioning failure that belongs on the
/// generic [OnboardingFailed] screen via [classifyOnboardingFailure]. Reads the
/// typed [WalletApiError.kind] only — never a free-form payload — so nothing
/// sensitive can leak (§5.4). Pure + total; a non-FRB error is `null` (→ generic).
({RestoreInputFault fault, int? wordIndex})? classifyRestoreInputFault(
  Object error,
) {
  if (error is! WalletApiError) return null;
  return switch (error.kind) {
    // The SDK reports a 0-based index (the word's position in the phrase); +1
    // for a human-facing "word N". `null` when no single word is at fault.
    WalletErrorKind_InvalidMnemonic(:final wordIndex) => (
      fault: RestoreInputFault.invalidWord,
      wordIndex: wordIndex == null ? null : wordIndex + 1,
    ),
    WalletErrorKind_SeedMismatch() => (
      fault: RestoreInputFault.seedMismatch,
      wordIndex: null,
    ),
    WalletErrorKind_WalletAlreadyExists() => (
      fault: RestoreInputFault.alreadyExists,
      wordIndex: null,
    ),
    WalletErrorKind_BirthdayInFuture() => (
      fault: RestoreInputFault.birthdayTooRecent,
      wordIndex: null,
    ),
    // Not input-fixable (device locked / no vault / disk full / corrupt store /
    // network …) → let classifyOnboardingFailure route it to the generic screen.
    _ => null,
  };
}
