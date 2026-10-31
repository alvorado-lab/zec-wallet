//! Pin-sync safety net (money/network review fold, S10e carve). The wallet
//! workspace (`sdk/Cargo.toml`) MIRRORS its dependency pins from the monorepo
//! root `[workspace.dependencies]` — the single source of truth pre-extraction.
//! A future bump that touches one copy but not the other is a SILENT
//! funds/crypto/TLS correctness hazard (a drifted `chacha20poly1305` /
//! `zcash_*` / `sha2` / `rustls` / `ring` pin between the seed-seal/checkpoint
//! code and the rest). This test fails on ANY drift of the EXACT-pinned
//! security set. Post-extraction (the SDK in its own repo, no monorepo root)
//! it SKIPS — `sdk/Cargo.toml` is then the sole owner.

use std::collections::{BTreeMap, HashMap};
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

// ─────────────────────────────────────────────────────────────────────────────
// The NU7 wave's supply-chain gates (ADR-0579 Decisions 2 and 3)
// ─────────────────────────────────────────────────────────────────────────────

use serde_json::Value; // Here, not at the top: registry rows cite the lines above.
use std::collections::BTreeSet;
use std::process::Command;
use std::time::Instant;

/// The cfgs no shipped build may carry. ONE array, and the scanner below reads
/// nothing else (ADR-0579 Decision 2):
///   * `zcash_unstable` turns on librustzcash's unreleased consensus code
///     (`nutachyon` needs it). ADR-0540 said a pins-policy row kept it out of
///     shipped builds; no such row existed until this one.
///   * `getrandom_backend` replaces the operating system's generator inside
///     `getrandom`, the generator the wallet proves and signs with (Decision 3),
///     and it does so silently: `--cfg getrandom_backend="custom"` is all it
///     takes (`getrandom-0.4.2/src/backends.rs:11`).
///   * `secp256k1_fuzz` replaces libsecp256k1 with the fuzzing stubs in
///     `secp256k1-sys`, whose `secp256k1_ec_pubkey_create` copies the secret key into
///     the public key (`secp256k1-sys-0.14.1/src/lib.rs:1518`, the stubs' module,
///     and `:1736-1746`), so a transparent spend would publish its key. A name with
///     no value: `secp256k1-0.33.1` and `secp256k1-sys-0.14.1`, the one locked
///     version of each in every lock under `sdk/` that has them, read it only as
///     `cfg(secp256k1_fuzz)` and declare it so (their `Cargo.toml:153` and `:79`).
///     Added at step 1g's second review fold; ADR-0579 Decision 2's list grows by
///     one here, owed to the next ADR that touches it.
///   * `hashes_fuzz` makes `bitcoin_hashes`' SHA-1, SHA-256, SHA-512 and RIPEMD-160
///     engines fold their input into one byte (their shared `input`,
///     `bitcoin_hashes-0.14.101/src/util.rs:88-94`); HMAC, sha256d, sha384, sha512_256
///     and hash160 inherit it, and SipHash-2-4 does not (`src/siphash24.rs:144`).
///     `bip39` 2.2.2 computes a mnemonic's checksum on them (`src/lib.rs:222`), checks
///     it (`:471`) and makes its seed with them; a mnemonic wallet's seed-derived keys
///     all come from it (`derivation.rs`; a raw seed never meets them). A
///     name with no value: 0.14.101, the one locked version in `sdk/Cargo.lock` and
///     both fuzz locks, reads it only as `cfg(hashes_fuzz)` (19 sites in `src/`) and
///     declares it (`Cargo.toml:138`). Added at step 1g's third fold, owed to that ADR.
const FORBIDDEN_CFGS: [&str; 4] = [
    "zcash_unstable",
    "getrandom_backend",
    "secp256k1_fuzz",
    "hashes_fuzz",
];

/// The `getrandom_backend` values that replace the operating system's generator,
/// the ones `rng.rs` refuses to compile under. Read in
/// `getrandom-0.4.2/src/backends.rs:11-37`, the `cfg_if!` that picks the source
/// before any per-OS arm: `custom` (:11), `rdrand` (:20), `rndr` (:23), `efi_rng`
/// (:26), `unsupported` (:32) and `extern_impl` (:35). The three other values it
/// names, `linux_getrandom` (:14), `linux_raw` (:17) and `windows_legacy` (:29),
/// still read the operating system. Read at `GETRANDOM_PIN`, which the generator
/// guard holds every declaration to, so a new `getrandom` reds until this list is
/// read again.
///
/// `getrandom` 0.3.4 is locked too (`rand_core` 0.9.5 <- `rand` 0.9.5 <-
/// `secp256k1` 0.33.1) and reads the same cfg (`getrandom-0.3.4/src/backends.rs:11-47`).
/// Its replacing values are this list's but `extern_impl`, plus `wasm_js` (:34),
/// which refuses itself on a native target (`src/backends/wasm_js.rs:7-8`), so the
/// refusal covers it on every target the SDK ships. Nothing ties a 0.3.x bump to
/// this list: the pin and `GETRANDOM_PIN` hold 0.4.2 only. `getrandom` 0.2.17 is
/// locked as well (`ring`, `rand_core` 0.6.4, `nanorand`) and reads no cfg: it
/// changes its source by feature (`custom`, `rdrand`), which is the plan's owed
/// resolved-features item, not this list's.
const REPLACING_BACKENDS: [&str; 6] = [
    "custom",
    "extern_impl",
    "rdrand",
    "rndr",
    "efi_rng",
    "unsupported",
];

/// The `getrandom` version `REPLACING_BACKENDS` was read at, as every SDK
/// declaration must spell it.
const GETRANDOM_PIN: &str = "=0.4.2";

/// The `zcash_unstable` values the pinned crates read, the ones `lib.rs` refuses
/// to compile under. Read by grepping the source of every package in
/// `sdk/Cargo.lock` for `zcash_unstable` (2026-10-23): code reads `nutachyon` in
/// `zcash_protocol-0.11.0` (32 sites, e.g. `src/consensus.rs`),
/// `zcash_primitives-0.31.0` (40), `pczt-0.10.0` (19), `zcash_client_sqlite-0.23.0`
/// (3, e.g. `src/wallet/common.rs:1258`), `zcash_client_backend-0.25.0`
/// (`src/data_api/testing.rs:1884`) and `zcash_pool_migration-0.2.0`
/// (`src/engine.rs:4944`), and `zip233` in `zcash_primitives-0.31.0` (48) and
/// `zcash_client_sqlite-0.23.0` (`src/wallet.rs:3226`, `:3237`). No code reads any
/// other value. `equihash-0.3.0` (`Cargo.toml:75`, `zfuture` and `nu7`) and
/// `zcash_encoding-0.5.0` (`Cargo.toml:71`, `nu7`) only declare values in their
/// check-cfg, and `nu7` is now on without the flag (`zcash_protocol-0.11.0`
/// CHANGELOG). Read at `ZCASH_UNSTABLE_READ_AT`.
const ZCASH_UNSTABLE_VALUES: [&str; 2] = ["nutachyon", "zip233"];

/// The fourteen packages in `sdk/Cargo.lock` whose source named `zcash_unstable`
/// when `ZCASH_UNSTABLE_VALUES` was read, at the versions read. It is a tripwire for
/// these fourteen only: a re-pin of one of them (step 7's final releases among them)
/// reds until the list is read again. A crate that enters or leaves the lock reds
/// through `LOCKED_NAMES_AT_READ`. A locked crate outside the fourteen that starts
/// to read the cfg in a new version, with no crate entering or leaving, is not seen:
/// deriving the lists from the registry sources inside this test is owed. Derived,
/// from `sdk/`, by
///
/// ```text
/// awk -F'"' '/^name = /{n=$2} /^version = /{print n"-"$2}' Cargo.lock |
///   while read p; do for d in ~/.cargo/registry/src/*/"$p"; do
///     [ -d "$d" ] && grep -rqw zcash_unstable "$d" && echo "$p"; done; done
/// ```
///
/// and the values by `grep -rhoE 'zcash_unstable *= *"[^"]*"'` over each one's
/// `src/` (then reading each site: a value only a `Cargo.toml` check-cfg declares
/// is not read).
const ZCASH_UNSTABLE_READ_AT: [(&str, &str); 14] = [
    ("equihash", "0.3.0"),
    ("f4jumble", "0.1.2"),
    ("pczt", "0.10.0"),
    ("zcash_address", "0.14.0"),
    ("zcash_client_backend", "0.25.0"),
    ("zcash_client_sqlite", "0.23.0"),
    ("zcash_encoding", "0.5.0"),
    ("zcash_keys", "0.17.0"),
    ("zcash_pool_migration", "0.2.0"),
    ("zcash_primitives", "0.31.0"),
    ("zcash_proofs", "0.31.0"),
    ("zcash_protocol", "0.11.0"),
    ("zcash_transparent", "0.11.0"),
    ("zip321", "0.10.0"),
];

/// The crate names in `sdk/Cargo.lock` when `ZCASH_UNSTABLE_VALUES` and
/// `ZCASH_UNSTABLE_READ_AT` were derived (2026-10-23), each once. A crate that enters
/// or leaves the lock reds until both are derived again (the command at
/// `ZCASH_UNSTABLE_READ_AT`) and this set moves with them, since a crate new to the
/// lock may read `zcash_unstable`.
const LOCKED_NAMES_AT_READ: &str = "
    addchain addr2line adler2 aead aes aho-corasick allo-isolate ambassador anyhow
    apple-secure-enclave arrayref arrayvec assert_matches async-trait atomic atomic-waker
    autocfg backtrace base64 bech32 bellman bip32 bip39 bit-set bit-vec bitcoin_hashes
    bitflags bitvec blake2b_simd blake2s_simd block-buffer bls12_381 borsh borsh-derive
    bounded-vec bs58 build-target bumpalo bytemuck byteorder bytes cbc cc cfg-if cfg_aliases
    chacha20 chacha20poly1305 chrono cipher cmov cobs combine constant_time_eq
    core-foundation core-foundation-sys core_detect corez cpubits cpufeatures
    crossbeam-channel crossbeam-deque crossbeam-epoch crossbeam-utils crunchy crypto-common
    ctutils daggy darling darling_core darling_macro dart-sys delegate-attr deranged digest
    document-features either embedded-io equihash equivalent errno f4jumble
    fallible-iterator fallible-streaming-iterator fastrand ff ff_derive find-msvc-tools
    fixedbitset flume flutter_rust_bridge flutter_rust_bridge_macros fnv foldhash fpe funty
    futures futures-channel futures-core futures-executor futures-io futures-macro
    futures-sink futures-task futures-util generic-array getrandom getset gimli groth16
    group h2 halo2_gadgets halo2_legacy_pdqsort halo2_poseidon halo2_proofs hashbrown
    hashlink heck hermit-abi hex hex-conservative hmac http http-body http-body-util
    httparse httpdate hybrid-array hyper hyper-timeout hyper-util id-arena ident_case
    incrementalmerkletree incrementalmerkletree-testing indexmap inout itertools itoa jni
    jni-macros jni-sys jni-sys-macros js-sys jubjub lazy_static leb128fmt libc libm
    libsqlite3-sys linux-raw-sys litrs lock_api log maybe-rayon md-5 memchr memuse
    minimal-lexical miniz_oxide mio multimap nanorand ndk-sys nom nonempty num-bigint
    num-conv num-integer num-traits num_cpus object once_cell opaque-debug openssl-src
    openssl-sys orchard oslog pairing paranoid-android pasta_curves pczt percent-encoding
    petgraph pin-project pin-project-internal pin-project-lite pkg-config poly1305
    portable-atomic postcard powerfmt ppv-lite86 prettyplease proc-macro-crate proc-macro2
    proptest prost prost-build prost-derive prost-types quick-error quote r-efi radium rand
    rand_chacha rand_core rand_distr rand_xorshift rayon rayon-core reddsa redjubjub regex
    regex-automata regex-syntax ring ripemd rusqlite rustc-demangle rustc_version rustix
    rustls rustls-pki-types rustls-webpki rustversion rusty-fork same-file sapling-crypto
    schemerz schemerz-rusqlite scopeguard secp256k1 secp256k1-sys secrecy security-framework
    security-framework-sys semver serde serde_core serde_derive serde_json serde_with
    serde_with_macros sha1 sha2 sharded-slab shardtree shlex simd_cesu8 simdutf8 sinsemilla
    slab smallvec socket2 spin static_assertions strsim subtle syn sync_wrapper tap tempfile
    thiserror thiserror-impl thread_local threadpool time time-core time-macros tinyvec
    tinyvec_macros tokio tokio-macros tokio-rustls tokio-stream tokio-util toml_datetime
    toml_edit toml_parser tonic tonic-build tonic-prost tonic-prost-build tower tower-layer
    tower-service tracing tracing-attributes tracing-core tracing-subscriber try-lock
    typenum uint unarray unicode-ident unicode-normalization unicode-xid universal-hash
    untrusted uuid vcpkg version_check visibility wagyu-zcash-parameters
    wagyu-zcash-parameters-1 wagyu-zcash-parameters-2 wagyu-zcash-parameters-3
    wagyu-zcash-parameters-4 wagyu-zcash-parameters-5 wagyu-zcash-parameters-6 wait-timeout
    walkdir want wasi wasip2 wasip3 wasm-bindgen wasm-bindgen-futures wasm-bindgen-macro
    wasm-bindgen-macro-support wasm-bindgen-shared wasm-encoder wasm-metadata wasmparser
    web-sys webpki-roots which winapi-util windows-link windows-sys windows-targets
    windows_aarch64_gnullvm windows_aarch64_msvc windows_i686_gnu windows_i686_gnullvm
    windows_i686_msvc windows_x86_64_gnu windows_x86_64_gnullvm windows_x86_64_msvc winnow
    wit-bindgen wit-bindgen-core wit-bindgen-rust wit-bindgen-rust-macro wit-component
    wit-parser wyz zcash_address zcash_client_backend zcash_client_sqlite zcash_encoding
    zcash_keys zcash_note_encryption zcash_pool_migration zcash_primitives zcash_proofs
    zcash_protocol zcash_script zcash_spec zcash_transparent zec-wallet-core
    zec-wallet-swap-near zec_wallet zerocopy zerocopy-derive zeroize zeroize_derive zip32
    zip321 zmij
