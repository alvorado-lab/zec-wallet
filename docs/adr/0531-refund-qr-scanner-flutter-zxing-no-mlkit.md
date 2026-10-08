# 0531 — The refund-address QR reader is `flutter_zxing` (on-device ZXing), NOT `mobile_scanner` (Google ML Kit) — no Google telemetry channel in a censorship-resistant wallet

- **Status:** Accepted
- **Date:** 2026-06-23
- **Links:** docs/specs/wallet-sdk.md §3.3b D6 (the IntoZec refund-address field + QR scan, IZ-4) / §5.4 (never-log list) · docs/ROADMAP.md §14 (IZ-4) · ADR-0005 (audited libraries WHOLE — Rule Zero) · the design invariants (works on censored networks; no runtime network fetches by UI deps; metadata is as sensitive as content) · `app/relim-flutter/pubspec.yaml` (`flutter_zxing`) · `app/relim-flutter/android/app/build.gradle.kts` (NDK r27 pin)

## Context

IZ-4 adds a camera QR reader so a user can scan a source-chain refund address
into the IntoZec form instead of typing it (the scan is ADDITIVE — paste/type
always works and is the only path on desktop). The spec tentatively named
**`mobile_scanner`** and flagged the slice "supply-chain-gated" — i.e. the
dependency choice was explicitly deferred to the supply-chain pass.

Two supply-chain rounds (S96) found:

- **`mobile_scanner` is hard-wired to Google ML Kit on Android.** The bundled
  model scans offline (good for censored networks), but standalone ML Kit also
  carries a background channel to Google: periodic model/bug-fix updates AND
  per-app usage/utilization telemetry (device info, app identifiers, performance
  metrics). Google's own ML Kit terms document this and provide **no documented
  opt-out** for the standalone SDK — no manifest flag, no Gradle property, no
  runtime API (the Firebase data-collection toggle does not apply). The Android
  backend cannot be swapped off ML Kit. So `mobile_scanner` cannot be made
  Google-telemetry-free on Android.
- A Google-operated channel baked into the binary directly contradicts the
  product's core posture (censorship-resistant, "no runtime network fetches by
  UI deps," "metadata is as sensitive as content"). For a privacy wallet, a
  microphone-grade trust signal ("why does my wallet talk to Google?") is a real
  cost, even though the scan itself works offline.

The QR scan is low-stakes for money correctness: a misscan only populates a
string the user verifies character-by-character at review (§3.3b D6) and the
Rust SDK validates. So scanner-library polish matters less than the privacy
posture.

## Decision

Use **`flutter_zxing` (^2.3.0)** as the refund-address QR reader.

- FFI to the audited **`zxing-cpp`** decoder (Rule Zero — audited library WHOLE,
  no hand-rolled scanning). MIT-licensed.
- **Fully on-device, ZERO background network** — no Google ML Kit, no Play
  Services, no telemetry. (Its only network-bearing API is a caller-supplied
  image-URL read, which we never call.)
- Camera-only: the scanner runs with `showGallery: false` and
  `enableAudio: false`, so the only new permission the app *intends* is CAMERA.

Consequences accepted and mitigated:

1. **Single-maintainer** (bus factor) — same risk class as `mobile_scanner`;
   documented in `pubspec.yaml`. Google-free is the deciding factor, not
   maintenance.
2. **NDK r28+ build failure** (upstream flutter_zxing #225 — `zxing-cpp` fails
   to compile). Mitigated by pinning the app to **NDK r27** in
   `build.gradle.kts` (the `flutter_zxing` Android module itself pins r27;
   aligning removes a cross-module split). r27 is a stable, Rust-supported NDK
   for the `zec_wallet` Cargokit build. Revisit when #225 is fixed.
3. **Transitive `camera_android_camerax` unions `RECORD_AUDIO` +
   `WRITE_EXTERNAL_STORAGE`** into the merged Android manifest. Both are stripped
   with `tools:node="remove"`; a test (`test/android/manifest_permissions_test.dart`)
   guards the suppression so a dep bump can't silently re-introduce a mic
   permission.
4. **Transitive `image`/`http`/`image_picker`** enter the tree for flutter_zxing's
   optional gallery/URL-scan features we do not use — inert (never invoked), no
   background network.

Platform gating: the scan affordance is offered only where the `camera` backend
exists — `!kIsWeb && (Platform.isAndroid || Platform.isIOS)` (both-natives per
flutter-patterns). Desktop/web are paste-only; the `ReaderWidget` never mounts
there.

## Alternatives considered

- **`mobile_scanner` (Google ML Kit), accept + disclose the telemetry** —
  rejected: an un-disablable Google channel in a censorship-resistant wallet is
  not acceptable when an equivalent Google-free reader exists.
- **`qr_code_scanner_plus` (ZXing-Java, AVFoundation)** — Google-free and BSD,
  but the publisher declares it maintenance-only ("postpone migration") and
  ZXing-Java lags `zxing-cpp`. Rejected in favor of the actively-released FFI
  path.
- **Defer the camera; ship paste-only IZ-4** — viable (paste already works
  everywhere) but under-delivers the planned slice; the founder chose the
  Google-free reader.

## Status of the spec's named dependency

This ADR supersedes the spec's tentative `mobile_scanner` naming for the IZ-4
reader. The spec (§3.3b D6 / build-order / §8 register), ROADMAP §14, and the
e2e checklist are updated to `flutter_zxing` in the same change (arch/ + spec
update in the same commit that alters the structure).
