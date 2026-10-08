import 'dart:async';

import 'package:meta/meta.dart' show visibleForTesting;
import 'package:zec_wallet/zec_wallet.dart';

import '../frb_wallet_session.dart';
import '../wallet_config.dart';
import '../wallet_session.dart';
import 'wallet_provisioner.dart';

/// The production [WalletProvisioner] (spec §3.2g iii-B-2-b): the boot-fork
/// adapter over the FRB [WalletHandle] statics. It owns the underlying wallet
/// handle across `createGenerated`/`open`, narrows the wide handle surface to
/// the provisioning + backup-read methods, and enforces the BOUNDEDNESS
/// CONTRACT the port documents — a wedged keychain/fsync must resolve to a typed
/// failure, never an unbounded onboarding spinner.
///
/// THE BOUNDED-WAIT (port BOUNDEDNESS CONTRACT, the iii-B-2-b obligation): every
/// step here does LOCAL work only (disk + keychain: the single-writer lock,
/// SQLCipher open, seed seal/unseal); none touches the network (first-sync is
/// the sync engine's job and surfaces as a `SyncStatus.stalled` EVENT, never a
/// hang here). The normal cost is sub-second. We still cap each call at
/// [_localIoBound] and, on expiry, surface a typed
/// [WalletErrorKind.keystoreUnavailable] — exactly the typed timeout the port
/// promises, which the host's `classifyOnboardingFailure` already routes to a
/// RETRYABLE "unlock your device and try again" (the likeliest cause of a
/// keychain stall is a locked device). The controller relies on THIS bound
/// instead of imposing its own (which would fight it). The cap is generous on
/// purpose: a slow-but-fine cold create must never spuriously time out.
///
/// CRASH-SAFETY of a timed-out create: a Dart `.timeout` does not cancel the
/// Rust work, so a create that exceeds the bound may STILL finish on disk. That
/// is safe — the controller writes the backup-confirmed flag `false` BEFORE
/// create, so the next boot reads `walletExists() == true` and resumes into
/// FORCED backup (never deposit-ready on an unconfirmed wallet), and a retried
/// create over a now-complete wallet hits the SDK's `walletAlreadyExists`
/// non-clobber under the lock. No double-mint, no funds path.
///
/// USES THE LOCK-FREE PROBE, NOT open-and-catch: [walletExists] calls the SDK's
/// dedicated `WalletHandle.walletExists` (the iii-B-2-b-i probe). An
/// `open`-and-catch-`notFound` would acquire the single-writer lock the
/// subsequent `open` needs and break the boot fork.
class FrbWalletProvisioner implements WalletProvisioner {
  /// [config] is the resolved host-policy config (see `wallet_config.dart`); its
  /// `dbDir` MUST already be a plain resolved path (no provider-`build()` async
  /// recompute — the double-create footgun).
  ///
  /// The SDK entry points are injected (defaulting to the real FRB statics) so
  /// the bounded-wait + timeout mapping and the typed-error passthrough are
  /// host-VM testable without a device (a real [WalletHandle] cannot be
  /// constructed in `flutter test`); production uses the defaults.
  FrbWalletProvisioner({
    required WalletConfig config,
    Duration localIoBound = defaultLocalIoBound,
    WalletExistsFn? walletExistsFn,
    CreateHandleFn? createGeneratedFn,
    OpenHandleFn? openFn,
    RestoreHandleFn? restoreFn,
    CreateWatchOnlyHandleFn? createWatchOnlyFn,
    RescanHandleFn? rescanFn,
    BirthdayEstimateFn? birthdayEstimateFn,
    CloseHandleFn? closeFn,
    WipeFn? wipeFn,
    WipeFn? forceWipeFn,
    CustodyDisclosureFn? custodyDisclosureFn,
    Duration wipeLockRetryBackoff = defaultWipeLockRetryBackoff,
    int wipeLockRetries = defaultWipeLockRetries,
  }) : _config = config,
       _localIoBound = localIoBound,
       _walletExistsFn = walletExistsFn ?? _defaultWalletExists,
       _createGeneratedFn = createGeneratedFn ?? _defaultCreateGenerated,
       _openFn = openFn ?? _defaultOpen,
       _restoreFn = restoreFn ?? _defaultRestore,
       _createWatchOnlyFn = createWatchOnlyFn ?? _defaultCreateWatchOnly,
       _rescanFn = rescanFn ?? _defaultRescan,
       _birthdayEstimateFn = birthdayEstimateFn ?? _defaultBirthdayEstimate,
       _closeFn = closeFn ?? _defaultClose,
       _wipeFn = wipeFn ?? _defaultWipe,
       _forceWipeFn = forceWipeFn ?? _defaultForceWipe,
       _custodyDisclosureFn = custodyDisclosureFn ?? _defaultCustodyDisclosure,
       _wipeLockRetryBackoff = wipeLockRetryBackoff,
       _wipeLockRetries = wipeLockRetries;

