# 0564 — Wallet UI: the SDK adopts the refreshed design, with host hooks for type, icons and surfaces

- **Status:** Accepted (S299, 2026-09-24): the founder's three rulings, asked directly in the
  wallet session, plus the contract list agreed with Relim at sync point 1. Stages S10–S13
  build it; each stage's plan names its own contracts.
- **Date:** 2026-09-24
- **Links:** FR-49 (`docs/handoff/host-feature-requests.md`) · Relim's spec
  `docs/specs/design-refresh.md` at Relim `c3d9e7c6` (§2 the design language, §3 tokens, §9 the
  wallet handoff, §9.5 W-1…W-11) · Relim's `DESIGN.md` (repo root, from `10d2c07b`) · the
  SDK-screen inventory, copied to `docs/handoff/sdk-screen-inventory-2026-09.md`

## Context

Relim's founder ordered a full visual refresh, and the wallet tab is `zec_wallet_ui`'s. His
words, relayed by Relim: "it's not only the color of balance card. you have to analzye the whole
layout and style and align it with the prototype reference … other UI interfaces that are not
shown - they have to be aligned with new refreshed design." Relim's spec passed product review
and hands the SDK eleven requests (W-1…W-11).

`zec_wallet_ui` is a published, universal package. A host takes its look either from the SDK's
`buildTheme` and presets, or from its own `ThemeData` with a `WalletColors` extension (README
step 3). The SDK has to change in two ways. It must restyle every surface to the new design
language. And it must let a host supply what the design needs but the SDK cannot own: a mono
face, an icon set, a deep brand surface, the coin's colours.

Three questions could only be answered by the founder. He answered all three in the wallet
session on 2026-09-24:

- **A. The SDK's own default look.** *Adopt the new look:* the SDK's `buildTheme` and presets
  take the new layout AND the green palette, and fonts come from the host.
- **B. FD-6.** *Adopt as written:* "Hide balance" / "Show balance".
- **C. The designer's icon drawings.** *No, Relim-only:* the SDK takes an icon hook and keeps
  Material icons by default. Nothing of the designer's ships on pub.dev.

## Decision

1. **One design, the SDK's default.** Every `zec_wallet_ui` surface follows the refreshed
   language (Relim spec §2), and the SDK's presets carry the §3.1 green palette. A non-Relim
   host gets the same design Relim does, in the SDK's own colours, which are the same greens.
   Widgets read only theme roles and `WalletColors`; no colour or radius literal stays in a
   widget (W-11).
2. **The host supplies type, icons and identity through theme extensions:**
   - `WalletTypography.mono` (a `TextStyle`), read at the five identifier sites. Default:
     `'monospace'` (W-2).
   - `WalletIcons`: one slot per SEMANTIC (shielded, transparent, in, out, swap, sync problem, …),
     Material by default (W-5, ruling C).
   - `WalletShapes`: the radii as named values (W-11).
   - New `WalletColors` fields: `deep`, `onDeep`, `deepMuted`, `warningOnDeep` (W-6, W-8), plus a
     coin token set that defaults to values derived from `deep` and `accent`. **The new
     `WalletColors` fields are REQUIRED:** a breaking change a host absorbs in the same change
     that bumps its SDK pin (Relim's `walletColorsFrom`, agreed at sync point 1).
   - An optional `qrInk` (default black; the white tile and quiet zone stay the SDK's) (W-10).
3. **The SDK bundles no font** (W-9, ruling A). `buildTheme` takes an optional `TextTheme`, and
   the example app bundles its own faces. It defines `headlineSmall`, `titleSmall` and
   `labelMedium` (W-3).
4. **`onAccent` is "content on `accent`"**, not "always white". Once text sits on `accent`, the
   contrast test grades it at ≥ 4.5:1 (W-1). The new presets meet that.
5. **The coin spins exactly once per event.** The events: the wallet's sync state changing from
   not-up-to-date to up-to-date, and a live incoming payment (the "Payment received" event).
   Either counts only while the tab is mounted and visible. One spin is in flight at most, with
   no queue. There is never a spin on the first build, a re-render, a resume into an
   already-synced wallet, under reduced motion, or offstage.
6. **Hide balance** (W-7, ruling B). The SDK exports `walletBalanceHiddenProvider`
   (session-only, default false), and a host may override it to persist. It hides every amount
   a bystander can read on the SDK's surfaces (Relim spec W-7, rev.2). It never hides an amount
   being typed or the totals under review before Confirm.
7. **A dialog or sheet with a text field** uses the scrollable Material dialog on every
   platform. Field-free confirms are adaptive. This is Relim's measured rule (`DESIGN.md` §6.9):
   at textScaler 2.0 with the keyboard up, the Cupertino and unscrolled Material shapes put the
   confirm out of reach.
8. **Behaviour does not change with the look.** Send stays gated on spendable > 0 (Relim spec
   N-23 rejects "send only after 100 % sync"). The SDK's balance label and as-of header stay
   (FD-7). The navigation locks, screenshot blocking, the QR payload and the large-send dialog
   are untouched.

## Alternatives considered

- **Hooks only; the SDK keeps its own look.** Ruled out (A). It means two designs to maintain,
  and the SDK's dark preset (white on violet, 3.69:1) would fail the 4.5:1 text grade once labels
  sit on the accent.
- **New layout with the SDK's own colours.** Ruled out (A), for the same repaint cost with no
  benefit named.
- **Ship Relim's glyph paths as the SDK default.** Ruled out (C). It puts a host's artwork into a
  published package and couples the SDK's look to one host.
- **Optional new `WalletColors` fields with derived fallbacks.** Rejected. A host that forgets
  one would get a derived colour nobody reviewed on a money surface. The SDK is pre-0.0.1, and its
  one embedding host agreed to the break.

## Consequences

- Four stages, one per session: **S10** the theme contract (the API above; no layout change),
  **S11** shared components (W-4 and §2.5–2.9/2.12), **S12** the wallet tab (money display:
  reviewed on the design and on the diff), **S13** every other surface.
- Every surface is re-walked on a device before its stage exits.
- The `WalletColors` constructor breaks at S10. Relim updates `walletColorsFrom` in the change
  that bumps its pin (sync point 2 carries the exact field list).
- The example app gains its own bundled fonts. A host that relied on the SDK's DM Sans through
  `buildTheme` must supply a `TextTheme`; none is known besides the example.
- "Hide balance" / "Show balance" enter the SDK's ARB; the other 15 locales are owed.
