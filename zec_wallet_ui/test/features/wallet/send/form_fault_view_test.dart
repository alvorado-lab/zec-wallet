import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send/form_fault_view.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart'
    show walletSyncPolicyProvider;
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// Render-level coverage for the inline form-fault view. The classifier routing
/// is pinned in send_state_test.dart; here we confirm the NEW 2e-2b-iv fault
/// reason actually surfaces its honest copy (the orange-transient treatment),
/// not a blank or a wrong string. [catchUp] pins the catch-up cue the view
/// watches for the insufficient-funds detail line (#381 (a)); the default (no
/// override) derives None (the harness has no wallet session). [syncPolicyOff]
/// flips the host's sync-policy seam the view reads itself.
Widget _host(
  SendFormFault fault, {
  bool queueOffered = false,
  WalletCatchUpCue? catchUp,
  bool syncPolicyOff = false,
}) => ProviderScope(
  overrides: [
    if (catchUp != null) walletCatchUpCueProvider.overrideWithValue(catchUp),
    if (syncPolicyOff) walletSyncPolicyProvider.overrideWithValue(false),
  ],
  child: MaterialApp(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: lightTheme,
    home: Scaffold(
      body: SendFormFaultView(fault: fault, queueOffered: queueOffered),
    ),
  ),
);