  /// The generous default bound on a local provisioning step (disk + keychain).
  /// Sub-second in practice; 30s only catches a genuine wedge so a slow-but-fine
  /// cold create never spuriously times out. Pinned by a boundary test (gate 7).
  static const Duration defaultLocalIoBound = Duration(seconds: 30);

  /// FR-14 delete-during-sync (#252, device-found): a `close()` cooperatively
  /// cancels the sync controller but a non-cancellable in-flight `spawn_blocking`
  /// scan batch keeps the wallet's single-writer lock held for a MOMENT after
  /// close returns (it must — wiping files under an active DB write would corrupt
  /// the teardown). So the immediate `wipe` can transiently see `WalletOpen`. The
  /// delete WAITS OUT that legitimately-held lock with a bounded linear backoff
  /// (~base × 1..N) rather than failing the user's delete — the batch finishes in
  /// well under this budget. A GENUINELY foreign holder (another app/window)
  /// outlasts the budget and surfaces `WalletOpen` honestly. Base delay + count
  /// are injectable so the retry is host-VM testable without real wall-clock waits.
  /// The SAME budget governs [open]'s lock wait (v-5c finding #2 — the
  /// recover-by-reopen after a rescan fault races the same finishing batch): one
  /// lock, one wait policy. COUPLED to the SDK's core quiesce bound
  /// (`QUIESCE_MAX`, 30s since the device calibration — a cancelled sparse
  /// scan batch really held the lock for minutes at the old 1_000-block batch
  /// size, so the core bounded the batch at 100 blocks AND raised its quiesce;
  /// this budget moved with it per the coupling contract): the rescan fails
  /// typed once the core gives up, and this backoff (500ms × 1..10 triangular
  /// ≈ 27.5s) catches the straggler band just past that bound — the
  /// failure landed `failedClosed` in exactly the band the old ~6.3s missed.
  /// The wait is spinner-covered (the rescan sheet's "Rebuilding…" state / the
  /// onboarding progress view), never a frozen UI.
  static const Duration defaultWipeLockRetryBackoff = Duration(
    milliseconds: 500,
  );
  static const int defaultWipeLockRetries = 10;

  final WalletConfig _config;

  /// Test-only flow-through probe: lets the composition
  /// test assert the HOST's config reached this adapter verbatim — without it
  /// a seam that silently rebuilt a default config would pass the wiring test.
  @visibleForTesting
  WalletConfig get config => _config;

  final Duration _localIoBound;
  final WalletExistsFn _walletExistsFn;
  final CreateHandleFn _createGeneratedFn;
  final OpenHandleFn _openFn;
  final RestoreHandleFn _restoreFn;
  final CreateWatchOnlyHandleFn _createWatchOnlyFn;
  final RescanHandleFn _rescanFn;
  final BirthdayEstimateFn _birthdayEstimateFn;
  final CloseHandleFn _closeFn;
  final WipeFn _wipeFn;
  final WipeFn _forceWipeFn;
  final CustodyDisclosureFn _custodyDisclosureFn;
  final Duration _wipeLockRetryBackoff;
  final int _wipeLockRetries;

  /// The live handle once created/opened on this instance — the source for
  /// [revealMnemonic]. Exactly one wallet is provisioned per instance.
  WalletHandle? _handle;

  @override
  Future<bool> walletExists() => _bounded(_walletExistsFn(_config));

  @override
  Future<WalletSession> createGenerated() async {
    final handle = await _bounded(_createGeneratedFn(_config));
    _handle = handle;
    return FrbWalletSession(handle, network: _config.network);
  }

