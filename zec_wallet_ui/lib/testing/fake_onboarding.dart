import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart' show AppLifecycleState;
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/bip39_wordlist.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_store.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart'
    show CustodyDisclosure, EraseAssurance, SyncServerChoice;

import 'fake_wallet_session.dart';

/// A host-VM fake [WalletProvisioner] — no native library, no device. Drives
/// the [OnboardingController] state machine deterministically: configure what
/// is on disk, make any step succeed or throw a TYPED failure, and count calls.
class FakeWalletProvisioner implements WalletProvisioner {
  FakeWalletProvisioner({
    this.exists = false,
    WalletSession? session,
    this.recoveryWords = _vectorWords,
  }) : session = session ?? FakeWalletSession();

  /// What [walletExists] reports (the on-disk fork). Mutable so a test can flip
  /// it (e.g. create makes a wallet exist for a later resume).
  bool exists;

  /// The session [createGenerated]/[open] return.
  WalletSession session;

  /// The words [revealMnemonic] returns.
  List<String> recoveryWords;

  /// The session [rescanFrom] returns on success — a DISTINCT session from
  /// [session] by default, so a test can assert the active gate swapped to a new
  /// identity (the live-sync-graph rebuild trigger). Override to pin a specific
  /// one.
  WalletSession? rescanSession;
  late final WalletSession _defaultRescanSession = FakeWalletSession();

  /// When set, the matching call throws this instead of succeeding. Use a real
  /// `WalletApiError` to exercise the failure classifier.
  Object? failWalletExists;
  Object? failCreate;
  Object? failOpen;
  Object? failReveal;
  Object? failRestore;
  Object? failCreateWatchOnly;
  Object? failExportUfvk;

  /// When set, [rescanFrom] throws this (the handle-closed path the controller
  /// recovers from by re-opening — set [failOpen] too to exercise the
  /// failed-AND-closed → OnboardingFailed route).
  Object? failRescan;

  /// When set, [deleteWallet] throws this (the wipe-fault path the controller
  /// recovers from by re-opening — set [failOpen] too to exercise the
  /// failed-AND-closed → OnboardingFailed route). A delete that throws does NOT
  /// flip [exists] (the keychain-first wipe deletes nothing on a fault).
  Object? failDelete;

  /// When set, [forceDeleteWallet] throws this (the rare post-force filesystem
  /// fault — the controller re-probes to a still-escapable failure). A force
  /// delete that throws does NOT flip [exists].
  Object? failForceDelete;

  /// Counts [forceDeleteWallet] calls (the #251 escape-hatch assertion).
  int forceDeleteCount = 0;

  /// When set, [custodyDisclosure] throws this.
  Object? failCustodyDisclosure;

  /// What [custodyDisclosure] returns — a hardware tier by default; override to
  /// exercise the best-effort copy (e.g. `tier: 'none'` /
  /// `eraseAssurance: EraseAssurance.bestEffort`).
  CustodyDisclosure disclosure = const CustodyDisclosure(
    tier: 'apple_secure_enclave',
    eraseAssurance: EraseAssurance.hardwareKeyDeleted,
    degraded: false,
  );

  /// When set, [rescanFrom] blocks on this until completed (AFTER the call is
  /// counted + recorded) — so a test can observe the transient running window.
  Completer<void>? holdRescan;

  /// When set, [deleteWallet] blocks on this until completed (AFTER the call is
  /// counted) — so a test can hold a delete in flight and prove a concurrent
  /// rescan is refused (the delete↔rescan mutual-exclusion).
  Completer<void>? holdDelete;

  /// When set, [open] blocks on this until completed — so a test can dispose
  /// the container WHILE an open is in flight (the disposal-safety guard).
  Completer<void>? holdOpen;

  /// When set, [createGenerated] blocks on this until completed (AFTER the
  /// call is counted and BEFORE it marks the wallet existing) — so a test can
  /// observe the transient OnboardingGenerating window on a slow seed-seal, or
  /// drive the controller deterministically to Generating. Mirrors [holdOpen].
  Completer<void>? holdCreate;

  /// When set, [restore] blocks on this until completed (AFTER the call is
  /// counted and recorded, BEFORE it marks the wallet existing) — so a test can
  /// observe the transient OnboardingRestoring window. Mirrors [holdCreate].
  Completer<void>? holdRestore;

