import 'package:flutter/material.dart';

/// An uppercase section header shared by the settings screens (Appearance,
/// Security). ONE source of truth so the screens can't silently diverge on the
/// header's padding/casing/style (duplicates are bugs).
class SettingsSectionHeader extends StatelessWidget {
  const SettingsSectionHeader(this.title, {super.key});

  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
      child: Text(
        title.toUpperCase(),
        style: Theme.of(context).textTheme.labelSmall,
      ),
    );
  }
}
