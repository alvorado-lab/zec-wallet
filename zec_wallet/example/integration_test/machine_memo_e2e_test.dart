import 'dart:typed_data' show Uint8List;

import 'package:flutter/foundation.dart' show debugPrint;

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_example/core/ffi/wallet_ffi.dart';

/// FR-27 / FR-28 device acceptance — the machine-memo round trip, from Dart.
///
/// **Why this file exists at all.** Both FRs name an outcome no host-VM test can
/// witness. FR-27's first acceptance row is *"a **Dart-driven** host reads back
/// the exact bytes, **over the bridge**, with no Rust-crate dependency"* — and
/// until this file, `machineMemos` was never called from Dart ANYWHERE: not in a
/// test, not in the example app, only inside two comments. Every other FR-27 row
/// is pinned in core, which is the Rust side, i.e. exactly the half the FR was
/// filed to avoid depending on. FR-28's close condition is *"a send driven
/// through the FR-25 prefill whose **on-chain** memo contains the exact bytes,
/// asserted byte-for-byte"* — the shipped Rust pin asserts through the audited
/// encoder and parser, which is not the same as witnessing the chain.
///
/// **TWO TIERS, deliberately split.** Tier 1 runs on any device with no funds
/// and proves the SEAM crosses the FFI. Tier 2 is the real acceptance and needs
/// a funded testnet wallet; it is skipped, loudly, when the funds are absent —
/// never silently green (a silent no-op that reads as success).
///
/// The wallet lives in a THROWAWAY leaf and is crypto-shredded at the end even
/// on assertion failure (its keychain namespace derives from the throwaway dir,
/// so the shred can never touch production custody) — same discipline as the
/// FR-24 airplane test next door.
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init(externalLibrary: walletExternalLibrary()));

  /// Relim's registered envelope lead, as the host would register it.
  final prefix = Uint8List.fromList([0x52, 0x4C, 0x4D, 0x01]); // "RLM\x01"

  WalletConfig configAt(
    String dir, {
    required List<Uint8List> prefixes,
    int? birthdayHeight,
  }) => WalletConfig(
    dbDir: dir,
    network: Network.test,
    endpointUrl: 'https://testnet.zec.rocks:443',
    tor: const TorPolicy.off(),
    seedPersistence: SeedPersistence.sealedKeychain,
    birthdayHeight: birthdayHeight,
    broadcastJitter: const JitterPolicy.none(),
    machineMemoPrefixes: prefixes,
  );

  Future<void> shred(WalletConfig config) async {
    try {
      if (await WalletHandle.walletExists(config: config)) {
        await WalletHandle.wipeForce(config: config);
      }
    } catch (e) {
      debugPrint('machine_memo_e2e: backstop wipe failed: $e');
    }
  }

  // ── TIER 1 — the seam crosses the FFI (no funds, any device) ───────────────

  testWidgets('fr27_machine_memos_crosses_the_bridge_from_dart', (
    tester,
  ) async {
    // What this DOES prove: a Dart host reaches the verb over the real bridge,
    // the opt-in refusal is typed and reaches Dart as a catchable kind, a
    // registered scope opens the verb, and a bad txid is refused typed.
    //
    // What it does NOT prove: that real on-chain bytes come back. A fresh
    // wallet has received nothing, so the answer is an empty list — honest, and
    // NOT the acceptance row. Tier 2 is that row; do not read this one as it.
    final support = await getApplicationSupportDirectory();
    final dir = '${support.path}/e2e_fr27_throwaway';
    final unscoped = configAt(dir, prefixes: const []);
    final scoped = configAt(dir, prefixes: [prefix]);
    await shred(unscoped);
    try {
      // (a) UNCONFIGURED scope ⇒ typed refusal, never an empty list. The
      // distinction is the whole design: an empty answer would read to a host
      // as "this transaction carries nothing" over a memo that is there.
      var handle = await WalletHandle.createGenerated(config: unscoped);
      final anyTxid = '00' * 32;
      try {
        await handle.machineMemos(txidHex: anyTxid);
        fail('an unregistered scope must refuse, not answer empty');
      } on WalletApiError catch (e) {
        expect(e.code, 'RW-PAY-010');
        expect(e.kind, isA<WalletErrorKind_MachineMemoScopeInvalid>());
      }
      await handle.close();

      // (b) REGISTERED scope ⇒ the verb is open. Empty here is the honest
      // "nothing in scope on this wallet", distinct from the refusal above.
      handle = await WalletHandle.open(config: scoped);
      expect(
        await handle.machineMemos(txidHex: anyTxid),
        isEmpty,
        reason: 'a registered scope is SERVED; an unknown txid carries nothing',
      );

      // (c) a malformed txid is refused typed, never a crash across the FFI.
      for (final bad in ['', 'not-a-txid', '00' * 31, 'zz' * 32]) {
        try {
          await handle.machineMemos(txidHex: bad);
          fail('a malformed txid must be refused: "$bad"');
        } on WalletApiError catch (e) {
          expect(e.code, 'RW-PAY-009', reason: 'txid "$bad"');
        }
      }
      await handle.close();
    } finally {
      await shred(unscoped);
    }
  });

  testWidgets('fr27_an_unusable_scope_is_refused_at_the_config_door', (
    tester,
  ) async {
    // An EMPTY prefix matches every memo ever written — the "returns
    // everything" failure FR-27's acceptance names explicitly. Refused before a
    // wallet exists, and the refusal reaches Dart typed.
    final support = await getApplicationSupportDirectory();
    final dir = '${support.path}/e2e_fr27_never_created';
    try {
      await WalletHandle.createGenerated(
        config: configAt(dir, prefixes: [Uint8List(0)]),
      );
      fail('an empty prefix must be refused at the config door');
    } on WalletApiError catch (e) {
      expect(e.code, 'RW-PAY-010');
    }
  });

  // ── TIER 2 — THE ACCEPTANCE ROW (needs a funded testnet wallet) ────────────

  testWidgets('fr27_fr28_round_trip_exact_bytes_through_the_chain', (
    tester,
  ) async {
    // THE row both FRs actually turn on, and the only one that witnesses the
    // CHAIN. A self-send is enough and is the cheapest shape: the memo read
    // scopes to outputs the primary account SENT or received, so one funded
    // wallet paying its own shielded address exercises both directions —
    // FR-28's write (the bytes reach the wire) and FR-27's read (a Dart host
    // gets them back), with the read PROVING the write landed on-chain.
    //
    // FUNDS ARRIVE BY RESTORE, NOT BY STAGING — learned by running this on a
    // device. `flutter test integration_test/...` UNINSTALLS the app
    // when it finishes, which destroys everything under the app's files dir.
    // So "stage a funded wallet at this leaf and re-run", which is what this
    // test said first, can never work: the leaf is gone before the second run
    // starts. The phrase comes in per-run instead.
    //
    //   flutter test integration_test/machine_memo_e2e_test.dart -d <device> \
    //     --dart-define=ZEC_E2E_TESTNET_PHRASE="word word …"
    //
    // A TESTNET throwaway phrase only. It rides the APK and the shell history,
    // which is acceptable for testnet play money and would not be for anything
    // else — do not point this at a wallet you care about.
    //
    // SKIPPED LOUDLY without it: a green run over a skipped body would be a
    // false discharge of an acceptance row.
    const phrase = String.fromEnvironment('ZEC_E2E_TESTNET_PHRASE');
    if (phrase.isEmpty) {
      markTestSkipped(
        'NOT RUN — no ZEC_E2E_TESTNET_PHRASE supplied. This is the FR-27/FR-28 '
        'ACCEPTANCE row and the tiers above do NOT discharge it. Re-run with '
        '--dart-define=ZEC_E2E_TESTNET_PHRASE="<24 testnet words>" pointing at '
        'a wallet holding spendable testnet ZEC.',
      );
      return;
    }

    // A BIRTHDAY is effectively required, not a nicety: `null` floors a restore
    // to Sapling activation, i.e. a full testnet scan, which will not finish
    // inside any sane test window. Pass the wallet's creation height.
    const birthday = int.fromEnvironment('ZEC_E2E_TESTNET_BIRTHDAY');
    final support = await getApplicationSupportDirectory();
    final dir = '${support.path}/e2e_fr27_funded';
    final config = configAt(
      dir,
      prefixes: [prefix],
      birthdayHeight: birthday == 0 ? null : birthday,
    );
    await shred(config);
    // Words lowercased + whitespace-collapsed: the audited BIP39 validator does
    // NOT case-fold, so a copy-pasted capitalised phrase would reject a correct
    // backup (the host-UI contract pinned in core's restore tests).
    final handle = await WalletHandle.restore(
      config: config,
      mnemonicWords: phrase.trim().toLowerCase().split(RegExp(r'\s+')),
    );
    try {
      await handle.startSync();

      // WAIT FOR SPENDABLE FUNDS. A restore has nothing until the scan reaches
      // the notes, and `propose` on an unsynced wallet fails for a reason that
      // has nothing to do with either FR — which would make this row fail
      // MISLEADINGLY rather than honestly.
      final syncDeadline = DateTime.now().add(const Duration(minutes: 45));
      var spendable = 0;
      while (DateTime.now().isBefore(syncDeadline)) {
        final snap = await handle.snapshot();
        spendable = snap.balance.spendableZat.toInt();
        if (spendable > 20000) break; // amount + fee headroom
        debugPrint(
          'machine_memo_e2e: waiting for spendable funds '
          '(status ${snap.syncStatus.runtimeType})',
        );
        await Future<void>.delayed(const Duration(seconds: 30));
      }
      expect(
        spendable,
        greaterThan(20000),
        reason:
            'the restored wallet never reached a spendable balance inside the '
            'window. Check the phrase, the birthday, and that the wallet '
            'actually holds testnet ZEC — this is a SETUP failure, not an '
            'FR-27/FR-28 failure, and must not be read as one.',
      );

      // The envelope a host would attach: its registered lead + a payload.
      final envelope = [...prefix, ...'e2e-order-7f3c'.codeUnits];
      final selfAddress = await handle.currentAddress();

      // FR-28 — compose through the SAME audited encoder the prefill path uses
      // (`encodePaymentUri` is the free function on the payments API, not a
      // handle method), so this walk covers the seam a host drives rather than
      // a hand-built URI.
      final uri = encodePaymentUri(
        network: Network.test,
        payments: [
          PaymentDraft(
            recipientAddress: selfAddress,
            amountZat: 10000,
            memoBytes: Uint8List.fromList(envelope),
          ),
        ],
      );
      final proposal = await handle.propose(requestUri: uri);
      final results = await handle.send(proposalId: proposal.proposalId);
      final txid = results
          .whereType<TxSubmitResult_Success>()
          .map((r) => r.txidHex)
          .single;
      debugPrint('machine_memo_e2e: broadcast, awaiting enhancement');

      // Memos arrive with tx ENHANCEMENT, not compact scan — a freshly received
      // transaction returns nothing until that pass runs, and enhancement is
      // driven by the SYNC loop (there is no manual hook on the handle). So:
      // keep syncing and poll the read. Poll rather than sleep-and-hope, and
      // FAIL on the timeout instead of passing empty.
      List<Uint8List> read = const [];
      final deadline = DateTime.now().add(const Duration(minutes: 20));
      while (DateTime.now().isBefore(deadline)) {
        read = await handle.machineMemos(txidHex: txid);
        if (read.isNotEmpty) break;
        await Future<void>.delayed(const Duration(seconds: 20));
      }
      expect(
        read,
        isNotEmpty,
        reason:
            'the memo never came back within the window — either it did not '
            'reach the chain (FR-28) or the read path is broken (FR-27). Do '
            'NOT relax this into a skip: an empty read is the failure.',
      );

      // FR-28's close condition, and FR-27's first row, in one assertion: the
      // LEADING bytes are exactly what was supplied. Not the length — the 0xFF
      // field is zero-padded to 511, so a length assertion proves nothing.
      expect(
        read.single.take(envelope.length).toList(),
        envelope,
        reason: 'the on-chain memo carries the host bytes BYTE-FOR-BYTE',
      );
      expect(
        read.single.length,
        511,
        reason:
            'the wire pads to the full field — a host frames its own length',
      );
      debugPrint(
        'machine_memo_e2e: round trip OK, ${read.single.length} bytes',
      );
    } finally {
      await handle.close();
      // Shredded like every other leaf here: the funds live in the PHRASE, not
      // in this restore, so the wallet on disk is disposable and leaving a
      // testnet seed sealed on the device would be gratuitous.
      await shred(config);
    }
  });
}
