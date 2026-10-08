/// zec_wallet_ui — the reusable wallet UI layer over the zec_wallet SDK.
///
/// The whole wallet (onboarding, balance, send and receive, history,
/// shielding, swap, recovery) as screens a host mounts under its own
/// `GoRouter` with `walletRoutes()`, wired by one `ProviderScope` override
/// list (`walletOnboardingOverrides`). The README is the integration guide;
/// `zec_wallet/example` is the reference consumer. Import only this
/// barrel: `package:zec_wallet_ui/zec_wallet_ui.dart`.
library;

// Phase 2 — the wallet strings under their own delegate: hosts compose
// `walletLocalizationsFallbackDelegate` into `localizationsDelegates` next to
// their own generated class (distinct types — no collision). The fallback
// wrapper — NOT the raw generated delegate — is the host contract: it serves
// English for locales the wallet ARB set doesn't cover instead of letting
// `Localizations` skip the delegate and crash the first wallet frame
// (review B2).
export 'l10n/wallet_localizations.dart';
export 'l10n/wallet_localizations_fallback.dart';

// Phase 1 — theme tokens, lifecycle seam, shared widgets.
export 'core/lifecycle/app_lifecycle_provider.dart';
export 'core/theme/colors.dart';
export 'core/theme/icons.dart';
export 'core/theme/shapes.dart';
export 'core/theme/sheet_layout.dart';
export 'core/theme/theme.dart';
export 'core/theme/typography.dart';
export 'shared/address_text.dart';
export 'shared/qr_tile.dart';
export 'shared/settings_section_header.dart';
export 'shared/theme/avatar_color.dart';
// FR-49 W-4 (S11 C2): the notice, in its line and card forms.
export 'shared/wallet_notice.dart';
// S13 §1.1: the (i) after a label, and the call-to-action height floor.
export 'shared/wallet_info_button.dart';
export 'shared/wallet_cta.dart';

// Phase 3 — the feature tree. The navigation contract: a host mounts
// walletRoutes() under its own router and navigates via the WalletRoutes
// constants; the appearance seam (walletAppearanceRoutePathProvider) is the
// one wallet→host navigation, hidden unless wired. The composition seams
// (walletOnboardingOverrides / walletSwapOverrides / swap config) are the
// "page of glue" a consumer wires at its ProviderScope root.
export 'core/router/wallet_router.dart';
export 'core/router/wallet_routes.dart';
export 'features/settings/backup_screen.dart';
export 'features/settings/security_screen.dart';
// FR-49 W-7 — hide balance: a host persists the choice by overriding the
// provider, and masks its own amount surfaces by reading it.
export 'features/wallet/hide_balance.dart'
    show
        walletBalanceHiddenProvider,
        WalletBalanceHidden,
        maskedAmountText,
        displayedAmount;
export 'features/wallet/receive_screen.dart';
export 'features/wallet/send/send_screen.dart';
// FR-25 — the prefilled-send entry seam: the public request type (+ its
// ZIP-321 `fromUri` door and typed rejection) and the `WalletSendEntry.push`
// helper a host calls from its own QR-scan / deep-link / pay-a-contact entry.
// Show-scoped: the two adapter-side mapper helpers (`walletSendRequestFromParsedLegs`
// / `walletSendRequestFaultFromApiError`) stay OFF the public API — they run the
// seam policy AFTER the bridge's network check, so a host calling them directly
// would skip validation; the adapter + tests import the file directly.
export 'features/wallet/send/wallet_send_entry.dart';
// FR-26 — what the entry reports back about the flow it opened. The whole
// family is exported; the delivery channel and the route envelope live under
// `lib/src/` instead, because a `show` list is documentation and
// `implementation_imports` is enforcement — everything under `lib/features/` is
// deep-importable, so scoping alone would have been a claim we could not keep.
//
// ── THE `lib/src/` RULE (debt; written down) ──────────────────────
// Until now one file lived there and every other internal lived under
// `lib/features/`, with no stated criterion — so the next contributor adding a
// forgeable DTO had nothing to reason from. The test that was actually applied,
// and the one to keep applying:
//
//   A type belongs under `lib/src/` if and only if a host CONSTRUCTING IT
//   DIRECTLY could fabricate a claim the SDK never made.
//
// Not "is it internal" — most of `lib/features/` is internal and stays there,
// reachable by a deep import, and that is fine. The bar is forgery: the channel
// and the route envelope qualify because a host could hand ITSELF a payment
// record the wallet never produced. A widget, a controller, or a mapper cannot
// forge anything by being constructed, so moving them would buy nothing and
// cost every consumer a real import.
//
// Corollary, and the half a rule can actually enforce: nothing under `lib/src/`
// may be re-exported from this barrel — an export would defeat the
// `implementation_imports` lint that is the whole mechanism. Pinned by
// `barrel_never_exports_lib_src` in `test/api_surface_test.dart`.
export 'features/wallet/send/wallet_send_report.dart';
export 'features/wallet/send/wallet_send_request.dart'
    show
        WalletSendRequest,
        WalletSendRequestException,
        WalletSendRequestFault,
        // FR-28's one required public type. It was missing from this list on
        // the way in, so the feature could only be used by deep-importing the
        // file — the exact habit the delivery internals were moved under
        // `lib/src/` to discourage.
        WalletMachineMemo;
