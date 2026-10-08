import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
// `Override` (the list element type) is surfaced from misc.dart in Riverpod 3.x.
import 'package:flutter_riverpod/misc.dart' show Override;

import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import '../../app.dart';
import '../../l10n/app_localizations.dart';
import '../theme/appearance_prefs.dart';
import '../theme/example_type.dart';

/// The persisted appearance prefs the boot path loads before `runApp` — held
/// here (not main.dart) so the boot gate below can render the failure shell
/// flash-free and the gate is testable without the real startup work.
class AppearanceBootPrefs {
  const AppearanceBootPrefs({
    required this.themeMode,
    required this.textScale,
    required this.amoled,
  });
  final ThemeMode themeMode;
  final double textScale;
  final bool amoled;
}

/// One bounded wallet-startup attempt: resolves the COMPLETE ProviderScope
/// override list, or `null` when the wallet couldn't start. Injected so the
/// gate is testable on the host VM (no FFI, no path_provider).
typedef WalletBootAttempt = Future<List<Override>?> Function();

/// The boot gate (#356-F1): renders the real app once the wallet wiring
/// succeeded, or the SDK's honest [WalletStartupFailedScreen] (with a retry
/// that re-runs the bounded startup work via [boot]) when it didn't. State
/// only ever moves failed → ready, so the `ProviderScope` is constructed
/// exactly once, with one fixed override list — never rebuilt with different
/// overrides.
class WalletBootGate extends StatefulWidget {
  const WalletBootGate({
    required this.prefs,
    required this.firstAttempt,
    required this.boot,
    this.attemptedPreRunApp = true,
    super.key,
  });

  final AppearanceBootPrefs prefs;

  /// The result of the pre-`runApp` attempt (`null` = failed) — on MOBILE the
  /// attempt runs under the native splash so the happy path adds no extra
  /// Flutter splash frame. Ignored when [attemptedPreRunApp] is false.
  final List<Override>? firstAttempt;

  /// Whether the first attempt already ran before `runApp` (mobile). DESKTOP
  /// passes false (reliability MED): there is no native splash there, so
  /// the gate runs the first attempt itself behind the themed pending shell —
  /// never a frozen blank window while a slow/hung stage burns its budget.
  final bool attemptedPreRunApp;

  /// Re-run by the failure screen's retry (and by the gate itself when
  /// [attemptedPreRunApp] is false). Must be retry-safe (the reference FFI
  /// init is memoized; the dir resolution is idempotent) and never throw.
  final WalletBootAttempt boot;

  @override
  State<WalletBootGate> createState() => _WalletBootGateState();
}

class _WalletBootGateState extends State<WalletBootGate> {
  List<Override>? _overrides;
  bool _retrying = false;

  /// True only during the in-gate FIRST attempt (desktop) — renders the
  /// pending shell instead of the failure screen.
  bool _firstAttemptPending = false;

  @override
  void initState() {
    super.initState();
    if (widget.attemptedPreRunApp) {
      _overrides = widget.firstAttempt;
    } else {
      _firstAttemptPending = true;
      unawaited(_firstAttempt());
    }
  }

  Future<void> _firstAttempt() async {
    List<Override>? overrides;
    try {
      overrides = await widget.boot();
    } catch (_) {
      // boot() promises never-throw; stay on the failure path if it ever does.
    }
    if (!mounted) return;
    setState(() {
      _overrides = overrides;
      _firstAttemptPending = false;
    });
  }

  Future<void> _retry() async {
    if (_retrying) return; // belt — the button is disabled while retrying
    setState(() => _retrying = true);
    try {
      final overrides = await widget.boot();
      if (!mounted) return;
      setState(() => _overrides = overrides);
    } catch (_) {
      // [boot] promises never-throw; if it ever does, the failure screen must
      // survive its own retry (never a stuck-disabled button over a broken
      // boot — the disease this gate treats).
    } finally {
      if (mounted) setState(() => _retrying = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final overrides = _overrides;
    if (overrides == null) {
      // A minimal shell for the pending/failure surfaces: the persisted theme
      // mode (no flash) + the wallet l10n. Deliberately NOT the full app —
      // the router/ProviderScope only exist once the wallet wiring is real.
      return MaterialApp(
        debugShowCheckedModeBanner: false,
        theme: exampleLightTheme,
        darkTheme: widget.prefs.amoled
            ? exampleAmoledDarkTheme
            : exampleDarkTheme,
        themeMode: widget.prefs.themeMode,
        localizationsDelegates: const [
          ...AppLocalizations.localizationsDelegates,
          walletLocalizationsFallbackDelegate,
        ],
        supportedLocales: AppLocalizations.supportedLocales,
        // Text-scale parity with the real app (reliability LOW): the
        // user's persisted in-app multiplier must apply on the one screen
        // where reading "your funds are not affected" matters most — not
        // only the OS factor.
        builder: (context, child) {
          final mq = MediaQuery.of(context);
          final factor = effectiveTextScale(
            osFactor: mq.textScaler.scale(1.0),
            userScale: widget.prefs.textScale,
          );
          return MediaQuery(
            data: mq.copyWith(textScaler: TextScaler.linear(factor)),
            child: child!,
          );
        },
        // Pending (desktop first attempt): a neutral themed spinner — never a
        // frozen blank window, never a premature failure claim.
        home: _firstAttemptPending
            ? const Scaffold(body: Center(child: CircularProgressIndicator()))
            : WalletStartupFailedScreen(onRetry: _retry, retrying: _retrying),
      );
    }
    return ProviderScope(overrides: overrides, child: const WalletExampleApp());
  }
}
