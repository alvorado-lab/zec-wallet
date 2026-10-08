import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/shapes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';

/// S11 C1: the named radii, their platform defaults, and the hook's
/// resolution order (the host's extension, else the theme platform's
/// defaults).
void main() {
  Future<WalletShapes> resolve(WidgetTester tester, ThemeData theme) async {
    late WalletShapes seen;
    await tester.pumpWidget(
      MaterialApp(
        theme: theme,
        home: Builder(
          builder: (context) {
            seen = WalletShapes.of(context);
            return const SizedBox();
          },
        ),
      ),
    );
    // A second theme in the same test animates in (ThemeData.lerp switches
    // platform at the midpoint): settle before reading it.
    await tester.pumpAndSettle();
    return seen;
  }

  group('forPlatform', () {
    test('iOS and macOS round a group to 26 and a sheet to 38', () {
      for (final p in [TargetPlatform.iOS, TargetPlatform.macOS]) {
        final s = WalletShapes.forPlatform(p);
        expect(s.group, 26, reason: '$p');
        expect(s.sheet, 38, reason: '$p');
      }
    });

    test('everything else takes 24 and 28', () {
      for (final p in [
        TargetPlatform.android,
        TargetPlatform.fuchsia,
        TargetPlatform.linux,
        TargetPlatform.windows,
      ]) {
        final s = WalletShapes.forPlatform(p);
        expect(s.group, 24, reason: '$p');
        expect(s.sheet, 28, reason: '$p');
      }
    });

    test("the other roles are Relim's values on every platform", () {
      for (final p in TargetPlatform.values) {
        final s = WalletShapes.forPlatform(p);
        expect(
          [
            s.hero,
            s.tile,
            s.bar,
            s.notice,
            s.qr,
            s.field,
            s.chip,
            s.dialog,
            s.progress,
          ],
          [28, 22, 20, 16, 16, 22, 18, 28, 3],
          reason: '$p',
        );
      }
    });
  });

  group('of', () {
    testWidgets(
      'with none registered it falls back to the theme platform, never throws',
      (tester) async {
        final ios = await resolve(
          tester,
          ThemeData(
            platform: TargetPlatform.iOS,
            extensions: [WalletColors.light],
          ),
        );
        expect(ios.group, 26);
        expect(ios.sheet, 38);

        final android = await resolve(
          tester,
          ThemeData(
            platform: TargetPlatform.android,
            extensions: [WalletColors.light],
          ),
        );
        expect(android.group, 24);
        expect(android.sheet, 28);
      },
    );

    testWidgets('a host-registered extension wins', (tester) async {
      const host = WalletShapes(group: 40, notice: 5);
      final seen = await resolve(
        tester,
        buildTheme(WalletColors.light, extensions: const [host]),
      );
      expect(identical(seen, host), isTrue);
    });
  });

  group('buildTheme registration', () {
    test('registers the platform defaults when the host passes none', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      addTearDown(() => debugDefaultTargetPlatformOverride = null);
      final theme = buildTheme(WalletColors.light);
      expect(theme.extension<WalletShapes>()!.group, 26);
      expect(theme.extension<WalletShapes>()!.sheet, 38);
    });

    test("keeps the host's own, once", () {
      const host = WalletShapes(group: 40);
      final theme = buildTheme(WalletColors.light, extensions: const [host]);
      expect(theme.extension<WalletShapes>(), same(host));
      expect(theme.extensions.values.whereType<WalletShapes>(), hasLength(1));
    });
  });

  test('copyWith replaces only what it names', () {
    const base = WalletShapes();
    final c = base.copyWith(group: 30, progress: 1);
    expect(c.group, 30);
    expect(c.progress, 1);
    expect(c.hero, base.hero);
    expect(c.sheet, base.sheet);
    expect(c.field, base.field);
  });

  test('lerp interpolates every role, and null keeps this', () {
    const a = WalletShapes(
      hero: 0,
      group: 0,
      tile: 0,
      bar: 0,
      notice: 0,
      qr: 0,
      field: 0,
      chip: 0,
      sheet: 0,
      dialog: 0,
      progress: 0,
    );
    const b = WalletShapes(
      hero: 10,
      group: 10,
      tile: 10,
      bar: 10,
      notice: 10,
      qr: 10,
      field: 10,
      chip: 10,
      sheet: 10,
      dialog: 10,
      progress: 10,
    );
    final m = a.lerp(b, 0.5);
    expect([
      m.hero,
      m.group,
      m.tile,
      m.bar,
      m.notice,
      m.qr,
      m.field,
      m.chip,
      m.sheet,
      m.dialog,
      m.progress,
    ], everyElement(5));
    expect(a.lerp(null, 0.5), same(a));
  });
}
