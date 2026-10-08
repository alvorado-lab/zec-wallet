// Stage S5 `row` (docs/plan/stage-5-the-device-log-a-user-can-send.md §2):
// the Settings row renders what the SDK answers, and Share sends text only.
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:share_plus/share_plus.dart' show ShareParams;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet/zec_wallet.dart' show DeviceLogLevel;
import 'package:zec_wallet_example/app.dart';
import 'package:zec_wallet_example/core/logging/device_log.dart';
import 'package:zec_wallet_example/core/logging/device_log_prefs.dart';
import 'package:zec_wallet_example/core/router/router.dart';
import 'package:zec_wallet_example/features/settings/device_log_section.dart';
import 'package:zec_wallet_ui/testing.dart';
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import '../../support/fake_device_log.dart';

void main() {
  late FakeDeviceLogBridge bridge;
  late DeviceLogController controller;
  late List<ShareParams> shared;

  setUp(() {
    SharedPreferences.setMockInitialValues({});
    bridge = FakeDeviceLogBridge();
    controller = DeviceLogController(
      bridge: bridge,
      store: storeDeviceLogChoice,
    );
    shared = [];
  });

  // By default tall enough that the whole Appearance list is on screen, so
  // every tap lands on its widget, not beside it. A phone-sized [size] makes
  // the list scroll.
  Future<void> pumpSettings(
    WidgetTester tester, {
    Size size = const Size(800, 3000),
  }) async {
    tester.view
      ..physicalSize = size
      ..devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(
          FakeWalletProvisioner(exists: false),
        ),
        onboardingStoreProvider.overrideWithValue(FakeOnboardingStore()),
        deviceLogProvider.overrideWithValue(controller),
        deviceLogShareProvider.overrideWithValue((p) async => shared.add(p)),
        deviceLogMailLinkProvider.overrideWithValue(false),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const WalletExampleApp(),
      ),
    );
    await tester.pumpAndSettle();
    container.read(routerProvider).go(AppRoutes.appearance);
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(find.byKey(deviceLogShareKey), 200);
  }

  DeviceLogLevel? selected(WidgetTester tester) => tester
      .widget<RadioGroup<DeviceLogLevel>>(
        find.byType(RadioGroup<DeviceLogLevel>),
      )
      .groupValue;

  testWidgets('the_row_renders_the_effective_level_not_the_choice', (
    tester,
  ) async {
    // A device where nothing can write: the SDK answers Off to any ask.
    bridge.effectiveFor = (_) => DeviceLogLevel.off;
    controller.arm(DeviceLogLevel.detailed);
    await pumpSettings(tester);
    expect(controller.choice, DeviceLogLevel.detailed);
    expect(selected(tester), DeviceLogLevel.off);
    expect(find.text("This device can't keep a log"), findsOneWidget);
  });

  testWidgets('picking_off_clears_the_ring', (tester) async {
    controller.arm(DeviceLogLevel.detailed);
    bridge
      ..emit('one')
      ..emit('two');
    await pumpSettings(tester);
    expect(controller.lines, hasLength(2));

    await tester.tap(find.byKey(deviceLogLevelKey(DeviceLogLevel.off)));
    await tester.pumpAndSettle();
    expect(controller.lines, isEmpty);
    expect(selected(tester), DeviceLogLevel.off);
    final prefs = await SharedPreferences.getInstance();
    expect(prefs.getString(deviceLogLevelPrefsKey), 'off');
  });

  testWidgets('share_sends_text_only_and_is_disabled_on_an_empty_ring', (
    tester,
  ) async {
    controller.arm(DeviceLogLevel.detailed);
    await pumpSettings(tester);
    final share = find.byKey(deviceLogShareKey);
    expect(tester.widget<ListTile>(share).enabled, isFalse);
    await tester.tap(share);
    await tester.pumpAndSettle();
    expect(shared, isEmpty);

    bridge.emit('wallet.dial arm=clearnet');
    await tester.pumpAndSettle();
    expect(tester.widget<ListTile>(share).enabled, isTrue);
    await tester.tap(share);
    await tester.pumpAndSettle();
    final p = shared.single;
    expect(p.files, isNull);
    expect(p.uri, isNull);
    expect(p.previewThumbnail, isNull);
    expect(p.text, controller.shareText(mailLink: false));
    expect(p.text!.split('\n').last, endsWith('wallet.dial arm=clearnet'));
  });

  // An iPad refuses to present a share sheet without an anchor, and macOS
  // puts its picker at the view's corner: the anchor is the tapped tile.
  testWidgets('share_is_anchored_to_the_tapped_tile', (tester) async {
    controller.arm(DeviceLogLevel.detailed);
    bridge.emit('wallet.dial arm=clearnet');
    await pumpSettings(tester);
    await tester.tap(find.byKey(deviceLogShareKey));
    await tester.pumpAndSettle();
    final origin = shared.single.sharePositionOrigin;
    expect(origin, isNotNull, reason: 'an iPad shows nothing without it');
    expect(origin!.isEmpty, isFalse);
    expect(origin, tester.getRect(find.byKey(deviceLogShareKey)));
  });

  // The same on a phone, where the list has to SCROLL to reach Share: the
  // anchor is where the tile is on screen now, not where it was laid out.
  testWidgets('share_is_anchored_to_the_tile_after_the_list_scrolls', (
    tester,
  ) async {
    controller.arm(DeviceLogLevel.detailed);
    bridge.emit('wallet.dial arm=clearnet');
    await pumpSettings(tester, size: const Size(390, 844));
    final tile = find.byKey(deviceLogShareKey);
    // `scrollUntilVisible` stops with the tile at the edge, where the bottom
    // bar can take the tap: bring it to the middle of the viewport.
    await Scrollable.ensureVisible(tester.element(tile), alignment: 0.5);
    await tester.pumpAndSettle();
    final scrolled = tester
        .state<ScrollableState>(
          find.ancestor(of: tile, matching: find.byType(Scrollable)).first,
        )
        .position
        .pixels;
    expect(scrolled, greaterThan(0), reason: 'the precondition: it scrolled');
    await tester.tap(tile);
    await tester.pumpAndSettle();
    final origin = shared.single.sharePositionOrigin;
    expect(origin, tester.getRect(tile));
    expect(
      const Rect.fromLTWH(0, 0, 390, 844).contains(origin!.center),
      isTrue,
      reason: 'the anchor is on screen: $origin',
    );
  });
}
