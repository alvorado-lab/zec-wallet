import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/router/router.dart';
import '../../core/theme/appearance_prefs.dart';
import '../../core/theme/theme_mode_provider.dart';
import '../../core/tor/tor_plugin.dart';
import '../../l10n/app_localizations.dart';
import 'device_log_section.dart';
import 'package:zec_wallet_ui/shared/settings_section_header.dart';

/// The "Use the built-in Tor plugin" switch.
const torPluginToggleKey = ValueKey('appearance.torPluginToggle');

/// Settings → Appearance: theme mode (System/Light/Dark), the AMOLED
/// true-black variant, and the in-app text-size multiplier. The frame's
/// one real feature surface — it exercises the whole theme system
/// end-to-end (app-frame spec §1).
class AppearanceScreen extends ConsumerWidget {
  const AppearanceScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final themeMode = ref.watch(themeModeProvider);
    final amoled = ref.watch(amoledProvider);
    final textScale = ref.watch(textScaleProvider);
    final useTorPlugin = ref.watch(useTorPluginProvider);
    // AMOLED only affects the dark theme — the toggle is honest about it:
    // disabled (with the explaining subtitle) while light is active.
    final darkActive = Theme.of(context).brightness == Brightness.dark;

    return Scaffold(
      appBar: AppBar(title: Text(l10n.appearanceTitle)),
      body: ListView(
        children: [
          // The optional Tor plugin (zec_wallet_tor), FIRST on the screen (S15
          // iPhone walk: under the theme it was not found). OFF by default; the
          // choice is read before the wallet opens, so it applies at the next
          // start (core/tor/tor_plugin.dart).
          SettingsSectionHeader(l10n.torPluginSectionTitle),
          SwitchListTile(
            key: torPluginToggleKey,
            value: useTorPlugin,
            onChanged: (v) => ref.read(useTorPluginProvider.notifier).set(v),
            title: Text(l10n.torPluginToggleTitle),
            subtitle: Text(l10n.torPluginToggleSubtitle),
          ),
          const Divider(),
          SettingsSectionHeader(l10n.themeSectionTitle),
          RadioGroup<ThemeMode>(
            groupValue: themeMode,
            onChanged: (mode) {
              if (mode != null) {
                ref.read(themeModeProvider.notifier).set(mode);
              }
            },
            child: Column(
              children: [
                RadioListTile<ThemeMode>(
                  value: ThemeMode.system,
                  title: Text(l10n.themeModeSystemTitle),
                  subtitle: Text(l10n.themeModeSystemSubtitle),
                ),
                RadioListTile<ThemeMode>(
                  value: ThemeMode.light,
                  title: Text(l10n.themeModeLightTitle),
                ),
                RadioListTile<ThemeMode>(
                  value: ThemeMode.dark,
                  title: Text(l10n.themeModeDarkTitle),
                ),
              ],
            ),
          ),
          SwitchListTile(
            value: amoled,
            onChanged: darkActive
                ? (v) => ref.read(amoledProvider.notifier).set(v)
                : null,
            title: Text(l10n.amoledTitle),
            subtitle: Text(l10n.amoledSubtitle),
          ),
          const Divider(),
          SettingsSectionHeader(l10n.textSizeSectionTitle),
          Slider(
            value: textScale,
            min: minTextScale,
            max: maxTextScale,
            // 0.05 steps across [0.85, 1.40].
            divisions: 11,
            label: '${(textScale * 100).round()}%',
            // One well-formed screen-reader node (control type + value);
            // an outer Semantics label would create a second overlapping
            // node read twice by TalkBack/VoiceOver (code review fold).
            semanticFormatterCallback: (v) =>
                l10n.textSizeSemanticValue((v * 100).round()),
            onChanged: (v) => ref.read(textScaleProvider.notifier).set(v),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            // Scales live with the slider: the MaterialApp builder applies
            // the effective text scale app-wide, preview included.
            child: Text(l10n.textSizePreview, style: textTheme.bodyLarge),
          ),
          const Divider(),
          // The SDK's device log: the level, remembered and applied at init
          // and after a Delete, and Share (core/logging/device_log.dart).
          const DeviceLogSection(),
          const Divider(),
          // Diagnostics entries live on this screen because it is the
          // example's only shell-owned settings surface (the flat-route
          // honest minimum) — a real host puts them behind its own dev menu.
          SettingsSectionHeader(l10n.diagIncomingDeveloperSection),
          ListTile(
            leading: const Icon(Icons.call_received),
            title: Text(l10n.diagIncomingEntryTitle),
            subtitle: Text(l10n.diagIncomingEntrySubtitle),
            onTap: () => context.push(AppRoutes.incomingEvents),
          ),
          ListTile(
            leading: const Icon(Icons.qr_code_scanner),
            title: Text(l10n.prefillDemoEntryTitle),
            subtitle: Text(l10n.prefillDemoEntrySubtitle),
            onTap: () => context.push(AppRoutes.prefilledSend),
          ),
          ListTile(
            leading: const Icon(Icons.data_object),
            title: const Text('Machine memo round trip'),
            subtitle: const Text(
              'FR-27/28 acceptance: a real self-send carrying opaque bytes, '
              'read back off-chain byte-for-byte.',
            ),
            onTap: () => context.push(AppRoutes.machineMemo),
          ),
        ],
      ),
    );
  }
}
