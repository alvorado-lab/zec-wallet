import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show TextInput;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../core/theme/shapes.dart';
import '../../shared/wallet_dialog.dart';
import '../../shared/wallet_sheet.dart';
import 'onboarding/onboarding_controller.dart';
import 'wallet_providers.dart';

/// The in-flight notice names the state the switch interrupts (the
/// device walk's two observations, folded — `sync-server-picker.md`
/// §3.4, phase-3 §6 "The device walk"). At the tip nothing is "in progress",
/// so the copy says the switch RECONNECTS; while connecting or scanning it
/// says the switch RESTARTS the pass and that funds may read as pending until
/// the new server's scan catches up (`spendable_ready` is upstream's summary
/// of a scan with unscanned ranges — the pending reading is the truthful one,
/// and the card beside the notice changes). `null` (the status not yet
/// delivered) and `idle` take the scanning copy: the claim that survives
/// being wrong is the cautious one.
String _switchNoticeBody(WalletLocalizations l10n, SyncStatus? status) =>
    switch (status) {
      SyncStatus_UpToDate() ||
      SyncStatus_UpToDateLimited() ||
      SyncStatus_UpToDateDegraded() => l10n.walletSyncServerSwitchNoticeAtTip,
      _ => l10n.walletSyncServerSwitchNotice,
    };

/// The sync-server picker (`docs/specs/sync-server-picker.md` §3.4, P3-13):
/// the servers the host OFFERS as rows (the one in use marked), "App default"
/// when the default is not among them, and a custom entry — a URL field, a
/// "Check server" step, a trust notice, then "Use this server". Every switch
/// is preceded by the in-flight notice (it restarts the sync pass in
/// progress; the balance and history stay), and a custom server by the trust
/// notice as well. Opened from the sync sheet's Server row.
Future<void> showSyncServerSheet(BuildContext context) {
  return showWalletSheet<void>(
    context,
    builder: (_) => const SyncServerSheet(),
  );
}

/// The copy for a [SyncServerFallback] — shared by the sync sheet's Server
/// row banner and the picker's own banner (one wording, two places).
String syncServerFallbackCopy(
  WalletLocalizations l10n,
  SyncServerFallback fallback, {
  required String host,
}) => switch (fallback) {
  SyncServerFallback_ChoiceNotOffered() =>
    l10n.walletSyncServerFallbackNotOffered(host),
  // FR-29 E12: a remembered unencrypted (http://) server under the app's
  // private path — said, never silent, and the choice is kept.
  SyncServerFallback_ChoiceRefusedByTransport() =>
    l10n.walletSyncServerFallbackRefusedByTransport(host),
  SyncServerFallback_ChoiceUnreadable() ||
  SyncServerFallback_Unknown() => l10n.walletSyncServerFallbackUnreadable(host),
};

/// Did the USER type the address this refusal is about? The one fact that
/// decides whether "check the address" is a step the reader can take
/// ([syncServerRefusalCopy]). A custom URL is typed; a predefined, default or
/// forward-compat choice came from the app's own list, so its address is not
/// the reader's to check. Exhaustive and wildcard-free: a choice added to the
/// bridge is a compile error here, never silently routed to the typed copy.
bool addressTypedByUser(SyncServerChoice choice) => switch (choice) {
  SyncServerChoice_Custom() => true,
  SyncServerChoice_Predefined() ||
  SyncServerChoice_Default() ||
  SyncServerChoice_Unknown() => false,
};