  /// The UFVK string [exportUfvk] returns. Override per test.
  String exportedUfvk = 'uview1fakeexportedviewingkeyxxxxxxxxxxxxxxxxxxxxxxxx';

  /// When set, [createWatchOnly] blocks on this until completed (AFTER the call
  /// is counted + recorded) — the transient creating window. Mirrors [holdCreate].
  Completer<void>? holdCreateWatchOnly;

  int walletExistsCount = 0;
  int createCount = 0;
  int openCount = 0;
  int revealCount = 0;
  int exportUfvkCount = 0;
  int restoreCount = 0;
  int createWatchOnlyCount = 0;

  /// What [createWatchOnly] last received — so a test can assert the pasted
  /// artifact + the picker's estimated birthday reached the adapter verbatim.
  String? lastWatchOnlyUfvk;
  int? lastWatchOnlyBirthdayHeight;
  int rescanCount = 0;
  int deleteCount = 0;
  int custodyDisclosureCount = 0;

  /// What [rescanFrom] last received — so a test can assert the sheet's
  /// selection reached the adapter as the intended sealed arm (a picked date
  /// / the wallet's floor height / all-history).
  RescanTarget? lastRescanTarget;

  /// What [estimateBirthdayHeight] last received — the size-cue threading.
  DateTime? lastEstimatedTime;

  /// What [restore] last received — so a test can assert the lowercase + trim
  /// contract held (the words reached the SDK normalized) and the birthday date
  /// was threaded.
  List<String>? lastRestoreWords;
  DateTime? lastRestoreCreationTime;

  @override
  Future<bool> walletExists() async {
    walletExistsCount++;
    if (failWalletExists != null) throw failWalletExists!;
    return exists;
  }

  @override
  Future<WalletSession> createGenerated() async {
    createCount++;
    if (holdCreate != null) await holdCreate!.future;
    if (failCreate != null) throw failCreate!;
    exists = true; // a created wallet now exists on disk
    return session;
  }

  @override
  Future<WalletSession> open() async {
    openCount++;
    if (holdOpen != null) await holdOpen!.future;
    if (failOpen != null) throw failOpen!;
    return session;
  }

  @override
  Future<WalletSession> restore(
    List<String> mnemonicWords, {
    DateTime? approximateCreationTime,
  }) async {
    restoreCount++;
    lastRestoreWords = mnemonicWords;
    lastRestoreCreationTime = approximateCreationTime;
    if (holdRestore != null) await holdRestore!.future;
    if (failRestore != null) throw failRestore!;
    exists = true; // a restored wallet now exists on disk
    return session;
  }

  @override
  Future<WalletSession> createWatchOnly(
    String ufvk, {
    required int birthdayHeight,
  }) async {
    createWatchOnlyCount++;
    lastWatchOnlyUfvk = ufvk;
    lastWatchOnlyBirthdayHeight = birthdayHeight;
    if (holdCreateWatchOnly != null) await holdCreateWatchOnly!.future;
    if (failCreateWatchOnly != null) throw failCreateWatchOnly!;
    exists = true; // a watch-only wallet now exists on disk
    (session as FakeWalletSession).isWatchOnlyResult = true;
    return session;
  }

  @override
  Future<String> exportUfvk() async {
    exportUfvkCount++;
    if (failExportUfvk != null) throw failExportUfvk!;
    return exportedUfvk;
  }

  @override
  Future<WalletSession> rescanFrom(RescanTarget target) async {
    rescanCount++;
    lastRescanTarget = target;
    if (holdRescan != null) await holdRescan!.future;
    if (failRescan != null) throw failRescan!;
    return rescanSession ?? _defaultRescanSession;
  }

  // ── The sync-server picker (P3-13) ────────────────────────────────────────

  /// The session [switchSyncServer] returns (a FRESH instance by default, the
  /// graph-rebuild trigger the controller's swap relies on).
  WalletSession? switchSession;

  /// Thrown by [switchSyncServer] instead of returning — a typed
  /// `WalletApiError` drives the controller's refused / recovered arms.
  Object? failSwitch;

  /// Holds [switchSyncServer] open until completed (in-flight latch tests).
  Completer<void>? holdSwitch;