  @override
  Future<WalletSession> open() async {
    // WAITS OUT a transiently-held single-writer lock (v-5c finding #2): a
    // rescan fault can leave an orphaned scan batch holding the lock for a
    // moment after the handle closed — the recover-by-reopen (and a boot-time
    // open racing a dying prior instance) lands in that window. Same bounded
    // budget as the delete path (#252); a genuinely foreign holder outlasts it
    // and surfaces `WalletOpen` honestly.
    final handle = await _waitingForLockRelease(
      () => _bounded(_openFn(_config)),
    );
    _handle = handle;
    return FrbWalletSession(handle, network: _config.network);
  }

  @override
  Future<WalletSession> restore(
    List<String> mnemonicWords, {
    DateTime? approximateCreationTime,
  }) async {
    // Stamp the optional birthday onto THIS restore's config — the one source of
    // truth the SDK reads (`config.birthdayHeight`). The DateTime→height
    // conversion rides the injected estimator (the real one is the native
    // `estimateBirthday` FRB call; injected so the threading is host-VM
    // testable). `estimateBirthday` is a CONSERVATIVE floor (never past the
    // given time), so a slightly-wrong date can only scan a little EARLY (safe,
    // finds everything), never skip funds. The network comes from the config —
    // the screen never needs to know mainnet/testnet.
    //
    // No date means the user chose to scan all history (the contract on
    // [WalletProvisioner.restore]), so the birthday is CLEARED, not inherited:
    // a host config's own `birthdayHeight` would otherwise start the scan
    // there and miss every older payment to the restored phrase.
    final config = _config.withBirthdayHeight(
      approximateCreationTime == null
          ? null
          : _birthdayEstimateFn(_config.network, approximateCreationTime),
    );
    // The words are trusted pre-normalized (the controller's lowercase
    // chokepoint); the adapter does not re-case key material. Bounded like
    // create: a wedged keychain seal surfaces as the typed retryable timeout,
    // never an unbounded restore spinner. The crash-safety of a timed-out
    // restore matches create (the false-flag-before, non-clobber-on-retry path
    // the controller owns).
    final handle = await _bounded(_restoreFn(config, mnemonicWords));
    _handle = handle;
    return FrbWalletSession(handle, network: _config.network);
  }

  @override
  Future<WalletSession> createWatchOnly(
    String ufvk, {
    required int birthdayHeight,
  }) async {
    // Stamp the REQUIRED birthday onto this create's config (the one source of
    // truth the SDK reads). Unlike restore's optional creation time, watch-only
    // has no lazy seed-path, so the height is mandatory and rides verbatim (it
    // is already a height — the picker converts the date via the shared
    // estimator BEFORE this call). The UFVK is decoded network-bound INSIDE the
    // SDK (typed `invalidViewingKey`/`networkMismatch` before any write —
    // decode-before-lock leaves no remnant on reject). Bounded like restore.
    final config = _config.withBirthdayHeight(birthdayHeight);
    final handle = await _bounded(_createWatchOnlyFn(config, ufvk));
    _handle = handle;
    return FrbWalletSession(handle, network: _config.network);
  }

  @override
  Future<WalletSession> rescanFrom(RescanTarget target) async {
    final handle = _handle;
    if (handle == null) {
      // Defensive: the wallet surface that triggers a rescan is reachable only
      // from OnboardingActive, which implies a create/open/restore ran on this
      // instance. Fail loud rather than silently no-op a money-recovery action.
      throw StateError('rescanFrom before createGenerated/open/restore');
    }
    // Per-arm floor conversion (#317): a picked date rides the SAME injected
    // estimator restore uses — a CONSERVATIVE floor (never past the time →
    // can only scan a little EARLY, never skip funds); the wallet-birthday
    // default passes its floor height VERBATIM (already a height — a second
    // conversion could only distort it); all-history ⇒ `null` ⇒ the SDK
    // floors to Sapling activation (full, money-safe). The network comes from
    // the config; the screen never needs to know mainnet/testnet.
    final fromHeight = switch (target) {
      RescanAllHistory() => null,
      RescanFromTime(:final earliestTime) => _birthdayEstimateFn(
        _config.network,
        earliestTime,
      ),
      RescanFromWalletBirthday(:final floorHeight) => floorHeight,
    };
    // DELIBERATELY NOT `_bounded`: rescan stops + joins the sync loop, THEN
    // rebuilds the data DB. The join can outlast a keychain op (it waits for the
    // current scan batch), and a Dart `.timeout` cannot cancel the Rust rebuild —
    // racing it with the controller's recovery re-open would put TWO openers on
    // one DB (a worse failure than waiting). The work is LOCAL + bounded by the
    // SDK; the UI shows an honest "rebuilding" progress for the duration. On ANY
    // error the handle is left closed (ADR-0534) and the typed `WalletApiError`
    // propagates UNCHANGED so the controller's recover-by-reopen sees the kind.
    await _rescanFn(handle, fromHeight);
    // Success: the SAME handle is rebuilt in place (`rescanFrom` is `&mut self`),
    // so `_handle` stays valid for `revealMnemonic`. A FRESH session over it is
    // what makes the live-sync graph (which keys off session identity) rebuild —
    // re-subscribing sync from the lower birthday and re-reading the now-empty,
    // repopulating balance + history.
    return FrbWalletSession(handle, network: _config.network);
  }

