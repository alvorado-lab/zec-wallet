import 'dart:async' show TimeoutException, unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../shared/address_text.dart';
import '../../shared/decimal_input_formatter.dart';
import '../../shared/qr_tile.dart';
import '../../shared/wallet_info_button.dart';
import '../../shared/wallet_notice.dart';
import 'send/form_fault_view.dart' show SendFormFaultView;
import 'send/zec_amount.dart';
import 'wallet_providers.dart';
import 'wallet_session.dart';
import 'wallet_ui_config.dart';

/// Which receive address the screen shows (Recv-2 / ADR-0528). DEFAULT = shielded:
/// the shielded UA is the recommended, privacy-preserving address; the transparent
/// address is an explicit opt-in for the documented real need (a sender — a CEX, a
/// swap-in — that can only pay a transparent address). The default is enforced by
/// the provider below and pinned by `transparent_address_toggle_defaults_to_shielded`.
enum ReceiveAddressType { shielded, transparent }

/// The receive screen (spec §3.3 read surface; §3.3a Recv-2): shows the wallet's
/// current receive address so the user can receive ZEC, with a SHIELDED ⇄ TRANSPARENT
/// toggle (default shielded). The address is PUBLIC by design (the spending key never
/// leaves Rust, §4.1), so displaying + copying + QR-encoding it is safe; §5.4 NEVER-LOG
/// still holds — it is rendered/copied/encoded, never logged.
class ReceiveScreen extends ConsumerStatefulWidget {
  const ReceiveScreen({super.key});

  @override
  ConsumerState<ReceiveScreen> createState() => _ReceiveScreenState();
}

class _ReceiveScreenState extends ConsumerState<ReceiveScreen> {
  /// Which address the user is viewing — ephemeral per-screen UI state (Rust stays the
  /// source of truth for the addresses themselves; this only chooses which to render).
  /// LOCAL, not a global provider: EVERY entry to the receive screen starts at the
  /// recommended shielded default. Riverpod 3.x has no `AutoDisposeNotifier`, and a
  /// root-scoped provider would keep a stale transparent selection across re-entry —
  /// breaking the default-shielded invariant (code review fold).
  ReceiveAddressType _selected = ReceiveAddressType.shielded;

  /// The VISIT-scoped freshly minted diversified UA (FR-8 / Recv-4, ADR-0537) —
  /// when set, the shielded tab renders it in place of the default address
  /// (QR + text + copy all switch together, the Recv-1 money-correctness
  /// invariant). LOCAL by the same rule as [_selected]: every entry to the
  /// screen starts at the default address; the mint itself is durable Rust-side
  /// (the address keeps receiving forever), only the DISPLAY choice is
  /// ephemeral. Cleared on a session/identity flip (below).
  String? _mintedAddress;

  /// True while a mint is in flight — disables the button + shows its spinner;
  /// never a second concurrent mint from this screen.
  bool _minting = false;

  /// ONE-SHOT liveRegion arming: the fresh note announces via a
  /// liveRegion (the platform-sanctioned mechanism — `SemanticsService.announce`
  /// is deprecated, Android retired accessibility announcements), but ONLY on
  /// the frame the mint lands. A post-frame flip disarms it, so re-inserting
  /// the note (every transparent⇄shielded toggle back) never re-announces a
  /// mint that didn't happen. Re-armed per successful mint.
  bool _freshNoteAnnounced = false;

  /// Request amount (S13 §1.7): the field is drawn once the user asks for it
  /// and stays for the visit. Held HERE, not in the address body, so the
  /// request survives a tab switch (whose first load replaces the body) and
  /// follows whichever address is on screen.
  bool _requestOpen = false;

  final TextEditingController _requestAmount = TextEditingController();

  /// Focused once, when the user opens the field — not on every re-insert
  /// (a tab switch), which an `autofocus` would do.
  final FocusNode _requestFocus = FocusNode();

