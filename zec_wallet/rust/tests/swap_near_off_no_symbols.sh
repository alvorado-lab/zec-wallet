#!/usr/bin/env bash
# Symbol-level half of `swap_near_feature_off_ships_no_adapter_symbols`
# (spec §3.5 layer 1 at the artifact level; ADR-0525).
#
# The manifest-shape half (deterministic, always-on in `just ci`) lives in
# `tests/feature_policy.rs`. THIS script is the empirical artifact proof, owed
# to a CI runner (like the desktop-Linux cross-check): build the bridge cdylib
# WITHOUT the `swap-near` feature and assert the resolved dep graph + the built
# artifact carry none of the NEAR adapter or its HTTP/TLS subtree.
#
# Run from the bridge crate dir (`sdk/zec_wallet/rust`):  bash tests/swap_near_off_no_symbols.sh
#
# Honest note: with only the seam's `use as _` binding (W-swap-2), the
# feature-ON symbols may be dead-code-eliminated, so the load-bearing proof is
# the dep-GRAPH absence below; the nm scan is a defence-in-depth check that
# hardens once W-swap-3 wires a real provider consumer.
# CORRECTED 2026-06-23 (kept in lockstep with `feature_policy.rs`): the SUBTREE
# bans ONLY the swap-REST-client-class crates. The base hyper/rustls stack
# (hyper/hyper-util/rustls/tokio-rustls/ring/webpki-roots) is the CORE lightwalletd
# gRPC transport (tonic), present UNCONDITIONALLY with or without `swap-near`, so it
# is NOT a swap-droppable subtree — grepping for it here produced FALSE failures.
# The real off-switch guarantee is the ADAPTER's absence below (+ the manifest part
# (a) in feature_policy.rs); the swap adapter REUSES the same raw-`hyper` stack
# (deliberately not `reqwest`), so there is no swap-exclusive transport crate to drop
# — only the adapter code itself.
set -euo pipefail

# P0-11. Two defects this script shipped with, both fixed here:
#   (1) It looked for the staticlib under `../../../target/debug` — the REPO ROOT's
#       target from the bridge crate dir — but `sdk/` is excluded from the root
#       workspace and builds to `sdk/target`. One `../` too many, so `LIB` was
#       always empty and the artifact half never ran. The target dir is now asked
#       of cargo itself (`cargo metadata`, which also honours CARGO_TARGET_DIR),
#       so there is no relative path to miscount.
#   (2) An absent artifact or `nm` printed `SKIP … not available on this runner`
#       and exited 0 — a runner limitation in the wording, a green gate in effect.
#       It is now COULD-NOT-RUN, exit 2, naming the path it looked in: the repo's
#       convention (`gate_leg` reads 2 as "not graded, and RED").
# Wired into a lane by `just sdk-swap-off-guard` (a `sdk-gate` leg, nightly);
# `SWAP_OFF_GUARD_FEATURES` exists so the gate can be WATCHED: set it to "" and
# the feature is ON, and both halves below must FAIL.
#
# STAGE 2 (FR-5 C2, `tor-plugin.md` §8 P15 `the_wallet_library_carries_no_arti_symbol`):
# the SDK compiles no arti under any feature since FR-5 C1 removed `BuiltIn`; Tor
# is the host's, or the separate plugin's. The second stage reads the DEFAULT-feature
# artifact — the one Cargokit ships — and refuses any crate-anchored arti/dialer
# symbol in it. It sits in THIS script because it is the same shape (build, take the
# artifact from the build's own message, nm to a file, grade the file) and the same
# carrier; its stage 1 is untouched and still honours SWAP_OFF_GUARD_FEATURES.

ADAPTER="zec-wallet-swap-near"
# Swap-REST-client-class crates ONLY (mirrors feature_policy.rs::SWAP_ONLY_HTTP_CLIENTS).
SUBTREE=(reqwest native-tls openssl ureq curl)
FEATURES="${SWAP_OFF_GUARD_FEATURES---no-default-features}"

# Parsed with sed, not python3 (Batch D item 1): the gate self-test drives
# this script in a sandbox where python3 is a stub, and a stubbed parser turned
# a real answer into an empty one. `target_directory` is a top-level string key
# that cargo emits exactly once; the first match is it.
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' | head -1)" || TARGET_DIR=""
if [ -z "$TARGET_DIR" ]; then
  echo "COULD-NOT-RUN: cargo metadata did not answer, so the artifact directory is unknown (exit 2)"
  exit 2
fi

