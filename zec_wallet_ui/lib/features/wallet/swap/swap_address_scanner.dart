import 'dart:async' show unawaited;
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_zxing/flutter_zxing.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../wallet_ui_config.dart';

/// Opens the camera QR reader and resolves to the decoded RAW payload, or
/// `null` if the user cancels / chooses manual entry / no camera is available.
typedef AddressScanner = Future<String?> Function(BuildContext context);

/// What the reader screen says it is scanning — resolves the title /
/// aiming-instruction / camera-unavailable copy. The reader itself is
/// purpose-blind; only the words change.
enum ScanPurpose {
  /// A foreign-chain address QR (the swap refund/destination fields).
  address,

  /// A `uview…` unified full viewing key QR (#397 P4 — the watch-only import;
  /// completes the in-app export→scan→import loop against the export screen's
  /// QR tile).
  viewingKey,
}

/// The scan affordance (the live camera reader). Injected so widget tests drive
/// the flow without a camera. Override in tests with a function returning a
/// canned QR payload (or `null` for cancel).
///
/// Shared, purpose-neutral brick: the IntoZec refund-address field, the
/// OutOfZec destination-address field (§3.3b D6/L8) AND the watch-only
/// viewing-key import (#397 §3.7 D5, via [viewingKeyScannerProvider]) drive
/// it. It carries ZERO chain or purpose knowledge; the caller picks the field
/// copy + normalizes the payload. (It predates the cross-feature reuse, which
/// is why it lives under swap/ — moving it is pure churn.)
final addressScannerProvider = Provider<AddressScanner>(
  (ref) => showAddressScanner,
);

/// The same brick wearing the viewing-key copy — a separate provider so widget
/// tests override the watch-only scan without touching the swap fields'
/// scanner (and vice versa).
final viewingKeyScannerProvider = Provider<AddressScanner>(
  (ref) =>
      (context) => showAddressScanner(context, purpose: ScanPurpose.viewingKey),
);

/// Whether to OFFER the live scan on this platform. The scan is ADDITIVE —
/// paste/type is always the canonical path (§3.3b L8) and the ONLY path on
/// desktop/web (flutter_zxing's camera backend is Android/iOS only). Gated as
/// both-natives, never one platform (flutter-patterns § Platform Capability
/// Gating). Overridable in tests (the test host is desktop → false by default).
final addressScannerSupportedProvider = Provider<bool>(
  (ref) => addressScannerSupported,
);

/// Test seam: open the reader in its camera-refused state. The host VM's
/// camera never reports an error (it stays loading), so without this the
/// refused body — and the host's Open settings hook on it — could not be
/// reached by a test.
@visibleForTesting
final addressScannerCameraRefusedProvider = Provider<bool>((ref) => false);

/// True iff a live camera QR scan is available on this platform.
bool get addressScannerSupported =>
    !kIsWeb && (Platform.isAndroid || Platform.isIOS);

/// Push the full-screen QR reader. The QR payload is opaque here and
/// §5.4-sensitive — NEVER logged. The caller normalizes it (URI-unwrap,
/// [normalizeScannedAddress] for addresses; case-fold for a viewing key) and
/// the SDK validates it Rust-side. The reader is fully on-device —
/// `flutter_zxing`/zxing-cpp, no Google ML Kit, no network (ADR-0531).
Future<String?> showAddressScanner(
  BuildContext context, {
  ScanPurpose purpose = ScanPurpose.address,
}) {
  return Navigator.of(context).push<String>(
    MaterialPageRoute<String>(
      fullscreenDialog: true,
      builder: (_) => _AddressScannerScreen(purpose: purpose),
    ),
  );
}

class _AddressScannerScreen extends ConsumerStatefulWidget {
  const _AddressScannerScreen({required this.purpose});

  final ScanPurpose purpose;

  @override
  ConsumerState<_AddressScannerScreen> createState() =>
      _AddressScannerScreenState();
}

class _AddressScannerScreenState extends ConsumerState<_AddressScannerScreen> {
  /// Set once on the first valid decode — the reader keeps streaming after a
  /// hit (scanDelaySuccess), so without this guard `onScan` would try to pop
  /// the route more than once.
  bool _handled = false;

  /// The camera failed to come up (permission denied / no camera / init
  /// error). We show an honest message; the manual-entry escape is always
  /// present regardless (honest degradation, §6).
  late bool _cameraFailed = ref.read(addressScannerCameraRefusedProvider);