  @override
  Future<WalletSession> switchSyncServer(SyncServerChoice choice) async {
    final handle = _handle;
    if (handle == null) {
      // Same reasoning as `rescanFrom`: the picker is reachable only from
      // OnboardingActive, which implies a create/open/restore on this instance.
      throw StateError('switchSyncServer before createGenerated/open/restore');
    }
    // NOT `_bounded`, for the rescan's reason: the switch stops + joins the
    // sync loop (the join can outlast a keychain op) and a Dart timeout cannot
    // cancel the Rust side. The SDK bounds the probe (15 s) and the quiesce
    // (30 s) itself. A pre-swap refusal leaves the handle open and propagates
    // typed; a post-stop fault leaves it closed, and the controller re-opens.
    await handle.switchSyncServer(choice: choice);
    // Success: the SAME handle carries the rebuilt session (`switchSyncServer`
    // replaces its inner in place). A FRESH session over it is what makes the
    // live-sync graph rebuild — the old session's status stream has ENDED with
    // the old controller, and the new one subscribes to the new server's.
    return FrbWalletSession(handle, network: _config.network);
  }

  @override
  int estimateBirthdayHeight(DateTime time) =>
      _birthdayEstimateFn(_config.network, time);

  @override
  Future<List<String>> revealMnemonic() {
    final handle = _handle;
    if (handle == null) {
      // Defensive: the controller always create/opens on this instance before
      // revealing (the backup screen is unreachable otherwise). Fail loud rather
      // than silently return no words.
      throw StateError('revealMnemonic before createGenerated/open');
    }
    // Bounded so a wedged keychain unseal surfaces as a retryable reveal error
    // (the backup view's "couldn't show — try again") instead of a permanent
    // spinner. RESIDUE NOTE (§10): on the rare timeout the Rust future still
    // completes and delivers the words into a Dart object no one awaits — a
    // brief orphaned, un-zeroizable copy GC'd non-deterministically. That is
    // within the already-accepted §10 exposure (the words are shown on screen
    // anyway); the reliability win outweighs the marginal extra copy.
    return _bounded(handle.revealMnemonic());
  }

  @override
  Future<String> exportUfvk() {
    final handle = _handle;
    if (handle == null) {
      // Defensive: the export screen is reachable only from an active wallet
      // (Settings → Security), which implies a create/open/restore/watch-only
      // ran on this instance. Fail loud rather than silently return nothing.
      throw StateError('exportUfvk before createGenerated/open/restore');
    }
    // Bounded so a wedged store read surfaces as the export view's retryable
    // error, never a permanent spinner. A UFVK is PUBLIC viewing capability
    // (no §10 seed-residue concern), so no redaction box — the plain String
    // is shown/copied/QR'd on the gated export screen and never logged (§5.4).
    return _bounded(handle.exportUfvk());
  }

  @override
  Future<CustodyDisclosure> custodyDisclosure() =>
      // A static, read-only tier probe over THIS wallet's config — no live handle
      // needed, no seed/unseal. Bounded like the other local keychain reads.
      _bounded(_custodyDisclosureFn(_config));