export 'features/wallet/swap/swap_config.dart';
export 'features/wallet/swap/swap_screen.dart';
export 'features/wallet/wallet_composition.dart';
// The REFERENCE wallet policy (endpoints, Tor-off default, the Sapling-floor
// date the pickers bound on) + `buildWalletConfig`: a host uses these verbatim
// as a starting default, or builds its own `WalletConfig` and ignores them.
// The seam (`walletOnboardingOverrides`) takes the config, so the reference
// policy is a convenience, never on the required path (3rd-party seam).
// NOTE this also surfaces the `WalletConfigBirthday` extension
// (`withBirthdayHeight`) — a host defining a same-named extension member on
// `WalletConfig` disambiguates via an explicit extension application.
export 'features/wallet/wallet_config.dart';
// The SDK types the wiring step NAMES (the README's
// bring-your-own-config sample must compile against the barrel alone).
// Show-scoped: only the config vocabulary — the full SDK surface stays behind
// `package:zec_wallet/zec_wallet.dart`, which a host imports anyway for
// `RustLib.init()`.
export 'package:zec_wallet/zec_wallet.dart'
    show
        WalletConfig,
        Network,
        TorPolicy,
        TorRuntimeConfig,
        SeedPersistence,
        JitterPolicy,
        // The swap-policy vocabulary (SwapHostPolicy's fields): a host wiring
        // swapHostPolicyProvider names these in the same breath (review
        // F5 — the wiring sample must compile against the barrel alone).
        SwapProviderConfig,
        SwapKill;
// iOS hosts that bring their OWN dbDir (instead of `resolveWalletDbDir`) must
// exclude it from iCloud/device backup themselves — the reference helper does
// it internally; this export keeps the primitive reachable for that path.
export 'features/wallet/backup_exclusion.dart';
// The caption clock is a test seam, not host API.
export 'features/wallet/wallet_screen.dart' hide balanceCaptionNow;

