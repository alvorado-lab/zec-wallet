import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// THE app-lifecycle seam (app-frame spec §3).
///
/// STANDING RULE (review gate, flutter-patterns § Stream Lifecycle):
/// every future FRB stream provider MUST watch this provider and
/// - pause its Rust-side stream on [AppLifecycleState.paused],
/// - resume AND re-fetch latest state on [AppLifecycleState.resumed]
///   ONLY when resuming from an actual pause (track "was paused"
///   locally; changes during the pause are otherwise silently missed).
///   `inactive → resumed` fires on every desktop window-focus change and
///   every mobile notification-shade pull — re-fetching there is wasted
///   network/battery, not correctness.
/// Desktop (spec §9): `paused` never fires on macOS/Linux/Windows — the
/// deepest state is `hidden` and a minimized app keeps running, so
/// streams stay live while hidden. That is the WANTED desktop behavior;
/// do not "fix" it by pausing on `hidden`. (`hidden` is NOT desktop-only:
/// Android fires it as an intermediate state before `paused` during
/// normal backgrounding — the rule holds on both platforms.)
/// ORTHOGONAL RULE — connection loss is not a lifecycle event: a stream
/// that dies mid-foreground (transport error, EOF, timeout) gets no
/// `paused` and, on desktop, no lifecycle transition at all. Every FRB
/// stream provider must ALSO treat stream error/completion as a
/// reconnect trigger, independent of this seam.
/// Without this: battery drain, app killed by the OS, broken UX. The
/// frame ships the seam so the first FFI consumer has no reason to
/// hand-roll its own observer.
class AppLifecycleNotifier extends Notifier<AppLifecycleState> {
  @override
  AppLifecycleState build() {
    final listener = AppLifecycleListener(
      onStateChange: (next) => state = next,
    );
    ref.onDispose(listener.dispose);
    // Before the engine reports anything, the app is in the foreground
    // (we are executing build) — `resumed` is the honest default.
    return WidgetsBinding.instance.lifecycleState ?? AppLifecycleState.resumed;
  }
}

final appLifecycleProvider =
    NotifierProvider<AppLifecycleNotifier, AppLifecycleState>(
      AppLifecycleNotifier.new,
    );