  @override
  Future<void> deleteWallet() async {
    // CLOSE FIRST: the SDK wipe refuses while an instance holds the single-writer
    // lock (WalletOpen). Closing frees the lock; `close` is idempotent (a no-op on
    // an already-closed / never-opened handle), so a wipe-then-retry is safe. We
    // null `_handle` immediately so a wipe FAULT can't leave a stale closed handle
    // around — the OnboardingController recovers by re-`open`ing (a fresh handle).
    final handle = _handle;
    _handle = null;
    if (handle != null) {
      // EDGE: a close that exceeds the bound throws keystoreUnavailable, but the
      // Rust close isn't cancelled (same as create/restore) and may still hold the
      // wallet lock briefly. The controller's recover-by-reopen then sees
      // `WalletOpen` and routes to OnboardingFailed (retry re-probes once the lock
      // frees) — a correct, never-stranded outcome, never a torn wallet.
      // CLOSE-SPECIFIC bound (review R1b): close's LEGITIMATE worst case is
      // now the stop-join trail (an in-flight proving/broadcast unit it must wait
      // out) + the raised 30s core quiesce — the plain `_localIoBound` (30s)
      // equals the quiesce alone, so the old bound turned the lawful worst case
      // into a deterministic typed timeout. Close is local + join-bounded by
      // design (never network-hung), so the wider belt loses no hang protection.
      await _boundedBy(_closeFn(handle), defaultCloseBound);
    }
    // The authoritative crypto-shred: keychain-first wrap-key sever → file
    // removal. Takes only the config (db_dir) — never blockable by transport
    // validation (the SDK consumes db_dir alone). WAITS OUT a transient
    // `WalletOpen` (#252): close() cooperatively cancels sync, but a finishing
    // in-flight scan batch legitimately holds the lock for a moment — retry until
    // it releases, rather than failing a delete fired while the wallet syncs.
    await _wipeWaitingForLockRelease(_wipeFn);
  }

  @override
  Future<void> forceDeleteWallet() async {
    // The #251 escape hatch: FORCE-complete a wipe whose custody key is ALREADY
    // GONE (a `needsRecovery` wallet — the plain [deleteWallet] fails closed with
    // `keystoreInconsistent` because its verify-real-sever guard finds nothing to
    // sever). The SEED is not on the device; it is recovered from the recovery
    // phrase, so force-removing the unreadable remnant loses no funds. Used ONLY
    // deliberately (the controller calls this AFTER a plain delete returned
    // `keystoreInconsistent`, never auto-escalated — the SDK `wipe_force` contract).
    // Same close-first + wait-out-the-lock discipline as [deleteWallet]; on the
    // failed path there is usually no live handle (open never succeeded), so the
    // close is a no-op, but it stays defensive + symmetric.
    final handle = _handle;
    _handle = null;
    if (handle != null) {
      await _boundedBy(_closeFn(handle), defaultCloseBound);
    }
    await _wipeWaitingForLockRelease(_forceWipeFn);
  }

  /// Run the crypto-shred, treating `WalletOpen` as the TRANSIENT "the wallet is
  /// still quiescing" signal (#252). The wipe is idempotent + fail-closed, so
  /// re-running it is always safe: each attempt either severs custody (success)
  /// or no-ops because the lock is still held.
  Future<void> _wipeWaitingForLockRelease(WipeFn wipeFn) =>
      _waitingForLockRelease(() => _bounded(wipeFn(_config)));

  /// Retry `attempt` with a bounded linear backoff while it throws the
  /// TRANSIENT `WalletOpen` — the single-writer lock held for a moment by a
  /// finishing (non-cancellable) in-flight scan batch. Two proven producers of
  /// that transient: a delete fired while the wallet syncs (#252, the wipe
  /// path), and a recover-by-reopen after a rescan fault while the orphaned
  /// batch still drains (v-5c finding #2 — the device-proven intermittent
  /// `WalletAlreadyOpen`; the SDK now quiesces core-side too, this is the
  /// belt). A non-`WalletOpen` fault (e.g. a wedged keychain →
  /// `keystoreUnavailable`, or the fail-closed `keystoreInconsistent`)
  /// propagates IMMEDIATELY — only the lock-contention transient is worth
  /// waiting on. If `WalletOpen` outlasts the budget (a genuinely foreign
  /// holder — another app/window), it surfaces so the host shows the honest
  /// "close it there, then try again".
  Future<T> _waitingForLockRelease<T>(Future<T> Function() attempt) async {
    for (var i = 0; ; i++) {
      try {
        return await attempt();
      } on WalletApiError catch (e) {
        // The held single-writer lock surfaces as TWO distinct kinds depending
        // on the operation: a refused WIPE throws `walletOpen` (RW-LIFE-004),
        // while a refused OPEN throws `walletAlreadyOpen` (RW-LIFE-001, from
        // `WalletLock::acquire`). Each caller only ever sees its own kind, so
        // one shared predicate matching BOTH keeps the helper honest for both
        // paths (an open-only `walletOpen` match was the review's
        // dead-belt finding — the retry never engaged on the open path).
        final lockHeld =
            e.kind == const WalletErrorKind.walletOpen() ||
            e.kind == const WalletErrorKind.walletAlreadyOpen();
        if (!lockHeld || i >= _wipeLockRetries) rethrow;
        // Linear backoff (base × the 1-indexed attempt: 1×, 2×, … N×): a scan
        // batch finishes well under the cumulative budget; the first wait is the
        // shortest, so the common quick-release case barely pauses.
        await Future<void>.delayed(_wipeLockRetryBackoff * (i + 1));
      }
    }
  }

