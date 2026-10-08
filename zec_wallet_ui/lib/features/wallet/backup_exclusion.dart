import 'dart:io' show Platform;

import 'package:flutter/services.dart';

/// Excludes the wallet DB directory from iOS device/iCloud backups
/// (`NSURLIsExcludedFromBackupKey`, set natively in `AppDelegate.swift`).
///
/// WHY (FR-14 / spec §3.2): the DB's encryption key is a `ThisDeviceOnly` Secure-
/// Enclave key that never leaves the device. The DB itself sits under Application
/// Support, which iCloud/iTunes back up by default. So a user who restores an iPhone
/// backup onto a NEW device (a new phone, or an erase-and-restore) gets the DB back
/// but NOT the key → the wallet can't be opened → the "Wallet setup couldn't finish"
/// (needsRecovery) screen. Excluding the DB dir means a device-restore brings back no
/// wallet data, so the app lands on the clean Welcome → Restore flow instead — and the
/// encrypted (and, off-device, useless) DB never sits in a cloud backup at all.
///
/// iOS-only: Android excludes app data app-wide via `allowBackup=false`; desktop/web
/// have no equivalent. The host owns this (the SDK is platform-agnostic and documents
/// it as the host's responsibility). `isIOS` is injectable so the gate is testable
/// without a real platform.
class BackupExclusion {
  const BackupExclusion({MethodChannel? channel, bool? isIOS})
    : _channel = channel ?? const MethodChannel(channelName),
      _isIOS = isIOS;

  static const String channelName = 'zec_wallet_ui/backup_exclusion';

  final MethodChannel _channel;
  final bool? _isIOS;

  /// Mark [path] excluded from backup. BEST-EFFORT: a failure NEVER blocks boot —
  /// the wallet still works; the only loss is a backup would carry the encrypted
  /// (cross-device-unusable) DB. A no-op off iOS.
  Future<void> exclude(String path) async {
    if (!(_isIOS ?? Platform.isIOS)) return;
    try {
      await _channel.invokeMethod<bool>('excludeFromBackup', path);
    } catch (_) {
      // Best-effort — never fail boot on a backup-flag write (a missing native
      // handler, an older OS, a transient FS error). The DB is encrypted regardless.
    }
  }
}
