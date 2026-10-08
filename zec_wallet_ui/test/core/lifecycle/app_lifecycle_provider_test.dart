import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';

void main() {
  final binding = TestWidgetsFlutterBinding.ensureInitialized();

  // testWidgets (not bare test) for per-test binding isolation
  // (reliability review fold).
  testWidgets('lifecycle_provider_emits_pause_and_resume', (tester) async {
    // Pin a known foreground start regardless of test ordering.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    final container = ProviderContainer();
    addTearDown(container.dispose);

    expect(container.read(appLifecycleProvider), AppLifecycleState.resumed);

    // Walk the LEGAL lifecycle state machine (AppLifecycleListener
    // asserts on transitions): resumed → inactive → hidden → paused.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    expect(container.read(appLifecycleProvider), AppLifecycleState.paused);

    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    expect(container.read(appLifecycleProvider), AppLifecycleState.resumed);

    // Graceful exit path: paused → detached must pass through
    // unconditionally (mobile review fold — the framework asserts
    // detached is only reachable from paused).
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.detached);
    expect(container.read(appLifecycleProvider), AppLifecycleState.detached);
  });

  testWidgets('lifecycle_provider_initial_state_reads_binding', (tester) async {
    // Android trampoline cold-start: the app can be born BACKGROUNDED
    // (push-triggered process start). The provider's first value must be
    // the binding's real state, not a hardcoded `resumed` (reliability
    // review fold).
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);

    final container = ProviderContainer();
    addTearDown(container.dispose);
    expect(container.read(appLifecycleProvider), AppLifecycleState.paused);

    // Restore foreground so later tests in this file start clean.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });
}