  /// Cap a local provisioning step at [_localIoBound]; a timeout becomes the
  /// typed [WalletErrorKind.keystoreUnavailable] the port contract promises (a
  /// `.timeout` races a timer — the underlying Rust work is not cancelled; see
  /// the create crash-safety note above). Any other error (a genuine typed
  /// [WalletApiError] from the SDK) propagates UNCHANGED so the classifier sees
  /// the real kind.
  Future<T> _bounded<T>(Future<T> op) => _boundedBy(op, _localIoBound);

  /// The close-before-wipe belt (review R1b): wide enough for close's
  /// LAWFUL long tail — waiting out one in-flight proving/broadcast unit at the
  /// stop-join (10–30s on phone silicon) plus the 30s core quiesce — while still
  /// bounding a genuine wedge typed. Sized: 30s quiesce + ~45s stop-trail + margin.
  static const Duration defaultCloseBound = Duration(seconds: 90);

  Future<T> _boundedBy<T>(Future<T> op, Duration bound) async {
    try {
      return await op.timeout(bound);
    } on TimeoutException {
      throw _timedOut();
    }
  }

  /// The host-synthesized typed timeout. `keystoreUnavailable` is the port's
  /// documented bounded-wait failure; the `RW-HOST-*` code is host-namespaced so
  /// a log reader never mistakes it for an SDK-emitted code, and the message is
  /// static + payload-free (§5.4 — no path/amount/secret).
  static WalletApiError _timedOut() => const WalletApiError(
    code: 'RW-HOST-IO-TIMEOUT',
    message: 'local provisioning step exceeded its bounded wait',
    kind: WalletErrorKind.keystoreUnavailable(),
  );

  static Future<bool> _defaultWalletExists(WalletConfig config) =>
      WalletHandle.walletExists(config: config);

  static Future<WalletHandle> _defaultCreateGenerated(WalletConfig config) =>
      WalletHandle.createGenerated(config: config);

  static Future<WalletHandle> _defaultOpen(WalletConfig config) =>
      WalletHandle.open(config: config);

  // The reference app restores STANDARD phrases (no BIP39 passphrase) — matching
  // its passphrase-free create path; `passphrase: null` is deliberate, not an
  // omission (an expert passphrase wallet is deferred, manager-flagged).
  static Future<WalletHandle> _defaultRestore(
    WalletConfig config,
    List<String> mnemonicWords,
  ) =>
      // `passphrase: null` is EXPLICIT (not an omitted optional) so the one FFI
      // site that could ever wire a BIP39 passphrase reads unambiguously to an
      // audit grep — the reference app restores standard, passphrase-free phrases.
      WalletHandle.restore(
        config: config,
        mnemonicWords: mnemonicWords,
        passphrase: null,
      );

  // #397 §3.7 D2: the watch-only create FFI. The UFVK is a plain String until
  // the audited core decoder (network-bound, typed reject) consumes it — no
  // key type crosses here. Injected like create/restore so the birthday
  // threading + typed-error passthrough are host-VM testable.
  static Future<WalletHandle> _defaultCreateWatchOnly(
    WalletConfig config,
    String ufvk,
  ) => WalletHandle.createWatchOnly(
    config: config,
    ufvk: ufvk,
    // `createWatchOnly` stamped the REQUIRED birthday via `withBirthdayHeight`
    // before this call (unlike create/open, which leave it null-able) — so it
    // is non-null by construction on this one path.
    birthdayHeight: config.birthdayHeight!,
  );

  // The `&mut self` rescan FFI — rebuilds the data DB in place at `fromHeight`
  // (`null` ⇒ Sapling activation). Injected (like create/open/restore) so the
  // handle-null precondition guard is host-VM testable; the success path needs a
  // real handle and is on-device-verified, as restore's is.
  static Future<void> _defaultRescan(WalletHandle handle, int? fromHeight) =>
      handle.rescanFrom(fromHeight: fromHeight);

