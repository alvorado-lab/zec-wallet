import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_tokens_provider.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The no-silent-retry pins (the s198_tail audit).
///
/// Riverpod 3's container default retries any non-`Error` exception up to 10
/// times with backoff (first delay 200 ms) — and `FrbException implements
/// Exception`, so every typed FFI failure rides it. What the swallow cost
/// each pinned reader (rationale CORRECTED by the review probe):
///  * custody + tokens: their consumers use default `when` flags, so the
///    honest error genuinely could not render until the retries exhausted —
///    the custody probe spun before the delete-decision screen and the token
///    picker spun through up-to-ten 30 s wedge cycles.
///  * parked + in-flight-swaps: their sections' `skipLoadingOnReload` already
///    surfaced the error arm after the FIRST failure (a retrying state
///    carries `hasError`), so there the pin kills the up-to-ten WASTED
///    background FFI retries cycling behind the line and settles the state —
///    at the accepted cost that a one-shot blip leaves the error standing
///    until the next invalidation edge (the reader's own retry cadence)
///    instead of self-healing in ~450 ms.
///
/// Each test lets the would-be first retry window ELAPSE under fake time
/// (pump ≥ 200 ms): at a parent without the pin the second underlying call
/// fires and the count assertion fails — the tests are discriminating, not
/// decorative (parent-verified: 1 vs 3 calls on all four). The deliberate
/// KEEPS (snapshot / recoverable / in-flight sends) are documented at their
/// readers and NOT pinned here.
void main() {
  Future<ProviderContainer> mount(
    WidgetTester tester, {
    FakeWalletSession? session,
    FakeWalletProvisioner? provisioner,
  }) async {
    final container = ProviderContainer(
      overrides: [
        if (session != null) walletSessionProvider.overrideWithValue(session),
        if (provisioner != null)
          walletProvisionerProvider.overrideWithValue(provisioner),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    return container;
  }

  /// Settle the initial read, then let the container-default retry's first
  /// backoff window (200 ms) elapse — a wrongly-scheduled silent retry fires
  /// its second underlying call inside this pump.
  Future<void> settlePastRetryWindow(WidgetTester tester) async {
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
  }

  testWidgets(
    'a parked-sends read failure surfaces in ONE attempt — no wasted retry '
    'cycles behind the already-honest error line',
    (tester) async {
      final fake = FakeWalletSession()
        ..listParkedSendsThrows = TimeoutException('wedged FFI');
      final container = await mount(tester, session: fake);
      container.listen(walletParkedSendsProvider, (_, _) {});
      await settlePastRetryWindow(tester);

      expect(container.read(walletParkedSendsProvider).hasError, isTrue);
      expect(fake.listParkedSendsCount, 1, reason: 'no silent retry');
    },
  );

  testWidgets(
    'an in-flight-swaps read failure surfaces in ONE attempt — the only '
    'wallet-side witness of a mid-flight swap settles honest, no covert '
    'retry cycling',
    (tester) async {
      final fake = FakeWalletSession()
        ..listInFlightSwapsThrows = TimeoutException('wedged FFI');
      final container = await mount(tester, session: fake);
      container.listen(walletInFlightSwapsProvider, (_, _) {});
      await settlePastRetryWindow(tester);

      expect(container.read(walletInFlightSwapsProvider).hasError, isTrue);
      expect(fake.listInFlightSwapsCount, 1, reason: 'no silent retry');
    },
  );

  testWidgets(
    'a custody-probe failure surfaces in ONE attempt — the honest probe-error '
    'card lands before the user reaches the delete decision, not ~38 s later',
    (tester) async {
      final provisioner = FakeWalletProvisioner()
        ..failCustodyDisclosure = TimeoutException('keychain wedge');
      final container = await mount(tester, provisioner: provisioner);
      container.listen(walletCustodyDisclosureProvider, (_, _) {});
      await settlePastRetryWindow(tester);

      expect(container.read(walletCustodyDisclosureProvider).hasError, isTrue);
      expect(provisioner.custodyDisclosureCount, 1, reason: 'no silent retry');
    },
  );

  testWidgets(
    'a token-list failure surfaces in ONE attempt — the picker error-with-'
    'retry renders instead of swallowed wedge cycles',
    (tester) async {
      final fake = FakeWalletSession()
        ..swapListTokensThrows = TimeoutException('wedged FFI');
      final container = await mount(tester, session: fake);
      container.listen(swapTokensProvider, (_, _) {});
      await settlePastRetryWindow(tester);

      expect(container.read(swapTokensProvider).hasError, isTrue);
      expect(fake.swapListTokensCount, 1, reason: 'no silent retry');
    },
  );

  testWidgets(
    'the deliberate KEEP: a transient snapshot fault self-heals via the '
    'container default retry under the view\'s retention',
    (tester) async {
      // First read fails once (an Exception, so the default retry fires);
      // the retry succeeds — the view lands on data without any manual
      // invalidation. This pins the documented KEEP at
      // walletSnapshotReadProvider: pinning retry off there would break this.
      final fake = _FlakySnapshotSession();
      final container = await mount(tester, session: fake);
      container.listen(walletSnapshotProvider, (_, _) {});
      await settlePastRetryWindow(tester);

      expect(fake.snapshotCalls, greaterThanOrEqualTo(2));
      expect(
        container.read(walletSnapshotProvider).hasValue,
        isTrue,
        reason: 'the transient blip healed silently',
      );
    },
  );
}

/// A session whose FIRST snapshot read fails with an `Exception` (the
/// container-default-retryable class) and every later read succeeds.
class _FlakySnapshotSession extends FakeWalletSession {
  int snapshotCalls = 0;

  @override
  Future<WalletState> snapshot() {
    snapshotCalls++;
    if (snapshotCalls == 1) {
      return Future<WalletState>.error(TimeoutException('transient blip'));
    }
    return super.snapshot();
  }
}
