import 'package:flutter/material.dart';

/// Every glyph the wallet UI draws, named for what it MEANS where it is drawn
/// (FR-49 W-5, ADR-0564). Code that decides WHICH icon to show — widgets and
/// the pure presentation functions alike — holds a [WalletGlyph]; only
/// [WalletIcon] turns one into pixels, through the host's [WalletIcons]
/// builder or the SDK's Material default.
///
/// One value per glyph, not per call site: where two sites draw the same
/// thing they share a value. The maintainer's ruling C keeps the
/// designer's artwork host-side; the SDK ships only Material defaults.
enum WalletGlyph {
  // ── Money movement ──
  /// The Send action.
  send(Icons.arrow_upward),

  /// The Receive action.
  receive(Icons.arrow_downward),

  /// The Swap action, and a swap in progress.
  swap(Icons.swap_horiz),

  /// An incoming transaction (activity row, tx detail), and "into ZEC".
  incoming(Icons.south_west),

  /// An outgoing transaction, and "from ZEC".
  outgoing(Icons.north_east),

  /// A send that is on its way (broadcast, not yet mined).
  inMotion(Icons.outbound),

  /// A transaction being re-broadcast.
  rebroadcast(Icons.replay_circle_filled_outlined),

  /// A send kept by the wallet (a transaction exists; no retry promised).
  saved(Icons.save_outlined),

  /// A send saved for a later retry.
  savedForRetry(Icons.schedule_outlined),

  /// Something scheduled, or a queued (parked) send.
  scheduled(Icons.schedule),

  /// A parked send that is sending now.
  sending(Icons.hourglass_top_outlined),

  /// A paused parked send.
  paused(Icons.pause_circle_outlined),

  /// A transaction carries a memo.
  memo(Icons.mail_outline),

  /// A payment label or note on the send form.
  label(Icons.label_outline),

  /// The wallet itself (the empty state, the balance line).
  wallet(Icons.account_balance_wallet_outlined),

  // ── Pools and privacy ──
  /// Shielded funds, or a transport that is not fully protected.
  shielded(Icons.shield_outlined),

  /// A fully protected transport (the filled shield).
  protected(Icons.shield),

  /// Transparent (public) funds and actions that expose them.
  transparent(Icons.public),

  /// A transparent receiving address.
  transparentAddress(Icons.public_outlined),

  /// The public (transparent) funds entry in the wallet's menu — the sheet
  /// that holds the auto-shield and Move-to-public controls.
  transparentFunds(Icons.public),

  /// A verified erase or check.
  verified(Icons.verified_user),

  /// A privacy caveat.
  privacyNotice(Icons.privacy_tip_outlined),

  /// Show something secret (the viewing key, the phrase). This meaning only:
  /// watch-only, the balance toggle and public funds have their own values.
  reveal(Icons.visibility_outlined),

  /// Something hidden or out of view; the balance toggle while the balance
  /// shows (a press hides it).
  hidden(Icons.visibility_off_outlined),

  /// The balance toggle while the balance is hidden (a press shows it).
  showBalance(Icons.visibility_outlined),

  /// A watch-only wallet: its badge, the watch-only entry in onboarding, and
  /// Send's watch-only state.
  watchOnly(Icons.preview_outlined),

  /// Locked, or blocked from screenshots.
  locked(Icons.lock_outline),

  /// Unlock (release a held send).
  unlock(Icons.lock_open_outlined),

  /// The recovery phrase.
  recoveryPhrase(Icons.vpn_key_outlined),

  // ── Sync and network ──
  /// Syncing.
  syncing(Icons.sync),

  /// A sync problem.
  syncProblem(Icons.sync_problem),

  /// Sync switched off by the host.
  syncOff(Icons.sync_disabled),

  /// Sync not running yet.
  syncIdle(Icons.pause_circle_outline),

  /// Connecting to the network.
  connecting(Icons.wifi_tethering),

  /// Offline: the wallet cannot reach the network (a sync state). A send
  /// saved for later is [savedForRetry], not this.
  offline(Icons.cloud_off),

  /// A service (the swap provider, the server) cannot be reached.
  serviceUnreachable(Icons.cloud_off_outlined),

  /// The server answers, degraded.
  serverDegraded(Icons.dns_outlined),

  /// Up to date, with a limit.
  upToDateLimited(Icons.update),

  /// Behind the chain tip.
  behind(Icons.history),

  /// Rescan.
  rescan(Icons.manage_history),

  /// Sweep or deep scan.
  sweep(Icons.manage_search),

  /// Restore.
  restore(Icons.restore),

  /// Recover (by restore, or one-time-address funds).
  recover(Icons.restart_alt),

  /// Device storage is full.
  storageFull(Icons.disc_full),

  // ── Swap tracking ──
  /// A swap waiting for its deposit.
  awaitingDeposit(Icons.upload_outlined),

  /// A swap whose deposit was detected.
  depositDetected(Icons.call_received),

  /// A refunded swap.
  refunded(Icons.undo),

  /// A swap the provider cannot find.
  notFound(Icons.search_off),

  /// A swap near its deadline.
  expiring(Icons.hourglass_bottom),