";

/// The values a build-side refusal of `cfg` must name, and its check-cfg declare, in
/// the order both write them. Empty for a name with no value.
fn refused_values(cfg: &str) -> &'static [&'static str] {
    match cfg {
        "getrandom_backend" => &REPLACING_BACKENDS,
        "zcash_unstable" => &ZCASH_UNSTABLE_VALUES,
        "secp256k1_fuzz" | "hashes_fuzz" => &[],
        other => panic!("no value list for `{other}`"),
    }
}

/// The attribute a build-side refusal of `cfg` must be: `#[cfg(any(<cfg> = "<v>", …))]`
/// over `refused_values` in its order, or `#[cfg(<cfg>)]` for a name with no value.
fn canonical_refusal(cfg: &str) -> String {
    match refused_values(cfg) {
        [] => format!("#[cfg({cfg})]"),
        values => {
            let each: Vec<String> = values.iter().map(|v| format!("{cfg} = \"{v}\"")).collect();
            format!("#[cfg(any({}))]", each.join(", "))
        }
    }
}

/// The check-cfg entry that declares `cfg`: `cfg(<cfg>, values("<v>", …))`, or
/// `cfg(<cfg>)` for a name with no value.
fn canonical_check_cfg(cfg: &str) -> String {
    match refused_values(cfg) {
        [] => format!("cfg({cfg})"),
        values => {
            let quoted: Vec<String> = values.iter().map(|v| format!("\"{v}\"")).collect();
            format!("cfg({cfg}, values({}))", quoted.join(", "))
        }
    }
}

/// How an admitted site is found and judged.
#[derive(Clone, Copy)]
enum Shape {
    /// A `compile_error!` under a `#[cfg]` that names the site's one cfg (`refusals`).
    Refusal,
    /// The `unexpected_cfgs` entry of `[workspace.lints.rust]` (`check_cfg_declarations`).
    CheckCfg,
}

/// A site that names a forbidden cfg ON PURPOSE: a file under `sdk/`, its shape, and
/// the cfgs it may name inside its span. Each must be found exactly once and in its
/// canonical shape: an admission that matches nothing is stale, or the scan no longer
/// reads the file it names.
struct Admitted {
    file: &'static str,
    what: &'static str,
    shape: Shape,
    cfgs: &'static [&'static str],
}

/// The five admitted sites: one build-side refusal per forbidden cfg, and the
/// check-cfg declaration that names all four.
const ADMITTED: [Admitted; 5] = [
    Admitted {
        file: "zec-wallet-core/src/rng.rs",
        what: "the `compile_error!` that refuses to build under a `getrandom_backend`",
        shape: Shape::Refusal,
        cfgs: &["getrandom_backend"],
    },
    Admitted {
        file: "zec-wallet-core/src/lib.rs",
        what: "the `compile_error!` that refuses to build under a `zcash_unstable`",
        shape: Shape::Refusal,
        cfgs: &["zcash_unstable"],
    },
    Admitted {
        file: "zec-wallet-core/src/lib.rs",
        what: "the `compile_error!` that refuses to build under `secp256k1_fuzz`",
        shape: Shape::Refusal,
        cfgs: &["secp256k1_fuzz"],
    },
    Admitted {
        file: "zec-wallet-core/src/lib.rs",
        what: "the `compile_error!` that refuses to build under `hashes_fuzz`",
        shape: Shape::Refusal,
        cfgs: &["hashes_fuzz"],
    },
    Admitted {
        file: "Cargo.toml",
        what: "the check-cfg declaration in `[workspace.lints.rust]`'s `unexpected_cfgs`",
        shape: Shape::CheckCfg,
        cfgs: &[
            "getrandom_backend",
            "zcash_unstable",
            "secp256k1_fuzz",
            "hashes_fuzz",
        ],
    },
];

/// One admitted site as found in its file: the lines it covers (0-based, both ends
/// included), and every way it departs from the shape it is written in.
struct Found {
    span: (usize, usize),
    problems: Vec<String>,
}

/// Every candidate for `site` in its file's `text`, judged.
fn find_site(site: &Admitted, text: &str) -> Vec<Found> {
    match site.shape {
        Shape::Refusal => refusals(text, site.cfgs[0]),
        Shape::CheckCfg => check_cfg_declarations(text, site.cfgs),
    }
}

/// Whether `text` names `word` as a whole identifier.
fn names_word(text: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word)
        .any(|(at, _)| !text[..at].ends_with(ident) && !text[at + word.len()..].starts_with(ident))
}

/// A Rust file roughly as the compiler reads it, char for char and line for line: `code`
/// with the comments blanked, `bare` with the insides of string and char literals blanked
/// too, and `literal` marking those insides. A blanked character becomes a space and a
/// newline stays, so an index means the same place in all three. It lexes what the core's
/// crate root carries today: `//` and nested `/* */` comments, strings with their escapes
/// (a `b` or `c` prefix is just a letter before the quote), raw strings `r#"…"#`, and char
/// literals told apart from lifetimes. It is not rustc's lexer, and a token it does not
/// model throws it out of step with the file (step 1g's third review: `'x'r"\"` hid a
/// `macro_rules! compile_error` from `shadows`). So what `refusals` and `shadows` read
/// through it is an early warning; the compiler is the judge
/// (`every_refusal_stops_a_build_of_the_core_under_its_cfg`).
struct RustText {
    code: Vec<char>,
    bare: Vec<char>,
    literal: Vec<bool>,
}

fn rust_text(text: &str) -> RustText {
    let src: Vec<char> = text.chars().collect();
    let n = src.len();
    let (mut code, mut bare, mut literal) = (src.clone(), src.clone(), vec![false; n]);
    let at = |k: usize| src.get(k).copied().unwrap_or('\0');
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    // The `#`s of a raw string that opens at `k`, if one does.
    let raw = |k: usize| {
        let starts_a_token = k == 0
            || !ident(src[k - 1])
            || (matches!(src[k - 1], 'b' | 'c') && (k < 2 || !ident(src[k - 2])));
        let hashes = (k + 1..n).take_while(|&j| src[j] == '#').count();
        (at(k) == 'r' && starts_a_token && at(k + 1 + hashes) == '"').then_some(hashes)
    };
    let mut i = 0;
    while i < n {
        // A comment, `from..to`, blanked in both views; or a literal's inside,
        // blanked in `bare`. Lexing resumes at `next`.
        let (comment, from, to, next) = if at(i) == '/' && at(i + 1) == '/' {
            let end = (i..n).find(|&k| src[k] == '\n').unwrap_or(n);
            (true, i, end, end)
        } else if at(i) == '/' && at(i + 1) == '*' {
            let (mut depth, mut k) = (0, i);
            while k < n {
                match (src[k], at(k + 1)) {
                    ('/', '*') => (depth, k) = (depth + 1, k + 2),
                    ('*', '/') => {
                        (depth, k) = (depth - 1, k + 2);
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => k += 1,
                }
            }
            (true, i, k.min(n), k)
        } else if at(i) == '"' {
            let mut k = i + 1;
            while k < n && src[k] != '"' {
                k += if src[k] == '\\' { 2 } else { 1 };
            }
            (false, i + 1, k.min(n), k + 1)
        } else if let Some(hashes) = raw(i) {
            let open = i + 1 + hashes;
            let close = (open + 1..n)
                .find(|&k| src[k] == '"' && (1..=hashes).all(|h| at(k + h) == '#'))
                .unwrap_or(n);
            (false, open + 1, close, close + 1 + hashes)
        } else if at(i) == '\'' && at(i + 1) == '\\' {
            let close = (i + 3..n).find(|&k| src[k] == '\'').unwrap_or(n);
            (false, i + 1, close, close + 1)
        } else if at(i) == '\'' && at(i + 2) == '\'' {
            (false, i + 1, i + 2, i + 3)
        } else {
            i += 1;
            continue;
        };
        for k in from..to {
            if src[k] != '\n' {
                bare[k] = ' ';
                if comment {
                    code[k] = ' ';
                } else {
                    literal[k] = true;
                }
            }
        }
        i = next.max(i + 1);
    }
    RustText {
        code,
        bare,
        literal,
    }
}

/// The index of the bracket that matches the one at `at` in `bare`, searching
/// forward from an opening one or back from a closing one, counting only its kind.
fn matching(bare: &[char], at: usize) -> Option<usize> {
    let (open, close, forward) = match bare[at] {
        '(' => ('(', ')', true),
        '[' => ('[', ']', true),
        ')' => ('(', ')', false),
        ']' => ('[', ']', false),
        _ => return None,
    };
    let mut depth = 0i32;
    let mut k = at;
    loop {
        match bare[k] {
            c if c == open => depth += if forward { 1 } else { -1 },
            c if c == close => depth += if forward { -1 } else { 1 },
            _ => {}
        }
        if depth == 0 {
            return Some(k);
        }
        if forward {
            k += 1;
            if k == bare.len() {
                return None;
            }
        } else if k == 0 {
            return None;
        } else {
            k -= 1;
        }
    }
}

/// Every `compile_error!` in a Rust file whose `#[cfg(…)]` names `cfg`, so a second
/// copy is counted, not merged, each judged against the shape the refusals are written
/// in. A text check, and an early warning: a refusal in this shape can still be dead in
/// a build (its file's own `#![cfg(…)]`, a `#[path]` or `#[cfg]` twin of `mod rng`, a
/// `[lib] path`), and one out of it can still be live. Whether each refusal stops a
/// build is `every_refusal_stops_a_build_of_the_core_under_its_cfg`'s to say. The shape:
///   * its attribute, with comments dropped and whitespace outside strings dropped,
///     EQUALS `canonical_refusal(cfg)`: a value commented out, `/* … */` around one,
///     an `all(…, target_os = "none")` arm or a dropped value is a departure;
///   * the attribute starts at column 0, outside every bracket of the file (so not in
///     a `mod`, a function or a macro's arms), and `compile_error!(…);` follows it;
///   * in the code before it there is only an item's end (`;`, `}`), an inner
///     attribute (`#![…]`) or the file's start, so no attribute stacked above it, with
///     or without a comment between, applies to it too; and the line above it is
///     blank or a `//` comment.
fn refusals(text: &str, cfg: &str) -> Vec<Found> {
    const MACRO: &str = "compile_error!";
    let RustText {
        code,
        bare,
        literal,
    } = rust_text(text);
    let raw: Vec<&str> = text.lines().collect();
    let line_of = |k: usize| bare[..k].iter().filter(|&&c| c == '\n').count();
    // The last character before `k` that is not whitespace.
    let before = |k: usize| (0..k).rev().find(|&j| !bare[j].is_whitespace());
    let after = |k: usize| (k..bare.len()).find(|&j| !bare[j].is_whitespace());
    let shown = |from: usize, to: usize| {
        let text: String = code[from..=to].iter().collect();
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let squeezed = |from: usize, to: usize| -> String {
        (from..=to)
            .filter(|&k| literal[k] || !code[k].is_whitespace())
            .map(|k| code[k])
            .collect()
    };
    let want = canonical_refusal(cfg);
    let mut in_quotes = false;
    let want_squeezed: String = want
        .chars()
        .filter(|&c| {
            in_quotes ^= c == '"';
            in_quotes || c == '"' || !c.is_whitespace()
        })
        .collect();
    let needle: Vec<char> = MACRO.chars().collect();
    let starts: Vec<usize> = (0..bare.len())
        .filter(|&k| bare[k..].starts_with(&needle))
        .collect();

    let mut out = Vec::new();
    for start in starts {
        // The attribute right before it: an outer `#[…]`.
        let Some(close) = before(start).filter(|&k| bare[k] == ']') else {
            continue;
        };
        let Some(open) = matching(&bare, close) else {
            continue;
        };
        if open == 0 || bare[open - 1] != '#' {
            continue;
        }
        let hash = open - 1;
        let names: String = bare[hash..=close].iter().collect();
        if !names_word(&names, cfg) {
            continue;
        }
        let mut problems = Vec::new();
        if squeezed(hash, close) != want_squeezed {
            problems.push(format!(
                "its attribute reads `{}`, and it must be `{want}`",
                shown(hash, close)
            ));
        }
        if hash > 0 && bare[hash - 1] != '\n' {
            problems.push("its attribute does not start at column 0".to_string());
        }
        let depth: i32 = bare[..hash]
            .iter()
            .map(|c| match c {
                '(' | '[' | '{' => 1,
                ')' | ']' | '}' => -1,
                _ => 0,
            })
            .sum();
        if depth != 0 {
            problems.push(
                "it sits inside a bracket of the file (a `mod`, a function or a macro), so it \
                 is compiled only where that is"
                    .to_string(),
            );
        }
        match before(hash) {
            None => {}
            Some(k) if matches!(bare[k], ';' | '}') => {}
            Some(k) if bare[k] == ']' => {
                let opened = matching(&bare, k).unwrap_or(0);
                if !(opened >= 2 && bare[opened - 1] == '!' && bare[opened - 2] == '#') {
                    problems.push(format!(
                        "an attribute comes before it, and applies to it too: `{}`",
                        shown(opened.saturating_sub(1), k)
                    ));
                }
            }
            Some(k) => problems.push(format!(
                "the code before it does not end an item (it ends `{}`)",
                bare[k]
            )),
        }
        let line = line_of(hash);
        if line > 0 {
            let above = raw[line - 1].trim();
            if !(above.is_empty() || above.starts_with("//")) {
                problems.push(format!(
                    "the line above it is neither blank nor a `//` comment: `{above}`"
                ));
            }
        }
        let end = after(start + MACRO.chars().count())
            .filter(|&k| bare[k] == '(')
            .and_then(|k| matching(&bare, k))
            .and_then(|k| after(k + 1))
            .filter(|&k| bare[k] == ';');
        if end.is_none() {
            problems.push("it is not written `compile_error!(…);`".to_string());
        }
        out.push(Found {
            span: (line, line_of(end.unwrap_or(start))),
            problems,
        });
    }
    out
}

/// Each place `word` stands in `text` as a whole identifier: its byte offset, and the
/// text after it with leading whitespace dropped.
fn whole_words<'t>(text: &'t str, word: &str) -> Vec<(usize, &'t str)> {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word)
        .filter(|&(at, _)| {
            !text[..at].ends_with(ident) && !text[at + word.len()..].starts_with(ident)
        })
        .map(|(at, _)| (at, text[at + word.len()..].trim_start()))
        .collect()
}

/// What in one of the core's source files could answer for `compile_error!` in place
/// of the builtin, so that a refusal compiles to nothing under its cfg: each line
/// (0-based) and what it names. Probed on rustc 1.99.1 at step 1g's second review
/// fold, each of these silenced a `#[cfg(probe)] compile_error!(…)` built with
/// `--cfg probe`: a `macro_rules! compile_error` whose textual scope reaches the
/// refusal, `use … as compile_error`, a `#[macro_use]` module that defines one, and a
/// `#[macro_use] extern crate` that exports one; `extern crate … as core` did the same
/// to `::core::compile_error!`. A glob import does not (rustc refuses the ambiguous
/// name, E0659). So, with comments and literals dropped, the word `compile_error`
/// stands only as a refusal's own `compile_error!` (a line inside `refusal_lines`),
/// and `macro_use`, `extern crate` and `include!`, which brings in code no scan reads,
/// not at all. The core carries none of them today. It reads the text through
/// `rust_text`, which a token it does not model throws out of step, so it is an early
/// warning and not the check: a shadow it misses still makes a refusal compile to
/// nothing, and `every_refusal_stops_a_build_of_the_core_under_its_cfg` reds on that.
fn shadows(text: &str, refusal_lines: &[(usize, usize)]) -> Vec<(usize, &'static str)> {
    let bare: String = rust_text(text).bare.into_iter().collect();
    let line_of = |at: usize| bare[..at].matches('\n').count();
    let mut out = Vec::new();
    for (at, rest) in whole_words(&bare, "compile_error") {
        let line = line_of(at);
        let in_refusal = refusal_lines
            .iter()
            .any(|&(first, last)| (first..=last).contains(&line));
        if !(in_refusal && rest.starts_with('!')) {
            out.push((line, "`compile_error` outside a refusal"));
        }
    }
    for (at, _) in whole_words(&bare, "macro_use") {
        out.push((line_of(at), "`macro_use`"));
    }
    for (at, rest) in whole_words(&bare, "extern") {
        let crate_next = rest
            .strip_prefix("crate")
            .is_some_and(|after| !after.starts_with(|c: char| c.is_alphanumeric() || c == '_'));
        if crate_next {
            out.push((line_of(at), "`extern crate`"));
        }
    }
    for (at, rest) in whole_words(&bare, "include") {
        if rest.starts_with('!') {
            out.push((line_of(at), "`include!`"));
        }
    }
    out.sort();
    out
}

/// A TOML line without its comment, and the strings on it, as written: `"…"` (its
/// `\` escapes kept) and `'…'`. In the line that comes back each string is empty, so
/// a bracket inside one is not counted. One line at a time, which is how the
/// check-cfg array writes its entries.
fn toml_line(line: &str) -> (String, Vec<String>) {
    let mut code = String::new();
    let mut strings = Vec::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '#' => break,
            '"' | '\'' => {
                let (mut s, mut escaped) = (String::new(), false);
                for d in chars.by_ref() {
                    if d == c && !escaped {
                        break;
                    }
                    escaped = c == '"' && d == '\\' && !escaped;
                    s.push(d);
                }
                code.push(c);
                code.push(c);
                strings.push(s);
            }
            _ => code.push(c),
        }
    }
    (code, strings)
}

