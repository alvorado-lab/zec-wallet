# 0007 — License: MIT (all Relim code, incl. servers); obfuscation engine quarantined to a GPL full-build flavor

> **Filename note:** the slug `0007-license-split-apache-gpl-agpl` is
> historical — it's named after the *original proposal* (a copyleft split,
> now rejected alternative C). **The decision is MIT-everywhere.** The slug
> is kept to avoid breaking cross-links + an immutable research snapshot;
> the title above is authoritative.

- **Status:** Accepted — Path B + **MIT servers** (founder, 2026-06-06).
  Server-license question (was open #11) now resolved: MIT everywhere.
- **Date:** 2026-06-05 (revised + decided 2026-06-06)
- **Supersedes the original "copyleft split" proposal** preserved below as
  rejected alternative C.

## Decision (Path B)

- **All Relim source code is MIT** — expressed for crates as the Rust-standard
  dual **MIT OR Apache-2.0** (matches Mostpost; MIT preference + optional
  patent grant). App, ffi, drivers, transport traits, reusable crates,
  **AND servers (relay, push waker, manifest service)**: all MIT/Apache.
- **The optional obfuscation engine (sing-box, GPLv3) is quarantined to the
  full-build flavor.** Because MIT is GPL-compatible, the *combined binary*
  of that flavor is distributed under GPLv3 terms while every Relim source
  file stays MIT. The **store flavor ships with the engine off → pure MIT**.
  This keeps the founder's MIT preference AND the best embedding engine
  (sing-box/libbox) — see ADR-0006.
- **No CLA needed** (inbound = outbound MIT); use **DCO** (`Signed-off-by`)
  for provenance.
- GPL-on-App-Store mechanics are moot (engine-carrying flavor is sideload/
  F-Droid, not the store build).
- **Links:** docs/PRODUCT_VISION.md § Open decisions #10 ·
  [research](../research/2026-06-05-licensing.md) · [0006](0006-obfuscation-engine-direction-singbox.md)

## Context

Relim's license was undecided and gates both the workspace scaffold
(per-crate `license` fields) and the obfuscation engine (sing-box is
GPLv3-or-later, actively enforced). Research findings: the "GPL banned from
App Store" myth is dead (Signal ships GPLv3 on iOS; the VLC takedown was a
multi-rightsholder problem — a sole/CLA-backed rightsholder cannot infringe
their own license); Element's relicense to AGPL is the documented
anti-enshittification + enterprise-revenue lever, and they deliberately
kept SDKs Apache-2.0 for ecosystem adoption; all-permissive scores 1/5 on
sing-box embedding (impossible if embedded); Apache-2.0 → GPLv3 consumption
of the Mostpost crates is one-way clean.

## Conflict map for the founder's MIT preference (added 2026-06-06)

**The entire accepted stack is MIT-compatible.** Verified licenses: iroh
(MIT/Apache), OpenMLS (MIT), librustzcash (MIT/Apache), rust-nostr (MIT),
quinn (MIT/Apache), arti (MIT/Apache), whisper-rs (Unlicense),
flutter_rust_bridge (MIT), RustCrypto (MIT/Apache), Mostpost crates
(MIT OR Apache-2.0), Tor PT binaries (BSD).

**Exactly ONE copyleft module exists in the whole plan: sing-box
(GPLv3-or-later, actively enforced).** The engine alternative —
**Xray-core — is MPL-2.0 (verified 2026-06-06)** with an MIT `libXray`
wrapper; MPL-2.0 is file-level copyleft, fully compatible with an MIT app.
AGPL-for-servers was a *strategy* proposal (fork protection + enterprise
lever), never a dependency constraint — MIT servers have zero legal
conflict.

Bonus under MIT: **no CLA is needed at all** (inbound = outbound MIT; DCO
optional); the GPL-on-App-Store mechanics become irrelevant.

## Why MIT servers (resolved 2026-06-06)

The server-license question turned on a reframing: **a license cannot stop a
covert malicious relay operator** (they ignore the license, live in an
adversarial jurisdiction, and are externally undetectable since a relay only
shuffles ciphertext). That threat is defeated by **architecture, not
license** — ciphertext-only relays, sealed sender, padding/delay, relay
diversity, multi-hop. AGPL only bites the *honest commercial extractor*
(Threat B), not the malicious operator.

For a censorship-resistant network, **relay proliferation and diversity is
itself a defense** (more relays, more parties, more jurisdictions = harder to
block/surveil/seize). MIT *maximizes* that proliferation; AGPL mildly chills
the commercial relay operators who'd otherwise add capacity. So MIT servers
**serve the threat model**, not just simplicity.

Cost accepted: MIT forfeits the *coercive* enterprise dual-license lever
(under MIT a company needn't buy a license). Recoverable: the founder holds
all copyright (DCO, no CLA), so commercial terms can be *added* later or a
*future version* made copyleft if extraction ever becomes a real, observed
problem. Starting permissive and tightening later is far easier than the
reverse.

The real anti-abuse tools, used instead of license: **reproducible builds**
(verify what first-party relays run) and **trademark** (stop a gutted fork
shipping as "Relim" — which copyleft can't prevent anyway).

## Alternatives considered

- **A — MIT everywhere + Xray-core (MPL-2.0) engine:** zero copyleft
  anywhere, but Xray's mobile in-process builds are reportedly painful
  (libbox was purpose-built for embedding) and loses sing-box's broader
  protocol set. Engine still evaluated at the v1.0b spec; Path B doesn't
  foreclose switching to Xray if the iOS investigation favors it.
- **C — copyleft split (original proposal):** Apache crates / GPLv3 app /
  AGPLv3 servers / no-assignment CLA. Strongest anti-enshittification + the
  enterprise dual-license lever, but overrides the MIT preference and adds a
  CLA the privacy/FOSS audience dislikes. Rejected for the app/crates; its
  AGPL-servers idea survives as an *open* server-license question (below).
- **All-AGPLv3:** poisons the store track and crate ecosystem (Element's
  keep-SDKs-permissive lesson). Rejected.

## Consequences

- Workspace scaffold sets every crate (incl. server crates) to
  `license = "MIT OR Apache-2.0"`. One license story across the whole repo.
- **Two build flavors are a first-class architectural fact:** store flavor
  (engine off, pure MIT) and full flavor (sing-box embedded, GPLv3-combined,
  sideload/F-Droid). Feature flags already planned (vision § Distribution).
- The obfuscation engine stays a separate-process swappable sidecar behind
  the Transport trait (right architecture anyway); the GPL flavor is the
  honest distribution form, not a process-boundary loophole.
- **F-Droid flavor cannot ship FCM/Google services → the push waker gains a
  UnifiedPush path** (Molly/MollySocket pattern) — added to the push-waker
  spec scope.
- DCO line in CONTRIBUTING; no CLA infrastructure needed.
- **Two abuse defenses must be carried as real work items** (since license
  isn't one): reproducible builds (verify first-party relay binaries) and a
  Relim trademark/brand-use policy (stop gutted forks shipping as "Relim").
  Trademark check is already vision open #9.
- Anti-malicious-relay protection is an **architecture** requirement on the
  relay-protocol spec (ciphertext-only, sealed sender, padding/delay, relay
  diversity), not a licensing one.
