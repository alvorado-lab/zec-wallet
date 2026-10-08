import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
// The BARREL for what a host touches: FR-51's contract is that a host reaches
// the drive through the public API, never a deep import.
import 'package:zec_wallet_ui/zec_wallet_ui.dart';
import 'package:zec_wallet_ui/testing.dart';
// Test-only: the name the wallet screen watches (the identity check) and the
// drive's state enum, which is deliberately NOT exported (a host must not word
// copy per drive state; see walletSyncPassesRunProvider's doc).
import 'package:zec_wallet_ui/features/wallet/wallet_sync_controller.dart'
    as internal
    show walletSyncControllerProvider, WalletSyncDrive;

/// FR-51 — a host runs sync whenever the app is in the foreground by listening
/// to [walletSyncDriveProvider] at its root, with no wallet screen mounted.
/// The drive's own behaviour (lifecycle, policy, failure) is pinned in
/// `wallet_sync_controller_test.dart`; this pins the PUBLIC door to it, and
/// the lock/unlock cycle a persistent root listen newly makes routine.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  /// A root-listened drive over a switchable session (null = locked/closed).
  Future<(ProviderContainer, StateProvider<WalletSession?>)> rootListen(
    WidgetTester tester,
  ) async {
    final session = StateProvider<WalletSession?>((ref) => null);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(session)),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    // What the README tells a host to do at its root.
    container.listen(walletSyncDriveProvider, (_, _) {});
    await tester.pumpAndSettle();
    return (container, session);
  }

  testWidgets('a host root listen, built before the wallet opens, starts sync '
      'when the session arrives — no wallet screen anywhere', (tester) async {
    final (container, session) = await rootListen(tester);
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.inactive,
      reason: 'no wallet yet: nothing started, nothing to cost',
    );

    // The host opens its wallet (Relim: after unlock).
    final fake = FakeWalletSession();
    container.read(session.notifier).state = fake;
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.running,
    );
    await tester.pumpAndSettle();
    expect(fake.startCount, 1, reason: 'the loop started with no screen');
    expect(find.byType(WalletScreen), findsNothing);
  });

  testWidgets('lock then unlock under one root listen: a FAILED start on the '
      'first session never survives into the next, the closed one is stopped, '
      'and the new one starts once (the #330 and #407 R7 class, now routine)', (
    tester,
  ) async {
    final (container, session) = await rootListen(tester);

    final first = FakeWalletSession()..failStart = true;
    container.read(session.notifier).state = first;
    container.read(walletSyncDriveProvider);
    await tester.pumpAndSettle();
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.failed,
      reason: 'precondition: the first session failed to start',
    );

    // The host locks (the session closes).
    container.read(session.notifier).state = null;
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.inactive,
    );
    await tester.pumpAndSettle();
    expect(first.stopCount, 1, reason: 'the closed session was stopped');

    // The host unlocks: a fresh session.
    final second = FakeWalletSession();
    container.read(session.notifier).state = second;
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.running,
      reason: 'the first session\'s failure does not ride the unlock',
    );
    await tester.pumpAndSettle();
    expect(second.startCount, 1);
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.running,
    );
    expect(first.startCount, 1, reason: 'the closed session never restarts');
  });

  testWidgets('a session that opens while the app is BACKGROUNDED after a '
      'failed one reads suspended, not the old failure (the only arm that reads '
      'the remembered failure, #407 R7)', (tester) async {
    final binding = TestWidgetsFlutterBinding.instance;
    final (container, session) = await rootListen(tester);
    final first = FakeWalletSession()..failStart = true;
    container.read(session.notifier).state = first;
    container.read(walletSyncDriveProvider);
    await tester.pumpAndSettle();
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.failed,
    );
    container.read(session.notifier).state = null;
    container.read(walletSyncDriveProvider);
    await tester.pumpAndSettle();

    // Background (a legal walk), then a fresh session opens.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    addTearDown(() {
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    });
    final second = FakeWalletSession();
    container.read(session.notifier).state = second;
    expect(
      container.read(walletSyncDriveProvider),
      internal.WalletSyncDrive.suspended,
      reason: 'a new wallet has not failed anything',
    );
    await tester.pumpAndSettle();
    expect(second.startCount, 0, reason: 'nothing starts in the background');
  });

  test('the public name IS the drive the wallet screen watches — a second '
      'provider would run a second loop against the same wallet', () {
    expect(
      identical(walletSyncDriveProvider, internal.walletSyncControllerProvider),
      isTrue,
    );
  });
}