/// The `unexpected_cfgs = …` entries of `[workspace.lints.rust]`, from the key to the
/// line their brackets close on, each judged: with comments dropped (a TOML `#` line
/// or tail inside the array counts for nothing), it declares each of `cfgs` in
/// exactly one string, and that string EQUALS `canonical_check_cfg`. A check-cfg
/// declaration tells rustc that a name and its values exist, so a typo in a refusal
/// still warns; it sets nothing.
fn check_cfg_declarations(text: &str, cfgs: &[&str]) -> Vec<Found> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut table = String::new();
    let mut i = 0;
    while i < lines.len() {
        let (line, mut strings) = toml_line(lines[i]);
        let line = line.trim();
        if line.starts_with('[') {
            table = line.to_string();
        } else if table == "[workspace.lints.rust]" && line.starts_with("unexpected_cfgs") {
            let start = i;
            let mut depth = depth_delta(line);
            while depth > 0 && i + 1 < lines.len() {
                i += 1;
                let (more, found) = toml_line(lines[i]);
                depth += depth_delta(&more);
                strings.extend(found);
            }
            let mut problems = Vec::new();
            for cfg in cfgs {
                let want = canonical_check_cfg(cfg);
                let declared: Vec<&String> =
                    strings.iter().filter(|s| names_word(s, cfg)).collect();
                if declared.len() != 1 || *declared[0] != want {
                    problems.push(format!(
                        "it declares `{cfg}` as {declared:?}, and it must declare it once, as \
                         `{want}`"
                    ));
                }
            }
            out.push(Found {
                span: (start, i),
                problems,
            });
        }
        i += 1;
    }
    out
}

/// `sdk/`, canonical: the SDK's root, and the public repository's once extracted.
fn sdk_root() -> PathBuf {
    // sdk/zec-wallet-core → sdk
    fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")).expect("sdk/ resolves")
}

/// The monorepo root, canonical, while this is the monorepo (`root_manifest`);
/// `None` once extracted.
fn monorepo_root() -> Option<PathBuf> {
    root_manifest()?;
    // sdk/zec-wallet-core → sdk → repo root
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    Some(fs::canonicalize(root).expect("the monorepo root resolves"))
}

/// `path` as the reader of a failure finds it: from the monorepo root while
/// there is one, else from `sdk/`.
fn shown(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Directories no build of the SDK reads from: build outputs, tool caches, and
/// test trees (a test is not a shipped build, and this file names the forbidden
/// cfgs itself).
const NOT_WALKED: &[&str] = &[
    ".dart_tool",
    ".git",
    ".gradle",
    "Pods",
    "benches",
    "build",
    "ephemeral",
    "integration_test",
    "node_modules",
    "target",
    "test",
    "tests",
];

/// Every file under `dir`, sorted, outside the `NOT_WALKED` directories.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    walk(dir, NOT_WALKED)
}