  void _openRequest() {
    setState(() => _requestOpen = true);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _requestFocus.requestFocus();
    });
  }

  @override
  void dispose() {
    _requestAmount.dispose();
    _requestFocus.dispose();
    super.dispose();
  }

  Future<void> _mintFreshAddress(WalletSession session) async {
    if (_minting) return;
    setState(() => _minting = true);
    try {
      // The action-level wedge bound (the package-wide 15 s FFI timeout): a hung
      // bridge surfaces the honest snackbar, never a stuck spinner on a money
      // surface. The mint is local + cheap; 15 s is generous.
      final minted = await session.mintDiversifiedAddress().timeout(
        walletFfiWedgeTimeout,
      );
      // Discard a completion that raced unmount or an identity flip — the
      // result belongs to the OLD wallet and must not render under the new one
      // (the #330 session-flip discipline).
      if (!mounted || !identical(ref.read(walletSessionProvider), session)) {
        return;
      }
      setState(() {
        _mintedAddress = minted.address;
        _minting = false;
        // Re-arm the ONE-SHOT liveRegion for this mint (fold, below).
        _freshNoteAnnounced = false;
      });
    } on TimeoutException {
      // The wedge bound tripping usually means the wallet is BUSY (a long
      // signing/proving pass holds the DB lock), not that the mint failed —
      // the mint may even land silently later (burned index, never shared,
      // harmless). Say "busy, retry", not "couldn't create".
      if (!mounted || !identical(ref.read(walletSessionProvider), session)) {
        return;
      }
      setState(() => _minting = false);
      final l10n = WalletLocalizations.of(context);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.walletReceiveFreshBusy)));
    } catch (_) {
      // §5.4: the failure payload is never logged or rendered; the default
      // address stays on screen — honest, recoverable, no dead-end.
      if (!mounted || !identical(ref.read(walletSessionProvider), session)) {
        return;
      }
      setState(() => _minting = false);
      final l10n = WalletLocalizations.of(context);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.walletReceiveFreshError)));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    // Distinguish "no wallet yet" from "address lookup failed" honestly (mirrors
    // the swap screen's session gate): a null session is the not-set-up state, not
    // an error. Only a genuine lookup failure shows the error copy.
    final session = ref.watch(walletSessionProvider);
    // A session/identity flip while this screen is open must drop the minted
    // display — it belongs to the previous wallet (the send-screen
    // precedent). The providers are identity-fenced already; this clears the
    // screen-local piece.
    ref.listen(walletSessionProvider, (previous, next) {
      if (!identical(previous, next) && (_mintedAddress != null || _minting)) {
        setState(() {
          _mintedAddress = null;
          _minting = false;
        });
      }
    });

    // S13 §1.7 — Share is offered only when the host supplied a share sheet.
    final onShare = ref.watch(walletUiConfigProvider).onShare;

    final Widget body;
    if (session == null) {
      body = _Centered(
        icon: WalletGlyph.wallet,
        text: l10n.walletReceiveUnavailable,
      );
    } else {
      final isTransparent = _selected == ReceiveAddressType.transparent;
      // Watch the provider for the SELECTED type. Each is a stable, local derivation
      // (§3.3a) with the same honest-degradation timeout — a wedged FFI load surfaces
      // the error state, not an infinite spinner on a money surface.
      final addressAsync = isTransparent
          ? ref.watch(walletTransparentAddressProvider)
          : ref.watch(walletReceiveAddressProvider);
      body = Column(
        children: [
          _AddressTypeToggle(
            selected: _selected,
            onChanged: (next) => setState(() => _selected = next),
          ),
          Expanded(
            child: addressAsync.when(
              // Honest, friendly loading — not a bare spinner. The address is a
              // LOCAL derivation, but it can queue behind a heavy first sync, so
              // the hint says WHY it may take a moment (transparency, never a
              // silent wait on a money surface).
              loading: () => const _AddressLoading(),
              // A genuine lookup/timeout failure is recoverable, never a
              // dead-end: re-attempt by invalidating the (selected) provider so
              // a fresh derive runs — no perpetual spinner, no stuck screen.
              error: (_, _) => _AddressError(
                onRetry: () => ref.invalidate(
                  isTransparent
                      ? walletTransparentAddressProvider
                      : walletReceiveAddressProvider,
                ),
              ),
              data: (addr) {
                final isFresh = !isTransparent && _mintedAddress != null;
                // Disarm the one-shot liveRegion AFTER the frame that renders
                // it — the announcement belongs to the mint frame only.
                if (isFresh && !_freshNoteAnnounced) {
                  WidgetsBinding.instance.addPostFrameCallback((_) {
                    if (mounted && !_freshNoteAnnounced) {
                      setState(() => _freshNoteAnnounced = true);
                    }
                  });
                }
                return _AddressBody(
                  // The FRESH minted UA (when present) replaces the default on
                  // the SHIELDED tab only — QR + text + copy all switch
                  // together (Recv-1 money-correctness); the transparent tab
                  // is untouched.
                  address: isTransparent ? addr : (_mintedAddress ?? addr),
                  isTransparent: isTransparent,
                  isFresh: isFresh,
                  freshNoteLive: isFresh && !_freshNoteAnnounced,
                  minting: _minting,
                  onMintFresh: isTransparent
                      ? null
                      : () => _mintFreshAddress(session),
                  // Request amount (S13 §1.7): the SDK's own ZIP-321 encoder,
                  // validated against the wallet's network.
                  composeUri: (recipient, amountZat) =>
                      session.composePaymentUri(
                        recipient: recipient,
                        amountZat: amountZat,
                      ),
                  onShare: onShare,
                  requestAmount: _requestAmount,
                  requestOpen: _requestOpen,
                  onOpenRequest: _openRequest,
                  requestFocus: _requestFocus,
                );
              },
            ),
          ),
        ],
      );
    }

    return Scaffold(
      appBar: AppBar(title: Text(l10n.walletReceive)),
      body: SafeArea(child: body),
    );
  }
}