  /// A quote's countdown.
  timer(Icons.timer_outlined),

  /// An expired quote.
  timerExpired(Icons.timer_off_outlined),

  // ── Status ──
  /// Success.
  success(Icons.check_circle_outline),

  /// An error or failure.
  error(Icons.error_outline),

  /// A warning.
  warning(Icons.warning_amber_rounded),

  /// Information.
  info(Icons.info_outline),

  /// A state that cannot be verified.
  unknown(Icons.help_outline),

  /// Waiting.
  waiting(Icons.hourglass_top),

  // ── Actions and controls ──
  /// Add or create.
  add(Icons.add),

  /// Copy.
  copy(Icons.copy),

  /// Paste from the clipboard.
  paste(Icons.content_paste),

  /// Share through the host's share sheet.
  share(Icons.share_outlined),

  /// Retry or refresh.
  retry(Icons.refresh),

  /// Close or dismiss.
  close(Icons.close),

  /// Search.
  search(Icons.search),

  /// Scan a QR code.
  scanQr(Icons.qr_code_scanner),

  /// Type it in instead.
  manualEntry(Icons.keyboard),

  /// The camera is unavailable.
  cameraUnavailable(Icons.videocam_off_outlined),

  /// Pick a date.
  pickDate(Icons.event_outlined),

  /// Delete the wallet.
  deleteWallet(Icons.delete_forever_outlined),

  /// The host's settings page (the wallet menu's "Settings" entry; the name
  /// is the seam's historical one — S15 broadened it from theme-only, and a
  /// palette beside "Settings" read as a theme control).
  appearance(Icons.settings_outlined),

  /// A row's forward chevron.
  chevronForward(Icons.chevron_right),

  /// Expand a section.
  expand(Icons.expand_more),

  /// Collapse a section.
  collapse(Icons.expand_less),

  /// A dropdown's arrow.
  dropdown(Icons.arrow_drop_down),

  /// A selected choice.
  selected(Icons.radio_button_checked),

  /// An unselected choice.
  unselected(Icons.radio_button_off);

  const WalletGlyph(this.material);

  /// The SDK's default rendering: the Material glyph.
  final IconData material;
}

/// A host's renderer for one glyph. Return null for a glyph the host does not
/// map, and the SDK draws its Material default. When [size] or [color] is
/// null the widget returned must take it from the ambient `IconTheme`, as
/// `Icon` does. A non-null `semanticLabel` must be announced (the SDK passes
/// one only where the glyph carries meaning no text beside it does); with
/// null, the glyph must stay silent, as a label-less `Icon` is.
typedef WalletIconBuilder =
    Widget? Function(
      BuildContext context,
      WalletGlyph glyph, {
      double? size,
      Color? color,
      String? semanticLabel,
    });

/// The host's icon hook (FR-49 W-5): one function from a [WalletGlyph] to a
/// widget, so a host's glyphs need not be `IconData` (Relim's are painted from
/// SVG paths). Registered in `ThemeData.extensions`; optional.
@immutable
class WalletIcons extends ThemeExtension<WalletIcons> {
  const WalletIcons({this.builder});

  /// The host's renderer, or null for Material everywhere.
  final WalletIconBuilder? builder;

  /// The SDK's own default: Material glyphs.
  static const material = WalletIcons();

  /// The registered hook, or [material] when the host registered none. A
  /// missing icon hook is not a wiring error (unlike missing colours): the
  /// SDK's glyphs are a complete, working default.
  static WalletIcons of(BuildContext context) =>
      Theme.of(context).extension<WalletIcons>() ?? material;

  /// [glyph] drawn by the host's [builder], or as its Material default when
  /// the host has none or does not map it.
  Widget draw(
    BuildContext context,
    WalletGlyph glyph, {
    double? size,
    Color? color,
    String? semanticLabel,
  }) =>
      builder?.call(
        context,
        glyph,
        size: size,
        color: color,
        semanticLabel: semanticLabel,
      ) ??
      Icon(
        glyph.material,
        size: size,
        color: color,
        semanticLabel: semanticLabel,
      );

  @override
  WalletIcons copyWith({WalletIconBuilder? builder}) =>
      WalletIcons(builder: builder ?? this.builder);

  /// A renderer does not interpolate: a theme switch takes the incoming one at
  /// the midpoint.
  @override
  WalletIcons lerp(WalletIcons? other, double t) =>
      other == null || t < 0.5 ? this : other;
}

/// One wallet glyph, drawn through the host's [WalletIcons] hook. The ONLY
/// way the wallet UI puts an icon on screen; a source test keeps it so.
class WalletIcon extends StatelessWidget {
  const WalletIcon(
    this.glyph, {
    super.key,
    this.size,
    this.color,
    this.semanticLabel,
  });

  final WalletGlyph glyph;

  /// Null takes the ambient `IconTheme` size.
  final double? size;

  /// Null takes the ambient `IconTheme` colour.
  final Color? color;

  final String? semanticLabel;

  @override
  Widget build(BuildContext context) => WalletIcons.of(context).draw(
    context,
    glyph,
    size: size,
    color: color,
    semanticLabel: semanticLabel,
  );
}