echo "== dep graph (swap-near OFF) must exclude the adapter + its swap-only HTTP subtree =="
# `--target all`: `cargo tree` defaults to the HOST triple, and these manifests
# carry `[target.'cfg(target_os = "android")'.dependencies]` tables — a
# target-gated edge to the adapter would be invisible on the macOS nightly
# (Batch B crypto pass). stderr is NOT discarded: a failing `cargo tree`
# ends the script through errexit, and the reason must be in the log.
# shellcheck disable=SC2086  # FEATURES is deliberately word-split (it may be empty)
TREE="$(cargo tree -p zec_wallet $FEATURES --edges normal --target all)"
# NOT VACUOUS: an empty or truncated tree matches nothing and would print "OK"
# (REVIEW.md §6, the shape P0-9 got a floor for). The tree must name the bridge
# crate itself and be a real graph, or the half is COULD-NOT-RUN.
tree_lines="$(printf '%s\n' "$TREE" | wc -l | tr -d ' ')"
if ! grep -qE '^zec_wallet v' <<<"$TREE" || [ "${tree_lines:-0}" -lt 20 ]; then
  echo "COULD-NOT-RUN: cargo tree answered $tree_lines line(s) and did not name zec_wallet — the dep-graph half was NOT graded (exit 2)"
  exit 2
fi
fail=0
for c in "$ADAPTER" "${SUBTREE[@]}"; do
  if grep -qE "(^|[[:space:]])${c}( |$| v)" <<<"$TREE"; then
    echo "FAIL: '$c' present in the feature-off dep graph"
    fail=1
  fi
done
[ "$fail" -eq 0 ] && echo "OK: dep graph clean"

echo "== built artifact (swap-near OFF) must carry no adapter symbols =="
BUILD_LOG="$(mktemp)"
# The artifact is taken FROM THE BUILD — cargo's own `compiler-artifact` message
# names the file it just wrote — never from a path assembled beside it. The
# Batch B review found the assembled `$TARGET_DIR/debug/libzec_wallet.a`
# grades a LEFTOVER host artifact whenever the build lands under a triple
# (`CARGO_BUILD_TARGET`, or a `--target` in SWAP_OFF_GUARD_FEATURES) — possibly
# the feature-ON one the watched mutant leaves behind — and prints "OK" over it.
# shellcheck disable=SC2086
if ! cargo build -p zec_wallet $FEATURES --lib --message-format=json-render-diagnostics >"$BUILD_LOG" 2>&1; then
  echo "FAIL: the feature-off build itself failed — last lines:"
  grep -v '^{' "$BUILD_LOG" | tail -20
  rm -f "$BUILD_LOG"
  exit 1