/// The SHIELDED ⇄ TRANSPARENT segmented toggle (Recv-2 / ADR-0528). Default shielded;
/// the user opts into transparent. Material 3 `SegmentedButton` carries the a11y
/// semantics + 44×44 touch targets for free (flutter-patterns § Widget Conventions).
class _AddressTypeToggle extends StatelessWidget {
  const _AddressTypeToggle({required this.selected, required this.onChanged});

  final ReceiveAddressType selected;
  final ValueChanged<ReceiveAddressType> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final isTransparent = selected == ReceiveAddressType.transparent;
    return Padding(
      padding: const EdgeInsets.fromLTRB(24, 16, 16, 0),
      child: Row(
        children: [
          Expanded(child: _segments(l10n)),
          const SizedBox(width: 4),
          // The address's explanation is not a state: behind the (i) after
          // the label it explains — the selected type (S13 §1.7, DESIGN §6.20).
          WalletInfoButton(
            key: const Key('receive-address-info'),
            label: isTransparent
                ? l10n.walletReceiveTypeTransparent
                : l10n.walletReceiveTypeShielded,
            body: isTransparent
                ? l10n.walletReceiveSubtitleTransparent
                : l10n.walletReceiveSubtitle,
          ),
        ],
      ),
    );
  }

  Widget _segments(WalletLocalizations l10n) {
    return SegmentedButton<ReceiveAddressType>(
      key: const Key('receive-address-type-toggle'),
      segments: [
        ButtonSegment<ReceiveAddressType>(
          value: ReceiveAddressType.shielded,
          icon: const WalletIcon(WalletGlyph.shielded),
          label: Text(l10n.walletReceiveTypeShielded),
        ),
        ButtonSegment<ReceiveAddressType>(
          value: ReceiveAddressType.transparent,
          // `public` (not `visibility`) — reinforces "visible to all on-chain", the
          // actual meaning of a transparent address, matching the warning's icon
          // (arch review design fold); `visibility` reads as "reveal/preview".
          icon: const WalletIcon(WalletGlyph.transparentAddress),
          label: Text(l10n.walletReceiveTypeTransparent),
        ),
      ],
      selected: {selected},
      showSelectedIcon: false,
      // single-select (the default) ⇒ exactly one element; `single` makes that
      // assumption explicit (throws loudly if a future multi-select breaks it).
      onSelectionChanged: (set) => onChanged(set.single),
    );
  }
}