  // UNIX SECONDS (UTC) from the DateTime; the SDK's `estimateBirthday` is a
  // conservative floor over bundled checkpoints (never past the time → never
  // skips notes). A pre-1970 input clamps to activation inside the SDK.
  static int _defaultBirthdayEstimate(Network network, DateTime creationTime) =>
      estimateBirthday(
        network: network,
        approxUnixSecs:
            creationTime.toUtc().millisecondsSinceEpoch ~/
            Duration.millisecondsPerSecond,
      );

  // The teardown FFI: `close` is `&mut self` on the live handle; `wipe`/
  // `custodyDisclosure` are `db_dir`-only statics (the seed never crosses, and a
  // destructive shred is never blocked by transport validation). Injected like
  // create/open so the close-then-wipe ordering + the recover-on-fault path are
  // host-VM testable without a real (opaque) WalletHandle.
  static Future<void> _defaultClose(WalletHandle handle) => handle.close();

  static Future<void> _defaultWipe(WalletConfig config) =>
      WalletHandle.wipe(config: config);

  static Future<void> _defaultForceWipe(WalletConfig config) =>
      WalletHandle.wipeForce(config: config);

  static Future<CustodyDisclosure> _defaultCustodyDisclosure(
    WalletConfig config,
  ) => WalletHandle.custodyDisclosure(config: config);
}

/// SDK entry points, injected for host-VM testability (defaults are the real FRB
/// statics). A test passes a never-completing future to exercise the
/// bounded-wait, or a throwing one to exercise the typed-error passthrough.
typedef WalletExistsFn = Future<bool> Function(WalletConfig config);
typedef CreateHandleFn = Future<WalletHandle> Function(WalletConfig config);
typedef OpenHandleFn = Future<WalletHandle> Function(WalletConfig config);

/// The restore entry point, injected for host-VM testability (a real
/// [WalletHandle] cannot be constructed in `flutter test`). A test asserts the
/// [WalletConfig] it receives carries the birthday height stamped from the
/// estimator — and that the words pass through unchanged (the adapter does not
/// re-normalize key material).
typedef RestoreHandleFn =
    Future<WalletHandle> Function(
      WalletConfig config,
      List<String> mnemonicWords,
    );

/// The watch-only create entry point (#397), injected for host-VM testability.
/// A test asserts the [WalletConfig] carries the birthday height stamped by the
/// picker's estimate, that the `ufvk` string passes through unchanged, and that
/// a typed `invalidViewingKey`/`networkMismatch` throw propagates so the
/// controller renders it as a fixable input fault.
typedef CreateWatchOnlyHandleFn =
    Future<WalletHandle> Function(WalletConfig config, String ufvk);

/// The rescan entry point, injected for host-VM testability. A test asserts the
/// `fromHeight` it receives is the estimator's output (or `null` for scan-all);
/// the success path needs a real [WalletHandle], so the threading is verified
/// here against the injected fn and end-to-end on device (mirrors [RestoreHandleFn]).
typedef RescanHandleFn =
    Future<void> Function(WalletHandle handle, int? fromHeight);

/// The DateTime→birthday-height estimator, injected so the date-threading is
/// host-VM testable without the native `estimateBirthday` FRB call.
typedef BirthdayEstimateFn =
    int Function(Network network, DateTime creationTime);

/// The handle-close entry point, injected for host-VM testability (a real
/// [WalletHandle] cannot be constructed in `flutter test`). A delete-wallet test
/// asserts close runs BEFORE wipe (the WalletOpen-refusal ordering).
typedef CloseHandleFn = Future<void> Function(WalletHandle handle);

/// The crypto-shred entry point (`WalletHandle.wipe`), injected for host-VM
/// testability. A test asserts the [WalletConfig] it receives is this wallet's
/// and that a throw propagates so the controller's recover-by-reopen path runs.
typedef WipeFn = Future<void> Function(WalletConfig config);

/// The custody-disclosure probe (`WalletHandle.custodyDisclosure`), injected for
/// host-VM testability. A test feeds a `CustodyDisclosure` and asserts the screen
/// renders the honest hardware-key vs best-effort copy.
typedef CustodyDisclosureFn =
    Future<CustodyDisclosure> Function(WalletConfig config);