void main() {
  testWidgets('the one-time-address-limit fault renders its honest copy '
      '(2e-2b-iv)', (tester) async {
    await tester.pumpWidget(
      _host(const SendCategoricalFault(SendFaultReason.oneTimeAddressLimit)),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );

    expect(find.text(l10n.walletSendFaultOneTimeAddressLimit), findsOneWidget);
    // The inline treatment is the orange warning (never the red money-loss
    // treatment) — the icon the view always uses for a fixable fault.
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
  });

  testWidgets('the storage-full fault renders the free-up-space copy (#373), '
      'as the orange fixable-fault treatment — never the red money-loss one', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(const SendCategoricalFault(SendFaultReason.storageFull)),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );
    expect(find.text(l10n.walletSendFaultStorageFull), findsOneWidget);
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
  });

  testWidgets('the not-synced copy names the queue ONLY where the surface '
      'offers it — the DEFAULT is the no-queue line (S152 review H1)', (
    tester,
  ) async {
    // Default (queueOffered false — e.g. the move-to-transparent sheet, or
    // the send form under a no-drain custody): the wait-for-sync line, no
    // invitation to an affordance the surface doesn't render.
    await tester.pumpWidget(
      _host(const SendCategoricalFault(SendFaultReason.notSyncedYet)),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );
    expect(find.text(l10n.walletSendFaultNotSyncedNoQueue), findsOneWidget);
    expect(find.text(l10n.walletSendFaultNotSynced), findsNothing);

    // The surface that DOES offer the queue keeps the invitation copy.
    await tester.pumpWidget(
      _host(
        const SendCategoricalFault(SendFaultReason.notSyncedYet),
        queueOffered: true,
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSendFaultNotSynced), findsOneWidget);
    expect(find.text(l10n.walletSendFaultNotSyncedNoQueue), findsNothing);
  });

  testWidgets('S205-b: under the host\'s sync-off policy the not-synced arm '
      'renders the SYNC-OFF copy — over BOTH queueOffered shapes (the view '
      'reads the policy itself, so Send and Move tell the same story)', (
    tester,
  ) async {
    // queueOffered false (the Move sheet / no-queue custody shape).
    await tester.pumpWidget(
      _host(
        const SendCategoricalFault(SendFaultReason.notSyncedYet),
        syncPolicyOff: true,
      ),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );
    expect(
      find.text(l10n.walletSendFaultNotSyncedSyncNotRunning),
      findsOneWidget,
    );
    expect(find.text(l10n.walletSendFaultNotSynced), findsNothing);
    expect(find.text(l10n.walletSendFaultNotSyncedNoQueue), findsNothing);

    // Even a (contradictory) queueOffered=true must not resurrect the
    // catch-up promise — the sync-off arm owns the copy outright.
    await tester.pumpWidget(
      _host(
        const SendCategoricalFault(SendFaultReason.notSyncedYet),
        queueOffered: true,
        syncPolicyOff: true,
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.text(l10n.walletSendFaultNotSyncedSyncNotRunning),
      findsOneWidget,
    );
    expect(find.text(l10n.walletSendFaultNotSynced), findsNothing);
  });

  testWidgets('the WIDGET default (no param) is the no-queue line — the Move '
      'sheet passes nothing, so a flipped default would silently reintroduce '
      'the queue lie there (S152 wrap review H2 mutation gap)', (tester) async {
    // Deliberately NOT via _host (whose own default would mask the widget's):
    // the bare constructor call is exactly the Move sheet's call shape.
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const Scaffold(
            body: SendFormFaultView(
              fault: SendCategoricalFault(SendFaultReason.notSyncedYet),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );
    expect(find.text(l10n.walletSendFaultNotSyncedNoQueue), findsOneWidget);
    expect(find.text(l10n.walletSendFaultNotSynced), findsNothing);
  });

  testWidgets('the insufficient-funds fault appends the catching-up detail '
      'ONLY while the wallet is below tip — the partial figure must not read '
      'as a verdict (#381 (a))', (tester) async {
    const fault = SendInsufficientFunds(
      availableZat: 100000000,
      requiredZat: 300000000,
      pendingIncomingZat: 0,
    );

    // Synced (cue None): the plain message, no catch-up hedge.
    await tester.pumpWidget(_host(fault, catchUp: const WalletCatchUpNone()));
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );
    expect(
      find.text(l10n.walletSendFaultInsufficientCatchingUp),
      findsNothing,
      reason: 'over a synced wallet the figure IS the verdict',
    );

    // Catching up: the detail line appears under the same message.
    await tester.pumpWidget(
      _host(fault, catchUp: const WalletCatchUpSyncing()),
    );
    await tester.pumpAndSettle();
    expect(
      find.text(l10n.walletSendFaultInsufficient('1', '3')),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletSendFaultInsufficientCatchingUp),
      findsOneWidget,
    );
  });

  testWidgets('the catching-up detail STACKS with the pending-incoming '
      'detail — both truths are independent (#381 (a))', (tester) async {
    const fault = SendInsufficientFunds(
      availableZat: 100000000,
      requiredZat: 300000000,
      pendingIncomingZat: 50000000,
    );
    await tester.pumpWidget(
      _host(fault, catchUp: const WalletCatchUpSyncing()),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );
    expect(
      find.text(l10n.walletSendFaultInsufficientPending('0.5')),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletSendFaultInsufficientCatchingUp),
      findsOneWidget,
    );
  });

  testWidgets('S205-c: the catching-up detail is OMITTED under the host\'s '
      'sync-off policy even while the cue is live — "as the wallet syncs" '
      'would claim the active progress the notSyncedYet arm just stopped '
      'claiming (the same card must not carry both)', (tester) async {
    const fault = SendInsufficientFunds(
      availableZat: 100000000,
      requiredZat: 300000000,
      pendingIncomingZat: 0,
    );
    await tester.pumpWidget(
      _host(fault, catchUp: const WalletCatchUpSyncing(), syncPolicyOff: true),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SendFormFaultView)),
    );

    expect(
      find.text(l10n.walletSendFaultInsufficient('1', '3')),
      findsOneWidget,
      reason: 'the fault itself still renders — only the qualifier drops',
    );
    expect(
      find.text(l10n.walletSendFaultInsufficientCatchingUp),
      findsNothing,
      reason: 'no active-progress claim while the host holds sync off',
    );
  });

  testWidgets(
    'P2-4 (founder S272 decision 3): the pending-incoming detail says '
    'the funds are spendable once the WALLET catches up — never "once it '
    'confirms". The amount is most often a note with thousands of '
    'confirmations that is held only until more of the chain is scanned '
    '(its witness is not yet available), so "confirms" named a wait that '
    'was already over and sent the user watching the wrong clock',
    (tester) async {
      const fault = SendInsufficientFunds(
        availableZat: 100000000,
        requiredZat: 300000000,
        pendingIncomingZat: 50000000,
      );
      await tester.pumpWidget(_host(fault));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SendFormFaultView)),
      );
      final detail = l10n.walletSendFaultInsufficientPending('0.5');
      expect(
        find.text(detail),
        findsOneWidget,
        reason: 'the pending detail renders through the l10n key',
      );
      // The two conjuncts of the decision, guarded separately: the true clause
      // is present, and the false one is gone. Mutant: the EN sentence reverted
      // to "…once it confirms." and the localizations regenerated — the first
      // expectation below reds (watched: "does not contain"); so would the second.
      expect(
        detail,
        contains('once the wallet catches up'),
        reason: 'the wait the user is told about is the wallet\'s scan',
      );
      expect(
        detail,
        isNot(contains('confirms')),
        reason: 'the common case already has thousands of confirmations',
      );
    },
  );

  testWidgets(
    'INC-018 (b), P2-2 (founder S272 decision 4): the retryable prepare '
    'refusal renders "try again in a moment" and NEVER the "check the '
    'details" copy — on the S257 device proof the details were correct and '
    'the identical send prepared fine two minutes later',
    (tester) async {
      await tester.pumpWidget(
        _host(
          const SendCategoricalFault(SendFaultReason.couldNotPrepareTransient),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SendFormFaultView)),
      );
      final transient = l10n.walletSendFaultCouldNotPrepareTransient;
      expect(find.text(transient), findsOneWidget);
      // Mutant: the couldNotPrepareTransient arm of `_categoricalFaultMessage`
      // pointed at `walletSendFaultCouldNotPrepare` — the next line reds.
      expect(
        find.text(l10n.walletSendFaultCouldNotPrepare),
        findsNothing,
        reason:
            'the deterministic class\'s copy must not render for the '
            'transient one',
      );
      expect(
        transient,
        isNot(contains('details')),
        reason: 'the copy must not send the user to correct correct input',
      );
      // The orange fixable-fault treatment, never the red money-loss one.
      expect(find.byIcon(Icons.error_outline), findsOneWidget);
    },
  );

  testWidgets(
    'INC-016: the network-upgrade fault says the funds are safe and stops '
    'there — never "you can still receive". A build that predates the '
    'upgrade can be blind to incoming payments too (Ironwood receipts ride a '
    'field it cannot read), so the promise was false exactly when it showed',
    (tester) async {
      await tester.pumpWidget(
        _host(
          const SendCategoricalFault(SendFaultReason.networkUpgradeUnsupported),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SendFormFaultView)),
      );
      final body = l10n.walletSendFaultNetworkUpgrade;
      expect(find.text(body), findsOneWidget);
      expect(
        body,
        endsWith('before it can send. Your funds are safe.'),
        reason: 'the sentence ends at the funds, with no receive clause',
      );
      // Every locale drops the clause, not only English: the receive verb of
      // each translation as it read before the INC-016 copy fix. Mutant: any
      // one non-English arb restored to its old sentence and regenerated reds
      // in the loop below (English reds at the endsWith above).
      const receiveStem = <String, String>{
        'en': 'receiv',
        'ar': 'الاستلام',
        'de': 'empfang',
        'es': 'recibi',
        'fi': 'vastaanott',
        'fr': 'recevoir',
        'he': 'לקבל',
        'it': 'ricever',
        'ja': '受け取',
        'nb': 'motta',
        'nl': 'ontvang',
        'pl': 'odbiera',
        'pt': 'receber',
        'ru': 'получат',
        'uk': 'отримуват',
        'zh': '接收',
      };
      expect(
        WalletLocalizations.supportedLocales.map((l) => l.languageCode).toSet(),
        receiveStem.keys.toSet(),
        reason: 'a new locale owes its own receive stem here',
      );
      for (final entry in receiveStem.entries) {
        final text = lookupWalletLocalizations(
          Locale(entry.key),
        ).walletSendFaultNetworkUpgrade;
        expect(
          text,
          isNot(contains(entry.value)),
          reason: '${entry.key}: the receive promise is gone',
        );
      }
    },
  );
}
