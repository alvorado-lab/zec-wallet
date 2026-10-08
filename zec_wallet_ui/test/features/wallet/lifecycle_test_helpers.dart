import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// Bring the test binding back to `resumed` from wherever a lifecycle test left
/// it, through LEGAL transitions (the `AppLifecycleListener` asserts on the
/// state-machine chain `resumed↔inactive↔hidden↔paused`), so the next test in
/// the file starts foreground. Shared by the per-stream lifecycle suites (sync,
/// swap, …) so a new one never adds a third copy.
void goResumedFromPausedSafeReset(TestWidgetsFlutterBinding binding) {
  final s = binding.lifecycleState;
  if (s == null || s == AppLifecycleState.resumed) return;
  // Only `paused`/`hidden`/`inactive` are reachable here; walk up the chain.
  if (s == AppLifecycleState.paused) {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
  }
  if (s == AppLifecycleState.paused || s == AppLifecycleState.hidden) {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
  }
  binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
}
