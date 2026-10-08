import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/shared/theme/avatar_color.dart';

void main() {
  test('avatar_color_deterministic_and_empty_seed_safe', () {
    const colors = WalletColors.dark;

    // Same seed → same colour, every call, every surface.
    final a = avatarColorFor('contact-fingerprint-1234', colors);
    final b = avatarColorFor('contact-fingerprint-1234', colors);
    expect(a, b);

    // Different seeds can differ (not a strict requirement, but the
    // common case must hold for THESE inputs or the hash is broken).
    final c = avatarColorFor('contact-fingerprint-5678', colors);
    expect(a == c, isFalse);

    // Missing data renders neutral instead of throwing.
    expect(avatarColorFor('', colors), colors.textMuted);
  });
}