  void _onScan(Code code) {
    final text = code.text;
    // `onScan` only fires on a valid decode; guard empties + double-fire.
    if (_handled || !mounted || text == null || text.isEmpty) return;
    _handled = true;
    Navigator.of(context).pop(text); // raw payload; NEVER logged (§5.4).
  }

  void _onControllerCreated(CameraController? controller, Exception? error) {
    if (error != null && mounted && !_cameraFailed) {
      setState(() => _cameraFailed = true);
    }
  }

  void _cancel() {
    if (mounted) Navigator.of(context).pop();
  }

  /// Purpose → copy. Total switches (no default) so a new [ScanPurpose] fails
  /// the build here, never ships with borrowed words.
  String _title(WalletLocalizations l10n) => switch (widget.purpose) {
    ScanPurpose.address => l10n.walletSwapScanTitle,
    ScanPurpose.viewingKey => l10n.walletWatchOnlyScanTitle,
  };

  String _instruction(WalletLocalizations l10n) => switch (widget.purpose) {
    ScanPurpose.address => l10n.walletSwapScanInstruction,
    ScanPurpose.viewingKey => l10n.walletWatchOnlyScanInstruction,
  };

  String _cameraUnavailable(WalletLocalizations l10n) =>
      switch (widget.purpose) {
        ScanPurpose.address => l10n.walletSwapScanCameraUnavailable,
        ScanPurpose.viewingKey => l10n.walletWatchOnlyScanCameraUnavailable,
      };