/// The copy for a switch/probe REFUSAL, by KIND — never by message text (the
/// FFI rule). Unknown kinds fall to the unreachable copy: the honest generic
/// ("couldn't reach this server") over a silent no-op.
///
/// [typedAddress] splits the unreachable arm, and stage S1 `copy` is why. That
/// arm is the CATCH-ALL: `probe_oracle` maps everything that is not a FAILED
/// private dial onto `syncServerUnreachable`, and since stage S1 a private
/// path that ACCEPTS the dial and then carries nothing no longer reports
/// `TorUnavailable` — deliberately, because blaming the path for what the
/// evidence cannot separate from a wedged server is the over-claim the stage
/// removed. So a censored path and a wedged server arrive here as the same
/// error, and the SDK genuinely cannot tell them apart. One sentence cannot
/// serve both readers: for a user who just TYPED an address, "check the
/// address" is the right first step; for a server the APP offered, the address
/// is not theirs to check and that step is dead advice. Each sentence names
/// the causes it cannot choose between and gives a step the reader can
/// actually take.
String syncServerRefusalCopy(
  WalletLocalizations l10n,
  WalletApiError error, {
  required bool typedAddress,
}) => switch (error.kind) {
  // The private path is GENUINELY down (a dial that failed, nothing
  // registered, a FAILED descriptor): the honest word is the path's, never
  // "check the address" — the same copy the sync badge uses.
  WalletErrorKind_Sync(:final stall) when stall == StallReason.torUnavailable =>
    l10n.walletStallTor,
  WalletErrorKind_NetworkMismatch() => l10n.walletSyncServerWrongNetwork,
  WalletErrorKind_InvalidEndpoint() => l10n.walletSyncServerInvalidUrl,
  // ADR-0568: a key or header the door refused — the key's copy, never the
  // address's (the user typed both; only the kind tells them which to fix).
  WalletErrorKind_InvalidEndpointAuth() => l10n.walletSyncServerKeyInvalid,
  WalletErrorKind_SyncServerNotOffered() => l10n.walletSyncServerNotOffered,
  WalletErrorKind_WalletBusy() => l10n.walletSyncServerBusy,
  _ =>
    typedAddress
        ? l10n.walletSyncServerUnreachable
        : l10n.walletSyncServerUnreachableOffered,
};

/// The host of a URL for display — never the whole URL (§5.4 / ).
String hostOf(String url) {
  final host = Uri.tryParse(url)?.host;
  return (host == null || host.isEmpty) ? url : host;
}

class SyncServerSheet extends ConsumerStatefulWidget {
  const SyncServerSheet({super.key});

  @override
  ConsumerState<SyncServerSheet> createState() => _SyncServerSheetState();
}

class _SyncServerSheetState extends ConsumerState<SyncServerSheet> {
  final _custom = TextEditingController();

  /// The user's key for their own server (ADR-0568), and the header it goes
  /// in. Never logged; cleared after a switch and disposed with the sheet
  /// (a Dart string cannot be zeroized — the spec §4 residency list says so).
  final _key = TextEditingController();
  final _keyHeader = TextEditingController();

  /// The key field shows its text (the toggle). Either way the field takes
  /// no suggestions, no autocorrect and no keyboard learning. SHOWN by
  /// default (S15 iPhone walk): an obscured field is a secure field to iOS,
  /// which offers its Passwords bar over it. The key is a bearer access
  /// handle, not a secret (the maintainer's ruling): shown only while typed,
  /// never filled back from the store, cleared after a switch, copyable to
  /// no clipboard (Paste only); Hide stays one tap away.
  bool _showKey = true;

  /// The custom entry is expanded.
  bool _customOpen = false;

  /// A probe is in flight (the actions disable; the row shows the spinner).
  bool _probing = false;

  /// The custom choice the last probe VERIFIED (reachable, this network) —
  /// URL and key exactly as checked. "Use this server" appears only for it
  /// and sends it unchanged; ANY edit to the URL, the key or the header
  /// clears it, so the server switched to is the server checked.
  SyncServerChoice? _verifiedChoice;

  /// A switch is in flight.
  bool _switching = false;

  /// The last refusal's copy (inline, under the actions), or the recovered
  /// failure's copy. Cleared by the next action.
  String? _notice;

  /// Custom hosts the user has accepted the trust notice for, in this
  /// sheet's life — keyed on (host, key present): the notice is shown ONCE
  /// per pair, and BEFORE the first probe (the crypto audit's MEDIUM: the
  /// probe already discloses the IP). Typing a key re-asks, because the
  /// keyed notice says something the keyless one did not (ADR-0568).
  final Set<String> _trustedHosts = {};

  @override
  void dispose() {
    _custom.dispose();
    _key.dispose();
    _keyHeader.dispose();
    super.dispose();
  }

  void _unverify() => setState(() {
    _verifiedChoice = null;
    _notice = null;
  });

  /// The custom choice the fields describe, or `null` with the inline copy
  /// set when a key was typed without its header. The header's own rules are
  /// the SDK's (`invalidEndpointAuth`), never a Dart check.
  SyncServerChoice? _customChoice(WalletLocalizations l10n) {
    final url = _custom.text.trim();
    final value = _key.text;
    if (value.isEmpty) return SyncServerChoice.custom(url: url, key: null);
    final header = _keyHeader.text.trim();
    if (header.isEmpty) {
      setState(() => _notice = l10n.walletSyncServerKeyHeaderNeeded);
      return null;
    }
    return SyncServerChoice.custom(
      url: url,
      key: SyncServerKey(header: header, value: value),
    );
  }

