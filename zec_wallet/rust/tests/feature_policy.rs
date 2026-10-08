//! Feature-pluggability policy — the manifest-shape half of the §8 row
//! `swap_near_feature_off_ships_no_adapter_symbols` (spec §3.5 layer 1 at the
//! ARTIFACT level; ADR-0525). DETERMINISTIC, no nested cargo: it proves
//! STRUCTURALLY that
//!   (a) the NEAR adapter is OPTIONAL and reachable ONLY via the `swap-near`
//!       feature (no other feature/default-direct edge pulls it), and
//!   (b) the SWAP REST CLIENT's HTTP-client subtree (`reqwest` et al.) has no
//!       direct path into the core or the bridge — the adapter is its only
//!       gateway, so a `--no-default-features` build carries none of it.
//!
//! CORRECTED 2026-06-23 (was born-red since the `swap-near` commit added
//! `hyper-util` to the denylist while the gRPC seam declared it in core): the
//! wallet's CORE lightwalletd transport is `tonic`, which pulls
//! `hyper`/`hyper-util`/`rustls`/`tokio-rustls`/`ring`/`webpki-roots`
//! UNCONDITIONALLY — and `zec-wallet-core` declares `hyper-util` DIRECTLY for the
//! `NetDialer`→tonic `TokioIo` bridge in `net/grpc.rs`. That base hyper/rustls
//! stack is therefore CORE and SHARED — present with or without `swap-near`, and
//! NOT droppable by turning swap off (the swap adapter REUSES the same raw-`hyper`
//! stack, deliberately not `reqwest`). The original `(b)` ("core declares NO
//! HTTP/TLS at all") predated the gRPC seam and became structurally
//! unsatisfiable once the wallet learned to talk to lightwalletd; this gate now
//! bans only the swap-REST-client-class crates ([`SWAP_ONLY_HTTP_CLIENTS`]) in
//! core/bridge, recognising the shared gRPC stack ([`CORE_GRPC_TRANSPORT`]) as
//! legitimately core. The REAL swap off-switch guarantee is `(a)` (adapter
//! optional + only `swap-near` enables it) plus the symbol-level runner script
//! `tests/swap_near_off_no_symbols.sh` (`cargo tree --no-default-features`, owed
//! to a CI runner) — the AUTHORITATIVE dep-graph proof; this scan is
//! defense-in-depth.

use std::fs;
use std::path::{Path, PathBuf};

const ADAPTER: &str = "zec-wallet-swap-near";

/// The SWAP REST client's HTTP-client-class crates — whose presence in the core
/// or bridge would mean a swap (or other non-gRPC) HTTP path leaked in. These are
/// NOT pulled by the core lightwalletd gRPC transport (`tonic` uses the
/// lower-level `hyper` directly, never a high-level client), so a direct
/// declaration of one in core/bridge is a real leak. DEFENSE-IN-DEPTH (a denylist
/// of known names): the AUTHORITATIVE proof remains the resolved dep-graph absence
/// (`cargo tree --no-default-features` in `swap_near_off_no_symbols.sh`).
const SWAP_ONLY_HTTP_CLIENTS: &[&str] = &["reqwest", "native-tls", "openssl", "ureq", "curl"];

/// Transport crates the CORE lightwalletd gRPC stack (`tonic`) pulls
/// UNCONDITIONALLY — present with or without `swap-near`, so NOT a swap-droppable
/// subtree. A direct declaration of one of these in the core is CORRECT (e.g.
/// `zec-wallet-core` declares `hyper-util` for the `NetDialer`→tonic `TokioIo` bridge
/// in `net/grpc.rs`); the swap adapter REUSES this same stack (raw `hyper` over
/// `NetDialer`, deliberately NOT `reqwest`), so none of it is swap-exclusive.
/// Allow-listed so the gate's model — "core gRPC transport" vs "swap REST client"
/// — is explicit and a future reviewer sees WHY these are not a swap leak.
const CORE_GRPC_TRANSPORT: &[&str] = &[
    "hyper",
    "hyper-util",
    "rustls",
    "tokio-rustls",
    "webpki-roots",
    "ring",
];

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// `(section_header_without_brackets, line)` for every comment-stripped,
/// non-empty, non-header line. Mirrors the adapter `tests/policy.rs` idiom but
/// section-aware so it can reason about `[features]` and `[target.*]` tables.
fn entries(manifest: &str) -> Vec<(String, String)> {
    let mut header = String::new();
    let mut out = Vec::new();
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim().to_owned();
        if line.is_empty() {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            header = inner.trim().to_owned();
            continue;
        }
        out.push((header.clone(), line));
    }
    out
}

/// The crate name a dependency line names: `tokio = { ... }` / `tokio = "1"` →
/// `tokio`. Assumes Cargo-standard whitespace around `=` (which `cargo fmt`
/// always emits); a compact `name={…}` would defeat the split — the "upgrade
/// to a structural TOML parse" note above applies here too.
fn dep_name(line: &str) -> &str {
    line.split(['=', ' ']).next().unwrap_or("").trim()
}

/// A dependency table that contributes to the SHIPPED artifact: `[dependencies]`
/// and any `[target.*.dependencies]`, but NOT dev/build tables (they never reach
/// the cdylib).
fn is_shipped_dep_table(header: &str) -> bool {
    let h = header.trim();
    if h.contains("dev-dependencies") || h.contains("build-dependencies") {
        return false;
    }
    h == "dependencies" || h.ends_with(".dependencies")
}

