import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_request.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// FR-25 — the prefilled-send entry. The screen opens with the request's fields
/// seeded into the (editable) form; `lockRecipient` opens the recipient
/// read-only (amount + memo stay editable); and a prefill is NOT pre-confirmed —
/// tapping Review runs the SAME compose → propose path as an organic send, so
/// the seeded values are what get proposed. Behind the fake session, no device.
void main() {
  Widget harness({WalletSendRequest? prefill, FakeWalletSession? session}) {
    final fake =
        session ??
        FakeWalletSession(
          current: const SyncStatus.upToDate(tip: 1),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 1),
            balance: balanceFixture(
              spendableZat: 500000000,
              totalZat: 500000000,
            ),
          ),
        );
    return ProviderScope(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: SendScreen(prefill: prefill),
      ),
    );
  }

  WalletLocalizations l10n(WidgetTester tester) =>
      WalletLocalizations.of(tester.element(find.byType(SendScreen)));

  String fieldText(WidgetTester tester, int i) =>
      tester.widget<TextField>(find.byType(TextField).at(i)).controller!.text;

  bool fieldReadOnly(WidgetTester tester, int i) =>
      tester.widget<TextField>(find.byType(TextField).at(i)).readOnly;

  testWidgets('seeds address, amount and memo into the editable form', (
    tester,
  ) async {
    await tester.pumpWidget(
      harness(
        prefill: const WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          memo: 'coffee',
        ),
      ),
    );
    await tester.pumpAndSettle();
    // address = 0, amount = 1, memo = 2.
    expect(fieldText(tester, 0), 'u1alice');
    expect(fieldText(tester, 1), '0.0015'); // formatZec(150000)
    expect(fieldText(tester, 2), 'coffee');
    // Editable by default — nothing locked.
    expect(fieldReadOnly(tester, 0), isFalse);
    // Review is enabled (the default fake classifies the recipient valid).
    final review = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    expect(review.onPressed, isNotNull);
  });

  testWidgets('an amount-less (donation) prefill leaves the amount empty', (
    tester,
  ) async {
    await tester.pumpWidget(
      harness(prefill: const WalletSendRequest(address: 'u1alice')),
    );
    await tester.pumpAndSettle();
    expect(fieldText(tester, 0), 'u1alice');
    expect(fieldText(tester, 1), isEmpty);
    expect(fieldText(tester, 2), isEmpty);
  });

  testWidgets('lockRecipient opens the recipient read-only; amount/memo stay '
      'editable', (tester) async {
    await tester.pumpWidget(
      harness(
        prefill: const WalletSendRequest(
          address: 'u1locked',
          amountZat: 100000,
          lockRecipient: true,
        ),
      ),
    );
    await tester.pumpAndSettle();
    // Recipient is read-only and its lock helper is shown/announced.
    expect(fieldReadOnly(tester, 0), isTrue);
    expect(fieldText(tester, 0), 'u1locked');
    expect(find.text(l10n(tester).walletSendRecipientLocked), findsOneWidget);
    // Recipient-ONLY: amount and memo remain editable.
    expect(fieldReadOnly(tester, 1), isFalse);
    expect(fieldReadOnly(tester, 2), isFalse);
  });

  testWidgets('no prefill leaves the organic form entirely empty', (
    tester,
  ) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();
    expect(fieldText(tester, 0), isEmpty);
    expect(fieldText(tester, 1), isEmpty);
    expect(fieldText(tester, 2), isEmpty);
    expect(fieldReadOnly(tester, 0), isFalse);
    expect(find.text(l10n(tester).walletSendRecipientLocked), findsNothing);
  });

  testWidgets('prefilled ≠ pre-confirmed: Review composes+proposes the seeded '
      'values (same path as an organic send)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
      ),
    );
    await tester.pumpWidget(
      harness(
        session: fake,
        prefill: const WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          memo: 'coffee',
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n(tester).walletSendReviewButton));
    await tester.pumpAndSettle();
    // The seeded values flowed through the real compose → propose path — a
    // prefill runs validation/review, it does not skip it.
    expect(fake.lastComposeRecipient, 'u1alice');
    expect(fake.lastComposeAmountZat, 150000);
    expect(fake.lastComposeMemo, 'coffee');
    expect(fake.proposeCount, 1);
  });

  testWidgets('a locked recipient is RELEASED on a session flip — never a '
      'read-only empty dead-end (converged review HIGH)', (tester) async {
    FakeWalletSession funded() => FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
      ),
    );
    final sessionSwitch = StateProvider<WalletSession?>((ref) => funded());
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        ],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const SendScreen(
            prefill: WalletSendRequest(
              address: 'u1locked',
              lockRecipient: true,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    // Locked at entry.
    expect(fieldReadOnly(tester, 0), isTrue);
    expect(find.text(l10n(tester).walletSendRecipientLocked), findsOneWidget);

    // The identity flips (decoy/duress) while the locked screen is open.
    ProviderScope.containerOf(
      tester.element(find.byType(SendScreen)),
      listen: false,
    ).read(sessionSwitch.notifier).state = funded();
    await tester.pumpAndSettle();

    // The lock is released: the (now-cleared) recipient field is EDITABLE again,
    // never a read-only empty dead-end — the form is a clean organic send.
    expect(fieldReadOnly(tester, 0), isFalse);
    expect(fieldText(tester, 0), isEmpty);
    expect(find.text(l10n(tester).walletSendRecipientLocked), findsNothing);
  });

  testWidgets('the locked field ANNOUNCES its lock in the field\'s own merged '
      'semantics node (S216 a11y — an ancestor Semantics label, not the '
      'suffix icon)', (tester) async {
    await tester.pumpWidget(
      harness(
        prefill: const WalletSendRequest(
          address: 'u1locked',
          lockRecipient: true,
        ),
      ),
    );
    await tester.pumpAndSettle();
    final handle = tester.ensureSemantics();
    // The FIELD node itself carries the lock label (announced on focus) —
    // pinned at the semantics level so a regression to a sibling-node-only
    // mechanism (suffix icon / helperText) fails here.
    final node = tester.getSemantics(find.byType(TextField).at(0));
    expect(
      node.label,
      contains(l10n(tester).walletSendRecipientLocked),
      reason:
          'the lock must be announced ON FOCUS of the field, not only on '
          'a separate swipe stop',
    );
    handle.dispose();
  });

  testWidgets('an in-place prefill update (didUpdateWidget) RE-SEEDS the form '
      '(S216 MED-1 — a host replace/go with a new extra must never leave the '
      'old request\'s fields under the new one)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
      ),
    );
    await tester.pumpWidget(
      harness(
        session: fake,
        prefill: const WalletSendRequest(
          address: 'u1first',
          amountZat: 100000,
          memo: 'first',
          lockRecipient: true,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(fieldText(tester, 0), 'u1first');
    expect(fieldReadOnly(tester, 0), isTrue);

    // The SAME element receives a NEW widget with a DIFFERENT prefill (an
    // in-place route update) — didUpdateWidget, not initState.
    await tester.pumpWidget(
      harness(
        session: fake,
        prefill: const WalletSendRequest(
          address: 'u1second',
          amountZat: 200000,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(fieldText(tester, 0), 'u1second');
    expect(fieldText(tester, 1), '0.002'); // formatZec(200000)
    expect(fieldText(tester, 2), isEmpty, reason: 'the old memo is cleared');
    expect(
      fieldReadOnly(tester, 0),
      isFalse,
      reason: 'the new request carries no lock — the old lock must not linger',
    );
  });

  testWidgets('a prefilled entry during an IN-FLIGHT step defers its reset to '
      'the landing — the previous send\'s outcome is never attributed to the '
      'new entry (S216 MED-2)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
      ),
    )..proposeGate = Completer<void>();
    // Screen A (organic): drive the controller into an IN-FLIGHT step.
    await tester.pumpWidget(harness(session: fake));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).at(0), 'u1organic');
    await tester.enterText(find.byType(TextField).at(1), '1');
    await tester.pump();
    await tester.tap(find.text(l10n(tester).walletSendReviewButton));
    await tester.pump(); // SendPreparing — held by the gate

    // Screen B: a NEW prefilled entry mounts while the step is in flight (a
    // fresh element via a distinct key — the WalletSendEntry.push shape).
    await tester.pumpWidget(
      ProviderScope(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: SendScreen(
            key: UniqueKey(),
            prefill: const WalletSendRequest(address: 'u1newrequest'),
          ),
        ),
      ),
    );
    await tester.pump();
    // The reset was a no-op (in-flight) — the entry honestly re-attaches to
    // the running step (its landing must not be swallowed)…
    expect(find.text(l10n(tester).walletSendPreparing), findsOneWidget);

    // …and the moment the step LANDS, the deferred reset fires: the NEW
    // entry's fresh prefilled form renders — never the old step's outcome.
    fake.proposeGate!.complete();
    await tester.pumpAndSettle();
    expect(fieldText(tester, 0), 'u1newrequest');
    expect(
      find.text(l10n(tester).walletSendReviewButton),
      findsOneWidget,
      reason: 'the new entry lands on ITS form, not the old flow\'s review',
    );
  });

  testWidgets('a lock over a NON-sendable recipient falls back to editable '
      '(never a read-only address that can never pass Review)', (tester) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 1),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 1),
              balance: balanceFixture(
                spendableZat: 500000000,
                totalZat: 500000000,
              ),
            ),
          )
          ..validateRecipientThrows = WalletApiError(
            code: 'RW-TEST',
            message: 'static',
            kind: const WalletErrorKind.addressInvalid(),
          );
    await tester.pumpWidget(
      harness(
        session: fake,
        prefill: const WalletSendRequest(
          address: 'not-an-address',
          lockRecipient: true,
        ),
      ),
    );
    await tester.pumpAndSettle();
    // The recipient classifies invalid → the lock is NOT applied, so the user
    // can correct the address (the inline status explains why).
    expect(fieldReadOnly(tester, 0), isFalse);
    expect(find.text(l10n(tester).walletSendRecipientLocked), findsNothing);
  });
}
