import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

/// The FR-27 / FR-28 machine-memo ROUND TRIP, driven from the example app.
///
/// This screen is the acceptance both FRs name and that no host-VM test can
/// witness: FR-28's *"a send whose ON-CHAIN memo contains the exact bytes,
/// asserted byte-for-byte"* and FR-27's *"a Dart-driven host reads back the
/// exact bytes, over the bridge"*. One self-send proves both — the memo read
/// scopes to outputs this account sent OR received, so reading it back is what
/// demonstrates the write reached the chain.
///
/// It is also the SDK's own second consumer of the seam, which is what makes
/// FR-27's `Universal:` claim compiled rather than argued.
///
/// **MONEY-MOVING, and on whatever network this build targets.** It signs and
/// broadcasts a real transaction. The amount goes to this wallet's OWN address,
/// so the only cost is the fee — but the MEMO is permanent and public, which is
/// why the demo prefix below is deliberately neutral and carries no Relim
/// marker. Nothing here should ever be pointed at a production envelope magic.
const List<int> kMachineMemoDemoPrefix = [0x5A, 0x45, 0x43, 0xE2];

class MachineMemoScreen extends ConsumerStatefulWidget {
  const MachineMemoScreen({super.key});

  @override
  ConsumerState<MachineMemoScreen> createState() => _MachineMemoScreenState();
}

class _MachineMemoScreenState extends ConsumerState<MachineMemoScreen> {
  final List<String> _log = [];
  bool _running = false;
  bool? _passed;

  void _say(String line) {
    if (!mounted) return;
    setState(() => _log.add(line));
  }

  Future<void> _run() async {
    final session = ref.read(walletSessionProvider);
    if (session == null) {
      _say('No wallet session — finish onboarding first.');
      return;
    }
    setState(() {
      _running = true;
      _passed = null;
      _log.clear();
    });
    try {
      // The envelope: a neutral 4-byte lead the wallet config registered, plus
      // a payload. NOT Relim's magic — this lands on a public chain forever.
      final envelope = Uint8List.fromList([
        ...kMachineMemoDemoPrefix,
        ...'e2e-${DateTime.now().millisecondsSinceEpoch}'.codeUnits,
      ]);
      _say(
        'envelope: ${envelope.length} bytes, lead '
        '${kMachineMemoDemoPrefix.map((b) => b.toRadixString(16)).join()}',
      );

      final selfAddress = await session.currentAddress();
      _say('composing a self-send…');

      // FR-28 — through the SAME audited encoder the prefill path uses.
      final uri = session.composePaymentUri(
        recipient: selfAddress,
        amountZat: 10000,
        memoBytes: envelope,
      );
      final proposal = await session.propose(uri);
      _say('proposal ready — fee ${proposal.feeZat} zat. Signing…');

      final results = await session.send(proposal.proposalId);
      final txids = results
          .whereType<TxSubmitResult_Success>()
          .map((r) => r.txidHex)
          .toList();
      if (txids.isEmpty) {
        // Report WHY, per result. The endpoint's own verdict is the only thing
        // that distinguishes "our envelope was malformed" from "this node
        // refused a perfectly good tx" — dropping it (as this screen first did)
        // leaves the acceptance unfalsifiable and the operator with nothing to
        // act on. `SubmitFailure` carries the endpoint code; the message is
        // deliberately not retained by the core (§5.4 never-log), so the code
        // is the whole signal we are allowed to have.
        _say('FAIL: nothing broadcast — ${results.length} result(s), no txid.');
        for (final r in results) {
          _say('  · ${_describeSubmitResult(r)}');
        }
        setState(() => _passed = false);
        return;
      }
      final txid = txids.single;
      _say('broadcast ${txid.substring(0, 12)}… — waiting for enhancement');

      // Memos arrive with tx ENHANCEMENT, driven by the sync loop, not by
      // compact scan. Poll; FAIL on timeout rather than reporting empty.
      List<Uint8List> read = const [];
      final deadline = DateTime.now().add(const Duration(minutes: 25));
      while (DateTime.now().isBefore(deadline)) {
        read = await session.machineMemos(txid);
        if (read.isNotEmpty) break;
        await Future<void>.delayed(const Duration(seconds: 15));
        if (!mounted) return;
      }
      if (read.isEmpty) {
        _say('FAIL: the memo never came back inside the window.');
        setState(() => _passed = false);
        return;
      }

      // FR-28's close condition + FR-27's first row, in one assertion.
      final got = read.single;
      final leadMatches =
          got.length >= envelope.length &&
          List<int>.from(got.take(envelope.length)).toString() ==
              envelope.toString();
      _say('read back ${got.length} bytes (the 0xFF field pads to 511)');
      _say(
        leadMatches
            ? 'BYTE-FOR-BYTE MATCH on the leading ${envelope.length} bytes'
            : 'FAIL: the bytes came back DIFFERENT',
      );
      setState(() => _passed = leadMatches && got.length == 511);
    } on WalletApiError catch (e) {
      _say('FAIL: ${e.code} (${e.kind.runtimeType})');
      setState(() => _passed = false);
    } catch (e) {
      _say('FAIL: $e');
      setState(() => _passed = false);
    } finally {
      if (mounted) setState(() => _running = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Machine memo round trip')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          const Text(
            'FR-27 / FR-28 acceptance. Signs and broadcasts a REAL self-send '
            '(10,000 zat to this wallet, so only the fee is spent) carrying '
            'opaque bytes, then reads those bytes back over the bridge. The '
            'memo is permanent and public; the prefix used here is neutral and '
            'names nothing.',
          ),
          const SizedBox(height: 16),
          FilledButton(
            onPressed: _running ? null : _run,
            child: Text(_running ? 'Running…' : 'Run the round trip'),
          ),
          if (_passed != null) ...[
            const SizedBox(height: 12),
            Text(
              _passed! ? 'PASS' : 'FAIL',
              style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                color: _passed! ? Colors.green : Colors.red,
              ),
            ),
          ],
          const SizedBox(height: 16),
          for (final line in _log)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: Text(
                line,
                style: const TextStyle(fontFamily: 'monospace'),
              ),
            ),
        ],
      ),
    );
  }
}

/// One line per submit result, naming the arm and carrying the endpoint code
/// where there is one. The arms mean materially different things to whoever is
/// reading this screen: `SubmitFailure` is a verdict FROM the endpoint (our tx
/// reached it and was refused — the code says why); `GrpcFailure` is NO verdict
/// (the tx may still have landed, so it is not proof of a failed send); and
/// `NotAttempted` means an earlier tx in the set never reached the endpoint, so
/// this one was deliberately held back.
String _describeSubmitResult(TxSubmitResult r) => switch (r) {
  TxSubmitResult_Success(:final txidHex) =>
    'accepted ${txidHex.substring(0, 12)}…',
  TxSubmitResult_SubmitFailure(:final code) =>
    'endpoint REJECTED it, code $code (the tx reached the node and was refused)',
  TxSubmitResult_GrpcFailure() =>
    'no endpoint verdict — transport fault; the tx MAY still have landed',
  TxSubmitResult_NotAttempted() =>
    'not attempted — an earlier tx in the set never reached the endpoint',
  // The core enum is `#[non_exhaustive]`, so the bridge carries an Unknown arm
  // for variants added later. Name it rather than fail to compile on the next
  // core bump: an unrecognised arm still tells the operator it did not succeed.
  _ =>
    'unrecognised submit result (${r.runtimeType}) — the bridge is newer than this screen',
};
