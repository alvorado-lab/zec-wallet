//! Pin-sync safety net (money/network review fold, S10e carve). The wallet
//! workspace (`sdk/Cargo.toml`) MIRRORS its dependency pins from the monorepo
//! root `[workspace.dependencies]` — the single source of truth pre-extraction.
//! A future bump that touches one copy but not the other is a SILENT
//! funds/crypto/TLS correctness hazard (a drifted `chacha20poly1305` /
//! `zcash_*` / `sha2` / `rustls` / `ring` pin between the seed-seal/checkpoint
//! code and the rest). This test fails on ANY drift of the EXACT-pinned
//! security set. Post-extraction (the SDK in its own repo, no monorepo root)
//! it SKIPS — `sdk/Cargo.toml` is then the sole owner.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The EXACT-pinned (`=x.y.z`) crypto + TLS deps where a version/feature drift
/// is a real correctness bug. Caret-floated deps (tokio/serde/hyper/…) resolve
/// independently per workspace by design and are deliberately NOT policed.
const POLICED: &[&str] = &[
    "chacha20poly1305",
    "zcash_address",
    "zcash_protocol",
    "bip39",
    "zip321",
    "sha2",
    "rustls",
    "ring",
    "tokio-rustls",
    "webpki-roots",
    "flutter_rust_bridge",
    "security-framework",
    "jni",
];

/// The monorepo root manifest — `Some` only when this IS the monorepo (it
/// carries the relay members); `None` post-extraction ⇒ the test skips.
fn root_manifest() -> Option<String> {
    // sdk/zec-wallet-core → sdk → repo root
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let s = fs::read_to_string(&p).ok()?;
    s.contains("crates/relim-relay-wire").then_some(s)
}

fn wallet_manifest() -> String {
    // sdk/zec-wallet-core → sdk
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"))
        .expect("sdk/Cargo.toml readable")
}

/// The local Tor dialer crate's manifest (`sdk/dialer-tor`, ADR-0550 — its own
/// workspace, §5 D-7). `Some` only while the crate is a sibling in this tree;
/// `None` once it is extracted to `alvorado-lab/dialer-tor` and consumed by
/// version, at which point its pins are that repo's to police.
fn dialer_tor_manifest() -> Option<String> {
    // sdk/zec-wallet-core → sdk → sdk/dialer-tor
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dialer-tor/Cargo.toml");
    let s = fs::read_to_string(&p).ok()?;
    s.contains("name = \"dialer-tor\"").then_some(s)
}

/// name → its `[workspace.dependencies]` spec, whitespace-normalized (so
/// formatting/comment differences between the two manifests never false-trip;
/// only the pin VALUE — version + features + default-features — is compared).
fn pins(manifest: &str) -> HashMap<String, String> {
    pins_in(manifest, "[workspace.dependencies]")
}

/// As `pins`, over an arbitrary dependency table. `sdk/dialer-tor` is its OWN
/// workspace (§5 D-7 — the resolver conflict arti's lock forces), so its pins
/// live in a plain `[dependencies]` table, not a workspace one.
fn pins_in(manifest: &str, table: &str) -> HashMap<String, String> {
    let mut in_wd = false;
    let mut out = HashMap::new();
    let mut pending: Option<(String, String, i32)> = None; // (name, text, depth)
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("");
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_wd = trimmed == table;
            continue;
        }
        if !in_wd || trimmed.is_empty() {
            continue;
        }
        if let Some((name, mut text, mut depth)) = pending.take() {
            // continuation of a multi-line spec
            text.push(' ');
            text.push_str(trimmed);
            depth += depth_delta(trimmed);
            if depth > 0 {
                pending = Some((name, text, depth));
            } else {
                out.insert(name, normalize(&text));
            }
            continue;
        }
        if let Some((name, rhs)) = trimmed.split_once('=') {
            let name = name.trim().to_string();
            let rhs = rhs.trim().to_string();
            let depth = depth_delta(&rhs);
            if depth > 0 {
                pending = Some((name, rhs, depth));
            } else {
                out.insert(name, normalize(&rhs));
            }
        }
    }
    out
}

fn depth_delta(s: &str) -> i32 {
    s.chars().fold(0, |d, c| match c {
        '{' | '[' => d + 1,
        '}' | ']' => d - 1,
        _ => d,
    })
}