// The host seams themselves (review H3): the provider interfaces a host
// implements (WalletProvisioner / OnboardingStore / ScreenSecurity), the root
// providers it overrides (directly, or via walletOnboardingOverrides), and
// the session surface its own code may read. Without these the README's
// "wire your own adapters" path forced deep imports of package internals.
export 'features/wallet/onboarding/bip39_wordlist.dart';
export 'features/wallet/onboarding/onboarding_providers.dart';
export 'features/wallet/onboarding/onboarding_store.dart';
export 'features/wallet/onboarding/wallet_provisioner.dart';
// The honest boot-wiring failure surface (#356-F1): a host that MEANT to wire
// the wallet but whose startup work failed renders this (with a retry) instead
// of degrading to the unwired seams' "arrives in a later build" copy — which
// is false on a build that ships the wallet. The example's boot gate is the
// canonical wiring.
export 'features/wallet/onboarding/wallet_startup_failed_screen.dart';
// The send-authorization seam (#327): the WalletSendAuthorizer interface a
// per-send-credential host implements (+ its intent/denial vocabulary); the
// provider it overrides lives in wallet_providers.dart below.
export 'features/wallet/send_authorization.dart';
// The recovery-phrase reveal re-auth seam (#333): the WalletRevealAuthorizer a
// credential host overrides (walletRevealAuthorizerProvider) to gate the
// Settings → back-up reveal behind its own passphrase/biometric prompt. Default
// is passthrough (no step); a reveal is not a spend, so it is a separate seam.
export 'features/wallet/reveal_authorization.dart';
// The reconnect seam (#404): the NetworkReachability port + the provider a host
// overrides to feed the sync loop its OWN connectivity signal (many apps already
// run one, and a second platform listener is wasted battery). Unoverridden it
// speaks the `zec_wallet_ui/network_reachability` EventChannel, which the
// optional `zec_wallet_ui_platform` companion answers; with neither, the stream
// is empty, nothing happens, and nothing is reported to `FlutterError` either
// (#407 R2). What that costs depends on the platform (#407 R10d corrected the
// blanket "self-heals" claim that stood here): MOBILE keeps the real
// background/resume self-heal, so only foreground reconnects are slower;
// DESKTOP never fires `paused`, so it has no such fallback and the sync sheet's
// manual "Try now" is the only recovery there.
export 'features/wallet/reconnect_kick.dart';
// The transparent-funds policy seams (§3.2i-3): the expert gate + auto-shield
// switch (persisted; hosts may wire their own settings UI to the notifiers or
// swap the store), the threshold/power-save host inputs, the policy loop's
// status (for a host-rendered cue), and the sheet entry point.
export 'features/wallet/transparent_funds/auto_shield_controller.dart';
export 'features/wallet/transparent_funds/transparent_funds_providers.dart';
export 'features/wallet/transparent_funds/transparent_funds_sheet.dart';
export 'features/wallet/transparent_funds/wallet_settings_store.dart';
// The server picker's entry point. It is a MODAL, so `WalletRoutes` mounts no
// path for it and the sync sheet's Server row is the only door inside this
// package — which left a host with nothing but a deep import into
// `features/` to offer "change the server" from its own settings screen
// (Relim needed exactly that, the maintainer found no way to set the
// server from Relim's wallet settings). Exported so a host never has to reach
// past this barrel: a deep import is legal (`implementation_imports` bans
// only `lib/src/`) but it makes any file move here a silent host break.
export 'features/wallet/sync_server_sheet.dart' show showSyncServerSheet;
// FR-51 — the sync drive, so a host can run sync whenever the app is in the
// foreground (maintainer, via Relim "it should work anytime the app is
// active"), not only once the wallet screen has rendered. Show-scoped: the
// controller class, its internal name AND the drive-state enum stay off the
// API. A host only listens; it must not word its own copy per drive state
// (the cause-agnostic rule on walletSyncPassesRunProvider, #401/#403/#405).
export 'features/wallet/wallet_sync_controller.dart'
    show walletSyncDriveProvider;
export 'features/wallet/wallet_providers.dart';
// S13 §1a: the host's services (share, open settings), by provider override.
export 'features/wallet/wallet_ui_config.dart'
    show WalletUiConfig, walletUiConfigProvider;
// The host-transport claim TYPE (its provider lives in wallet_providers.dart,
// but a Dart export re-exports only DECLARATIONS — the provider file's
// `show WalletHostTransport` import does not surface the type; review
// H1). Show-scoped: the tone/presentation mapping stays internal.
export 'features/wallet/sync_status_presentation.dart' show WalletHostTransport;
export 'features/wallet/wallet_session.dart';
// The production WalletSession adapter over an FRB `WalletHandle` — for the
// SESSION-ONLY host configuration: a host that provisions its own handle
// (its own custody/onboarding; e.g. a host-supplied seed) wraps it in this
// and overrides `walletSessionProvider` with the result. Without this export
// that host would deep-import package internals or re-implement ~25
// forwarding methods.
export 'features/wallet/frb_wallet_session.dart';
