//! §3.5 swap kill-door composition gate (wallet-sdk §8 register:
//! `no_swap_traffic_path_bypasses_the_kill_door`).
//!
//! A SOURCE-shape gate over `src/swap_provider.rs` — the ONE place the concrete
//! NEAR adapter is named and wired into the wallet. The BEHAVIORAL halves run
//! where the live infrastructure lives: the kill monotonicity + `set_kill`
//! semantics in zec-wallet-core's `swap::service` tests, the dialer fail-closed /
//! shared-`tor_fell_back` behavior in `swap_dialer` + `net::dialer` tests, and
//! `NearIntentsProvider::new` (TLS over the injected dialer) in the adapter's
//! tests. This gate guards the COMPOSITION's structural invariant that no
//! behavioral test can see: that the constructed `Arc<dyn SwapPort>` is consumed
//! by `SwapService` and never escapes, and that the kill door is applied — so a
//! refactor that returned the bare provider (callable past a `Hard` kill, §3.5)
//! or dropped the `set_kill` would fail CI here, not silently in production.
//!
//! It is feature-INDEPENDENT (reads source text, not symbols), so it runs even
//! when `swap-near` is off — the file is always present.

use std::fs;
use std::path::PathBuf;

fn swap_provider_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/swap_provider.rs");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("gate must read {}: {e}", path.display()));
    // ENFORCE the line-granularity assumption `strip_comments` relies on: a block
    // comment that OPENS mid-line (`let x = …; /* … */`) would NOT be stripped, so a
    // future `/* never yields Arc<dyn SwapPort> */` riding a code line could falsely
    // satisfy (or trip) the scan. The current file uses only line-start `//`/`/* */`
    // comments; assert that stays true so the latent gap can never become a live one.
    for (i, line) in raw.lines().enumerate() {
        let trimmed = line.trim_start();
        if line.contains("/*") && !trimmed.starts_with("/*") {
            panic!(
                "swap_provider.rs:{} opens a block comment mid-line; this gate's \
                 line-granularity `strip_comments` can't see past it — keep block \
                 comments line-start (or `//`)",
                i + 1
            );
        }
    }
    strip_comments(&raw)
}

/// The gate polices CODE, not prose: drop `//`-style and block-comment lines so
/// the module doc may freely NAME the hazards it forbids ("never yields a bare
/// `Arc<dyn SwapPort>`") without tripping the scan. LINE-GRANULARITY by design —
/// it strips whole comment lines, NOT mid-line block comments; `swap_provider_src`
/// enforces that no mid-line `/*` exists so the assumption can't silently break.
fn strip_comments(src: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    for line in src.lines() {
        let trimmed = line.trim_start();
        if in_block {
            if trimmed.contains("*/") {
                in_block = false;
            }
            continue;
        }
        if trimmed.starts_with("/*") {
            if !trimmed.contains("*/") {
                in_block = true;
            }
            continue;
        }
        // `//` line comments, and `*`-led block-comment continuation lines (the
        // javadoc `* …` style). Rust code never starts a STATEMENT with a bare `*`
        // (a deref is `*x = …`, never line-leading in this file — and the mid-line
        // `/*` guard in `swap_provider_src` keeps interior `*` lines inside a real
        // block), so this drops only comment continuations, never code.
        if trimmed.starts_with("//") || trimmed.starts_with("*") {
            continue;
        }
        let code = line.split("//").next().unwrap_or(line);
        out.push_str(code);
        out.push('\n');
    }
    out
}

#[test]
fn no_swap_traffic_path_bypasses_the_kill_door() {
    let src = swap_provider_src();

    // (1) The provider rides the WALLET's transport — constructed over the resolved
    // `swap_dialer()`, never a fresh/independent dialer (no separate egress, §3.2 m5).
    assert!(
        src.contains("NearIntentsProvider::new(") && src.contains("swap_dialer()"),
        "the provider must be constructed over the wallet's own resolved dialer \
         (`NearIntentsProvider::new(config, wallet.swap_dialer()?)`)"
    );

    // (2) The constructed provider is handed to `enable_swap` — owned by
    // `SwapService`, behind the kill switch. This is the consume that makes the
    // kill door reachable for all swap traffic.
    let enable_at = src
        .find("enable_swap(")
        .expect("the provider must be handed to `Wallet::enable_swap` (kill-door consume)");

    // (3) The kill door is APPLIED after the consume: the host's manifest flag is
    // lowered through `resolve_manifest_kill` into `set_kill` on the returned service
    // (`set_kill(resolve_manifest_kill(...))` — the resolve nests INSIDE the apply).
    assert!(
        src.contains("resolve_manifest_kill("),
        "the host swap-kill manifest flag must be lowered via `resolve_manifest_kill`"
    );
    let set_kill_at = src
        .find("set_kill(")
        .expect("the resolved severity must be applied via `SwapService::set_kill`");
    assert!(
        enable_at < set_kill_at,
        "the kill door must be applied (`set_kill`) AFTER the provider is attached (`enable_swap`)"
    );

    // (4) The bare provider NEVER escapes: the composition returns `()`, not the
    // `Arc<dyn SwapPort>` (or the service). A retained provider handle would be
    // callable directly, past a `Hard` kill (§3.5). Assert the function signature
    // returns `Result<(), …>` and yields no SwapPort/provider type to a caller.
    assert!(
        src.contains("-> Result<(), SwapError>"),
        "`enable_near_swap` must return `Result<(), SwapError>` — never a provider handle"
    );
    assert!(
        !src.contains("-> Arc<dyn SwapPort>")
            && !src.contains("-> Arc<dyn zec_wallet_core::SwapPort>"),
        "the composition must NEVER return a bare `Arc<dyn SwapPort>` (it would bypass the kill door)"
    );
}
