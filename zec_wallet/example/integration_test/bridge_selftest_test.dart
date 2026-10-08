import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_example/bridge_selftest.dart';
import 'package:zec_wallet_example/core/ffi/wallet_ffi.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  // same static-link loader handling as the app (see core/ffi/wallet_ffi.dart)
  setUpAll(() async => RustLib.init(externalLibrary: walletExternalLibrary()));

  testWidgets('every bridge self-test check passes', (tester) async {
    final results = await runBridgeSelftest();
    for (final r in results) {
      expect(r.passed, isTrue, reason: '${r.name}: ${r.detail}');
    }
    // The suite must stay COMPLETE: a vanished check is the regression this
    // guards. Bump this count deliberately when you ADD a check to
    // runBridgeSelftest (a decrease is what must never slip through).
    // 15 → 16: the #324 relative-dbDir typed rejection (RW-CFG-003).
    // 16 → 17: FR-27's empty machine-memo prefix refusal (RW-PAY-010) — a
    // prefix that matches every memo ever written, refused at the config door.
    expect(results.length, 17);
  });
}