  Future<void> _probeCustom(WalletLocalizations l10n) async {
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    final choice = _customChoice(l10n);
    if (choice == null) return;
    final keyed = _key.text.isNotEmpty;
    // The trust notice precedes the FIRST PACKET to a custom server — the
    // probe is that packet (it reveals the IP unless Tor is on, and sends the
    // key if one is given) — so it is asked here, before the Check, not at
    // the switch. Cancel sends nothing.
    final trustKey = '${hostOf(_custom.text.trim())}|$keyed';
    if (!_trustedHosts.contains(trustKey)) {
      final accepted = await _confirm(
        l10n,
        title: l10n.walletSyncServerTrustTitle,
        body: keyed
            ? '${l10n.walletSyncServerTrustNotice}\n\n'
                  '${l10n.walletSyncServerTrustNoticeKey}'
            : l10n.walletSyncServerTrustNotice,
        action: l10n.walletSyncServerContinue,
      );
      if (!mounted || !accepted) return;
      _trustedHosts.add(trustKey);
    }
    setState(() {
      _probing = true;
      _notice = null;
      _verifiedChoice = null;
    });
    try {
      await session.probeSyncServer(choice);
      if (!mounted) return;
      setState(() => _verifiedChoice = choice);
    } on WalletApiError catch (e) {
      if (!mounted) return;
      // The user typed this URL into the field above — "check the address" is
      // a step they can take.
      setState(
        () => _notice = syncServerRefusalCopy(l10n, e, typedAddress: true),
      );
    } catch (_) {
      if (!mounted) return;
      setState(() => _notice = l10n.walletSyncServerUnreachable);
    } finally {
      if (mounted) setState(() => _probing = false);
    }
  }

  /// The switch, behind the in-flight notice (before ANY switch — a dialog
  /// the user confirms; cancelling changes nothing). The notice's body names
  /// the state the switch interrupts (`_switchNoticeBody`). A CUSTOM server's
  /// trust notice was already accepted at its first probe (`_probeCustom`),
  /// which is the step that first reaches the server; a verified custom URL
  /// cannot arrive here without it.
  Future<void> _switchTo(
    WalletLocalizations l10n,
    SyncServerChoice choice,
    SyncStatus? syncStatus,
  ) async {
    if (!await _confirm(
      l10n,
      title: l10n.walletSyncServerSheetTitle,
      body: _switchNoticeBody(l10n, syncStatus),
      action: l10n.walletSyncServerContinue,
    )) {
      return;
    }
    if (!mounted) return;
    setState(() {
      _switching = true;
      _notice = null;
    });
    final result = await ref
        .read(onboardingControllerProvider.notifier)
        .switchSyncServer(choice);
    if (!mounted) return;
    switch (result) {
      case SwitchServerSuccess():
        // Reset BEFORE the pop: a host that embeds the sheet at a root route
        // (a widget test, a settings pane) keeps it on screen, and a spinner
        // left running over a finished switch would be a lie. The key leaves
        // the field: the wallet holds it now (ADR-0568) — and the platform's
        // autofill context closes WITHOUT offering to save it.
        _key.clear();
        TextInput.finishAutofillContext(shouldSave: false);
        setState(() {
          _switching = false;
          _verifiedChoice = null;
        });
        Navigator.of(context).maybePop();
        return;
      case SwitchServerRefused(:final error):
        setState(() {
          _switching = false;
          _notice = syncServerRefusalCopy(
            l10n,
            error,
            typedAddress: addressTypedByUser(choice),
          );
        });
      case SwitchServerFailedRecovered():
        // The session was re-opened; the providers re-read the server in use.
        final status = await ref.read(walletSyncServerStatusProvider.future);
        if (!mounted) return;
        setState(() {
          _switching = false;
          _notice = l10n.walletSyncServerSwitchFailedRecovered(
            hostOf(status?.effectiveUrl ?? ''),
          );
        });
      case SwitchServerNotActive():
        setState(() => _switching = false);
      case SwitchServerFailedClosed():
        // The failed surface has taken over; nothing to show here.
        Navigator.of(context).maybePop();
    }
  }