/// Every file under `dir`, sorted, outside any directory named in `not_walked`.
/// Symlinked directories are not walked (the example app's iOS `.symlinks` point
/// back into this tree); `config_dirs` follows them, for the configs a build reads
/// through one. An unreadable directory fails: a walk that silently read
/// less is the scan that examines nothing (`docs/REVIEW.md` §6).
fn walk(dir: &Path, not_walked: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|e| panic!("{} is unreadable: {e}", dir.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|e| panic!("{} lists badly: {e}", dir.display()));
            let path = entry.path();
            let kind = entry
                .file_type()
                .unwrap_or_else(|e| panic!("{} has no type: {e}", path.display()));
            let skipped = entry
                .file_name()
                .to_str()
                .is_some_and(|name| not_walked.contains(&name));
            if kind.is_dir() && !skipped {
                stack.push(path);
            } else if kind.is_file() || (kind.is_symlink() && path.is_file()) {
                // A symlinked FILE is read (cargo would read it); a symlinked
                // directory is not walked.
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Which carrier of a cfg the file at `rel` (a path under `sdk/`) is, if any: the
/// plan's list (`docs/plan/nu7-wave.md` §3 step 1g), plus what the Flutter plugins'
/// own builds read.
///   * cargokit (`sdk/*/cargokit/`) composes the rustflags of every shipped Android
///     build (`CARGO_ENCODED_RUSTFLAGS`, under which cargo ignores a
///     `.cargo/config.toml`'s). Its per-crate options file, `cargokit.yaml`, is not
///     read: it is refused by its existence (`no_cargo_config_or_cargokit_yaml_…`).
///   * A podspec's `pod_target_xcconfig` and `script_phase` environment reach every
///     iOS and macOS build: Xcode runs the phase's `build_pod.sh` in that
///     environment, and cargokit's runner hands its whole environment to cargo
///     (`build_tool/lib/src/util.dart:86`, `includeParentEnvironment = true`).
///   * A plugin's Gradle and CMake files outside `cargokit/` reach every Android,
///     Linux and Windows build of it: each applies cargokit, and either can run cargo
///     itself, in an environment of its own.
///   * A cargo config is refused by its existence too, unless
///     `ALLOWED_BUILD_CONFIG_FILES` names it; it is read here as well, so one that
///     list names is still scanned.
fn carrier(rel: &Path) -> Option<&'static str> {
    let name = rel.file_name()?.to_str()?;
    let parts: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
    let dirs = &parts[..parts.len() - 1];
    Some(match name {
        "Cargo.toml" => "manifest",
        "build.rs" => "build script",
        "config" | "config.toml" if dirs.last() == Some(&".cargo") => "cargo config",
        _ if name.ends_with(".podspec") => "podspec",
        _ if dirs.contains(&"cargokit") => "cargokit file",
        _ if name.ends_with(".gradle") || name.ends_with(".gradle.kts") => "Gradle file",
        _ if name == "CMakeLists.txt" || name.ends_with(".cmake") => "CMake file",
        _ if dirs.windows(2).any(|w| *w == [".github", "workflows"]) => "CI workflow",
        _ if name.ends_with(".rs") && dirs.contains(&"src") => "non-test source",
        _ => return None,
    })
}

/// Whether the file at `path` is a script: a `.sh` or `.py` file, or one that names
/// its interpreter on its first line (`#!`), as the Git hooks do.
fn is_script(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with(".sh")
        || name.ends_with(".py")
        || fs::read(path).is_ok_and(|bytes| bytes.starts_with(b"#!"))
}

/// The marker of a whole-line comment in `path`'s language. A line that starts
/// with it is prose, not a flag: the rationale beside every admitted site names
/// the cfg in comments. Every other line is read whole, a trailing comment
/// included (a false red there is loud and costs a line break; a false green
/// would be silent), and a file of no known language is read whole.
fn comment_marker(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    match name.rsplit_once('.').map(|(_, ext)| ext) {
        Some("rs" | "dart" | "gradle" | "kts") => Some("//"),
        Some("toml" | "yml" | "yaml" | "sh" | "py" | "cmake" | "ps1" | "podspec") => Some("#"),
        _ if matches!(name, "Justfile" | "config" | "CMakeLists.txt") => Some("#"),
        _ => None,
    }
}

/// The plugins' podspecs. Each is read, and each must be: through Xcode, one
/// reaches every iOS or macOS build of its plugin.
const PODSPECS: [&str; 5] = [
    "zec_wallet/ios/zec_wallet.podspec",
    "zec_wallet/macos/zec_wallet.podspec",
    "zec_wallet_tor/ios/zec_wallet_tor.podspec",
    "zec_wallet_tor/macos/zec_wallet_tor.podspec",
    "zec_wallet_ui_platform/ios/zec_wallet_ui_platform.podspec",
];

/// The plugins that vendor cargokit, at `sdk/<plugin>/cargokit/`. Each must have
/// cargokit files read.
const CARGOKIT_PLUGINS: [&str; 2] = ["zec_wallet", "zec_wallet_tor"];

/// The build files that apply cargokit in each plugin of `CARGOKIT_PLUGINS`. Each is
/// read: through Gradle or CMake, one reaches every Android, Linux or Windows build of
/// its plugin.
const PLUGIN_BUILD_FILES: [&str; 6] = [
    "zec_wallet/android/build.gradle",
    "zec_wallet/linux/CMakeLists.txt",
    "zec_wallet/windows/CMakeLists.txt",
    "zec_wallet_tor/android/build.gradle",
    "zec_wallet_tor/linux/CMakeLists.txt",
    "zec_wallet_tor/windows/CMakeLists.txt",
];

/// The script every gated cargo build in the monorepo runs through (`just slot`, the
/// watch driver). It is read, and must be.
const CARGO_SLOT: &str = "scripts/stage/cargo-slot.sh";

/// Every file the cfg scan reads, with its kind: the carriers under `sdk/`
/// (`carrier`), and in the monorepo the command runners, both CI workflow
/// directories (the public repository's lives in `scripts/release/public-root/`),
/// the scripts under `scripts/` that the runners and hooks call (`is_script`; not the
/// public repository's files, whose workflows are read and whose rest is prose), and
/// a root `.cargo/`, which cargo reads as an ancestor of `sdk/`.
fn carriers(sdk: &Path, root: Option<&Path>) -> Vec<(PathBuf, &'static str)> {
    let mut read: Vec<(PathBuf, &str)> = files_under(sdk)
        .into_iter()
        .filter_map(|path| {
            let kind = carrier(path.strip_prefix(sdk).ok()?)?;
            Some((path, kind))
        })
        .collect();
    if let Some(root) = root {
        for runner in ["Justfile", "lefthook.yml"] {
            let path = root.join(runner);
            assert!(
                path.is_file(),
                "{runner} is missing from the monorepo root: the scan would skip a carrier it \
                 claims to read"
            );
            read.push((path, "command runner"));
        }
        for dir in [
            ".github/workflows",
            "scripts/release/public-root/.github/workflows",
        ] {
            read.extend(
                files_under(&root.join(dir))
                    .into_iter()
                    .map(|p| (p, "CI workflow")),
            );
        }
        let public = root.join("scripts/release/public-root");
        read.extend(
            files_under(&root.join("scripts"))
                .into_iter()
                .filter(|p| !p.starts_with(&public) && is_script(p))
                .map(|p| (p, "script")),
        );
        for config in [".cargo/config", ".cargo/config.toml"] {
            let path = root.join(config);
            if path.is_file() {
                read.push((path, "cargo config"));
            }
        }
    }
    read
}

/// The carrier kinds whose lines can set cargo's variables or pass it flags.
const COMMAND_CARRIERS: [&str; 7] = [
    "command runner",
    "CI workflow",
    "script",
    "podspec",
    "cargokit file",
    "Gradle file",
    "CMake file",
];

/// The variables under which cargo takes, from a command, what this scan does not
/// read: a program run in place of or around every `rustc` (`RUSTC_WRAPPER`,
/// `RUSTC_WORKSPACE_WRAPPER`, and `CARGO_BUILD_RUSTC…`, the forms of `build.rustc`
/// and its wrappers), a source replacement (`CARGO_SOURCE_…`), a target's linker,
/// runner or flags (`CARGO_TARGET_<TRIPLE>_…`; `CARGO_TARGET_DIR`, where output goes,
/// is not one), and another cargo home with its own config (`CARGO_HOME`). Matched in
/// any case, since cargokit writes the one it sets in lower case and upper-cases it.
/// `cargo_overrides` adds `RUSTC` itself and a `--config` flag.
const CARGO_OVERRIDES: [&str; 6] = [
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC",
    "CARGO_SOURCE_",
    "CARGO_TARGET_",
    "CARGO_HOME",
];

/// The one cargo variable a carrier sets on purpose: cargokit points cargo's Android
/// linker at its own executable, which then runs the NDK's clang
/// (`build_tool/lib/src/android_environment.dart:124-125`, set at `:161`). Admitted
/// by its text, in that file, once in each plugin of `CARGOKIT_PLUGINS`.
const ADMITTED_OVERRIDE: (&str, &str) = (
    "cargokit/build_tool/lib/src/android_environment.dart",
    "'cargo_target_${target.rust.replaceAll('-', '_')}_linker'.toUpperCase();",
);

/// Lines that pass `--config` to a tool other than cargo, admitted by their text (a
/// path under the monorepo root, and the line trimmed), each once: the release
/// extractor hands gitleaks its own config, and says so in its doc. A gitleaks config
/// is TOML too, so the argument cannot tell the two tools apart.
const FOREIGN_CONFIG_FLAGS: [(&str, &str); 2] = [
    (
        "scripts/release/extract.py",
        "from the tree (an explicit --config wins over it), no `.gitleaksignore`",
    ),
    (
        "scripts/release/extract.py",
        "out = subprocess.run([exe, \"dir\", str(pub), \"--config\", str(config),",
    ),
];

/// What `line` hands cargo that this scan does not read: each `CARGO_OVERRIDES` name
/// it carries, `RUSTC` as a whole word as written (the command `rustc` is everywhere
/// in lower case), and a `--config` flag (`passes_cargo_config`).
fn cargo_overrides(line: &str) -> Vec<&'static str> {
    let upper = line.to_ascii_uppercase();
    let not_dir = |after: &str| {
        !after.starts_with("DIR")
            || after[3..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
    };
    let mut out: Vec<&str> = CARGO_OVERRIDES
        .into_iter()
        .filter(|name| {
            upper
                .match_indices(name)
                .any(|(at, _)| *name != "CARGO_TARGET_" || not_dir(&upper[at + name.len()..]))
        })
        .collect();
    if names_word(line, "RUSTC") {
        out.push("RUSTC");
    }
    if passes_cargo_config(line) {
        out.push("--config");
    }
    out
}

/// Whether `line` passes a `--config` flag that cargo would take: the flag as a word,
/// unless its argument is a YAML file. Cargo takes `KEY=VALUE` or a TOML file, so
/// `dart run ffigen --config ffigen.yaml` is ffigen's, not cargo's.
fn passes_cargo_config(line: &str) -> bool {
    line.match_indices("--config").any(|(at, flag)| {
        let after = &line[at + flag.len()..];
        if after.starts_with(|c: char| c.is_alphanumeric() || c == '-' || c == '_') {
            return false;
        }
        let argument: String = after
            .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '=' | '\'' | '"' | ','))
            .chars()
            .take_while(|c| !c.is_whitespace() && !matches!(c, '\'' | '"' | ',' | ')' | ';'))
            .collect();
        !(argument.ends_with(".yaml") || argument.ends_with(".yml"))
    })
}

/// The keys a TOML line sets, each as its dotted segments, with whitespace and
/// quotes dropped: a `[table]` header's path, or every `key =` on the line, in
/// order, inline tables included (`build = { rustc-wrapper = "x" }` sets `build`
/// and then `rustc-wrapper`). Coarse on purpose: a string that holds `x = y`
/// reads as a key, a false red that is loud, where a missed key would be silent.
fn keys_set(line: &str) -> Vec<Vec<String>> {
    let line: String = line
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '"' | '\''))
        .collect();
    let split = |path: &str| path.split('.').map(str::to_string).collect::<Vec<_>>();
    if let Some(header) = line.strip_prefix('[') {
        let path = header.trim_start_matches('[');
        return vec![split(path.split(']').next().unwrap_or(""))];
    }
    line.match_indices('=')
        .map(|(at, _)| line[..at].rsplit(['{', ',', '[']).next().unwrap_or(""))
        .filter(|key| !key.is_empty())
        .map(split)
        .collect()
}

/// The lines of a manifest that override where a dependency comes
/// from: a `[patch.…]` or `[replace]` table, or the same keys set at the root
/// (`patch.crates-io.x = …`, `patch = { … }`), 1-based. A table or root key whose
/// first segment is spelled with a `\` escape is returned with them: to TOML,
/// `"patch"` is `patch`.
fn overrides(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut at_root = true;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_start();
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            at_root = false;
        } else if !at_root {
            continue;
        }
        // A header names one path; at the root, the line's own key is its first.
        let Some(key) = keys_set(line).into_iter().next() else {
            continue;
        };
        let first = key.first().map_or("", String::as_str);
        if matches!(first, "patch" | "replace") || first.contains('\\') {
            out.push((i + 1, raw.trim().to_string()));
        }
    }
    out
}