  int switchCount = 0;
  SyncServerChoice? lastSwitchChoice;

  @override
  Future<WalletSession> switchSyncServer(SyncServerChoice choice) async {
    switchCount++;
    lastSwitchChoice = choice;
    if (holdSwitch != null) await holdSwitch!.future;
    if (failSwitch != null) throw failSwitch!;
    return switchSession ?? FakeWalletSession();
  }

  /// Fixed blocks-per-day slope so estimate-threading tests are deterministic
  /// (the real adapter rides the SDK's checkpoint-backed estimator).
  @override
  int estimateBirthdayHeight(DateTime time) {
    lastEstimatedTime = time;
    return estimateHeightFor(time);
  }

  /// The fake estimator: days since 2020-01-01 × 1152 blocks (the ~75s
  /// cadence), floored at 1. Public so a test can compute its expectation
  /// through the same slope.
  static int estimateHeightFor(DateTime time) {
    final days = time.toUtc().difference(DateTime.utc(2020)).inDays;
    return math.max(1, days * 1152);
  }

  @override
  Future<List<String>> revealMnemonic() async {
    revealCount++;
    if (failReveal != null) throw failReveal!;
    return recoveryWords;
  }

  @override
  Future<CustodyDisclosure> custodyDisclosure() async {
    custodyDisclosureCount++;
    if (failCustodyDisclosure != null) throw failCustodyDisclosure!;
    return disclosure;
  }

  @override
  Future<void> deleteWallet() async {
    deleteCount++;
    if (holdDelete != null) await holdDelete!.future;
    if (failDelete != null) throw failDelete!;
    exists = false; // the wallet is erased — gone from disk
  }

  @override
  Future<void> forceDeleteWallet() async {
    forceDeleteCount++;
    if (failForceDelete != null) throw failForceDelete!;
    exists = false; // the unreadable remnant is force-removed — gone from disk
  }
}

/// An in-memory [OnboardingStore]. Backs the backup-confirmed flag; can be made
/// to throw on persist to exercise the gate-stays-closed path. Seedable so a
/// "previous launch" state (a confirmed or unconfirmed prior wallet) is set up.
class FakeOnboardingStore implements OnboardingStore {
  FakeOnboardingStore({bool? confirmed}) : _confirmed = confirmed;

  bool? _confirmed;

  /// When set, [setBackupConfirmed] throws (a prefs write failing). The stored
  /// value is left UNCHANGED so the test can assert the gate stayed closed.
  Object? failSet;

  /// When set, [setBackupConfirmed] throws ONLY on the `confirmed: true` write —
  /// so a restore test can let the crash-safe `false` gate-close succeed, the
  /// wallet restore succeed, then fail just the confirm-open persist (the
  /// "restored but confirmation didn't survive → hold at forced backup" path).
  Object? failSetTrue;

  /// When set, [setBackupConfirmed] blocks on this until completed BEFORE it
  /// mutates the flag — mirroring [FakeWalletProvisioner.holdOpen]. Lets a test
  /// dispose the container WHILE a confirm/create-reset persist is in flight,
  /// then release the gate and assert no post-dispose `state =` throws (the
  /// `_disposed` guard after the await must swallow the continuation).
  Completer<void>? holdSet;

  /// When set, [isBackupConfirmed] throws (a degraded prefs read on boot) — so a
  /// test can prove the controller reads the flag BEFORE opening the wallet (a
  /// failed read must not leave an opened-then-orphaned handle holding the lock).
  Object? failGet;

  int getCount = 0;
  int setCount = 0;
  bool? lastSetValue;

  /// How many times the gate was OPENED (a `confirmed: true` write) vs CLOSED
  /// (the create-time `false` write). [setCount] counts both, so a money test
  /// that wants "the confirmation persisted exactly once" asserts on this — a
  /// double-tap that double-activated would push it above 1.
  int trueSetCount = 0;

  /// Direct peek for assertions — what the persisted flag currently is.
  bool? get persisted => _confirmed;

  @override
  Future<bool> isBackupConfirmed() async {
    getCount++;
    if (failGet != null) throw failGet!;
    return _confirmed ?? false;
  }