  // The always-present manual-entry escape label. Purpose-resolved (F2a):
  // the address manual path is "Enter manually" (type it), but the viewing-key
  // manual path is PASTE — and shared "Enter manually" contradicted the
  // camera-unavailable body's own "Paste the key manually" on one screen.
  String _manualEntry(WalletLocalizations l10n) => switch (widget.purpose) {
    ScanPurpose.address => l10n.walletSwapScanManualEntry,
    ScanPurpose.viewingKey => l10n.walletWatchOnlyScanManualEntry,
  };

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);

    return Scaffold(
      // The camera stage (S13 §1.5): dark in every theme — the preview is the
      // content — and read from the two stage roles, never a literal, so a
      // host can tune it. The bar sits on the same stage, so its title reads.
      backgroundColor: colors.stage,
      appBar: AppBar(
        backgroundColor: colors.stage,
        foregroundColor: colors.onStage,
        title: Text(_title(l10n)),
        // Close: a 44 glass circle at the top-leading corner (DESIGN §6.18).
        leading: Center(
          child: SizedBox.square(
            dimension: 44,
            child: IconButton(
              key: const Key('scanner-close'),
              padding: EdgeInsets.zero,
              style: IconButton.styleFrom(
                backgroundColor: colors.onStage.withValues(alpha: 0.16),
                foregroundColor: colors.onStage,
              ),
              icon: const WalletIcon(WalletGlyph.close),
              tooltip: l10n.walletSwapScanCancel,
              onPressed: _cancel,
            ),
          ),
        ),
      ),
      body: _cameraFailed
          ? _CameraUnavailable(
              text: _cameraUnavailable(l10n),
              onOpenSettings: ref.watch(walletUiConfigProvider).onOpenSettings,
            )
          : _scanner(l10n),
      // The manual-entry escape is ALWAYS present — the a11y path (a blind user
      // can't aim a camera) and the fallback when the camera never comes up.
      bottomNavigationBar: SafeArea(
        minimum: const EdgeInsets.all(16),
        child: SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            key: const Key('scanner-manual-entry'),
            // Colour only: the theme owns the size and shape. On the stage in
            // every theme (S13 §1.5: the page's text colour was dark on black
            // in the light theme).
            style: OutlinedButton.styleFrom(
              foregroundColor: colors.onStage,
              side: BorderSide(color: colors.onStage.withValues(alpha: 0.5)),
            ),
            icon: const WalletIcon(WalletGlyph.manualEntry),
            label: Text(_manualEntry(l10n)),
            onPressed: _cancel,
          ),
        ),
      ),
    );
  }

  Widget _scanner(WalletLocalizations l10n) {
    final colors = WalletColors.of(context);
    return Stack(
      children: [
        // QR-only reader. Gallery OFF (keeps the permission surface to CAMERA
        // alone — no photo-library prompt) and camera-toggle OFF (an address
        // QR is always the rear camera). Flashlight ON for low light (explicit,
        // matching the ZODL torch affordance — not relying on the default).
        ReaderWidget(
          codeFormat: Format.qrCode,
          showGallery: false,
          showToggleCamera: false,
          showFlashlight: true,
          // #397 device-tail finding (real-QR aim): the package
          // defaults — a 720p analysis stream, a center-50% crop, and the
          // quick decode pass — latch address-sized QRs (~v8: ~7 px/module
          // in the crop) but could NOT decode a 510-char UFVK QR (~v23,
          // 113 modules ≈ 2–3 px/module) at ANY aim distance — the exact
          // payload ScanPurpose.viewingKey exists for. 1080p + an 80% crop
          // put the dense case back at ~7 px/module, and tryHarder engages
          // zxing's thorough pass (the added per-frame latency is fine on a
          // deliberate scan screen). Applies to BOTH purposes: address QRs
          // only get more headroom.
          resolution: ResolutionPreset.veryHigh,
          cropPercent: 0.8,
          tryHarder: true,
          onScan: _onScan,
          onControllerCreated: _onControllerCreated,
          loading: ColoredBox(color: colors.stage),
        ),
        // Sighted-user aiming hint; the whole screen's intent is also carried
        // by the AppBar title + the manual-entry button for screen readers.
        // NO Semantics label wrapper (a11y review): the Text announces
        // itself, and a duplicate label on the ancestor merged into one node
        // that read the instruction TWICE.
        Align(
          alignment: Alignment.topCenter,
          child: SafeArea(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: DecoratedBox(
                // A pill (S13 §1.5): fully rounded, radius half its height.
                decoration: ShapeDecoration(
                  color: colors.stage.withValues(alpha: 0.55),
                  shape: const StadiumBorder(),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 12,
                  ),
                  child: Text(
                    _instruction(l10n),
                    textAlign: TextAlign.center,
                    style: TextStyle(color: colors.onStage),
                  ),
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

/// Honest "camera unavailable" body (permission denied / no camera). Not a dead
/// black screen — it names the situation and points at the always-present
/// manual-entry button below. Copy is purpose-resolved by the caller.
///
/// S13 §1.5 — where the camera was refused, "Open settings" through the
/// host's OPTIONAL hook ([WalletUiConfig.onOpenSettings]); with no hook the
/// button is not drawn (DESIGN §3.2), and the SDK bundles no plugin for it.
class _CameraUnavailable extends StatefulWidget {
  const _CameraUnavailable({required this.text, this.onOpenSettings});

  final String text;
  final Future<void> Function()? onOpenSettings;

  @override
  State<_CameraUnavailable> createState() => _CameraUnavailableState();
}

class _CameraUnavailableState extends State<_CameraUnavailable> {
  /// The host's hook threw: said inline, the button stays for a retry.
  bool _openFailed = false;

  Future<void> _openSettings(Future<void> Function() hook) async {
    final ok = await callWalletHostHook(hook);
    if (!mounted) return;
    setState(() => _openFailed = !ok);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final hook = widget.onOpenSettings;
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(
              WalletGlyph.cameraUnavailable,
              size: 48,
              color: colors.onStage.withValues(alpha: 0.7),
            ),
            const SizedBox(height: 16),
            Text(
              widget.text,
              textAlign: TextAlign.center,
              style: TextStyle(color: colors.onStage),
            ),
            if (hook != null) ...[
              const SizedBox(height: 16),
              OutlinedButton(
                key: const Key('scanner-open-settings'),
                style: OutlinedButton.styleFrom(
                  foregroundColor: colors.onStage,
                  side: BorderSide(
                    color: colors.onStage.withValues(alpha: 0.5),
                  ),
                ),
                onPressed: () => unawaited(_openSettings(hook)),
                child: Text(l10n.walletScanOpenSettings),
              ),
              if (_openFailed) ...[
                const SizedBox(height: 8),
                Text(
                  l10n.walletScanOpenSettingsFailed,
                  key: const Key('scanner-open-settings-failed'),
                  textAlign: TextAlign.center,
                  style: TextStyle(color: colors.onStage),
                ),
              ],
            ],
          ],
        ),
      ),
    );
  }
}