/// ADR-0579 Decision 2's row, the one ADR-0540 claimed and never had: no cfg in
/// `FORBIDDEN_CFGS` reaches a shipped build.
///
/// **What it reads** (`carriers`): each manifest under `sdk/`, whole, so a
/// `rustflags` key in any table is seen; each `build.rs` (the bridge's emits
/// `rustc-cfg` today, and at least one that does must be read); the SDK's
/// non-test sources, whole files, never stopping at a `#[cfg(test)]`; cargokit; the
/// plugins' five podspecs and their Gradle and CMake files; the files that set
/// `RUSTFLAGS` for a build: the CI workflows, and in the monorepo the `Justfile`,
/// `lefthook.yml` and the scripts under `scripts/`; and any cargo config under
/// `sdk/` or at the monorepo root, which
/// `no_cargo_config_or_cargokit_yaml_reaches_a_build_of_the_sdk` refuses by its
/// existence unless its allow-list names it. It prints how many files of each kind
/// it read, fails on zero, and holds each kind that must be there to a floor: every
/// podspec in `PODSPECS`, cargokit files for every plugin in `CARGOKIT_PLUGINS`, every
/// build file in `PLUGIN_BUILD_FILES`, a CI workflow, and in the monorepo both command
/// runners and `CARGO_SLOT` (`docs/REVIEW.md` §6).
///
/// **What a carrier may not hand cargo.** In a command runner, a CI workflow, a
/// script, a podspec, cargokit or a plugin's Gradle or CMake file
/// (`COMMAND_CARRIERS`) it refuses each variable in `CARGO_OVERRIDES`, `RUSTC`, and a
/// `--config` flag: each brings in a program, a source, a config or flags this scan
/// does not read. It admits cargokit's own Android linker (`ADMITTED_OVERRIDE`) and
/// the release extractor's `--config`, which is gitleaks' (`FOREIGN_CONFIG_FLAGS`).
/// In a manifest it refuses `cargo-features`, which turns on nightly cargo's manifest
/// features, per-package `rustflags` (`profile-rustflags`) among them.
///
/// **What it admits.** Exactly the five `ADMITTED` sites, by name: the four
/// `compile_error!` refusals, each for its own cfg only, and the check-cfg
/// declaration in `sdk/Cargo.toml` that names all four. Each is judged by its shape
/// (`refusals`, `check_cfg_declarations`): with comments dropped it must EQUAL the
/// canonical text built from the value lists (`REPLACING_BACKENDS`,
/// `ZCASH_UNSTABLE_VALUES`, none for `secp256k1_fuzz` or `hashes_fuzz`), and a refusal
/// must sit at column 0, outside every bracket of its file, with no attribute before
/// it. Nothing in the core's sources may name what could answer for `compile_error!`
/// in place of the builtin (`shadows`). Both read text, so they warn early and refuse
/// nothing the compiler would not: whether each refusal stops a build is
/// `every_refusal_stops_a_build_of_the_core_under_its_cfg`'s to say, and that the
/// files they read are the crate root cargo builds is
/// `the_cores_lib_target_is_src_lib_rs`'s. The check-cfg's shape is this test's own
/// door: it sets nothing, and keeps a typo in a refusal a warning. And
/// `sdk/Cargo.lock` must still carry the crates the lists were read at
/// (`ZCASH_UNSTABLE_READ_AT`, `LOCKED_NAMES_AT_READ`; the generator guard holds
/// `getrandom` to `GETRANDOM_PIN`). Any other line that names a forbidden cfg reds,
/// naming the file, the line and the cfg. The match is on the bare name, so
/// `--cfg zcash_unstable`, `cargo:rustc-cfg=…` and
/// `#[cfg(zcash_unstable = "nutachyon")]` are all seen.
///
/// **What the build closes, and what it does not.** `zec-wallet-core` refuses to
/// compile under each `zcash_unstable` value a pinned crate's code reads (`lib.rs`),
/// under each `getrandom_backend` value that replaces the operating system's
/// generator (`rng.rs`: six of the nine 0.4.2 names; the three per-OS ones still read
/// the operating system), and under `secp256k1_fuzz` and `hashes_fuzz` (`lib.rs`),
/// shown live by `every_refusal_stops_a_build_of_the_core_under_its_cfg` across what its
/// builds vary (debug assertions, features, `panic`), with no build script in the core
/// (`the_core_has_no_build_script`), for a flag that reaches its compile from anywhere,
/// `$CARGO_HOME/config.toml` or an exported `RUSTFLAGS` among them. A twin keyed on any
/// other flag a host adds (a target, a `target_feature`, a non-forbidden `--cfg`) passes
/// it, owed. It does not hold for a flag aimed at another crate alone (a rustc
/// wrapper that adds it for `getrandom` only, nightly per-package `profile-rustflags`, a
/// `[target.<triple>.<links>] rustc-cfg` build-script override); for the Tor plugin,
/// a separate cargo build with no core in it (its lock carries `getrandom` 0.4.3 and
/// 0.3.4), where nothing refuses; or for a binary built elsewhere (cargokit's
/// `precompiled_binaries`, refused with `cargokit.yaml`). This scan does not read a
/// script outside `scripts/` (the one-off probes under `docs/`, the registry checkers
/// under `evals/`) or a file behind a symlinked directory (`walk`; the floors red when a
/// podspec, a plugin build file, the slot or a plugin's whole `cargokit/` sits behind
/// one), and no tree scan reads a machine's cargo home or the environment a build
/// inherits.
///
/// Mutants (the plan's): `sdk/.cargo/config.toml` with `rustflags = ["--cfg",
/// "zcash_unstable=\"nutachyon\""]`; the same file with
/// `getrandom_backend=\"custom\"`; `#[cfg(zcash_unstable = "nutachyon")]` in one
/// source file. The first two are refused by their existence too, in the test that
/// does that. Planted there they also reach the build that runs these tests (each
/// trips its `compile_error!` before any test runs), so they are watched with
/// `CARGO_ENCODED_RUSTFLAGS` set to the empty string: cargo takes it before any
/// config file's `rustflags`, and an empty value leaves the cached build fresh.
#[test]
fn no_forbidden_cfg_reaches_a_shipped_build() {
    let sdk = sdk_root();
    let root = monorepo_root();
    let base = root.clone().unwrap_or_else(|| sdk.clone());
    let read = carriers(&sdk, root.as_deref());

    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut emitting_cfg = 0;
    let mut sites_found = [0; ADMITTED.len()];
    let mut sites_admitting = [0; ADMITTED.len()];
    // Every way an admitted site departs from its shape, with the site named.
    let mut misshapen = Vec::new();
    let mut violations = Vec::new();
    let mut handed_to_cargo = Vec::new();
    // Per plugin, how often its cargokit's linker line was admitted.
    let mut linker_admitted = [0; CARGOKIT_PLUGINS.len()];
    // Per line of `FOREIGN_CONFIG_FLAGS`, how often it was admitted.
    let mut foreign_admitted = [0; FOREIGN_CONFIG_FLAGS.len()];
    // What in the core's sources could answer for `compile_error!`.
    let mut shadowed = Vec::new();
    for (path, kind) in &read {
        *kinds.entry(*kind).or_default() += 1;
        let bytes = fs::read(path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()));
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text.lines().collect();
        if *kind == "build script" && text.contains("rustc-cfg=") {
            emitting_cfg += 1;
        }
        // This file's admitted spans, each tagged with its site.
        let rel = path.strip_prefix(&sdk).ok();
        let mut spans = Vec::new();
        for (site, admitted) in ADMITTED.iter().enumerate() {
            if rel == Some(Path::new(admitted.file)) {
                let found = find_site(admitted, &text);
                sites_found[site] += found.len();
                for problem in found.iter().flat_map(|f| &f.problems) {
                    misshapen.push(format!(
                        "sdk/{}, {}: {problem}",
                        admitted.file, admitted.what
                    ));
                }
                spans.extend(found.into_iter().map(|f| (site, f.span)));
            }
        }
        if rel.is_some_and(|r| r.starts_with("zec-wallet-core/src")) {
            let refusal_lines: Vec<(usize, usize)> = spans
                .iter()
                .filter(|(site, _)| matches!(ADMITTED[*site].shape, Shape::Refusal))
                .map(|(_, span)| *span)
                .collect();
            for (line, what) in shadows(&text, &refusal_lines) {
                shadowed.push(format!(
                    "{}:{}: {what}: {}",
                    shown(path, &base),
                    line + 1,
                    lines.get(line).map_or("", |l| l.trim())
                ));
            }
        }
        // The plugin whose cargokit linker line this file may carry.
        let linker_file = CARGOKIT_PLUGINS
            .iter()
            .position(|plugin| rel == Some(Path::new(plugin).join(ADMITTED_OVERRIDE.0).as_path()));
        // This file as `FOREIGN_CONFIG_FLAGS` names one, from the monorepo root.
        let from_root = root.as_deref().and_then(|r| path.strip_prefix(r).ok());
        let marker = comment_marker(path);
        for (i, line) in lines.iter().enumerate() {
            if marker.is_some_and(|m| line.trim_start().starts_with(m)) {
                continue;
            }
            let mut handed = Vec::new();
            if COMMAND_CARRIERS.contains(kind) {
                handed = cargo_overrides(line);
                if let Some(plugin) = linker_file
                    && line.trim() == ADMITTED_OVERRIDE.1
                    && handed == ["CARGO_TARGET_"]
                {
                    linker_admitted[plugin] += 1;
                    handed.clear();
                }
                let foreign = FOREIGN_CONFIG_FLAGS.iter().position(|&(file, admitted)| {
                    from_root == Some(Path::new(file)) && line.trim() == admitted
                });
                if let Some(at) = foreign
                    && handed == ["--config"]
                {
                    foreign_admitted[at] += 1;
                    handed.clear();
                }
            }
            if *kind == "manifest"
                && keys_set(line)
                    .iter()
                    .any(|key| key.iter().any(|k| k == "cargo-features"))
            {
                handed.push("cargo-features");
            }
            for what in handed {
                handed_to_cargo.push(format!(
                    "{}:{}: `{what}` in a {kind}: {}",
                    shown(path, &base),
                    i + 1,
                    line.trim()
                ));
            }
            for cfg in FORBIDDEN_CFGS.into_iter().filter(|cfg| line.contains(cfg)) {
                let admitted_by = spans.iter().find(|(site, (first, last))| {
                    ADMITTED[*site].cfgs.contains(&cfg) && (*first..=*last).contains(&i)
                });
                match admitted_by {
                    Some((site, _)) => sites_admitting[*site] += 1,
                    None => violations.push(format!(
                        "{}:{}: `{cfg}` in a {kind}: {}",
                        shown(path, &base),
                        i + 1,
                        line.trim()
                    )),
                }
            }
        }
    }

    let total: usize = kinds.values().sum();
    println!(
        "no_forbidden_cfg_reaches_a_shipped_build: read {total} files {kinds:?}; \
         {emitting_cfg} build script(s) emit `rustc-cfg`"
    );
    assert!(
        total > 0,
        "read zero files: a scan that examined nothing is not a pass"
    );
    // The floors: each kind that must be there was read (`docs/REVIEW.md` §6).
    let mut floors = Vec::new();
    for spec in PODSPECS {
        if !read
            .iter()
            .any(|(p, k)| *k == "podspec" && *p == sdk.join(spec))
        {
            floors.push(format!("the podspec sdk/{spec} was not read"));
        }
    }
    for plugin in CARGOKIT_PLUGINS {
        let vendored = sdk.join(plugin).join("cargokit");
        if !read
            .iter()
            .any(|(p, k)| *k == "cargokit file" && p.starts_with(&vendored))
        {
            floors.push(format!("no cargokit file of sdk/{plugin} was read"));
        }
    }
    for file in PLUGIN_BUILD_FILES {
        if !read
            .iter()
            .any(|(p, k)| matches!(*k, "Gradle file" | "CMake file") && *p == sdk.join(file))
        {
            floors.push(format!("the plugin build file sdk/{file} was not read"));
        }
    }
    if kinds.get("CI workflow").copied().unwrap_or(0) == 0 {
        floors.push("no CI workflow was read".to_string());
    }
    let runners = kinds.get("command runner").copied().unwrap_or(0);
    if root.is_some() && runners != 2 {
        floors.push(format!(
            "{runners} command runner(s) were read in the monorepo; there are two"
        ));
    }
    if let Some(root) = &root
        && !read
            .iter()
            .any(|(p, k)| *k == "script" && *p == root.join(CARGO_SLOT))
    {
        floors.push(format!("the script {CARGO_SLOT} was not read"));
    }
    assert!(
        floors.is_empty(),
        "the scan read less than the tree carries, so it would pass over a carrier it claims to \
         read. Either a carrier moved (update `PODSPECS`, `CARGOKIT_PLUGINS`, \
         `PLUGIN_BUILD_FILES`, `CARGO_SLOT` or `carriers` deliberately), or the walk no longer \
         reaches it:\n{}",
        floors.join("\n")
    );
    assert!(
        violations.is_empty(),
        "a forbidden cfg ({FORBIDDEN_CFGS:?}) is named outside the admitted sites. In a shipped \
         build it turns on unreleased consensus code or replaces the code that generates or signs \
         (ADR-0579 Decisions 2 and 3). Remove it; a deliberate refusal is admitted by name in \
         `ADMITTED`, never by widening the match:\n{}",
        violations.join("\n")
    );
    assert!(
        handed_to_cargo.is_empty(),
        "a carrier hands cargo what this scan does not read: a program run around or in place of \
         every `rustc`, another source, linker, runner or cargo home, a `--config`, or nightly \
         manifest features. Any of them can add a flag to every compile, or swap a crate, unseen. \
         Remove it:\n{}",
        handed_to_cargo.join("\n")
    );
    for (plugin, admitted) in CARGOKIT_PLUGINS.iter().zip(linker_admitted) {
        assert_eq!(
            admitted, 1,
            "cargokit's Android linker line was admitted {admitted} time(s) in sdk/{plugin}/{}; it \
             must be once. Either cargokit changed it (read the new line, then update \
             `ADMITTED_OVERRIDE` deliberately), or the scan no longer reads that file",
            ADMITTED_OVERRIDE.0
        );
    }
    if root.is_some() {
        for ((file, line), admitted) in FOREIGN_CONFIG_FLAGS.iter().zip(foreign_admitted) {
            assert_eq!(
                admitted, 1,
                "the line in {file} that passes gitleaks, not cargo, a `--config` was admitted \
                 {admitted} time(s); it must be once. Either the line changed (read it, then \
                 update `FOREIGN_CONFIG_FLAGS` deliberately), or the scan no longer reads that \
                 file: {line}"
            );
        }
    }
    for (site, admitted) in ADMITTED.iter().enumerate() {
        assert!(
            sites_found[site] == 1 && sites_admitting[site] > 0,
            "the admitted site in sdk/{}, {}, was found {} time(s) and admitted {} line(s); it \
             must be found once and admit at least one. Either it moved or changed shape (update \
             `ADMITTED` deliberately), or the scan no longer reads that file",
            admitted.file,
            admitted.what,
            sites_found[site],
            sites_admitting[site]
        );
    }
    assert!(
        misshapen.is_empty(),
        "an admitted site is not in the shape it is written in. A refusal that misses a value \
         builds under it, and one under another attribute or inside a `mod` is compiled only where \
         that is (an early warning: whether each refusal stops a build is \
         `every_refusal_stops_a_build_of_the_core_under_its_cfg`'s to say); a check-cfg that misses \
         one leaves a typo in the refusal silent. The value lists are `REPLACING_BACKENDS` and \
         `ZCASH_UNSTABLE_VALUES` above, each with its sources:\n{}",
        misshapen.join("\n")
    );
    assert!(
        shadowed.is_empty(),
        "the core's sources could answer for `compile_error!` in place of the builtin, and a \
         refusal that resolves to another macro compiles to nothing under its cfg. The word \
         `compile_error` belongs to the refusals alone, and the core uses no `macro_use`, \
         `extern crate` or `include!` (`shadows`). Remove it:\n{}",
        shadowed.join("\n")
    );
    let lock = lock_packages(
        &fs::read_to_string(sdk.join("Cargo.lock")).expect("sdk/Cargo.lock readable"),
    );
    let mut moved = Vec::new();
    for (name, read_at) in ZCASH_UNSTABLE_READ_AT {
        let versions: Vec<&str> = lock
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .collect();
        if versions != [read_at] {
            moved.push(format!(
                "`{name}`: sdk/Cargo.lock carries {versions:?}, the list was read at {read_at}"
            ));
        }
    }
    assert!(
        moved.is_empty(),
        "a crate that names `zcash_unstable` moved since `ZCASH_UNSTABLE_VALUES` was read, so the \
         refusal in zec-wallet-core/src/lib.rs may miss a value it now reads. Grep every locked \
         crate's source for `zcash_unstable` again, update the list, the refusal and the \
         check-cfg in sdk/Cargo.toml, then `ZCASH_UNSTABLE_READ_AT`:\n{}",
        moved.join("\n")
    );
    let read_names: BTreeSet<&str> = LOCKED_NAMES_AT_READ.split_whitespace().collect();
    let locked_names: BTreeSet<&str> = lock.iter().map(|(name, _)| name.as_str()).collect();
    let entered: Vec<&&str> = locked_names.difference(&read_names).collect();
    let left: Vec<&&str> = read_names.difference(&locked_names).collect();
    assert!(
        entered.is_empty() && left.is_empty(),
        "the crates in sdk/Cargo.lock changed since `ZCASH_UNSTABLE_VALUES` was read (entered: \
         {entered:?}; left: {left:?}), and a crate new to the lock may read `zcash_unstable`. \
         Derive the crates and their values again with the command at `ZCASH_UNSTABLE_READ_AT`, \
         update the list, the refusal, the check-cfg and that table, then `LOCKED_NAMES_AT_READ`"
    );
    assert!(
        emitting_cfg > 0,
        "no `build.rs` that emits `rustc-cfg` was read (the bridge's emits \
         `zec_wallet_device_timing`): the build-script half of the scan read nothing it exists for"
    );
}

