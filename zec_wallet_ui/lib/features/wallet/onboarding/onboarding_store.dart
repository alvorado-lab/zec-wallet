/// Persists onboarding PROGRESS — specifically the money-safety gate "the user
/// has confirmed they backed up the recovery phrase" — across launches.
///
/// WHY a host-side flag and NOT Rust state: the SDK knows a wallet EXISTS on
/// disk but has no concept of "the human wrote the words down". That
/// acknowledgement is pure host-UX progress, not wallet/crypto/message data —
/// so it is NOT the "Rust owns all state" application data (design invariant 1);
/// it is the same category as the theme/text-scale rendering prefs the frame
/// already persists. It stores NO secret — never the words, only a boolean.
///
/// SAFETY BY CONSTRUCTION (this is the heart of the money-safety gate):
///  - The flag flips to `true` ONLY after an explicit user confirm, so a
///    provisioned-but-unconfirmed wallet can NEVER be mistaken for
///    deposit-ready.
///  - Unknown/unset reads as `false` (fail-safe: the gate stays CLOSED → backup
///    is re-forced, never skipped).
///  - [OnboardingController] resets it to `false` at create-start, BEFORE the
///    new wallet is written to disk — so even a stale `true` from a wiped prior
///    wallet (or a reinstall over an existing DB) cannot leak a fresh wallet
///    past the gate. The only reachable failure direction is re-forcing an
///    already-done backup (annoying, never dangerous).
///
/// A port (design invariant 2) so the [OnboardingController] state machine is
/// host-VM testable with an in-memory fake; the shared_preferences-backed
/// production impl lands with the on-device onboarding slice.
abstract interface class OnboardingStore {
  /// Whether the user has confirmed the recovery-phrase backup for the wallet
  /// currently on disk. `false` when unknown/unset (fail-safe).
  Future<bool> isBackupConfirmed();

  /// Set the confirmed flag. `true` is recorded only after an explicit confirm;
  /// `false` resets the gate for a fresh wallet at create-start. Must DURABLY
  /// persist before it resolves — the controller only goes deposit-ready after
  /// a `true` write completes, and only writes the new wallet after a `false`
  /// write completes (the crash-safe ordering).
  Future<void> setBackupConfirmed({required bool confirmed});

  /// The #390 post-restore deep-scan note's lifecycle for the wallet currently
  /// on disk. `notApplicable` when unknown/unset (fail-safe: a wallet whose
  /// provenance we can't read gets NO note). This is a durable provenance +
  /// acknowledgement fact — the same host-UX category as [isBackupConfirmed],
  /// NOT wallet/crypto state (design invariant 1); it stores NO secret.
  Future<DeepScanRestoreNoteState> deepScanRestoreNoteState();

  /// Persist the note lifecycle (written definitively at each activation — see
  /// [OnboardingController]). BEST-EFFORT: a failed write is non-fatal (the
  /// worst case is the one-time nudge not showing, the fail-safe direction), so
  /// unlike [setBackupConfirmed] this NEVER throws.
  Future<void> setDeepScanRestoreNoteState(DeepScanRestoreNoteState state);
}

/// The one-time post-restore deep-scan note's lifecycle (#390, D2). Written
/// definitively at each wallet activation so a new wallet never inherits a prior
/// one's value (no separate reset needed):
///  - [notApplicable] — a CREATED wallet (or unknown/unset): never show. A
///    created wallet pays the deep scan's full cost for no restore-invisible
///    funds, so the proactive nudge is wrong there.
///  - [pending] — a RESTORED wallet whose note has not been acknowledged: show
///    it once the first catch-up completes (never beside a still-empty balance).
///  - [done] — the user checked or dismissed it: never show again.
enum DeepScanRestoreNoteState { notApplicable, pending, done }