class _AddressBody extends StatefulWidget {
  const _AddressBody({
    required this.address,
    required this.isTransparent,
    required this.composeUri,
    required this.requestAmount,
    required this.requestOpen,
    required this.onOpenRequest,
    required this.requestFocus,
    this.isFresh = false,
    this.freshNoteLive = false,
    this.minting = false,
    this.onMintFresh,
    this.onShare,
  });

  final String address;

  /// True when rendering the TRANSPARENT address — surfaces the PUBLIC warning and
  /// keys the QR/copy widgets distinctly so a toggle re-renders the correct payload.
  final bool isTransparent;

  /// True when [address] is a freshly minted diversified UA (FR-8) — surfaces the
  /// fresh-address note so the user knows what changed and why it is safe.
  final bool isFresh;

  /// True ONLY on the frame the mint lands (the screen's one-shot arming):
  /// the note's liveRegion announces once, then re-inserts silent.
  final bool freshNoteLive;

  /// True while a mint is in flight — the fresh-address button disables and
  /// shows its in-button spinner (the demo-hardening pattern).
  final bool minting;

  /// Non-null on the shielded tab only: mints the next diversified UA (FR-8).
  final VoidCallback? onMintFresh;

  /// The SDK's ZIP-321 encoder (`WalletSession.composePaymentUri`) for a
  /// requested amount; may throw a typed `WalletApiError`.
  final String Function(String recipient, int amountZat) composeUri;

  /// The host's share sheet ([WalletUiConfig.onShare]); null = no Share.
  final Future<void> Function(String text, {String? subject})? onShare;

  /// The requested amount's text (owned by the screen, see its doc).
  final TextEditingController requestAmount;

  /// Whether the amount field is drawn (else the "Request amount" button).
  final bool requestOpen;
  final VoidCallback onOpenRequest;
  final FocusNode requestFocus;

  @override
  State<_AddressBody> createState() => _AddressBodyState();
}

class _AddressBodyState extends State<_AddressBody> {
  /// The last composed request, keyed by what it was composed from, so a
  /// rebuild does not cross the bridge again for the same (address, amount).
  /// A null URI records that the encoder REFUSED that pair: the failure is
  /// state, so the field can say so (below) instead of the payload quietly
  /// dropping the amount.
  (String, int, String?)? _composed;

  /// The requested amount, or null when none (an empty field) or when the
  /// field does not parse — a malformed amount changes nothing: the payload
  /// stays the plain address, and the field says why.
  ZecAmountResult? get _parsedAmount {
    final text = widget.requestAmount.text;
    if (text.trim().isEmpty) return null;
    return parseZecAmount(text);
  }