/// Collapse all whitespace runs to a single space (compare values, not layout).
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn wallet_pins_mirror_root_or_extracted() {
    let Some(root) = root_manifest() else {
        return; // extracted to its own repo — sdk/Cargo.toml is the sole owner
    };
    let root_pins = pins(&root);
    let wallet_pins = pins(&wallet_manifest());
    for name in POLICED {
        let r = root_pins
            .get(*name)
            .unwrap_or_else(|| panic!("`{name}` missing from root [workspace.dependencies]"));
        let w = wallet_pins.get(*name).unwrap_or_else(|| {
            panic!("`{name}` missing from sdk/Cargo.toml [workspace.dependencies]")
        });
        assert_eq!(
            w, r,
            "PIN DRIFT on `{name}`: sdk/Cargo.toml does not mirror the root pin — re-sync \
             (the wallet workspace mirrors root's pins until extraction; a drifted crypto/TLS \
             pin is a silent funds-path correctness bug)"
        );
    }
}

/// FR-5 C2 — the reader the arch review angle on C0 owed (plan §2 C2, MINOR):
/// `sdk/dialer-tor` is its own workspace, so NOTHING above reads its manifest,
/// and it is the crate that brings arti — the SDK's largest body of network and
/// unsafe code — into every host that registers the plugin.
///
/// Two halves:
///   * **arti is pinned EXACTLY.** `arti-client` and `tor-rtcompat` carry `=`
///     pins, so a patch release of arti is new network code arriving by review,
///     never by `cargo update`. Policed against the crate's OWN manifest only:
///     this checkout's root carries no arti pin, so there is nothing to mirror
///     until Relim's `main` and `wallet-track` share a tree (plan §2 C2). Until
///     then the crate's `=` pins and its `deny.toml` are the police.
///   * **`rustls` AGREES across all three manifests.** The plan's row first
///     assumed the dialer's pin DIVERGED from the SDK's; it CONVERGED instead,
///     and in the opposite direction (measured 2026-09-18): Relim moved to OUR
///     `=0.23.45` because two `=` pins on one semver line make cargo refuse to
///     resolve at all. So the assertion is that the three specs are equal —
///     the dialer's `[dependencies]`, `sdk/Cargo.toml`'s workspace table, and
///     (while this is the monorepo) the root's. A drift between them is the
///     wallet's TLS and arti's TLS resolving to different rustls builds in one
///     host binary, or a lock that no longer resolves.
///
/// Watched red: the dialer's `arti-client` pin moved off `=0.45.0` (to a caret
/// `0.45.0`) → the exact-pin half names `arti-client`.
#[test]
fn the_dialer_tor_pins_arti_exactly_and_its_rustls_agrees_with_the_wallet() {
    let Some(dialer) = dialer_tor_manifest() else {
        return; // extracted — alvorado-lab/dialer-tor polices its own pins
    };
    let dialer_pins = pins_in(&dialer, "[dependencies]");
    assert!(
        dialer_pins.len() >= 5 && dialer_pins.contains_key("arti-client"),
        "sdk/dialer-tor/Cargo.toml's `[dependencies]` parsed to {} entries without \
         `arti-client` — the reader is not reading the table it thinks it is: {:?}",
        dialer_pins.len(),
        dialer_pins.keys().collect::<Vec<_>>()
    );

    for name in ["arti-client", "tor-rtcompat"] {
        let spec = dialer_pins
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` missing from sdk/dialer-tor [dependencies]"));
        // Both TOML spellings of an exact pin: the inline table (`{ version =
        // "=x", … }`) and the shorthand (`= "=x"`) — the reviewer found the
        // shorthand refused, a false red (C2 review, INFO).
        assert!(
            spec.contains("version = \"=") || spec.starts_with("\"="),
            "`{name}` is not EXACT-pinned in sdk/dialer-tor/Cargo.toml (got `{spec}`). arti \
             is the largest body of network code the SDK ships; a patch release must arrive \
             by review, never by `cargo update` — pin it with `=`."
        );
    }

    let dialer_rustls = dialer_pins
        .get("rustls")
        .expect("`rustls` is a direct dependency of sdk/dialer-tor (it names ONE crypto provider)");
    let wallet_rustls = pins(&wallet_manifest())
        .get("rustls")
        .cloned()
        .expect("`rustls` in sdk/Cargo.toml [workspace.dependencies]");
    assert_eq!(
        dialer_rustls, &wallet_rustls,
        "RUSTLS DRIFT between sdk/dialer-tor and sdk/Cargo.toml. The wallet's TLS and \
         arti's TLS to guards would resolve to different rustls builds in one host binary \
         — and two `=` pins on one semver line do not resolve at all. Move them together."
    );
    if let Some(root) = root_manifest() {
        let root_rustls = pins(&root)
            .get("rustls")
            .cloned()
            .expect("`rustls` in the root [workspace.dependencies]");
        assert_eq!(
            dialer_rustls, &root_rustls,
            "RUSTLS DRIFT between sdk/dialer-tor and the monorepo root — the three pins \
             converged on one value at Relim's `dialer-tor` flip (2026-09-18) and must move \
             together."
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// `incrementalmerkletree` is a DEV-dep here, and this is the guard that says so
// ─────────────────────────────────────────────────────────────────────────────

/// The maintainer's 2026-09-12 decision declared `incrementalmerkletree` as a
/// dev-dependency (A6/A6b's spendability + anchor drive, and BIND-1-R's
/// `witness_stabilized` clause — `…-phase-1.md` §4b) and named the guard as
/// *"a `cargo tree -e features` check that it reaches no production target"*.
///
/// **THAT GUARD IS UNSATISFIABLE AS STATED, AND THE MEASUREMENT IS WHY — it is
/// recorded rather than quietly re-worded.** `cargo tree -e no-dev -i
/// incrementalmerkletree` from `sdk/` (measured) answers that the crate is
/// ALREADY a production transitive dependency and was before this declaration:
/// `incrementalmerkletree v0.8.2 → orchard v0.15.5 → {zcash_keys, zcash_client_backend,
/// zcash_client_sqlite} → zec-wallet-core`. Seven occurrences in
/// `cargo tree -e no-dev -p zec-wallet-core`. A test asserting it reaches no
/// production target would fail on the tree as it stands, and one written to
/// pass would have to assert something else while claiming that.
///
/// **What the decision actually buys, and what this guard actually checks.**
/// Because the crate is already in the shipped graph, declaring it adds ZERO to
/// that graph — which is the whole reason the decision was cheap. The risk the
/// maintainer's sentence was aimed at is the real one and it is checkable: that a
/// dev-only test convenience quietly becomes something **our own production code
/// calls**. That is a manifest fact, not a graph fact. `incrementalmerkletree`
/// must appear in the SDK's crate manifests under `[dev-dependencies]` **only** —
/// never `[dependencies]`, never `[build-dependencies]`, in any SDK crate — and
/// cargo then makes `use incrementalmerkletree::…` in non-test `src/` a compile
/// error, which is a stronger enforcement than any assertion here.
///
/// Watched red: move the `zec-wallet-core` line from `[dev-dependencies]` to
/// `[dependencies]` and this fails naming that crate and that table.
///
/// **Why this guard lives in `pins_policy.rs` and not in a file of its own** (the
/// arch review asked; the reason was real and written nowhere): the nightly's
/// carrier hand-names the test binaries it runs — `sdk-gate-core`'s leg is
/// `cargo test -p zec-wallet-core --lib --test extraction_policy --test pins_policy`
/// — and `supply_chain_policy::every_sdk_test_target_is_named_by_a_live_carrier`
/// refuses a `tests/*.rs` that no recipe names. A new file here is therefore a
/// Justfile change AND a gate change, and a guard that "cleaned up" its placement
/// into `tests/dev_dep_policy.rs` would either fail that gate or, with the gate
/// edited to match, run in no carrier until someone wired it. It sits beside the
/// pin guards because the pin guards are already carried.
///
/// **Two holes the first pass found, closed (§4ac rows 8):** the member list
/// was hardcoded (a fifth SDK crate would be scanned by nothing), and a RENAMED
/// declaration — `imt = { package = "incrementalmerkletree", … }` — was invisible
/// to a key-name match. The members now come from `[workspace] members` in
/// `sdk/Cargo.toml`, and a line counts as a declaration when its key is the crate
/// OR its `package = "…"` names it OR it opens a `[<table>.incrementalmerkletree]`
/// sub-table. Watched: the renamed form under `[dependencies]` reds.
#[test]
fn incrementalmerkletree_is_declared_as_a_dev_dependency_only() {
    const DEP: &str = "incrementalmerkletree";
    let sdk = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    // Every crate manifest in the sdk workspace — read from the workspace itself,
    // so a member added tomorrow is scanned tomorrow. The root manifest's
    // `[workspace.dependencies]` table is a pin declaration, not an edge, and is
    // not a member.
    let crates = workspace_members(&sdk);
    assert!(
        crates.iter().any(|m| m == "zec-wallet-core") && crates.len() >= 2,
        "sdk/Cargo.toml's `[workspace] members` did not parse to a list containing \
         zec-wallet-core (got {crates:?}) — the scan below would then examine nothing"
    );

    let mut declared_dev_in = Vec::new();
    for member in &crates {
        let path = sdk.join(member).join("Cargo.toml");
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("sdk/{member}/Cargo.toml readable: {e}"));
        let mut table = String::new();
        for raw in text.lines() {
            // Strip comments first — the rationale comment beside the declaration
            // names the crate, and a comment is not a dependency edge.
            let line = raw.split('#').next().unwrap_or("");
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                let header = trimmed.trim_matches(['[', ']']).to_string();
                // `[dev-dependencies.incrementalmerkletree]` — the sub-table form
                // of a declaration; the table it belongs to is the parent path.
                if header.rsplit('.').next().unwrap_or("") == DEP {
                    let parent = header.rsplit_once('.').map(|(p, _)| p).unwrap_or("");
                    assert_dev_table(DEP, member, parent);
                    declared_dev_in.push(member.clone());
                }
                table = header;
                continue;
            }
            // Every cargo-legal spelling of "this line declares DEP" (the Batch B
            // review found three the inline-table match could not see):
            //   name = …                    name.workspace = …
            //   alias = { package = "name", … }        (inline table)
            //   alias.package = "name"                 (dotted key)
            //   [dependencies.alias] … package = "name" (sub-table; the table is the
            //                                          header's PARENT)
            // Whitespace around `=` and the quote style (`"` or `'`) carry no meaning
            // in TOML, so the line is normalised before it is read.
            let normalised: String = trimmed
                .chars()
                .filter(|c| !c.is_whitespace())
                .map(|c| if c == '\'' { '"' } else { c })
                .collect();
            let Some((key, value)) = normalised.split_once('=') else {
                continue;
            };
            let package_eq_dep = format!("package=\"{DEP}\"");
            let key_names_it = key.split('.').next().unwrap_or("") == DEP;
            let inline_table_names_it = value.contains(&package_eq_dep);
            let dotted_key_names_it = key.ends_with(".package") && value == format!("\"{DEP}\"");
            let sub_table_names_it = key == "package" && value == format!("\"{DEP}\"");
            if sub_table_names_it {
                // `package = "DEP"` inside `[<table>.alias]`: the declaration's table
                // is the header's parent, not the alias sub-table itself.
                let parent = table.rsplit_once('.').map(|(p, _)| p).unwrap_or("");
                assert_dev_table(DEP, member, parent);
                declared_dev_in.push(member.clone());
                continue;
            }
            if !(key_names_it || inline_table_names_it || dotted_key_names_it) {
                continue;
            }
            assert_dev_table(DEP, member, &table);
            declared_dev_in.push(member.clone());
        }
    }

    // Not vacuous: the declaration must actually be somewhere, or this test
    // passes over a manifest that no longer has it and says nothing (the
    // shape — a scan whose subject vanished still reports green).
    assert_eq!(
        declared_dev_in,
        vec!["zec-wallet-core".to_string()],
        "`{DEP}`'s dev-dependency declaration is not where this guard expects it \
         (found in {declared_dev_in:?}). It is declared in zec-wallet-core ONLY — if a \
         second crate needs it, widen this expectation deliberately rather than letting \
         the guard pass over a crate it never looked at."
    );

    /// Accept the dev table, and only it. `target.'cfg(…)'.dev-dependencies` is a
    /// dev table too — match on the last path segment.
    fn assert_dev_table(dep: &str, member: &str, table: &str) {
        let leaf = table.rsplit('.').next().unwrap_or("");
        assert_eq!(
            leaf, "dev-dependencies",
            "`{dep}` is declared in sdk/{member}/Cargo.toml under `[{table}]`. It is a \
             TEST-ONLY declaration (the founder's 2026-09-12 decision: the hand-built \
             Ironwood prior-roots frontier the pinned `InitialChainState` has no field \
             for) and it must stay in `[dev-dependencies]`. Moving it makes \
             `use {dep}::…` legal in production `src/`, which is exactly the thing the \
             decision's guard exists to refuse."
        );
    }

    /// The `[workspace] members = [...]` list of `sdk/Cargo.toml`, as written
    /// (one line, quoted paths). FAILS if the table is not found: a guard that
    /// silently scanned zero members is the shape.
    fn workspace_members(sdk: &Path) -> Vec<String> {
        let root = fs::read_to_string(sdk.join("Cargo.toml")).expect("sdk/Cargo.toml readable");
        let mut in_workspace = false;
        for raw in root.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.starts_with('[') {
                in_workspace = line == "[workspace]";
                continue;
            }
            if !in_workspace {
                continue;
            }
            if let Some(rest) = line.strip_prefix("members") {
                let rest = rest.trim_start().strip_prefix('=').unwrap_or("").trim();
                let inner = rest
                    .strip_prefix('[')
                    .and_then(|r| r.strip_suffix(']'))
                    .expect("`members = [ … ]` on one line, as sdk/Cargo.toml writes it");
                return inner
                    .split(',')
                    .map(|m| m.trim().trim_matches('"').to_string())
                    .filter(|m| !m.is_empty())
                    .collect();
            }
        }
        panic!("sdk/Cargo.toml has no `[workspace] members = [...]` line — nothing to scan");
    }
}

/// (the supply-chain pass on the `oslog` pin): the bridge's PLATFORM-SCOPED
/// dependencies — the `[target.'cfg(…)'.dependencies]` tables of
/// `zec_wallet/rust/Cargo.toml` — are pinned EXACTLY. They are the crate's tiny
/// unsafe-FFI shims (`paranoid-android` over `__android_log_write`, `oslog` over
/// the `os_log` macros), each read in full before its pin, and each pin is
/// LOAD-BEARING beyond supply chain: `paranoid-android`'s writer `unwrap()`s a
/// `flush` that cannot fail only at 0.2.2, inside a `tracing` dispatch. The rule
/// (the project's Rust rules: "pin tiny unsafe-FFI crates with `=`") was
/// a comment beside each line and nothing else — `wallet_pins_mirror_root_…`
/// polices the WORKSPACE table, and a platform table is not in it.
///
/// A `workspace = true` entry is skipped: its version is the workspace table's,
/// policed above. Anti-vacuity: both platform tables are found, and the pinned
/// set is exactly the two shims — a third platform dependency lands here for a
/// decision, not by default.
///
/// Mutant: `oslog`'s `"=0.2.0"` → `"0.2.0"` → red, naming the crate.
#[test]
fn the_bridges_platform_scoped_dependencies_are_pinned_exactly() {
    let manifest = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../zec_wallet/rust/Cargo.toml"),
    )
    .expect("the bridge manifest is readable");
    let tables: Vec<&str> = manifest
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("[target.") && l.ends_with(".dependencies]"))
        .collect();
    assert_eq!(
        tables.len(),
        2,
        "the bridge has two platform tables (Android, Apple); found {tables:?}"
    );

    let mut pinned = Vec::new();
    for table in tables {
        for (name, spec) in pins_in(&manifest, table) {
            if spec.contains("workspace = true") {
                continue;
            }
            // `"=1.2.3"`, or `{ version = "=1.2.3", … }`
            let version = spec
                .split_once("version =")
                .map_or(spec.as_str(), |(_, rest)| rest)
                .trim()
                .trim_start_matches('"');
            assert!(
                version.starts_with('='),
                "{name} in {table} must be pinned with `=` (got `{spec}`): a platform FFI shim's \
                 patch bump returns to review, it is never inherited"
            );
            pinned.push(name);
        }
    }
    pinned.sort();
    assert_eq!(
        pinned,
        ["oslog", "paranoid-android"],
        "the bridge's platform-scoped, non-workspace dependencies are exactly the two log shims"
    );
}
