import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:share_plus/share_plus.dart' show ShareParams;
import 'package:zec_wallet/zec_wallet.dart' show DeviceLogLevel;
import 'package:zec_wallet_ui/shared/settings_section_header.dart';

import '../../core/logging/device_log.dart';
import '../../l10n/app_localizations.dart';

/// One level's radio row.
ValueKey<String> deviceLogLevelKey(DeviceLogLevel level) =>
    ValueKey('appearance.deviceLog.${level.name}');

/// The "Share device log" action.
const deviceLogShareKey = ValueKey('appearance.deviceLog.share');

/// Settings → Device log (stage S5 `row`): Off / Errors only / Detailed, and
/// Share.
///
/// The radios render the EFFECTIVE level — what the SDK answered — never the
/// pick, so the row cannot claim a log that does not exist; when the two
/// differ, one line says so. Share sends the ring as text and nothing else,
/// through the OS share sheet (maintainer, Q5).
class DeviceLogSection extends ConsumerWidget {
  const DeviceLogSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final log = ref.watch(deviceLogProvider);
    return ListenableBuilder(
      listenable: log,
      builder: (context, _) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SettingsSectionHeader(l10n.deviceLogSectionTitle),
          RadioGroup<DeviceLogLevel>(
            groupValue: log.effective,
            onChanged: (level) {
              if (level != null) log.choose(level);
            },
            child: Column(
              children: [
                for (final (level, title) in [
                  (DeviceLogLevel.off, l10n.deviceLogOffTitle),
                  (DeviceLogLevel.errors, l10n.deviceLogErrorsTitle),
                  (DeviceLogLevel.detailed, l10n.deviceLogDetailedTitle),
                ])
                  RadioListTile<DeviceLogLevel>(
                    key: deviceLogLevelKey(level),
                    value: level,
                    title: Text(title),
                  ),
              ],
            ),
          ),
          if (log.effective != log.choice)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Text(
                l10n.deviceLogUnavailable,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          // A Builder, so the share sheet is anchored to THIS tile: its
          // context's render object is the tile's, not the whole section's.
          Builder(
            builder: (tileContext) => ListTile(
              key: deviceLogShareKey,
              leading: const Icon(Icons.share),
              title: Text(l10n.deviceLogShareTitle),
              enabled: log.lines.isNotEmpty,
              onTap: () => _share(context, ref, log, _originOf(tileContext)),
            ),
          ),
        ],
      ),
    );
  }

  /// The tapped tile's rectangle in global coordinates: where the share sheet
  /// is anchored. An iPad REFUSES to present without one (share_plus returns
  /// an error and shows nothing), and macOS places its picker at the view's
  /// corner. Read on the tap, before any await.
  static Rect? _originOf(BuildContext tileContext) {
    final box = tileContext.findRenderObject();
    if (box is! RenderBox || !box.hasSize) return null;
    return box.localToGlobal(Offset.zero) & box.size;
  }

  Future<void> _share(
    BuildContext context,
    WidgetRef ref,
    DeviceLogController log,
    Rect? origin,
  ) async {
    final messenger = ScaffoldMessenger.of(context);
    final failed = AppLocalizations.of(context).deviceLogShareFailed;
    final share = ref.read(deviceLogShareProvider);
    final text = log.shareText(mailLink: ref.read(deviceLogMailLinkProvider));
    try {
      await share(ShareParams(text: text, sharePositionOrigin: origin));
    } catch (e) {
      // The TYPE only; a failed share carries the log text in its message.
      debugPrint('device log share failed (${e.runtimeType})');
      messenger.showSnackBar(SnackBar(content: Text(failed)));
    }
  }
}
