/// Test-support library for hosts embedding the wallet UI: the fakes needed
/// to pump wallet surfaces in widget tests without the native library —
/// [FakeWalletSession] (a scripted WalletSession) and the fake onboarding
/// seams (provisioner / store / screen-security). DELIBERATELY free of any
/// `flutter_test` import, so depending on it never constrains a consumer's
/// test-framework solve; import it from test code only.
library;

export 'testing/fake_onboarding.dart';
export 'testing/fake_reveal_authorizer.dart';
export 'testing/fake_send_authorizer.dart';
export 'testing/fake_wallet_session.dart';
export 'testing/fake_wallet_settings_store.dart';
