/// Shared layout constants for the wallet's modal bottom sheets.
///
/// DESKTOP (the maintainer-committed Win/Mac/Linux target): a bottom sheet on a wide
/// desktop window otherwise stretches a narrow form/confirm into a full-width
/// strip that's hard to read. Capping the sheet width keeps the content centred
/// and legible — the SAME value across the shield, rescan, and move-to-transparent
/// sheets so the expert actions stay visually consistent on desktop. Mobile is
/// unaffected (the cap is far wider than any phone). Single source of truth, so
/// the three sheets can never drift apart (no per-sheet magic number — gate 7).
///
/// HOST INTEGRATORS: 560 sits deliberately between a typical form cap (~440) and
/// a content-list cap (~600) — these sheets are forms WITH money-breakdown content.
/// A host with its own layout system should substitute its own form/content
/// width per sheet phase if it needs strict alignment; this value is the
/// package's standalone default, not a contract.
const double walletSheetMaxWidth = 560;
