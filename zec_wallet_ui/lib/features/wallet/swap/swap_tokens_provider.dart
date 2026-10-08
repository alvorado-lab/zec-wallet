import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../wallet_providers.dart';

/// The D5 token list (spec §3.3b D5/L6) — the dynamic `/v0/tokens` picker source,
/// shared by the IntoZec SOURCE picker and the OutOfZec TARGET picker (symmetric
/// since an earlier revision; both open `showSwapTokenPicker`, which watches this provider).
///
/// LAZY by construction: an `autoDispose` `FutureProvider` is fetched only while
/// a picker sheet watches it (the IntoZec form is the default landing, so the
/// fetch fires when the user opens swap and taps the asset field — NOT on app
/// launch, §5.2 honest-off: an idle/uninterested user emits no token traffic). It
/// disposes when the sheet closes and re-fetches on the next open, which matches
/// the SDK contract that `list_tokens` always re-fetches online so the picker is
/// current (IZ-2).
///
/// SERVE-STALE (L6) is RUST-side: on a network fault the SDK returns the last good
/// list with [SwapTokenList.fresh] = false (a fast, successful return — the
/// picker renders a "couldn't refresh, showing cached" banner). So a Dart `error`
/// here is NOT a transient network blip — it is a first-ever fault with no cache
/// (a typed `SwapApiError`); the screen surfaces it with a retry
/// (`ref.invalidate(swapTokensProvider)`), never an infinite spinner.
///
/// The `.timeout` is the SAME honest-degradation guard the receive-address
/// providers carry: the SDK already bounds the NETWORK fetch (its dial timeout,
/// serving stale within it), so this generous bound only catches a WEDGED FFI
/// boundary (blocking-pool starvation) — it never false-fires on a slow-but-
/// working fetch (which returns stale quickly).
///
/// RETRY PINNED OFF: the "never an infinite spinner" contract
/// above was undermined by riverpod's container default, which silently
/// swallowed the error — up to ten 30 s wedge cycles, each discarding the
/// in-flight call — before the picker's error-with-retry could render. The
/// SDK already retries/serve-stales transport faults internally, so a Dart
/// error here is precisely the case the picker's own retry affordance exists
/// for ([walletNoSilentRetry]).
final swapTokensProvider = FutureProvider.autoDispose<SwapTokenList>((ref) {
  final session = ref.watch(walletSessionProvider);
  if (session == null) {
    // Defensive: the IntoZec form is gated to a live, swap-enabled wallet, so a
    // null session here is a mid-screen close — surface the honest unavailable
    // state (an error the picker renders), never a silent empty list.
    return Future<SwapTokenList>.error(
      StateError('swapTokensProvider read with no wallet session'),
    );
  }
  return session.swapListTokens().timeout(swapTokensTimeout);
}, retry: walletNoSilentRetry);

/// The token-fetch FFI-wedge bound (no-magic-numbers, gate 7). Generous: well
/// past any real `/v0/tokens` round-trip (the SDK serves stale within its own
/// dial timeout on a network hang), so it only fires on a wedged FFI boundary.
const swapTokensTimeout = Duration(seconds: 30);