fi
# The LAST compiler-artifact message for the `zec_wallet` target that names a
# `libzec_wallet.a` — grep, not python3 (see the metadata parse above). Each
# artifact line carries its target's `"name":"…"` exactly once, so the filter
# is the same one the python parser applied: reason, target name, filename.
LIB="$(grep '"reason":"compiler-artifact"' "$BUILD_LOG" \
  | grep '"name":"zec_wallet"' \
  | grep -o '"[^"]*libzec_wallet\.a"' | tail -1 | tr -d '"')" || LIB=""
rm -f "$BUILD_LOG"
if [ -z "$LIB" ] || [ ! -f "$LIB" ]; then
  echo "COULD-NOT-RUN: the build's own artifact messages named no libzec_wallet.a (got '${LIB:-}') — the crate-type or the build changed; the symbol half was NOT graded (exit 2)"
  exit 2
fi
echo "note: grading the artifact THIS build wrote: $LIB (debug profile — Cargokit ships release; the symbol half is a host-debug proof)"
if ! command -v nm >/dev/null 2>&1; then
  echo "COULD-NOT-RUN: nm is not on PATH, so the symbol half was NOT graded (exit 2)"
  exit 2
fi
# (3) The THIRD defect, found by watching this half with the feature ON: it read
#     `nm "$LIB" | grep -q …` under `set -o pipefail`. On a match `grep -q` exits at
#     the first hit, `nm` (hundreds of MB of output on this staticlib) dies of
#     SIGPIPE, the pipeline's status is nm's 141, and the `if` takes the ELSE arm —
#     "OK: no adapter symbols" printed over an artifact carrying 4,536 of them. So
#     this half could never fail. nm now writes to a file, its own exit is checked,
#     and the count is read from the file.
SYMS="$(mktemp)"
NM_ERR="$(mktemp)"
# Apple's nm exits 1 on this archive while still listing ~600k symbols: a few
# std/panic_unwind members carry bitcode from a newer LLVM than Xcode's reader
# ("Unknown attribute kind"). The archive members OUR code lives in are read
# fine, so the grade is on the OUTPUT — an empty listing is COULD-NOT-RUN, a
# non-empty one is graded, and nm's own status plus its member errors are printed
# beside the verdict rather than hidden.
nm_rc=0
nm "$LIB" >"$SYMS" 2>"$NM_ERR" || nm_rc=$?   # errexit must not end the script here
sym_lines="$(wc -l <"$SYMS" | tr -d ' ')"
member_errors="$(grep -c 'error:' "$NM_ERR" || true)"
rm -f "$NM_ERR"
echo "note: nm exit $nm_rc, $sym_lines symbol lines, ${member_errors:-0} unreadable member(s)"
if [ "${sym_lines:-0}" -eq 0 ]; then
  rm -f "$SYMS"
  echo "COULD-NOT-RUN: nm listed NO symbols for $LIB (exit $nm_rc), so the symbol half was NOT graded (exit 2)"
  exit 2
fi
hits="$(grep -ciE 'zec_wallet_swap_near|chaindefuser|1click' "$SYMS" || true)"
rm -f "$SYMS"
if [ "${hits:-0}" -gt 0 ]; then
  echo "FAIL: $hits adapter symbol/string line(s) found in $LIB"
  fail=1
else
  echo "OK: no adapter symbols in $LIB"
fi

echo "== P15 (FR-5 C2): the DEFAULT-feature artifact must carry no arti / dialer-tor symbol =="
# CRATE-ANCHORED needles only (plan §5 D-2, §0.1 F8). The bare `arti` and `tor_` the
# spec first named match 3270 and 491 symbols of today's Tor-free archive — the
# vendored OpenSSL SQLCipher links (`_ossl_…_partial…`, `_ossl_rsa_mp_factor_names`,
# `_ossl_bn_generator_2`) — so they would be red on a clean tree. A crate name is
# what Rust mangling writes into every symbol a crate contributes, under both the
# legacy (`_ZN11arti_client…`) and v0 (`…11arti_client…`) schemes.
TOR_NEEDLES=(arti_client dialer_tor tor_proto tor_rtcompat tor_netdir tor_guardmgr tor_cell)
# Anti-vacuity: a zero-hit grep means something only over a REAL listing. The
# debug archive lists ~600k symbol lines; a listing under this floor
# is not the artifact the needles were measured against, and is COULD-NOT-RUN.
MIN_SYMBOL_LINES=100000
if [ "${#TOR_NEEDLES[@]}" -eq 0 ]; then
  echo "COULD-NOT-RUN: the P15 needle list is empty — nothing was graded (exit 2)"
  exit 2
fi
BUILD_LOG="$(mktemp)"
if ! cargo build -p zec_wallet --lib --message-format=json-render-diagnostics >"$BUILD_LOG" 2>&1; then
  echo "FAIL: the default-feature build itself failed — last lines:"
  grep -v '^{' "$BUILD_LOG" | tail -20
  rm -f "$BUILD_LOG"
  exit 1
fi
LIB="$(grep '"reason":"compiler-artifact"' "$BUILD_LOG" \
  | grep '"name":"zec_wallet"' \
  | grep -o '"[^"]*libzec_wallet\.a"' | tail -1 | tr -d '"')" || LIB=""
rm -f "$BUILD_LOG"
if [ -z "$LIB" ] || [ ! -f "$LIB" ]; then
  echo "COULD-NOT-RUN: the default-feature build's own artifact messages named no libzec_wallet.a (got '${LIB:-}') — P15 was NOT graded (exit 2)"
  exit 2
fi
echo "note: P15 grades the artifact THIS build wrote: $LIB (default features)"
SYMS="$(mktemp)"
NM_ERR="$(mktemp)"
nm_rc=0
nm "$LIB" >"$SYMS" 2>"$NM_ERR" || nm_rc=$?
sym_lines="$(wc -l <"$SYMS" | tr -d ' ')"
member_errors="$(grep -c 'error:' "$NM_ERR" || true)"
rm -f "$NM_ERR"
echo "note: nm exit $nm_rc, $sym_lines symbol lines, ${member_errors:-0} unreadable member(s)"
if [ "${sym_lines:-0}" -lt "$MIN_SYMBOL_LINES" ]; then
  rm -f "$SYMS"
  echo "COULD-NOT-RUN: nm listed $sym_lines symbol lines for $LIB, under the $MIN_SYMBOL_LINES floor — a zero-hit needle over that proves nothing, so P15 was NOT graded (exit 2)"
  exit 2
fi
tor_fail=0
for needle in "${TOR_NEEDLES[@]}"; do
  n="$(grep -cF "$needle" "$SYMS" || true)"
  echo "  $needle: ${n:-0}"
  if [ "${n:-0}" -gt 0 ]; then
    echo "FAIL: '$needle' names ${n} symbol line(s) in $LIB — first of them:"
    grep -F "$needle" "$SYMS" | head -3
    tor_fail=1
  fi
done
rm -f "$SYMS"
if [ "$tor_fail" -eq 0 ]; then
  echo "OK: P15 — no arti / dialer-tor symbol in $LIB (${#TOR_NEEDLES[@]} crate-anchored needles over $sym_lines lines)"
else
  fail=1
fi

exit "$fail"
