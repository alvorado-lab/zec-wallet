import 'package:flutter/material.dart';

import '../core/theme/colors.dart';
import '../core/theme/typography.dart';

/// Displays a (long) wallet address in monospace with its FIRST and LAST few
/// characters emphasized (bold, full text colour) and the middle de-emphasized
/// (muted) — the at-a-glance verification pattern every wallet uses: the ends
/// are what a person checks against a known address, so they read first while
/// the middle recedes. The FULL address is always present (never truncated) and
/// stays selectable, so copy-by-hand and screen-reader read-out are unaffected.
///
/// Used wherever an address is SHOWN read-only (receive shielded UA + transparent
/// t-address, the IntoZec deposit address). NOT for a VERIFICATION step (the send
/// recipient review, the swap refund echo), which renders every character in
/// equal-weight chunks — use [AddressVerificationText] there.
class AddressText extends StatelessWidget {
  const AddressText(this.address, {super.key, this.edge = 6, this.style});

  /// The full address. Only the MIDDLE is visually muted — nothing is dropped.
  final String address;

  /// How many leading and trailing characters to emphasize.
  final int edge;

  /// Base style (size/spacing); per-span colour + weight are applied on top.
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final base = WalletTypography.of(context).monoOn(
      (style ?? Theme.of(context).textTheme.bodyLarge ?? const TextStyle())
          .copyWith(letterSpacing: 0.3, height: 1.35),
    );
    final strong = base.copyWith(
      color: colors.text,
      fontWeight: FontWeight.w700,
    );
    final faint = base.copyWith(
      color: colors.textMuted,
      fontWeight: FontWeight.w400,
    );

    final a = address;
    // Too short to split meaningfully — emphasize the whole thing.
    if (a.length <= edge * 2) {
      return SelectableText(a, style: strong);
    }
    return SelectableText.rich(
      TextSpan(
        children: [
          TextSpan(text: a.substring(0, edge), style: strong),
          TextSpan(text: a.substring(edge, a.length - edge), style: faint),
          TextSpan(text: a.substring(a.length - edge), style: strong),
        ],
      ),
    );
  }
}

/// Visually group an address into space-separated, fixed-size [groupSize] chunks
/// ("u1qx z9k2 …") so a long string is scannable character-by-character. The ONE
/// source of truth for the equal-weight VERIFICATION rendering (the send recipient
/// review + the swap refund-verification echo). Purely a render aid: it is
/// loss-free — stripping the inserted spaces yields the original verbatim (an
/// address never contains a space) — so a grouping bug could only mis-render,
/// never change the value a user verifies. Pure + total; tested at its boundaries.
String groupAddress(String address, {int groupSize = 4}) {
  assert(groupSize > 0, 'groupSize must be positive');
  if (address.isEmpty) return '';
  final buf = StringBuffer();
  for (var i = 0; i < address.length; i += groupSize) {
    if (i > 0) buf.write(' ');
    final end = i + groupSize;
    buf.write(
      address.substring(i, end < address.length ? end : address.length),
    );
  }
  return buf.toString();
}

/// Renders an address in equal-weight, space-separated MONOSPACE chunks for
/// character-by-character VERIFICATION (a typo to the wrong address loses coins).
/// The companion to [AddressText]: that emphasizes head+tail for at-a-glance
/// read-only display; THIS gives every glyph equal weight because the user is
/// checking the WHOLE string against a known address.
///
/// Deliberately NOT selectable — the user is verifying, not copying, and a copy
/// would capture the render-only spaces (a paste-back footgun). The FULL,
/// ungrouped address is the screen-reader label, so a non-sighted user hears the
/// canonical string rather than a stutter of chunks. An empty address renders an
/// em dash (defensive — the caller still gates on its own acknowledgment).
class AddressVerificationText extends StatelessWidget {
  const AddressVerificationText(this.address, {super.key, this.style});

  /// The full address. Rendered in full (grouped) — nothing is ever dropped.
  final String address;

  /// Base style (size/spacing); monospace + the verification colour are applied
  /// on top. Defaults to `bodyMedium`.
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final base = WalletTypography.of(context).monoOn(
      (style ?? Theme.of(context).textTheme.bodyMedium ?? const TextStyle())
          .copyWith(letterSpacing: 0.3, color: colors.text),
    );
    if (address.isEmpty) return Text('—', style: base);
    return Semantics(
      label: address,
      excludeSemantics: true,
      child: Text(groupAddress(address), style: base),
    );
  }
}