  @override
  Future<void> setBackupConfirmed({required bool confirmed}) async {
    setCount++;
    if (confirmed) trueSetCount++;
    lastSetValue = confirmed;
    if (holdSet != null) await holdSet!.future;
    if (failSet != null) throw failSet!;
    if (confirmed && failSetTrue != null) throw failSetTrue!;
    _confirmed = confirmed;
  }

  /// The #390 post-restore note lifecycle — seedable so a test can model a
  /// "previous launch" restored-and-pending or already-done wallet.
  DeepScanRestoreNoteState deepScanNote =
      DeepScanRestoreNoteState.notApplicable;

  /// Mirrors [holdSet] for the #390 note write: when set,
  /// [setDeepScanRestoreNoteState] blocks on this BEFORE mutating the flag — so
  /// a test can prove the controller AWAITS the note write BEFORE it flips the
  /// session to OnboardingActive.
  Completer<void>? holdNoteSet;

  int noteSetCount = 0;

  @override
  Future<DeepScanRestoreNoteState> deepScanRestoreNoteState() async =>
      deepScanNote;

  @override
  Future<void> setDeepScanRestoreNoteState(
    DeepScanRestoreNoteState state,
  ) async {
    noteSetCount++;
    if (holdNoteSet != null) await holdNoteSet!.future;
    deepScanNote = state;
  }
}

/// A controllable [appLifecycleProvider] override for DETERMINISTIC lifecycle
/// tests. Overriding the provider bypasses the platform binding entirely — no
/// `handleAppLifecycleStateChanged` transition-legality, no AppLifecycleListener
/// coalescing across pump boundaries (both of which make binding-driven hides
/// flaky and unable to distinguish "this state is not a trigger" from "the event
/// never fired"). `emit` drives the state directly so a `ref.listen` fires for
/// exactly the state asked for. Subclasses the real [AppLifecycleNotifier] (so
/// it matches the provider's `overrideWith` type) but overrides `build` to NOT
/// register the platform `AppLifecycleListener` — the binding plays no part.
class TestLifecycleNotifier extends AppLifecycleNotifier {
  @override
  AppLifecycleState build() => AppLifecycleState.resumed;

  void emit(AppLifecycleState next) => state = next;
}

/// A host-VM fake [ScreenSecurity] — counts the backup screen's protection
/// requests so a widget test can assert it engages on show and releases on
/// dispose. [isScreenshotBlockSupported] is configurable so a test can drive
/// both the Android (block) and the elsewhere (advise-a-private-setting) copy.
class FakeScreenSecurity implements ScreenSecurity {
  FakeScreenSecurity({this.isScreenshotBlockSupported = true, bool? engages})
    : _engages = engages;

  @override
  final bool isScreenshotBlockSupported;

  /// What [enable] reports back — whether the (fake) native side ENGAGED.
  /// Defaults to the capability, mirroring a correctly wired host; pass
  /// `engages: false` to play a capable platform whose host never wired the
  /// native handler (the B1 shape — the UI must stay pessimistic).
  late final bool engages = _engages ?? isScreenshotBlockSupported;
  final bool? _engages;

  int enableCount = 0;
  int disableCount = 0;

  @override
  Future<bool> enable() async {
    enableCount++;
    return engages;
  }

  @override
  Future<void> disable() async => disableCount++;
}

/// A tiny BIP39 wordlist fixture for the restore widget tests — just the words
/// the tests use (sorted, lowercase, like the real bundled asset). The restore
/// field's live validity/autocomplete reads this; the SDK is still the real gate.
/// Restore harnesses override `bip39WordlistProvider` with this so validity is
/// deterministic (every test word reads valid → the count-only gate applies).
final testBip39Wordlist = Bip39Wordlist.fromLines(
  'abandon\nability\nable\nabout\nart\nzoo\n',
);

// The canonical 24-word BIP39 vector — the fake's default reveal payload (its
// exact value is irrelevant to the state machine, which never inspects words).
const _vectorWords = <String>[
  'abandon', 'abandon', 'abandon', 'abandon', 'abandon', 'abandon', //
  'abandon', 'abandon', 'abandon', 'abandon', 'abandon', 'abandon', //
  'abandon', 'abandon', 'abandon', 'abandon', 'abandon', 'abandon', //
  'abandon', 'abandon', 'abandon', 'abandon', 'abandon', 'art', //
];