fn bridge_manifest() -> String {
    read(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
}

fn core_manifest() -> String {
    // CARGO_MANIFEST_DIR = .../sdk/zec_wallet/rust ; the core is the path dep.
    read(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../zec-wallet-core/Cargo.toml"))
}

/// Assert no shipped dependency table in `manifest` declares a SWAP-REST-client
/// HTTP crate directly (the adapter itself is allowed — it is the GATEWAY,
/// optional and feature-gated, not a member of the banned subtree). The shared
/// core gRPC transport stack ([`CORE_GRPC_TRANSPORT`]) is NOT banned — it is
/// present unconditionally and is not swap-droppable (see the module doc).
fn no_direct_swap_http(manifest: &str, who: &str) {
    for (header, line) in entries(manifest) {
        if !is_shipped_dep_table(&header) {
            continue;
        }
        let name = dep_name(&line);
        assert!(
            !SWAP_ONLY_HTTP_CLIENTS.contains(&name),
            "{who}: swap-REST-client HTTP crate `{name}` declared directly in \
             [{header}] — feature-off `swap-near` would NOT drop it (it is not part \
             of the core gRPC transport): {line}"
        );
    }
}

#[test]
fn swap_near_feature_off_ships_no_adapter_symbols() {
    let bridge = bridge_manifest();
    let bridge_entries = entries(&bridge);

    // (a.1) the adapter is declared OPTIONAL, and only in the normal
    // `[dependencies]` table (never a hard/target dep).
    let adapter_lines: Vec<&(String, String)> = bridge_entries
        .iter()
        .filter(|(_, l)| dep_name(l) == ADAPTER)
        .collect();
    assert_eq!(
        adapter_lines.len(),
        1,
        "the adapter must be declared exactly once; found {adapter_lines:?}"
    );
    let (adapter_header, adapter_line) = adapter_lines[0];
    assert_eq!(
        adapter_header, "dependencies",
        "the adapter must live in [dependencies], not a target/hard table: {adapter_line}"
    );
    assert!(
        adapter_line.contains("optional = true"),
        "the adapter dep MUST be `optional = true` (else it links unconditionally): {adapter_line}"
    );

    // (a.2) the `swap-near` feature is the ONLY enabler, and it enables ONLY
    // the adapter via the `dep:` syntax (so the dep cannot be implicitly
    // turned on by any other feature edge).
    let features: Vec<&(String, String)> = bridge_entries
        .iter()
        .filter(|(h, _)| h == "features")
        .collect();
    // This gate reasons about feature arrays as SINGLE LINES. Guard that
    // assumption LOUDLY: a multi-line array (a reformat) would scatter a
    // re-enabling `"dep:…"` token onto a continuation line that the per-line
    // scan below could miss. Any feature value that opens `[` without closing
    // `]` on the same line fails here with an actionable message.
    for (_, line) in &features {
        if let Some((_, val)) = line.split_once('=') {
            let val = val.trim();
            // a complete single-line array: opens `[` ⇒ must close `]` here
            assert!(
                !val.starts_with('[') || val.ends_with(']'),
                "feature arrays MUST be single-line for this gate to reason about \
                 them — reformat `{line}` onto one line (or upgrade this test to a \
                 structural TOML parse)"
            );
        }
    }
    let swap_near = features
        .iter()
        .find(|(_, l)| dep_name(l) == "swap-near")
        .unwrap_or_else(|| panic!("no `swap-near` feature declared"));
    let rhs = swap_near
        .1
        .split_once('=')
        .expect("feature = value")
        .1
        .trim();
    assert_eq!(
        rhs, "[\"dep:zec-wallet-swap-near\"]",
        "`swap-near` must enable EXACTLY the adapter dep: got {rhs}"
    );
    // default must include swap-near (published SDK is feature-complete) ...
    let default = features
        .iter()
        .find(|(_, l)| dep_name(l) == "default")
        .expect("a default feature list");
    assert!(
        default.1.contains("swap-near"),
        "default features must include `swap-near`: {}",
        default.1
    );
    // ... and NO OTHER feature may reference the adapter (reachable ONLY via
    // `swap-near`), so an unrelated feature flip can never resurrect it.
    for (_, line) in &features {
        if dep_name(line) == "swap-near" {
            continue;
        }
        assert!(
            !line.contains(ADAPTER) && !line.contains("zec_wallet_swap_near"),
            "feature other than `swap-near` references the adapter: {line}"
        );
    }

    // The two transport classes MUST be disjoint — a crate is either the shared
    // core gRPC stack or a swap-only HTTP client, never both — so the allow-list
    // can never accidentally un-ban a genuine swap-leak crate.
    for c in CORE_GRPC_TRANSPORT {
        assert!(
            !SWAP_ONLY_HTTP_CLIENTS.contains(c),
            "`{c}` is in BOTH CORE_GRPC_TRANSPORT and SWAP_ONLY_HTTP_CLIENTS — \
             the allow-list would mask a swap leak"
        );
    }

    // (b) the SWAP REST CLIENT's HTTP subtree has no direct path into the bridge
    // or the core (its only other path dep). The shared core gRPC transport
    // (`hyper`/`rustls` via tonic) is NOT checked here — it is unconditional and
    // not swap-droppable (module doc). With the adapter gone (feature-off) nothing
    // re-introduces the swap REST client.
    no_direct_swap_http(&bridge, "bridge");
    no_direct_swap_http(&core_manifest(), "zec-wallet-core (core)");
}
