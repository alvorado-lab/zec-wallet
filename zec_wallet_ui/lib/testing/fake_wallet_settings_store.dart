import 'dart:async';

import '../features/wallet/transparent_funds/wallet_settings_store.dart';

/// An in-memory [WalletSettingsStore] for tests — scripted reads (parkable via
/// [readGate] to pin the loading-window rules) and recorded writes (so a test
/// proves persist-then-publish and the failed-write honesty).
class FakeWalletSettingsStore implements WalletSettingsStore {
  FakeWalletSettingsStore({this.expertValue, this.autoShieldValue});

  /// The stored flags; `null` models unset (each read then applies its own
  /// fail-safe default: expert `false`, auto-shield `true`).
  bool? expertValue;
  bool? autoShieldValue;

  /// When set, every READ parks until this completes — the disk-read window
  /// the loading rules guard (a persisted OFF must win the race against the
  /// first sync edge).
  Completer<void>? readGate;

  /// When true, every WRITE throws — the platform reporting a failed persist.
  bool failWrites = false;

  /// When set, every WRITE parks until this completes — the persist window a
  /// mid-write identity re-key must survive (the notifier drops a stale publish
  /// if the store changed while the write was in flight).
  Completer<void>? writeGate;

  /// When true, every READ throws — the AsyncError flag state (a failed
  /// SharedPreferences read) the retry-on-open + read-error rules guard
  /// (both shipped unpinned because this knob was missing).
  bool failReads = false;

  int reads = 0;

  int expertWrites = 0;
  int autoShieldWrites = 0;

  @override
  Future<bool> isExpertTransparentFunds() async {
    reads++;
    final gate = readGate;
    if (gate != null) await gate.future;
    if (failReads) throw Exception('scripted settings read failure');
    return expertValue ?? false;
  }

  @override
  Future<void> setExpertTransparentFunds({required bool enabled}) async {
    expertWrites++;
    final gate = writeGate;
    if (gate != null) await gate.future;
    if (failWrites) throw Exception('scripted settings write failure');
    expertValue = enabled;
  }

  @override
  Future<bool> isAutoShieldEnabled() async {
    reads++;
    final gate = readGate;
    if (gate != null) await gate.future;
    if (failReads) throw Exception('scripted settings read failure');
    return autoShieldValue ?? true;
  }

  @override
  Future<void> setAutoShieldEnabled({required bool enabled}) async {
    autoShieldWrites++;
    final gate = writeGate;
    if (gate != null) await gate.future;
    if (failWrites) throw Exception('scripted settings write failure');
    autoShieldValue = enabled;
  }
}
