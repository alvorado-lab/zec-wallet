import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/arrival_cue.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/testing.dart';

/// S13 §1.6 / M3 — the "Payment received" watermark's own branches, against
/// the notifier: a re-delivery is suppressed, a null span always shows, and a
/// new session (a rescan, a server switch) starts clean.
void main() {
  final a = FakeWalletSession();
  final b = FakeWalletSession();
  late StateProvider<WalletSession?> session;
  late ProviderContainer c;

  setUp(() {
    session = StateProvider<WalletSession?>((ref) => a);
    c = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(session)),
      ],
    );
    addTearDown(c.dispose);
  });

  bool admit(int? span) =>
      c.read(walletArrivalCueProvider.notifier).admit(span);

  test('a span at or below one already shown is suppressed; a higher one '
      'shows', () {
    expect(admit(50), isTrue);
    expect(admit(50), isFalse);
    expect(admit(49), isFalse);
    expect(admit(51), isTrue);
  });

  test('a null span always shows, and moves nothing', () {
    expect(admit(50), isTrue);
    expect(admit(null), isTrue);
    expect(admit(null), isTrue);
    expect(admit(50), isFalse, reason: 'the watermark is still 50');
  });

  test(
    'a new session starts clean: a height the old one showed shows again',
    () {
      expect(admit(50), isTrue);
      c.read(session.notifier).state = b;
      expect(admit(50), isTrue);
      expect(admit(50), isFalse);
    },
  );
}