  Future<bool> _confirm(
    WalletLocalizations l10n, {
    required String title,
    required String body,
    required String action,
  }) {
    // The trust notice opens over the focused URL field: the helper unfocuses
    // first, so the keyboard does not sit behind the dialog or come back
    // after Cancel.
    return showWalletConfirm(
      context,
      title: title,
      body: body,
      cancelLabel: l10n.walletSyncServerCancel,
      confirmLabel: action,
      kind: WalletConfirmKind.forward,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final servers = ref.watch(walletSyncServersProvider).value ?? const [];
    final status = ref.watch(walletSyncServerStatusProvider).value;
    // Watched HERE so the value is live when a row is tapped (the callbacks
    // close over it; `ref.watch` belongs in build, never in a callback) — the
    // in-flight notice names the state the switch interrupts.
    final syncStatus = ref.watch(syncStatusProvider).value;
    final effective = status?.effectiveUrl;
    final defaultUrl = status?.defaultUrl;
    final defaultOffered =
        defaultUrl != null && servers.any((s) => s.url == defaultUrl);
    final busy = _probing || _switching;
    // ADR-0568: the server in use is a custom one the user gave a key for.
    // The status names the header only — the key itself never leaves Rust.
    final keySaved = switch (status?.choice) {
      SyncServerChoice_Custom(:final key) => key != null,
      _ => false,
    };

    Widget row({
      required String label,
      required String host,
      required bool inUse,
      required VoidCallback? onTap,
      Key? key,
    }) {
      return Semantics(
        button: true,
        selected: inUse,
        child: InkWell(
          key: key,
          onTap: onTap,
          // A row of the server list: the ripple reads the group radius.
          borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 44),
            child: Row(
              children: [
                WalletIcon(
                  inUse ? WalletGlyph.selected : WalletGlyph.unselected,
                  size: 20,
                  color: inUse ? colors.green : colors.textMuted,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        label,
                        style: textTheme.bodyMedium?.copyWith(
                          color: colors.text,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      Text(
                        host,
                        style: textTheme.bodySmall?.copyWith(
                          color: colors.textMuted,
                        ),
                      ),
                    ],
                  ),
                ),
                if (inUse)
                  Text(
                    l10n.walletSyncServerInUse,
                    style: textTheme.labelMedium?.copyWith(color: colors.green),
                  ),
              ],
            ),
          ),
        ),
      );
    }

    return SafeArea(
      child: SingleChildScrollView(
        // Above the keyboard, as the other wallet sheets with a field are
        // (S15 iPhone walk: the custom-server fields sat under it).
        padding: EdgeInsets.fromLTRB(
          20,
          16,
          20,
          24 + MediaQuery.viewInsetsOf(context).bottom,
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            WalletSheetHeader(
              title: l10n.walletSyncServerSheetTitle,
              closeKey: const Key('sync-server-close'),
            ),
            if (status?.fallback != null && effective != null) ...[
              const SizedBox(height: 8),
              Text(
                syncServerFallbackCopy(
                  l10n,
                  status!.fallback!,
                  host: hostOf(effective),
                ),
                style: textTheme.bodyMedium?.copyWith(color: colors.orange),
              ),
            ],
            const SizedBox(height: 12),
            for (final s in servers) ...[
              row(
                key: ValueKey('sync-server-${s.id}'),
                label: s.label,
                host: hostOf(s.url),
                inUse: s.url == effective,
                onTap: busy || s.url == effective
                    ? null
                    : () => _switchTo(
                        l10n,
                        SyncServerChoice.predefined(id: s.id),
                        syncStatus,
                      ),
              ),
              const SizedBox(height: 4),
            ],
            if (defaultUrl != null && !defaultOffered) ...[
              row(
                key: const ValueKey('sync-server-default'),
                label: l10n.walletSyncServerAppDefault,
                host: hostOf(defaultUrl),
                inUse: defaultUrl == effective,
                onTap: busy || defaultUrl == effective
                    ? null
                    : () => _switchTo(
                        l10n,
                        const SyncServerChoice.default_(),
                        syncStatus,
                      ),
              ),
              const SizedBox(height: 4),
            ],
            const SizedBox(height: 8),
            Semantics(
              button: true,
              child: InkWell(
                key: const ValueKey('sync-server-custom-toggle'),
                onTap: busy
                    ? null
                    : () => setState(() => _customOpen = !_customOpen),
                borderRadius: BorderRadius.circular(
                  WalletShapes.of(context).group,
                ),
                child: ConstrainedBox(
                  constraints: const BoxConstraints(minHeight: 44),
                  child: Row(
                    children: [
                      WalletIcon(
                        _customOpen ? WalletGlyph.collapse : WalletGlyph.expand,
                        size: 20,
                        color: colors.textMuted,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          l10n.walletSyncServerCustom,
                          style: textTheme.bodyMedium?.copyWith(
                            color: colors.text,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            if (keySaved && effective != null) ...[
              Text(
                '${hostOf(effective)} · ${l10n.walletSyncServerKeySaved}',
                key: const ValueKey('sync-server-key-saved'),
                style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
              ),
              const SizedBox(height: 4),
            ],
            // ADR-0568 (security review, the built diff): the URL beside an
            // obscured field reads to iOS as a login form and offers to save
            // the key to the password manager. One autofill group, cancelled
            // on dispose and on a switch, and no autofill hint on any field.
            // Since S15 the key is shown by default, so a secure field exists
            // only after the user taps Hide; the group still covers that path.
            if (_customOpen)
              AutofillGroup(
                onDisposeAction: AutofillContextAction.cancel,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const SizedBox(height: 8),
                    TextField(
                      key: const ValueKey('sync-server-custom-url'),
                      controller: _custom,
                      enabled: !busy,
                      keyboardType: TextInputType.url,
                      autocorrect: false,
                      autofillHints: null,
                      onChanged: (_) => _unverify(),
                      decoration: InputDecoration(
                        hintText: l10n.walletSyncServerCustomHint,
                      ),
                    ),
                    const SizedBox(height: 8),
                    // ADR-0568: the user's own server may need a key. Whichever way
                    // the toggle is, the field takes no suggestions, no autocorrect,
                    // no keyboard learning and no autofill (security review L6).
                    TextField(
                      key: const ValueKey('sync-server-key'),
                      controller: _key,
                      enabled: !busy,
                      obscureText: !_showKey,
                      enableSuggestions: false,
                      autocorrect: false,
                      enableIMEPersonalizedLearning: false,
                      autofillHints: null,
                      keyboardType: TextInputType.visiblePassword,
                      // Paste only (S15 security review): a shown field would
                      // otherwise offer Copy and Cut, and a key on the system
                      // clipboard outlives the sheet (and syncs to the user's
                      // other devices on iOS).
                      contextMenuBuilder: (context, state) =>
                          AdaptiveTextSelectionToolbar.buttonItems(
                            anchors: state.contextMenuAnchors,
                            buttonItems: [
                              for (final item in state.contextMenuButtonItems)
                                if (item.type == ContextMenuButtonType.paste)
                                  item,
                            ],
                          ),
                      onChanged: (_) => _unverify(),
                      decoration: InputDecoration(
                        labelText: l10n.walletSyncServerKeyLabel,
                        suffixIcon: TextButton(
                          key: const ValueKey('sync-server-key-toggle'),
                          onPressed: () => setState(() => _showKey = !_showKey),
                          child: Text(
                            _showKey
                                ? l10n.walletSyncServerKeyHide
                                : l10n.walletSyncServerKeyShow,
                          ),
                        ),
                      ),
                    ),
                    if (_key.text.isNotEmpty) ...[
                      const SizedBox(height: 8),
                      TextField(
                        key: const ValueKey('sync-server-key-header'),
                        controller: _keyHeader,
                        enabled: !busy,
                        enableSuggestions: false,
                        autocorrect: false,
                        enableIMEPersonalizedLearning: false,
                        autofillHints: null,
                        onChanged: (_) => _unverify(),
                        decoration: InputDecoration(
                          labelText: l10n.walletSyncServerKeyHeaderLabel,
                        ),
                      ),
                    ],
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        OutlinedButton(
                          key: const ValueKey('sync-server-check'),
                          onPressed: busy || _custom.text.trim().isEmpty
                              ? null
                              : () => _probeCustom(l10n),
                          child: Text(
                            _probing
                                ? l10n.walletSyncServerChecking
                                : l10n.walletSyncServerCheck,
                          ),
                        ),
                        const SizedBox(width: 8),
                        if (_verifiedChoice != null)
                          FilledButton(
                            key: const ValueKey('sync-server-use'),
                            onPressed: busy
                                ? null
                                : () => _switchTo(
                                    l10n,
                                    _verifiedChoice!,
                                    syncStatus,
                                  ),
                            child: Text(l10n.walletSyncServerUse),
                          ),
                      ],
                    ),
                  ],
                ),
              ),
            if (_switching) ...[
              const SizedBox(height: 12),
              Row(
                children: [
                  const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    l10n.walletSyncServerSwitching,
                    style: textTheme.bodyMedium?.copyWith(
                      color: colors.textMuted,
                    ),
                  ),
                ],
              ),
            ],
            if (_notice != null) ...[
              const SizedBox(height: 12),
              Text(
                _notice!,
                key: const ValueKey('sync-server-notice'),
                style: textTheme.bodyMedium?.copyWith(color: colors.orange),
              ),
            ],
            const SizedBox(height: 16),
            Align(
              alignment: AlignmentDirectional.centerEnd,
              child: TextButton(
                onPressed: () => Navigator.of(context).maybePop(),
                child: Text(l10n.walletSyncSheetClose),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