/// The pin set (ADR-0579 Decision 1) as the build resolves it: no manifest under
/// `sdk/` overrides where a dependency comes from. A `[patch]` or `[replace]`
/// table makes the build that was tested differ from the one a host builds. The
/// published package is staged from these manifests and the lock
/// (`scripts/release/stage-pub.py`); the bridge's manifest becomes that package's
/// workspace root, so a `[patch]` there, which cargo ignores in this workspace,
/// applies in the published build. `sdk/Cargo.toml:8` has said "NO [patch]" since
/// the carve; this is the guard that holds it. A cargo config, which can override a
/// source too, is refused by its existence
/// (`no_cargo_config_or_cargokit_yaml_reaches_a_build_of_the_sdk`).
///
/// Read as text (`overrides`), since `cargo metadata` does not report either
/// table: a table header, or the same key set at the root, in any quoting or
/// spacing. A `[patch]` that cargo applies here changes the patched crate's source
/// in the lock, where `extraction_policy::every_sdk_lock_package_resolves_from_a_registry`
/// refuses any source but crates.io; this guard also catches one that does not
/// apply here and would in the published build. The lock is no backstop for every
/// override: a cargo config's `paths` or `[source] replace-with` leaves the lock's
/// crates.io source and checksum as they were.
///
/// Mutant: a `[patch.crates-io]` table for `getrandom` in the bridge's manifest.
#[test]
fn no_patch_or_replace_overrides_a_dependency_of_the_sdk() {
    let sdk = sdk_root();
    let root = monorepo_root();
    let base = root.clone().unwrap_or_else(|| sdk.clone());
    let read = carriers(&sdk, root.as_deref());

    let mut manifests = 0;
    let mut found = Vec::new();
    for (path, _) in read.iter().filter(|(_, kind)| *kind == "manifest") {
        manifests += 1;
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()));
        for (line, what) in overrides(&text) {
            found.push(format!("{}:{line}: {what}", shown(path, &base)));
        }
    }
    assert!(
        manifests > 0 && read.iter().any(|(p, _)| *p == sdk.join("Cargo.toml")),
        "the walk under sdk/ read {manifests} manifest(s) without sdk/Cargo.toml: this guard \
         would read nothing"
    );
    assert!(
        found.is_empty(),
        "a manifest overrides where a dependency comes from, so the build that was tested is not \
         the one a host builds from the published package (ADR-0579 Decision 1). Remove it; a \
         fix the SDK needs ships as a pin to a published release:\n{}",
        found.join("\n")
    );
}

/// The files outside the manifests that configure a build of the SDK and are named
/// on purpose: paths under `sdk/`; a parent's config is never admitted. Empty, and
/// none exists. A cargo config named here is still read by the cfg scan, but every
/// other key in it (a `paths` override, a source replacement, a wrapper, an
/// `include`) applies unread, so it is reviewed whole before it is named.
const ALLOWED_BUILD_CONFIG_FILES: [&str; 0] = [];

/// Where cargo and cargokit look a build's configuration up, from a directory: cargo's
/// two config files under `.cargo/`, in the directory a build runs from and in each
/// parent, and cargokit's per-crate options beside a manifest
/// (`build_tool/lib/src/options.dart:219-230`).
const CONFIG_LOOKUPS: [&str; 3] = [".cargo/config", ".cargo/config.toml", "cargokit.yaml"];

/// Every directory under `dir` a build can take its configuration from, `dir` included,
/// sorted, and how many of them were reached through a symlink: each one outside `.git`
/// and `target`, build outputs and test trees included. A symlinked directory is
/// followed, as cargo and cargokit follow it (step 1g's third review made `sdk/.cargo` a
/// link, which `walk` never enters), and each real directory is listed once, so a link
/// back into the tree ends. Not followed: a link inside Flutter's plugin-link
/// directories, `.symlinks/` (beside an app's Podfile) and `.plugin_symlinks/` (in a
/// desktop app's `ephemeral/`), which Flutter writes. Each points at a plugin's root:
/// this repository's plugins are walked where they are, and the rest are packages in
/// the pub cache, outside this repository as `$CARGO_HOME` is, which following them
/// would walk. An unreadable directory fails.
fn config_dirs(dir: &Path) -> (Vec<PathBuf>, usize) {
    let mut seen = BTreeSet::new();
    let (mut out, mut linked) = (Vec::new(), 0);
    let mut stack = vec![(dir.to_path_buf(), false)];
    while let Some((dir, via_link)) = stack.pop() {
        let real = fs::canonicalize(&dir)
            .unwrap_or_else(|e| panic!("{} does not resolve: {e}", dir.display()));
        if !seen.insert(real) {
            continue;
        }
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|e| panic!("{} is unreadable: {e}", dir.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|e| panic!("{} lists badly: {e}", dir.display()));
            let path = entry.path();
            let skipped = matches!(entry.file_name().to_str(), Some(".git" | "target"));
            if skipped || !path.is_dir() {
                continue;
            }
            let link = entry.file_type().is_ok_and(|t| t.is_symlink());
            let plugin_link = path.components().any(|c| {
                matches!(
                    c.as_os_str().to_str(),
                    Some(".symlinks" | ".plugin_symlinks")
                )
            });
            if !(link && plugin_link) {
                stack.push((path, via_link || link));
            }
        }
        linked += usize::from(via_link);
        out.push(dir);
    }
    out.sort();
    (out, linked)
}

/// What in `dir` itself configures a build, each file once: every path of
/// `CONFIG_LOOKUPS` that resolves when looked up as cargo and cargokit look it up (the
/// operating system follows a link, and on a case-insensitive volume, the default on
/// macOS, finds `.Cargo/Config.toml` too: step 1g's third review planted that
/// spelling); and every entry named, in any case, `cargokit.yaml`, or `config` or
/// `config.toml` inside a `.cargo`, so a case-sensitive volume reds on a spelling a
/// case-insensitive one would resolve. A file both find is shown as its name is on disk.
fn build_configs_in(dir: &Path) -> Vec<PathBuf> {
    let named = |path: &Path, names: &[&str]| {
        path.file_name()
            .is_some_and(|n| names.contains(&n.to_string_lossy().to_lowercase().as_str()))
    };
    let listed = |dir: &Path| -> Vec<PathBuf> {
        fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{} is unreadable: {e}", dir.display()))
            .map(|e| {
                e.unwrap_or_else(|e| panic!("{} lists badly: {e}", dir.display()))
                    .path()
            })
            .collect()
    };
    let mut found = Vec::new();
    for path in listed(dir) {
        if named(&path, &["cargokit.yaml"]) {
            found.push(path);
        } else if named(&path, &[".cargo"]) && path.is_dir() {
            found.extend(
                listed(&path)
                    .into_iter()
                    .filter(|p| named(p, &["config", "config.toml"])),
            );
        }
    }
    found.extend(
        CONFIG_LOOKUPS
            .iter()
            .map(|lookup| dir.join(lookup))
            .filter(|path| fs::metadata(path).is_ok()),
    );
    let mut seen = BTreeSet::new();
    found.retain(|path| {
        let real = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        seen.insert(real.to_string_lossy().to_lowercase())
    });
    found
}

/// ADR-0579 Decisions 1 to 3 at the files that configure a build from outside the
/// manifests: none exists under `sdk/` or at the monorepo root, unless
/// `ALLOWED_BUILD_CONFIG_FILES` names it. It looks in every directory a build can take
/// one from (`config_dirs`: all of `sdk/` but `.git` and `target`, through symlinked
/// directories) as cargo and cargokit look, and by name in any case
/// (`build_configs_in`). Refused by EXISTENCE, not by their keys:
///   * a `.cargo/config` or `.cargo/config.toml`. Cargo obeys one in the directory a
///     build runs from and in every parent, and some of what it sets shows nowhere a
///     guard reads: `paths = [...]` and `[source.crates-io] replace-with` swap a
///     crate's source while `Cargo.lock` keeps crates.io's source and checksum, so
///     `extraction_policy::every_sdk_lock_package_resolves_from_a_registry`,
///     `cargo deny` and `cargo audit` all stay green (probed at step 1g's second
///     review fold, 2026-10-23). Rustflags, wrappers and `include` are the rest;
///   * a `cargokit.yaml`, cargokit's per-crate options beside a manifest
///     (`build_tool/lib/src/options.dart:219-230`). Its `extra_flags` reach `cargo
///     build` on every host that builds the published plugin (`builder.dart:150-168`)
///     and can carry a `--config`; it can also name a toolchain, or
///     `precompiled_binaries`, a binary built elsewhere that bypasses every refusal in
///     the core. (A host's own `cargokit_options.yaml` can only turn precompiled
///     binaries on, which needs this file.)
///
/// What it cannot see: a consumer's build reads cargo configs from wherever its own
/// tool runs cargo (cargokit runs it from the app's build directory, or Xcode's) and
/// from `$CARGO_HOME`, none of them this repository's; a parent of the monorepo root;
/// what sits behind a link in Flutter's plugin-link directories; and a link that does
/// not resolve when it runs. A machine that carries a config under `sdk/` (a scratch
/// file from a watch, say) reds here until it is removed.
///
/// Mutants: a cargo config with `paths`, one with `[source.crates-io] replace-with`,
/// and a `cargokit.yaml` with `precompiled_binaries`; the cfg scan's own config
/// mutants red here too. The third review's: `sdk/.cargo` a symlink to a directory
/// holding a `config.toml`, and `sdk/.Cargo/config.toml`.
#[test]
fn no_cargo_config_or_cargokit_yaml_reaches_a_build_of_the_sdk() {
    let sdk = sdk_root();
    let root = monorepo_root();
    let base = root.clone().unwrap_or_else(|| sdk.clone());
    let (dirs, linked) = config_dirs(&sdk);
    let mut found: Vec<PathBuf> = dirs.iter().flat_map(|dir| build_configs_in(dir)).collect();
    if let Some(root) = &root {
        found.extend(build_configs_in(root));
    }
    println!(
        "no_cargo_config_or_cargokit_yaml_reaches_a_build_of_the_sdk: looked in {} directories \
         under sdk/ ({linked} through a symlink), found {found:?}",
        dirs.len()
    );
    assert!(
        dirs.contains(&sdk) && dirs.contains(&sdk.join("zec-wallet-core/src")),
        "the walk under sdk/ did not reach sdk/zec-wallet-core/src: this guard would look in \
         nothing"
    );
    let mut refused: Vec<String> = found
        .iter()
        .filter(|p| {
            !ALLOWED_BUILD_CONFIG_FILES
                .iter()
                .any(|a| **p == sdk.join(a))
        })
        .map(|p| shown(p, &base))
        .collect();
    refused.sort();
    refused.dedup();
    assert!(
        refused.is_empty(),
        "a file configures a build of the SDK from outside the manifests. A cargo config can swap \
         a crate's source with no trace in Cargo.lock, or add flags, a wrapper or another config; \
         a cargokit.yaml can pass cargo any flag or ship a binary built elsewhere (ADR-0579 \
         Decisions 1 to 3). Remove it; one the SDK needs is named in \
         `ALLOWED_BUILD_CONFIG_FILES`, after a review of every key in it:\n{}",
        refused.join("\n")
    );
}

/// The nineteen ZEC crates whose single copy the NU7 survey checked in the trial
/// lock (`docs/plan/nu7-survey.md` S1.1).
const ZEC_CRATES: [&str; 19] = [
    "zcash_protocol",
    "zcash_address",
    "zip321",
    "zcash_proofs",
    "zcash_primitives",
    "orchard",
    "shardtree",
    "zcash_script",
    "zcash_client_backend",
    "zcash_client_sqlite",
    "zcash_keys",
    "zcash_transparent",
    "sapling-crypto",
    "zip32",
    "incrementalmerkletree",
    "pczt",
    "zcash_pool_migration",
    "bip32",
    "secp256k1",
];

/// The fuzz workspaces' locks, under `sdk/`. Each builds the core, so each
/// carries the whole ZEC stack, and no `cargo update` in `sdk/` touches either.
const FUZZ_LOCKS: [&str; 2] = [
    "zec-wallet-core/fuzz/Cargo.lock",
    "zec-wallet-swap-near/fuzz/Cargo.lock",
];

