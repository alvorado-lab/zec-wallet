/// ONE source of truth for "the trailing element can no longer share this row"
/// (duplicates are bugs). Four sites carried their own copy of this
/// rule before #409 R3 — two of them written as `scale(100) > 140`, which
/// probes the WRONG font size (see [walletTextScaleForcesStack]).
///
/// The rule is deliberately layout-only: it decides GEOMETRY, never what is
/// shown. Nothing money-bearing is ever hidden by a stack — the point is the
/// opposite, that the copy and the figures keep the full row width when the
/// trailing widget would otherwise starve them.
///
/// NOT exported from `zec_wallet_ui.dart`, unlike the rest of `lib/shared/`.
/// The other files there are widgets a host composes with; this is internal
/// layout policy whose constants are measured against THIS package's own
/// surfaces, and publishing it would freeze them as API. (A host can still
/// import the path directly — the package has no `lib/src/` — so treat that
/// as unsupported rather than prevented.)
library;

import 'package:flutter/material.dart';

/// Past this multiple of the body text size, a low-vision user's chosen size
/// must WIN the row: the trailing action moves onto its own line rather than
/// squeezing the text it sits beside.
///
/// Measured origin (parked rows, arch review M-A1 swap rows, #409 R3
/// notices): above ~1.4x a trailing button's intrinsic width exceeds what the
/// shared row can give it, and the flexible text collapses toward zero.
const double kWalletStackTextScale = 1.4;

/// Below this row width the same starvation happens at EVERY text scale, so
/// width alone forces the stack.
///
/// Measured in situ on the wallet screen (#409 R3, 320dp viewport ⇒ a 288dp
/// row inside the surface's 16dp padding), at the DEFAULT 1.0x:
///
/// * the sync-start notice's message keeps **59dp of 288** in `en` and
///   **0.0dp** in `de`, where the card runs 1250dp tall;
/// * the rescan-failed notice keeps 80dp at 1.3x;
/// * the in-flight swap row keeps **71dp at 1.0x and 36dp at 1.3x**, growing
///   that one row to 658dp tall.
///
/// **None of those throws a RenderFlex overflow in `en`** — the notice's first
/// throw is at 360dp/3.0x — which is exactly why a `takeException` pin alone
/// never caught the class, and why the pins measure width. (`de` is harsher:
/// the pre-fix swap row DID throw 14px at 320dp/1.0x there.)
///
/// This is a ROW width, not a screen width, so the same constant lands at a
/// different viewport per surface — measured breakpoints: swap row ≥552dp,
/// compact notice ≥578dp, prominent notice ≥586dp. Between 560 and 585dp a
/// foldable can therefore show a stacked notice above an inline swap row.
const double kWalletStackRowWidth = 520;

/// Whether the OS text scale alone forces the trailing element onto its own
/// line.
///
/// Probes the BODY text size, not an arbitrary large one. `TextScaler.scale`
/// is size-dependent by contract, and on Android 14+ [MediaQuery.textScalerOf]
/// returns `SystemTextScaler`, which forwards to the platform's NON-LINEAR
/// curve — that curve boosts small text more than large. Probing at font size
/// 100 therefore under-reports what the row's real (body-sized) text is doing
/// and the stack engages late, or not at all, on precisely the devices whose
/// users need it. `labeled_zat_row.dart` documented that reasoning first; this
/// is the same probe, shared.
bool walletTextScaleForcesStack(BuildContext context) {
  final refSize = Theme.of(context).textTheme.bodyMedium?.fontSize ?? 14.0;
  return MediaQuery.textScalerOf(context).scale(refSize) >
      refSize * kWalletStackTextScale;
}

/// Whether a row that carries text plus its trailing action(s) must move them
/// under the text. (One action on the notices, two on the parked-send and
/// in-flight-swap rows — the more actions, the earlier the row starves.)
///
/// [availableWidth] is the row's own maximum width — take it from a
/// [LayoutBuilder] rather than the screen, so an embedding inside a narrow
/// column (a host's split view, a sheet) is judged on the space the row
/// actually has.
///
/// A NON-FINITE width stacks. Every caller's inline branch holds an `Expanded`,
/// which asserts under unbounded width, so an unbounded embedding must not be
/// answered with the inline layout: `infinity < 520` is `false`, which would
/// pick exactly the branch that cannot survive it. Unreachable through the
/// package's own surfaces today (they all sit in a bounded `ListView`), but
/// this is now the single definition and it should be safe for the degenerate
/// input rather than accidentally correct for the reachable one.
///
/// The row still needs a bounded-width ANCESTOR for its own `Expanded`/
/// `Flexible` children — the stack only decides where the action goes, it
/// cannot rescue a layout with no width to divide.
bool walletRowStacksAction(BuildContext context, double availableWidth) =>
    !availableWidth.isFinite ||
    availableWidth < kWalletStackRowWidth ||
    walletTextScaleForcesStack(context);