  /// THE payload: what the QR encodes, Copy writes and Share hands the host.
  /// With a valid requested amount it is exactly `composePaymentUri`'s output
  /// for (the address on screen, that amount); otherwise the address itself.
  /// `composeFailed` is true when the amount parsed but the encoder refused
  /// it: the payload is then the plain address (still correct to receive
  /// on), and the field MUST say the amount is not in it.
  ({String payload, bool composeFailed}) get _payload {
    final address = widget.address;
    final parsed = _parsedAmount;
    if (parsed is! ZecAmountValid) {
      return (payload: address, composeFailed: false);
    }
    final cached = _composed;
    if (cached == null || cached.$1 != address || cached.$2 != parsed.zat) {
      String? uri;
      try {
        uri = widget.composeUri(address, parsed.zat);
      } catch (_) {
        // Never logged (§5.4); kept as state and surfaced under the field.
        uri = null;
      }
      _composed = (address, parsed.zat, uri);
    }
    final uri = _composed!.$3;
    return uri == null
        ? (payload: address, composeFailed: true)
        : (payload: uri, composeFailed: false);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final address = widget.address;
    final isTransparent = widget.isTransparent;
    final minting = widget.minting;
    final onMintFresh = widget.onMintFresh;
    final onShare = widget.onShare;
    final (:payload, :composeFailed) = _payload;
    final parsed = _parsedAmount;
    // ONE binding for the copy: the button's `onPressed` and the semantics
    // action below read it, so a screen reader's double-tap and a finger run
    // the same code.
    Future<void> copy() async {
      await Clipboard.setData(ClipboardData(text: payload));
      unawaited(HapticFeedback.lightImpact());
      if (!context.mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.walletReceiveCopied)));
    }

    // Null while a mint is in flight — the SAME predicate `enabled:` reads, so
    // the node never advertises an action it refuses.
    final mint = minting ? null : onMintFresh;
    final copyButton = Semantics(
      // `container: true` so iOS VoiceOver doesn't merge the copy button with
      // the SelectableText address above it into one element (flutter-patterns
      // § iOS Semantics merging) — the tap would otherwise retarget to the text.
      container: true,
      button: true,
      // excludeSemantics so the button's own label doesn't duplicate the
      // container's — the press surface is ONE a11y node, never merged
      // with the address text (flutter-patterns § iOS Semantics merging).
      excludeSemantics: true,
      label: l10n.walletReceiveCopy,
      // `excludeSemantics` drops the child subtree and the button's own
      // tap action with it, so the action must live on THIS node — a
      // screen reader otherwise hears "Copy address, button" and its
      // double-tap has nothing to invoke (Relim `0c5ae1bd`, #661/#721).
      onTap: () => unawaited(copy()),
      // Tonal (S13 §1.7, as the deposit screen's Copy in Build A).
      child: FilledButton.tonalIcon(
        key: const Key('receive-copy'),
        icon: const WalletIcon(WalletGlyph.copy),
        label: Text(l10n.walletReceiveCopy),
        onPressed: () => unawaited(copy()),
      ),
    );
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (isTransparent) ...[
            // Honest framing (§3.3a): the transparent address is PUBLIC — visible
            // on-chain and reused-address-linkable. The UI never hides this; the
            // shielded address stays the recommended default. A STATE, so it
            // stays on screen (S13's text rule); the explanation above moved
            // behind the toggle's (i).
            _TransparentWarning(text: l10n.walletReceiveTransparentWarning),
            const SizedBox(height: 24),
          ],
          // The QR encodes the EXACT full payload (Recv-1 money-correctness,
          // spec §3.3a): the QR payload == the copied string == the shared
          // string == the SDK address — or, with a requested amount, exactly
          // the SDK's ZIP-321 URI for it; a dropped character would send funds
          // to nowhere. The pinned name `_QrTile.payload` is what the
          // money-correctness widget tests assert.
          Center(
            child: QrTile(
              payload: payload,
              tileKey: const Key('receive-qr-tile'),
              // a distinct a11y label per type so a screen reader doesn't announce
              // the shielded and transparent QRs identically (code review a11y fold).
              label: isTransparent
                  ? l10n.walletReceiveQrLabelTransparent
                  : l10n.walletReceiveQrLabel,
            ),
          ),
          const SizedBox(height: 24),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              // Bold first/last chars, muted middle — the at-a-glance verification
              // pattern; still fully selectable + the exact full address (shared
              // AddressText, used identically on the deposit screen).
              child: AddressText(address),
            ),
          ),
          const SizedBox(height: 16),
          if (onShare == null)
            copyButton
          else
            Row(
              children: [
                Expanded(child: copyButton),
                const SizedBox(width: 12),
                Expanded(
                  // S13 §1.7: the host's share sheet, only from this tap. The
                  // text is the payload (the address, or the request URI);
                  // the subject is NONE — never an amount, memo or label.
                  // Whatever the hook does — completes, the user dismisses
                  // the sheet, or it throws — shows nothing: a dismissed
                  // sheet is not an error (Relim), and the payload is still
                  // on screen to copy. `callWalletHostHook` contains a throw.
                  child: FilledButton.tonalIcon(
                    key: const Key('receive-share'),
                    icon: const WalletIcon(WalletGlyph.share),
                    label: Text(l10n.walletReceiveShare),
                    onPressed: () =>
                        unawaited(callWalletHostHook(() => onShare(payload))),
                  ),
                ),
              ],
            ),
          const SizedBox(height: 8),
          if (!widget.requestOpen)
            Align(
              alignment: AlignmentDirectional.centerStart,
              child: TextButton(
                key: const Key('receive-request-amount'),
                onPressed: widget.onOpenRequest,
                child: Text(l10n.walletReceiveRequestAmount),
              ),
            )
          else
            TextField(
              key: const Key('receive-request-amount-field'),
              controller: widget.requestAmount,
              focusNode: widget.requestFocus,
              keyboardType: const TextInputType.numberWithOptions(
                decimal: true,
              ),
              // The one amount formatter (S13 §1.1): a comma reads as the
              // point, a second separator or a stray character is refused
              // whole; 20 chars > the longest valid amount.
              inputFormatters: const [
                WalletDecimalInputFormatter(maxLength: 20),
              ],
              decoration: InputDecoration(
                labelText: l10n.walletReceiveRequestAmountLabel,
                // A malformed amount — or one the encoder refused — leaves
                // the payload the plain address; say so, so nobody shares a
                // QR believing it carries one.
                errorText: switch (parsed) {
                  ZecAmountInvalid(:final fault) =>
                    SendFormFaultView.amountFaultMessage(l10n, fault),
                  _ when composeFailed => l10n.walletSendFaultUriInvalid,
                  _ => null,
                },
              ),
              onChanged: (_) => setState(() {}),
            ),
          if (onMintFresh != null) ...[
            const SizedBox(height: 12),
            // The FR-8 mint affordance (shielded tab only) — SECONDARY prominence
            // (OutlinedButton) under the primary Copy: sharing the default address
            // stays the mainline; a fresh unlinkable address is the opt-in for a
            // contact/invoice. Same one-node semantics pattern as Copy.
            Semantics(
              container: true,
              button: true,
              excludeSemantics: true,
              // excludeSemantics drops the child button's own enabled flag, so
              // mirror it here — a screen-reader user must hear the button as
              // disabled while the mint is in flight.
              enabled: !minting,
              label: l10n.walletReceiveFreshAddress,
              // …and it drops the button's tap action too, so the action lives
              // here as well. Null while minting: the node must not offer a
              // double-tap it will ignore (Relim `0c5ae1bd`, #661/#721).
              onTap: mint,
              child: OutlinedButton.icon(
                key: const Key('receive-fresh-address'),
                onPressed: mint,
                icon: minting
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator.adaptive(
                          strokeWidth: 2,
                        ),
                      )
                    : const WalletIcon(WalletGlyph.add),
                label: Text(l10n.walletReceiveFreshAddress),
              ),
            ),
          ],
          if (widget.isFresh) ...[
            const SizedBox(height: 16),
            // The FRESH-address note (FR-8 / Recv-4). On screen: the display's
            // impermanence ("copy it now") — a STATE, so it stays (S13 §1.7).
            // Behind its (i), WHOLE under its existing key: what changed (a
            // new, unlinkable address) and the money-safety facts (payments
            // arrive in this wallet; earlier addresses keep working). BELOW
            // the action buttons: inserting it above shifted
            // Copy / Fresh mid-reach — a device-observed mis-tap; down here
            // nothing the user is about to tap moves.
            _FreshAddressNote(
              text: l10n.walletReceiveFreshCopyNow,
              infoLabel: l10n.walletReceiveFreshAddress,
              infoBody: l10n.walletReceiveFreshCaption,
              liveRegion: widget.freshNoteLive,
            ),
          ],
        ],
      ),
    );
  }
}