/// The `name` and `version` of each `[[package]]` in a `Cargo.lock`.
fn lock_packages(lock: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut name = None;
    for line in lock.lines() {
        if line == "[[package]]" {
            name = None;
        } else if let Some(n) = line.strip_prefix("name = ") {
            name = Some(n.trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("version = ")
            && let Some(n) = name.take()
        {
            out.push((n, v.trim_matches('"').to_string()));
        }
    }
    out
}

/// ADR-0579 Decision 1 at the locks, and the gap the Ironwood survey found
/// (`docs/plan/ironwood-phase-b-survey.md` C7): nothing checked a lock for a
/// single copy, and a fuzz lock is a second lock that no `cargo update` in
/// `sdk/` touches.
///
/// `sdk/Cargo.lock` carries ONE copy of each of the nineteen crates, and is the
/// reference. Every other lock under `sdk/` is read too, found by walking, so a
/// new one is read the day it appears. Each fuzz lock must carry all nineteen at
/// the reference's versions, one copy each. A lock that carries none of them
/// (the Tor dialer's and the Tor plugin's, today) passes, and a ZEC crate that
/// enters one is held to the same rule. The monorepo's own locks are not read:
/// its root workspace excludes `sdk/`, resolves none of the nineteen, and builds
/// no SDK code, and they are gone once the SDK is extracted.
///
/// Mutant (the plan's): the swap fuzz lock's `zcash_protocol` left at 0.10.5;
/// red, naming that lock and the crate.
#[test]
fn every_lock_carries_one_copy_of_each_zec_crate_at_the_sdk_version() {
    let sdk = sdk_root();
    let base = monorepo_root().unwrap_or_else(|| sdk.clone());
    let reference = lock_packages(
        &fs::read_to_string(sdk.join("Cargo.lock")).expect("sdk/Cargo.lock readable"),
    );
    let mut sdk_version = HashMap::new();
    let mut not_single = Vec::new();
    for krate in ZEC_CRATES {
        let copies: Vec<&str> = reference
            .iter()
            .filter(|(name, _)| name == krate)
            .map(|(_, version)| version.as_str())
            .collect();
        match copies.as_slice() {
            [one] => {
                sdk_version.insert(krate, *one);
            }
            many => not_single.push(format!(
                "sdk/Cargo.lock carries {} copies of `{krate}` ({many:?})",
                many.len()
            )),
        }
    }
    // Every doubled crate is listed, not the first: the one a member's old pin
    // brings in usually brings its own dependencies' second copies with it.
    assert!(
        not_single.is_empty(),
        "the SDK resolves ONE copy of each ZEC crate, the one the pin set names (ADR-0579 \
         Decision 1):\n{}",
        not_single.join("\n")
    );

    let locks: Vec<PathBuf> = files_under(&sdk)
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "Cargo.lock"))
        .filter(|p| *p != sdk.join("Cargo.lock"))
        .collect();
    for fuzz in FUZZ_LOCKS {
        assert!(
            locks.contains(&sdk.join(fuzz)),
            "sdk/{fuzz} was not found: a fuzz lock moved or went, and this guard would pass over \
             it. Update `FUZZ_LOCKS` deliberately"
        );
    }
    let mut problems = Vec::new();
    for lock in &locks {
        let at = shown(lock, &base);
        let packages = lock_packages(
            &fs::read_to_string(lock).unwrap_or_else(|e| panic!("{at} unreadable: {e}")),
        );
        let fuzz = FUZZ_LOCKS.iter().any(|f| *lock == sdk.join(f));
        for krate in ZEC_CRATES {
            let want = sdk_version[&krate];
            let copies: Vec<&str> = packages
                .iter()
                .filter(|(name, _)| name == krate)
                .map(|(_, version)| version.as_str())
                .collect();
            match copies.as_slice() {
                [] if fuzz => problems.push(format!(
                    "{at} carries no `{krate}` (a fuzz lock builds the core: it carries {want}, \
                     as sdk/Cargo.lock does)"
                )),
                [] => {}
                [v] if *v == want => {}
                [v] => problems.push(format!("{at} carries `{krate}` {v}, sdk/Cargo.lock {want}")),
                many => problems.push(format!(
                    "{at} carries {} copies of `{krate}` ({many:?}), sdk/Cargo.lock one ({want})",
                    many.len()
                )),
            }
        }
    }
    println!(
        "every_lock_carries_one_copy_of_each_zec_crate_at_the_sdk_version: sdk/Cargo.lock and \
         {:?}",
        locks.iter().map(|l| shown(l, &base)).collect::<Vec<_>>()
    );
    assert!(
        problems.is_empty(),
        "a lock under sdk/ disagrees with sdk/Cargo.lock on a ZEC crate, so what it builds (the \
         fuzz targets, for a fuzz lock) runs a different consensus and proving stack from the one \
         the wallet ships. Move every lock with the pin set (ADR-0579 Decision 1):\n{}",
        problems.join("\n")
    );
}

/// The source cargo names for a crates.io dependency.
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// Cargo's own reading of every manifest under `sdk/`: the `packages` of one
/// `cargo metadata --no-deps --format-version 1 --offline` per workspace, found
/// by walking (`sdk/`, `sdk/dialer-tor`, the Tor plugin's crate and both fuzz
/// workspaces today), and the run time. Each dependency there is as cargo will
/// build it, whatever the spelling: a quoted or dotted key, a key under a
/// `[target]` table, a renamed crate, a member's `{ workspace = true }` with its
/// features merged in. Every manifest the walk finds must belong to a package or
/// be a workspace root that was read; a run that fails, fails the guard.
fn sdk_packages(sdk: &Path) -> (Vec<Value>, usize, u128) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| env!("CARGO").to_string());
    let canonical = |p: &str| fs::canonicalize(p).unwrap_or_else(|e| panic!("{p} resolves: {e}"));
    let manifests: Vec<PathBuf> = files_under(sdk)
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "Cargo.toml"))
        .collect();
    let started = Instant::now();
    let (mut packages, mut covered, mut workspaces) = (Vec::new(), Vec::new(), 0);
    for manifest in &manifests {
        if covered.contains(manifest) {
            continue;
        }
        let run = Command::new(&cargo)
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--offline",
            ])
            .arg("--manifest-path")
            .arg(manifest)
            .output()
            .unwrap_or_else(|e| panic!("`{cargo} metadata` did not start: {e}"));
        assert!(
            run.status.success(),
            "`cargo metadata` failed on {}, so the declarations below could not be read:\n{}",
            manifest.display(),
            String::from_utf8_lossy(&run.stderr)
        );
        let meta: Value = serde_json::from_slice(&run.stdout).expect("cargo metadata prints JSON");
        workspaces += 1;
        let root = meta["workspace_root"].as_str().expect("a workspace_root");
        covered.push(canonical(root).join("Cargo.toml"));
        for package in meta["packages"].as_array().expect("a packages array") {
            let path = package["manifest_path"].as_str().expect("a manifest_path");
            covered.push(canonical(path));
            packages.push(package.clone());
        }
    }
    let elapsed = started.elapsed().as_millis();
    let unread: Vec<&PathBuf> = manifests.iter().filter(|m| !covered.contains(m)).collect();
    assert!(
        unread.is_empty() && covered.contains(&sdk.join("Cargo.toml")),
        "cargo metadata did not read every manifest under sdk/ (unread: {unread:?}): the checks \
         below would pass over a crate"
    );
    (packages, workspaces, elapsed)
}

