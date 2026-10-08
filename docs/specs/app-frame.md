# Relim app frame — `app/relim-flutter` scaffold + theme system

**Status: Draft** (lifecycle: Draft → Reviewed → Implementing → Shipped)
**Track:** wallet-track W4 (founder decision 2026-06-10, brief §1: app frame
sequenced after the SDK core; spec + `flutter create` + Mostpost theme port
in one chunk per brief §5).
**Owner refs:** `docs/arch/overview.md` §3 (`app/relim-flutter` row),
the project's Flutter rules (normative widget/theming/lifecycle
rules), Mostpost theme sources (READ-ONLY reference, brief §1).

---

## 1. Design decisions

**Problem.** Relim has no Flutter app project. Every UI milestone (Messaging
UX, network transparency panel, wallet screens) needs a shell to land in:
project scaffold, theme system, navigation, lifecycle plumbing, CI wiring.
Building that shell *with* the first feature couples two reviews and
retrofits theming (Mostpost's own history: late i18n/theming = "refactor on
every translation"). The frame is the cheap-now, expensive-later layer.

**Decision: a deliberately THIN frame.** Ship the scaffold + theme system +
appearance settings + lifecycle/navigation seams, and nothing else. No FFI
binding (there is no `relim-ffi` surface yet — the app README's old
"Messaging UX milestone" note described exactly this dependency; the
founder re-cut the *frame* to W4 because the frame does not need FFI).
No messaging UI, no network panel, no wallet screens — those are later
milestones that *consume* the frame.

**Theme: copy-and-diverge from Mostpost** (founder, 2026-06-10). The five
source files (`core/theme/{theme,colors,theme_mode_provider,appearance_prefs}.dart`,
`shared/theme/avatar_color.dart`) are app code, not an ADR-0002 consume
crate — the two products' designs diverge by design, and Mostpost is
read-only reference. Divergences (each with a WHY, §2.2):

- `MostpostColors` → **`RelimColors`**; Mostpost's 5 `ai*` tokens (draft
  surfaces — a Mostpost-only feature) are DROPPED. Message-bubble tokens
  (`bubbleMine`/`bubbleTheirs`) are KEPT — Relim is a messenger.
- `accountDotColorFor` (multi-inbox mail affordance) is DROPPED;
  `avatarColorFor(seed, colors)` is KEPT with the seed documented as the
  contact's stable identifier (fingerprint/contact id — set at Messaging
  UX), never a PII string requirement.
- Palette VALUES ship as-is: they are WCAG-audited (Mostpost phase 5.5.E +
  the Alvorado light pass) and brand-neutral enough for an alpha. The
  Relim brand accent is an OPEN founder decision (§11) — tokens make the
  re-skin a constants change, which is the entire point of tokens.

**Alternatives rejected:**
- *Shared theme package between Mostpost and Relim* — couples release
  cadence of two products' design systems for ~500 lines; contradicts the
  copy-and-diverge founder decision.
- *Defer the frame to Messaging UX (the old README plan)* — serializes the
  tracks for no reason; the frame has zero dependency on relim-ffi.
- *`google_fonts` runtime fetching* — banned (flutter-patterns): a network
  fetch for glyphs is a privacy leak and a censored-network failure mode.
  The SDK bundles no font since S10 (FR-49 W-9, ADR-0564); the host supplies
  its faces as a `TextTheme`, and the example app bundles its own.
- *Riverpod codegen (`riverpod_annotation`)* — not yet; the frame's five
  providers are hand-written `Notifier`s (KISS; codegen joins when a real
  feature needs families/auto-dispose graphs).

**Tradeoff accepted:** the frame ships screens with almost no content
(placeholder home + Settings → Appearance). That is honest — the app tells
the truth about what works right now (invariant 6) — and it gives every
later milestone a reviewed shell instead of a greenfield.

## 2. Domain types

No Rust types in this chunk (no FFI; serde/zeroize n/a — first spec section
where that is true, stated per template).

### 2.1 Dart-side state (all ephemeral-or-preference; "Rust owns all state"
governs application data, NOT rendering preferences — the Mostpost
precedent comment is ported verbatim)