/// The FRESH-address note (FR-8 / Recv-4) — the [_TransparentWarning] sibling in a
/// POSITIVE register (a positive notice line: this is the more-private option, not
/// a caution). [liveRegion] is the screen's ONE-SHOT arming: an
/// always-on liveRegion re-announces on every re-insert (each transparent⇄shielded
/// toggle back), implying a fresh mint that never happened, while the deprecated
/// `SemanticsService.announce` is not an option (Android retired announcements) —
/// so the region is live only on the mint frame and silent thereafter.
class _FreshAddressNote extends StatelessWidget {
  const _FreshAddressNote({
    required this.text,
    required this.infoLabel,
    required this.infoBody,
    this.liveRegion = false,
  });

  final String text;

  /// The (i)'s label and its explanation (S13 §1.7).
  final String infoLabel;
  final String infoBody;
  final bool liveRegion;

  @override
  Widget build(BuildContext context) {
    return WalletNotice(
      key: const Key('receive-fresh-note'),
      tone: WalletNoticeTone.positive,
      glyph: WalletGlyph.success,
      message: text,
      liveRegion: liveRegion,
      actions: [
        WalletInfoButton(
          key: const Key('receive-fresh-info'),
          label: infoLabel,
          body: infoBody,
        ),
      ],
    );
  }
}