/// `=MAJOR.MINOR.PATCH`, with an optional `-pre` suffix: the one requirement that
/// admits exactly one release. `=0.4` admits every 0.4.x.
fn is_exact_pin(req: &str) -> bool {
    let Some(version) = req.strip_prefix('=') else {
        return false;
    };
    let release = version
        .split_once('-')
        .map_or(version, |(release, _)| release);
    let parts: Vec<&str> = release.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The crates that are user-space generators, or exist to build one. With any of
/// them a direct dependency, a seeded stream is nameable beside the generator that
/// signs. `chacha20` joins them with its `rng` feature, and any crate with a
/// feature named `rand` (`bip39`'s makes `bip39::rand::rngs::StdRng` nameable).
/// `rand_core` is not here: it is the traits, and its 0.6 `OsRng` is the
/// operating system's generator our own draws use (the project's cryptography
/// rules).
const USER_SPACE_GENERATORS: [&str; 10] = [
    "rand",
    "rand_chacha",
    "rand_xoshiro",
    "rand_pcg",
    "rand_xorshift",
    "rand_hc",
    "rand_isaac",
    "fastrand",
    "oorandom",
    "nanorand",
];

/// ADR-0579 Decision 3, at the manifests: the generator the wallet proves and
/// signs with is the operating system's, through `getrandom`, and no SDK crate
/// can name a user-space one. Judged on cargo's own reading of every manifest
/// under `sdk/` (`sdk_packages`), in every dependency kind and target:
///   * Every declaration of `getrandom` is the pin. It is `=MAJOR.MINOR.PATCH`, so
///     a patch release arrives by review, never by `cargo update`, and exactly
///     `GETRANDOM_PIN`, the version `REPLACING_BACKENDS` was read at. Its default
///     features are off and its one feature is `sys_rng` (the seam's `SysRng` and
///     `UnwrapErr`, nothing else), counting what a member adds to
///     `{ workspace = true }`, since features unify across the graph. It comes
///     from crates.io, never a path, a git repository or another registry. No
///     `[features]` entry adds to it (`getrandom/…`, `getrandom?/…`,
///     `dep:getrandom`).
///   * No crate in `USER_SPACE_GENERATORS` is declared, no `chacha20` with its
///     `rng` feature, and no dependency with a feature named `rand`, whether in
///     its declaration or in a `[features]` entry (`dep/rand`, `dep?/rand`).
///
/// A `[workspace.dependencies]` entry that no member takes reaches no build, and
/// is not judged. Mutants (the plan's): `rand` added to any SDK manifest; the `=`
/// dropped from the pin; a feature other than `sys_rng` added. The fold's: the
/// spellings cargo accepts that a hand parser missed (a quoted key, a dotted key
/// under a `[target]` table), `rand_chacha`, `bip39`'s `rand`, and the pin at
/// `=0.4`.
#[test]
fn the_os_generator_is_getrandom_pinned_exactly_and_rand_is_not_a_direct_dependency() {
    let sdk = sdk_root();
    let base = monorepo_root().unwrap_or_else(|| sdk.clone());
    let (packages, workspaces, millis) = sdk_packages(&sdk);
    let core = sdk.join("zec-wallet-core/Cargo.toml");

    let mut generators = Vec::new();
    let mut pins = Vec::new();
    let mut added = Vec::new();
    for package in &packages {
        let path = package["manifest_path"].as_str().expect("a manifest_path");
        let manifest = fs::canonicalize(path).unwrap_or_else(|e| panic!("{path} resolves: {e}"));
        let at = shown(&manifest, &base);
        // The key each dependency goes by in `[features]`: its rename, else its name.
        let mut by_key = HashMap::new();
        for dep in package["dependencies"]
            .as_array()
            .expect("a dependencies array")
        {
            let name = dep["name"].as_str().expect("a dependency name");
            let key = dep["rename"].as_str().unwrap_or(name);
            by_key.insert(key, name);
            let features: Vec<&str> = dep["features"]
                .as_array()
                .expect("a features array")
                .iter()
                .filter_map(Value::as_str)
                .collect();
            let declared = format!(
                "{at} declares `{name}`{} ({}{})",
                if key == name {
                    String::new()
                } else {
                    format!(" as `{key}`")
                },
                dep["kind"].as_str().unwrap_or("normal"),
                dep["target"]
                    .as_str()
                    .map(|t| format!(", target `{t}`"))
                    .unwrap_or_default()
            );
            if USER_SPACE_GENERATORS.contains(&name) {
                generators.push(declared.clone());
            }
            if name == "chacha20" && features.contains(&"rng") {
                generators.push(format!("{declared} with its `rng` feature"));
            }
            if features.contains(&"rand") {
                generators.push(format!("{declared} with its `rand` feature"));
            }
            if name == "getrandom" {
                pins.push((manifest.clone(), declared, dep, features));
            }
        }
        for (feature, enables) in package["features"].as_object().expect("a features map") {
            for item in enables
                .as_array()
                .expect("a feature's list")
                .iter()
                .filter_map(Value::as_str)
            {
                let (key, enabled) = match item.strip_prefix("dep:") {
                    Some(key) => (key, None),
                    None => match item.split_once('/') {
                        Some((key, enabled)) => (key.trim_end_matches('?'), Some(enabled)),
                        None => continue, // one of the package's own features
                    },
                };
                let Some(&name) = by_key.get(key) else {
                    continue;
                };
                let entry = format!("{at} `[features]` {feature} = [\"{item}\"]");
                if name == "getrandom" {
                    added.push(entry.clone());
                }
                if enabled == Some("rand") || (name == "chacha20" && enabled == Some("rng")) {
                    generators.push(entry);
                }
            }
        }
    }
    println!(
        "the_os_generator_is_getrandom_pinned_exactly_and_rand_is_not_a_direct_dependency: cargo \
         metadata read {} packages in {workspaces} workspaces in {millis} ms",
        packages.len()
    );
    assert!(
        generators.is_empty(),
        "a user-space generator is nameable from the SDK. Its stream can be seeded, so it must \
         not sit beside the generator that proves and signs (ADR-0579 Decision 3). Take \
         randomness from `getrandom` through `zec-wallet-core/src/rng.rs`, or `rand_core` 0.6's \
         `OsRng` for our own draws:\n{}",
        generators.join("\n")
    );
    assert!(
        added.is_empty(),
        "a `[features]` entry adds to the `getrandom` pin. Features unify across the graph, so it \
         reaches every crate's generator whenever the feature is on:\n{}",
        added.join("\n")
    );
    assert!(
        pins.iter().any(|(manifest, ..)| *manifest == core),
        "zec-wallet-core does not declare `getrandom`: the seam's crate takes the pin, so a check \
         that found no declaration there read nothing"
    );
    for (_, declared, dep, features) in &pins {
        let req = dep["req"].as_str().unwrap_or("");
        assert!(
            is_exact_pin(req),
            "{declared} at `{req}`, which is not an EXACT pin. It is the generator the wallet \
             proves and signs with: a release must arrive by review, never by `cargo update`. \
             Pin it as `=MAJOR.MINOR.PATCH`."
        );
        assert_eq!(
            req, GETRANDOM_PIN,
            "{declared} at `{req}`, and `REPLACING_BACKENDS` was read at `{GETRANDOM_PIN}`. Read \
             the new version's `src/backends.rs`, update that list, `rng.rs`'s refusal and the \
             check-cfg in sdk/Cargo.toml, then `GETRANDOM_PIN`"
        );
        assert_eq!(
            dep["uses_default_features"].as_bool(),
            Some(false),
            "{declared} with its default features: they must be off, so its features are exactly \
             the ones named"
        );
        assert_eq!(
            features.as_slice(),
            ["sys_rng"],
            "{declared} with features {features:?}. The pin carries `sys_rng` alone and a member's \
             additions merge into it, so a feature added anywhere reaches every crate's \
             generator. The seam needs `SysRng` and `UnwrapErr` and nothing else (ADR-0579 \
             Decision 3)"
        );
        assert_eq!(
            dep["source"].as_str(),
            Some(CRATES_IO),
            "{declared} from {}: the generator that signs comes from crates.io, never a path, a \
             git repository or another registry",
            dep["source"]
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The refusals, compiled (step 1g's third review fold)
// ─────────────────────────────────────────────────────────────────────────────

/// The core's lib target, as cargo reads its manifest (`sdk_packages`), is
/// `zec-wallet-core/src/lib.rs`. The cfg scan's admitted sites and `shadows` read that
/// file as the crate root, so a `[lib] path` elsewhere would leave them reading text no
/// build compiles (step 1g's third review planted `[lib] path = "src/core.rs"`, a copy of
/// `lib.rs` without its refusals). The liveness test compiles whatever the target is, and
/// reds on that plant by itself; this keeps the text checks reading the right file.
///
/// Mutant: that plant.
#[test]
fn the_cores_lib_target_is_src_lib_rs() {
    let sdk = sdk_root();
    let (packages, ..) = sdk_packages(&sdk);
    let core = packages
        .iter()
        .find(|p| p["name"] == "zec-wallet-core")
        .expect("cargo metadata read zec-wallet-core");
    let libs: Vec<PathBuf> = core["targets"]
        .as_array()
        .expect("a targets array")
        .iter()
        .filter(|t| {
            t["kind"]
                .as_array()
                .is_some_and(|k| k.iter().any(|k| k == "lib"))
        })
        .map(|t| {
            let path = t["src_path"].as_str().expect("a src_path");
            fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
        })
        .collect();
    assert_eq!(
        libs,
        [sdk.join("zec-wallet-core/src/lib.rs")],
        "the core's lib target is not zec-wallet-core/src/lib.rs, so the refusals the cfg scan \
         admits and the file `shadows` reads as the crate root are not what a build compiles. \
         Keep the crate root at src/lib.rs"
    );
}

/// The builds of the core's lib target the liveness test compiles: a name, cargo's
/// arguments, then rustc's for the core alone. Every combination of three things a host's
/// build sets for the core with no build script in it (`the_core_has_no_build_script`):
/// debug assertions on and off (`-C debug-assertions=off`, what a release build changes);
/// the default features and none (`swap` off, a host flavor); and the panic strategy,
/// unwind and abort (`-C panic=abort`, which sets `cfg(panic = "abort")`; a host's
/// `RUSTFLAGS` can carry it, and the bridge's `seed_port_cabi.rs` names such hosts).
/// A host can build each of the eight, so a twin keyed on any one of them, or on any
/// conjunction, reds in one. rustc takes `-C panic=abort` on the core alone over unwind
/// dependencies in a check build (probed at step 1g's fifth fold). For the host the test
/// runs on; a twin keyed on any other flag a host adds (a target, a `target_feature`, a
/// non-forbidden `--cfg`) passes it, owed.
const LIVENESS_BUILDS: [(&str, &[&str], &[&str]); 8] = [
    ("debug, default features", &[], &[]),
    (
        "debug assertions off, default features",
        &[],
        &["-C", "debug-assertions=off"],
    ),
    (
        "debug, no default features",
        &["--no-default-features"],
        &[],
    ),
    (
        "debug assertions off, no default features",
        &["--no-default-features"],
        &["-C", "debug-assertions=off"],
    ),
    (
        "debug, default features, panic=abort",
        &[],
        &["-C", "panic=abort"],
    ),
    (
        "debug assertions off, default features, panic=abort",
        &[],
        &["-C", "debug-assertions=off", "-C", "panic=abort"],
    ),
    (
        "debug, no default features, panic=abort",
        &["--no-default-features"],
        &["-C", "panic=abort"],
    ),
    (
        "debug assertions off, no default features, panic=abort",
        &["--no-default-features"],
        &["-C", "debug-assertions=off", "-C", "panic=abort"],
    ),
];

/// One `cargo rustc` of the core's lib target: whether it built, the text of each error
/// rustc gave for the core, and cargo's stderr.
struct CoreBuild {
    built: bool,
    errors: Vec<String>,
    stderr: String,
}

/// Builds the core's lib target, from `sdk/` as every gate runs cargo, with `cargo rustc
/// --profile check` (cargo's `check`, at the dev profile's settings), `--locked
/// --offline` so it never moves the lock or fetches, into `target_dir`. `rustc_args`
/// and `cfg` reach the core's compile alone, so the dependencies are built once there
/// and kept.
fn build_core_lib(
    target_dir: &Path,
    cargo_args: &[&str],
    rustc_args: &[&str],
    cfg: Option<&str>,
) -> CoreBuild {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| env!("CARGO").to_string());
    let mut command = Command::new(&cargo);
    command
        .current_dir(sdk_root())
        .args([
            "rustc",
            "-p",
            "zec-wallet-core",
            "--lib",
            "--profile",
            "check",
        ])
        .args(["--locked", "--offline", "--message-format", "json"])
        .arg("--target-dir")
        .arg(target_dir)
        .args(cargo_args)
        .arg("--")
        .args(rustc_args);
    if let Some(cfg) = cfg {
        command.args(["--cfg", cfg]);
    }
    let run = command
        .output()
        .unwrap_or_else(|e| panic!("`{cargo} rustc` did not start: {e}"));
    let errors = String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|m| {
            m["reason"] == "compiler-message"
                && m["target"]["name"] == "zec_wallet_core"
                && m["message"]["level"] == "error"
        })
        .filter_map(|m| m["message"]["message"].as_str().map(str::to_string))
        .collect();
    CoreBuild {
        built: run.status.success(),
        errors,
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
    }
}

/// Whether `error` is the refusal of `cfg`: each refusal's message opens with the cfg it
/// refuses, ``a `<cfg>` cfg ``, and closes `: build without it`.
fn is_refusal_of(error: &str, cfg: &str) -> bool {
    error.starts_with(&format!("a `{cfg}` cfg ")) && error.ends_with(": build without it")
}

/// The last lines of a cargo run's stderr, for a failure message.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(15)..].join("\n")
}

/// ADR-0579 Decisions 2 and 3 where they hold or fail, at the compiler: under each
/// forbidden cfg value the core's lib target does not build, and it fails on that cfg's
/// own refusal. The compiler sees what a text check of the refusals cannot be trusted
/// to: the lib target cargo builds (a `[lib] path`), each module where `mod` and
/// `#[path]` put it, each `#[cfg]` twin and inner `#![cfg(…)]` as these eight builds set
/// them, and whatever `compile_error!` resolves to (step 1g's third review fooled the
/// text checks each of those ways). A twin on a cfg only a build script could set has no
/// source: `the_core_has_no_build_script`. The text checks only warn; this checks.
///
/// For each value (`FORBIDDEN_CFGS` over `refused_values`: two `zcash_unstable` values,
/// six `getrandom_backend` ones, `secp256k1_fuzz` and `hashes_fuzz`) in each of
/// `LIVENESS_BUILDS`, it runs `cargo rustc -p zec-wallet-core --lib --profile check …
/// -- --cfg <value>`, and the build must fail with an error that is the refusal of that
/// cfg (`is_refusal_of`). Each build first runs with no forbidden cfg and must build,
/// so a red is the refusal's and not a broken build's. The cfg reaches the core's
/// compile alone, so the dependencies are checked once into their own target directory,
/// `refusal-liveness` under `CARGO_TARGET_TMPDIR` (this cargo never waits on the lock of
/// the build running the test), and kept; after the first run each build rechecks the
/// core only. It prints how many builds it ran and how long they took.
///
/// What it does not compile: a target other than the host's, so a refusal made dead
/// only for iOS or Android (a `target_os` twin) passes here, and so does a twin keyed on
/// any other flag a host adds that the builds do not vary (a `target_feature`, a
/// non-forbidden `--cfg`), owed; the core's third feature,
/// `apple-insecure-raw-keychain-fallback`, which no shipped build sets; and a build with
/// no core in it (the Tor plugin, a binary built elsewhere; the cfg scan's doc names
/// them).
///
/// Mutants, each a refusal the text checks passed (step 1g's third review): a
/// `macro_rules! compile_error` that a lexer desync hides from `shadows`; an inner
/// `#![cfg(debug_assertions)]` in `rng.rs` with a twin `mod rng` for the release build;
/// `#[path = "rng_live.rs"] mod rng;`; `#[cfg(test)]` and `#[cfg(not(test))]` twins of
/// `mod rng`; `[lib] path` in the core's manifest. The fold's: the `hashes_fuzz`
/// refusal deleted, and alive only with debug assertions on; `rng.rs` only with `swap`.
/// The fifth review's: `#[cfg_attr(panic = "abort", path = "rng_live.rs")] mod rng;`.
#[test]
fn every_refusal_stops_a_build_of_the_core_under_its_cfg() {
    let target_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("refusal-liveness");
    let values: Vec<(&str, String)> = FORBIDDEN_CFGS
        .into_iter()
        .flat_map(|cfg| match refused_values(cfg) {
            [] => vec![(cfg, cfg.to_string())],
            values => values
                .iter()
                .map(|v| (cfg, format!("{cfg}=\"{v}\"")))
                .collect(),
        })
        .collect();
    let started = Instant::now();
    let (mut builds, mut problems) = (0, Vec::new());
    for (build, cargo_args, rustc_args) in LIVENESS_BUILDS {
        let control = build_core_lib(&target_dir, cargo_args, rustc_args, None);
        builds += 1;
        if !control.built {
            problems.push(format!(
                "{build}, no forbidden cfg: the core's lib did not build, so no refusal can be \
                 read in this build:\n{}",
                tail(&control.stderr)
            ));
            continue;
        }
        for (cfg, value) in &values {
            let run = build_core_lib(&target_dir, cargo_args, rustc_args, Some(value));
            builds += 1;
            if run.built {
                problems.push(format!("{build}, `--cfg {value}`: the core's lib built"));
            } else if !run.errors.iter().any(|e| is_refusal_of(e, cfg)) {
                problems.push(format!(
                    "{build}, `--cfg {value}`: the build failed, but not on the refusal of \
                     `{cfg}` (errors: {:?}):\n{}",
                    run.errors,
                    tail(&run.stderr)
                ));
            }
        }
    }
    println!(
        "every_refusal_stops_a_build_of_the_core_under_its_cfg: {builds} builds of the core's lib \
         ({} values in {} builds, and a control in each) in {:.1} s, into {}",
        values.len(),
        LIVENESS_BUILDS.len(),
        started.elapsed().as_secs_f64(),
        target_dir.display()
    );
    assert!(
        problems.is_empty(),
        "a refusal does not stop a build of the core under its cfg, so that build can carry it \
         (ADR-0579 Decisions 2 and 3). Each refusal in zec-wallet-core/src/lib.rs and rng.rs \
         must fail every build below under every value it lists:\n{}",
        problems.join("\n")
    );
}

/// The core has no build script: no target of kind `custom-build` in cargo's reading of
/// its manifest (`sdk_packages`), so neither a `build.rs` beside it nor a `build = …` key.
/// A build script sees what `cargo rustc … -- --cfg …` never reaches (`PROFILE`,
/// `OPT_LEVEL`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_CFG_*`, the whole environment) and can
/// set a cfg for the core alone (`cargo:rustc-cfg`). A `#[cfg_attr(<that cfg>, path =
/// …)] mod rng;` twin could then keep a refusal out of a release build, or out of any
/// build whose flags carry a `--cfg`, and pass every build the liveness test compiles
/// (step 1g's fourth review planted three: keyed on `PROFILE`, on `OPT_LEVEL`, and on an
/// environment value holding `--cfg`). With no build script, the cfgs a dev and a
/// release build of the core can differ in are the profile's own: `debug_assertions` and
/// `panic` (which a host's flags can set too), and `LIVENESS_BUILDS` sets each both ways.
/// The core has no build script today, and so no `links` key, which needs one.
///
/// Mutants: those three plants, each a `build.rs`, `#[cfg_attr(…, path = "rng_live.rs")]`
/// on `mod rng;` and that file, `rng.rs` without its refusal.
#[test]
fn the_core_has_no_build_script() {
    let sdk = sdk_root();
    let (packages, ..) = sdk_packages(&sdk);
    let core = packages
        .iter()
        .find(|p| p["name"] == "zec-wallet-core")
        .expect("cargo metadata read zec-wallet-core");
    let kinds: Vec<(&str, &str)> = core["targets"]
        .as_array()
        .expect("a targets array")
        .iter()
        .flat_map(|t| {
            let path = t["src_path"].as_str().unwrap_or("?");
            let kinds = t["kind"].as_array().expect("a kind array");
            kinds.iter().filter_map(move |k| Some((k.as_str()?, path)))
        })
        .collect();
    assert!(
        kinds.iter().any(|(kind, _)| *kind == "lib"),
        "cargo metadata listed no lib target for zec-wallet-core ({kinds:?}): this test would \
         read nothing"
    );
    let scripts: Vec<&str> = kinds
        .iter()
        .filter(|(kind, _)| *kind == "custom-build")
        .map(|(_, path)| *path)
        .collect();
    assert!(
        scripts.is_empty(),
        "zec-wallet-core has a build script ({scripts:?}). It can set a cfg for the core from \
         the profile or the environment, which no build the liveness test compiles sets, and \
         so switch a twin of a module past its refusal. The core takes no build script; one \
         it needs is a review of all it emits, and a change to this test"
    );
}