| Type | Values | Persisted |
|---|---|---|
| `ThemeMode` (Flutter) | system / light / dark | SharedPreferences `theme_mode` |
| text scale `double` | clamped [0.85, 1.40], default 1.0 | SharedPreferences `text_scale` |
| AMOLED `bool` | true-black dark variant, dark-only | SharedPreferences `amoled_dark` |
| device-log level `String?` | `off` / `errors` / `detailed`; ABSENT = never chosen (the read is nullable and checks the key, so it never collapses into `off`); any other value also reads never chosen. Resolved by `resolveDeviceLogLevel`: this, else the `ZEC_WALLET_DEVICE_LOG` define, else Detailed in debug and Errors only otherwise (founder, S308 Q2) | SharedPreferences `device_log_level`. **NOT cosmetic** — the second recorded exception to `appearance_prefs.dart`'s rule, beside `use_tor_plugin`, for the same reason: it is read straight after the FFI init, before the wallet exists. It holds a level name only, no wallet state and no secret (stage S5 `row`) |
| `AppLifecycleState` (Flutter) | resumed/inactive/paused/… | never (live) |

### 2.2 `RelimColors` (`ThemeExtension<RelimColors>`)

20 tokens (Mostpost's 25 minus the 5 `ai*`): `bg, bgCard, bgHover, border,
accent, accentSoft, accentText, text, textMuted, textDim, green, orange,
red, purple, cyan, bubbleMine, bubbleTheirs, toggleOff, inactiveElement,
onAccent`. Three schemes: `dark` (forest-tinted, violet accent),
`darkAmoled` (= dark with pure-black surfaces, borders lifted),
`light` (warm Alvorado). WCAG annotations ride along as comments; the
CONTRACT is the contrast tests (§8), not the literals.

Widgets access tokens via `RelimColors.of(context)` ONLY. `Colors.white`/
hardcoded hex in widgets = review reject (flutter-patterns).

## 3. Interface design (providers are the app's ports)

Inbound (UI watches):
- `themeModeProvider`, `textScaleProvider`, `amoledProvider` — `Notifier`s
  seeded flash-free via `initial*Provider` ProviderScope overrides loaded
  BEFORE `runApp` (one guarded SharedPreferences read; failure ⇒ defaults).
- `appLifecycleProvider: Notifier<AppLifecycleState>` — THE lifecycle seam.
  **Standing rule (review gate):** every future FRB stream provider MUST
  watch it and pause on `paused` / resume + re-fetch on `resumed` **only
  when resuming from an actual pause** (track "was paused" locally):
  `inactive → resumed` fires on every window-focus change on desktop and
  every notification-shade pull / permission dialog on mobile — re-fetching
  there is network chatter that unstable networks and batteries pay for.
  Desktop honesty (§9): `paused` never fires on macOS/Linux/Windows (the
  deepest state is `hidden`; a minimized app keeps running), so streams
  correctly stay live while the window is hidden — the wanted desktop
  behavior for a messenger, by construction, not by accident. On Android
  (Flutter 3.13+) `hidden` is NOT desktop-only: it fires as an
  intermediate state during normal backgrounding (`inactive → hidden →
  paused`) — don't-pause-on-`hidden` holds on both platforms because
  mobile's `paused` arrives right after.
  **Lifecycle gating is NOT connection-loss recovery (orthogonal rule,
  S6 second review):** a stream that dies mid-foreground (transport
  error, EOF, timeout) sets no "was paused" flag and gets no lifecycle
  event at all on desktop — stream error/completion MUST be its own
  reconnect trigger in every FRB stream provider, independent of this
  seam. The seam recovers from voluntary OS pauses; the reconnect path
  recovers from unstable networks; a provider needs both.
  (flutter-patterns CRITICAL). The frame ships the seam so the first FFI
  consumer has no excuse to hand-roll its own observer. The re-fetch-gate
  AND reconnect behaviors get their named tests with the first real FRB
  stream consumer (they are consumer behavior — the seam itself just
  mirrors states).
- `routerProvider` (GoRouter): `/` (home placeholder) and
  `/settings/appearance`. Unknown route → recoverable error screen with a
  "go home" action (next steps, never codes). `redirect` stays empty (and
  must stay FAST when it lands — gotcha noted in code).

Outbound: SharedPreferences (rendering prefs only) — wrapped in the
notifiers; no other I/O of any kind exists in the frame.

Would this survive a different adapter? Yes: the providers are the
interface; swapping SharedPreferences for anything else (or the future
Rust-side settings sync, §10) changes only notifier internals.

## 4. Security

- **No key material, no FFI, no network, no native channels.** The frame's
  entire attack surface is SharedPreferences (non-sensitive rendering
  prefs; hostile values are clamped/enum-parsed on read — unknown theme
  string → `system`, scale clamped to bounds).
- Dependencies (pub, pinned by caret + lockfile, committed):
  `flutter_riverpod`, `go_router`, `shared_preferences`,
  `flutter_localizations`/`intl` — all already vetted in Mostpost
  production. No Rust dep changes; `deny.toml` untouched.
- No telemetry, no crash reporting, no analytics — and none may EVER be
  added to this app without a spec (vision: metadata is as sensitive as
  content).
- **Android backup/transfer OFF** (`allowBackup="false"`,
  `fullBackupContent="false"`): the platform default would silently ship
  app data to Google backup. Today that is only rendering prefs; the
  posture must already hold when wallet state exists in the app sandbox.
  Selective transfer, if ever wanted, comes via explicit
  `dataExtractionRules` in its own spec.
- The Material `colorScheme.onError`/`onSecondary` pairs are
  contrast-gated (≥4.5:1) alongside the raw tokens — in dark themes the
  on-color is the near-black `bg`, not white (white on the dark palette's
  bright green/red fills measures 2.2–2.8:1; wallet success/error
  surfaces reach for exactly these pairs).
- Fonts bundled in-app (no CDN); OFL 1.1 license text ships next to the
  TTFs.

## 5. Privacy & metadata

**Zero bytes leave the device from anything in this chunk.** No server
fields, no traffic patterns, nothing for a relay to see. Locale and theme
never leave the device. Logging: the frame logs nothing in release; debug
prints are banned (`avoid_print` lint enforced); when structured logging
arrives it comes via the Rust core, not Dart.

## 6. Error handling & degradation

| Case | Behavior | User sees |
|---|---|---|
| SharedPreferences read fails at boot | defaults (system/1.0/off); app boots normally | nothing (correct: cosmetic loss only) |
| SharedPreferences write fails on toggle | in-memory state updates; persist silently skipped (Mostpost pattern: never throw from a fire-and-forget onTap) | theme switches now; may not survive restart |
| Unknown/corrupt pref value | parsed to default / clamped | nothing |
| Unknown route | error screen + "go home" button | plain-language message, a next step, no codes |
| Zero connectivity | irrelevant by construction — the frame performs no I/O but prefs | fully working app shell |

No silent failures with consequences exist: the two swallowed prefs
failures are cosmetic-by-construction and documented at the swallow site.

## 7. Performance

- **Cold start:** exactly ONE disk read (SharedPreferences) before
  `runApp`, already required for flash-free theming. No other pre-runApp
  work permitted in the frame.
- **Theme switch:** animated via `ThemeExtension.lerp`,
  `themeAnimationDuration = 200 ms` (flutter-patterns convention; named
  constant in `core/theme/theme.dart` — Dart lowerCamelCase identifiers
  are the normative names throughout).
- **No timers, no polling, nothing background** — the frame is inert when
  not on screen (mobile battery posture inherited by construction).
- Text-scale composition is pure math in the MaterialApp `builder`
  (per-frame cost negligible); effective scale hard-clamped to
  `[minEffectiveTextScale = 0.8, maxEffectiveTextScale = 2.0]` so no
  OS × user combination breaks layouts; iOS floors at
  `iosHigMinTextScale = 1.12` (HIG ~17pt body vs Material 14sp —
  clamp, not multiply, preserves Dynamic-Type users; Mostpost rationale
  ported with the code).

## 8. Testing strategy (test contract — named tests per gate)

Widget/unit tests, host VM, no device needed (CI via `just flutter-ci`,
which activates on `app/relim-flutter/pubspec.yaml` existing):

| Named test | Gate(s) |
|---|---|
| `light_theme_contrast_meets_wcag_aa` | 1, 2 — every text token ≥4.5:1 on bg+bgCard+bubbles, `accentText` ≥4.5 on `accentSoft`, `inactiveElement` ≥3:1, `onAccent` ≥3:1 on accent (computed, not annotated) |
| `dark_theme_contrast_meets_wcag_aa` | 1, 2 — same gate for dark + AMOLED variants (goes BEYOND the source palette, which only gated light). Port finding: the source's claim that white works on green/red holds ONLY in light (dark measures 2.2–2.8:1) — `onAccent` is documented accent-only; green/red-surface text is a Messaging-UX design decision |
| `theme_mode_persists_and_unknown_value_falls_back` | 3, 6 — set/persist roundtrip (mocked prefs) + corrupt stored value → `system` |
| `text_scale_composition_clamped_at_extremes` | 6, 7 — OS 0.5×–3.0× × user bounds stays inside [0.8, 2.0]; iOS floor applies; Android untouched |
| `appearance_screen_switches_theme_with_44px_semantic_targets` | 2, edge — tapping Dark flips `themeMode` + rendered brightness; every interactive element ≥44×44 AND exposes tile-level selected/enabled semantics (regression to a bare GestureDetector fails the flag asserts, not just sizes); AMOLED switch honestly DISABLED under light at widget + screen-reader level, re-enabled under dark; slider announces one l10n-keyed value node; finders l10n-keyed, never English literals |
| `app_boots_offline_to_home_first_frame_themed` | 4, 6 — pumps the real bootstrap path with overrides; no I/O but prefs; first frame already carries the persisted theme |
| `unknown_route_shows_recoverable_error` | 6 — bad location → error screen with a working home action |
| `avatar_color_deterministic_and_empty_seed_safe` | 3 — same seed → same color across calls; empty seed → `textMuted` fallback |
| `lifecycle_provider_emits_pause_and_resume` | 4 — the FRB-stream seam observably transitions, incl. the paused→detached exit path |
| `lifecycle_provider_initial_state_reads_binding` | 4, 6 — a BACKGROUNDED cold start (Android push trampoline) seeds the provider with the binding's real state, not a hardcoded `resumed` |
| `app_renders_without_overflow_at_max_text_scale` | 2, 6, 7 — OS 3.0× × user 1.4× → clamped 2.0× on a 360×640 viewport renders home + appearance with zero RenderFlex overflow (this test caught a real 60px overflow on first run; centered screens scroll when they can't fit) |

E2E: none in this chunk (nothing crosses a process boundary); the first
device E2E lands with the first FFI consumer. Gate 5 (observability): n/a
by construction (no state transitions beyond prefs, which are tested);
first tracing spans arrive with FFI. Gate 8 (i18n): **gen-l10n wired NOW,
`en`-only** — every user-facing string in the frame lives in
`app_en.arb` from day one (the Mostpost lesson: late i18n = refactor every
string), enforced by analyzer lint once wired.

Self-review checklist: no unsafe/crypto (n/a) · no PII in logs (no logs) ·
works offline (no I/O) · honest degradation (placeholder home SAYS it's a
frame) · accessible (44×44 + Semantics + dynamic type + WCAG gates) ·
no test vectors needed (no crypto).

## 9. Multi-platform

- **Android:** `minSdk = maxOf(23, flutter.minSdkVersion)` — floor 23 is
  the wallet SDK's Android-Keystore floor (the app will host that SDK;
  shipping lower would force a store-visible RAISE later), and the maxOf
  guard rides Flutter's own floor when it moves past 23. `compileSdk`
  delegates to `flutter.compileSdkVersion` (36 in Flutter 3.41.6; the
  AGP-8 requirement of ≥35 is met by delegation).
- **iOS:** floor 13 (Flutter default; the wallet SDK podspec matches);
  HIG text-scale floor per §7; status-bar brightness follows theme via the
  global `AnnotatedRegion` (ported — covers AppBar-less screens).
- **Desktop (macOS/Linux/Windows):** committed (founder 2026-06-12, after
  mobile), kept honest via the standing per-chunk desktop-portability
  review — seam honesty, not a parallel build track. Frame status:
  - **macOS debug build PROVEN green** (S6, `flutter build macos` →
    `relim_app.app`); Linux/Windows runners are scaffolded — their compile
    proof is owed to a CI lane (manager Q6 territory, no such runner on
    this host).
  - **Lifecycle semantics differ and the §3 seam rule is written
    desktop-honest:** `paused` never fires on desktop (deepest state is
    `hidden`; minimized apps keep running — streams stay live, wanted);
    `inactive` fires on every focus change (the §3 re-fetch gate exists
    for exactly this). "Stay live" is a lifecycle statement, not a
    network guarantee — liveness against connection loss is the
    reconnect path's job (§3 orthogonal rule), and it matters MOST on
    desktop, where no lifecycle event will ever nudge a dead stream.
  - `overlayStyleFor` (status-bar icon brightness) is a mobile no-op on
    desktop — harmless by construction.
  - Window sizing: the frame sets no min-window-size; screens scroll when
    they can't fit (the §8 overflow gate covers small viewports). A real
    min-size/layout policy lands with the Messaging UX shell (A3), where
    desktop gets real layout decisions (rail vs bottom nav).
  - Keyboard navigation: the appearance controls are stock Material
    (RadioListTile/SwitchListTile/Slider) — focus traversal and arrow keys
    work without custom code; custom shortcuts are out of frame scope.
  - SharedPreferences is supported on all three desktops (NSUserDefaults /
    file / registry) — rendering prefs survive unchanged.
- **Web: OUT**, same posture as the wallet SDK (§9 there): the eventual
  FFI core is native-only and key handling in a JS runtime is off the
  table; revisit only with a real WASM-core decision.
- Platform gating rule (flutter-patterns): any native gate is
  `Platform.isAndroid || Platform.isIOS`, never one alone.
- **No dart-defines exist yet** ⇒ bare `flutter build` is not yet broken
  (the README gotcha activates when the first define lands; the `just`
  recipe seam exists from day one so habits form early).

## 10. Multi-device & sync

Appearance preferences are **deliberately device-local** (a phone and a
desktop legitimately differ in theme/text size); they are rendering
preferences, not application data, so they sit OUTSIDE the future
relim-sync scope. Nothing in the frame assumes one-user-one-device. If a
"sync my appearance" feature is ever wanted it becomes Rust-owned state
behind FFI — the provider seam (§3) survives that unchanged.

## 11. Open rows

| # | Item | Owner |
|---|---|---|
| A1 | Relim brand accent + light-palette identity (currently Mostpost's WCAG-proven values as placeholder) | founder |
| A2 | Bundle/application id (`app.relim.*` placeholder) — must finalize before ANY store/TestFlight upload | founder/manager |
| A3 | Tab/shell structure (bottom nav vs rail) — lands with Messaging UX (P1 #5), not the frame | manager re-cut |
| A4 | Bootstrap error surface (BootstrapErrorApp-class) — REQUIRED when FFI init can fail; not needed while boot = prefs-read-with-defaults | with first FFI chunk |
| A5 | `app/relim-flutter/README.md` rewritten by this chunk (old "Messaging UX milestone" timing note superseded by the founder's W4 re-cut — contradiction logged in wallet-log S5) | this chunk |
| A6 | **Amount-display convention** — amount `Text` widgets MUST NOT use `overflow: ellipsis`/`maxLines` that can clip digits (the classic hidden-digit money hazard); the only sanctioned treatments are wrap or scroll. Pin as a flutter-patterns review-reject rule before the first wallet screen | with first wallet-screen chunk |
| A7 | **ZEC amount formatting** — amounts cross the bridge as `int` zatoshis; the single zat→display conversion and locale treatment (decimal-comma locales, copy-paste safety, significant figures) must be ONE shared helper decided before the first wallet screen, never per-screen `'${zat / 1e8}'` | with first wallet-screen chunk |
| A8 | **Screen-content protection** — wallet screens (balance, receive address, tx detail) must not appear in the Android recents thumbnail / iOS app-switcher snapshot: decide `FLAG_SECURE` whole-Activity vs per-route + the iOS background-blur pattern. Must land WITH the first wallet screen — shipping without it is itself a decision and needs sign-off. **Desktop honesty (S6):** there is no FLAG_SECURE equivalent; partial analogues exist (macOS `NSWindow.sharingType = .none`, Windows `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`), X11 Linux has **none** — the A8 decision must state per-OS what is and isn't protected, never imply desktop parity | with first wallet-screen chunk |
| A9 | **macOS App Sandbox network entitlement (S6 finding):** `Release.entitlements` AND `DebugProfile.entitlements` in `app/relim-flutter/macos/` and `sdk/zec_wallet/example/macos/` carry no `com.apple.security.network.client` (template default; debug's `network.server` covers incoming only) — every OUTGOING connection fails closed in a sandboxed macOS build. Deliberately NOT granted while the frame has zero network I/O (least privilege); MUST be added, with a WHY comment, in the same change as the first networked chunk (wallet sync / swap / relay) or that chunk ships desktop-dead silently. **Binds BOTH targets** — the app AND the SDK example harness (`sdk/zec_wallet/example/macos/`, which is where desktop swap/sync E2E first runs; without it the first desktop swap E2E strands mid-flow and reads as an engine bug, not a sandbox one). **Error-classification note (S6 second review):** sandbox denial of outbound TCP surfaces as an immediate `EPERM`-class error, NOT a timeout — the networked chunk's connection-error handler must classify it separately from transient network failures (it is "never going to work without an entitlement", not "retry with backoff"), or sandboxed builds spin retries forever while looking like an unstable network | with first networked chunk |