/// The PUBLIC honest-framing banner for the transparent address (§3.3a). A notice
/// line in the warning tone — a warning, not an error, since using the transparent
/// address is a legitimate opt-in, just less private.
class _TransparentWarning extends StatelessWidget {
  const _TransparentWarning({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    // liveRegion so VoiceOver/TalkBack ANNOUNCES the public-address warning when it
    // appears on toggle (an honest-degradation element a screen-reader user must not
    // miss), not only when navigated to (code review a11y fold).
    return WalletNotice(
      key: const Key('receive-transparent-warning'),
      tone: WalletNoticeTone.warning,
      glyph: WalletGlyph.transparent,
      message: text,
      liveRegion: true,
    );
  }
}

/// The honest loading state while the receive address derives (Recv-2). A bare
/// spinner reads as "stuck" on a money surface; this names what is happening and
/// — since the derive can queue behind a heavy first sync — reassures the user it
/// is normal for the first one to take a moment. Pure presentation; the address
/// itself is still derived once Rust-side.
class _AddressLoading extends StatelessWidget {
  const _AddressLoading();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    // Scrollable so the headline never overflows at large system text scales
    // on a short screen (a11y; matches _AddressBody's pattern). The hint (why
    // it can take a moment) is an explanation: behind the (i) after the
    // headline (S13 §1.7).
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(
              width: 28,
              height: 28,
              child: CircularProgressIndicator.adaptive(strokeWidth: 3),
            ),
            const SizedBox(height: 20),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Flexible(
                  child: Text(
                    l10n.walletReceivePreparing,
                    textAlign: TextAlign.center,
                    style: textTheme.titleMedium,
                  ),
                ),
                WalletInfoButton(
                  key: const Key('receive-preparing-info'),
                  label: l10n.walletReceivePreparing,
                  body: l10n.walletReceivePreparingHint,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// The recoverable address-load failure (Recv-2): an honest message plus a
/// Try-again that re-runs the derive — never a dead-end error or an infinite
/// spinner (honest degradation, invariant 6; no silent failure, invariant 10).
class _AddressError extends StatelessWidget {
  const _AddressError({required this.onRetry});

  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    // liveRegion so a screen reader ANNOUNCES the failure when the derive
    // transitions loading→error WHILE the user is on-screen (#386 made this
    // reachable: `retry: null` stopped riverpod swallowing the 60 s derive
    // timeout into a perpetual spinner, so the error now actually renders in
    // place — a TalkBack user must not keep hearing "Preparing your address…"
    // through a genuine wedge). Matches every sibling honest-degradation
    // element (_TransparentWarning, the quote countdown, the store-busy retry;
    // a11y fold).
    return Semantics(
      container: true,
      liveRegion: true,
      child: Center(
        child: SingleChildScrollView(
          key: const Key('receive-address-error'),
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const WalletIcon(WalletGlyph.error, size: 48),
              const SizedBox(height: 16),
              Text(
                l10n.walletReceiveError,
                textAlign: TextAlign.center,
                style: textTheme.bodyLarge,
              ),
              const SizedBox(height: 20),
              FilledButton.icon(
                key: const Key('receive-address-retry'),
                onPressed: onRetry,
                icon: const WalletIcon(WalletGlyph.retry),
                label: Text(l10n.walletReceiveRetry),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Centered extends StatelessWidget {
  const _Centered({required this.icon, required this.text});

  final WalletGlyph icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(icon, size: 48),
            const SizedBox(height: 16),
            Text(text, textAlign: TextAlign.center, style: textTheme.bodyLarge),
          ],
        ),
      ),
    );
  }
}
