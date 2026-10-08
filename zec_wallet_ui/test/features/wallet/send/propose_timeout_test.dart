import 'dart:async' show TimeoutException;

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';

/// Send's own propose bound (`walletFfiWedgeTimeout`) running out is a
/// wallet still busy with its scan, not bad input. It must read "try again in
/// a moment" (the transient reason), never "check the details".
void main() {
  test('a propose that outlives its bound is transient, not the input', () {
    final fault = classifyProposeFailure(TimeoutException('propose'));
    expect(
      (fault as SendCategoricalFault).reason,
      SendFaultReason.couldNotPrepareTransient,
    );
  });
}
