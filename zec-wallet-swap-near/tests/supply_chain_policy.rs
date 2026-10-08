//! P0-7 — the money-path supply chain, and what actually calls it.
//!
//! Contract: `docs/plan/production-readiness-phase-0.md` §4h, assertions A3 and
//! A4. Rules: `docs/plan/production-readiness.md` §2.1 (IT-3 red-first, IT-4
//! mutant row), §2.1a (IT-1 +A: the assertion list is a floor), §2.1b (IT-10:
//! for every negative case, name the mechanism that refuses it).
//!
//! WHY THIS FILE LIVES IN `zec-wallet-swap-near` AND NOT IN `zec-wallet-core`
//! ------------------------------------------------------------------------
//! The contract points at `sdk/zec-wallet-core/tests/pins_policy.rs` as the
//! nearest precedent, and that is where it belongs by subject. It is here
//! instead for one mechanical reason, stated so nobody has to re-derive it:
//! A4 must enumerate the workspace members **as cargo resolves them**, which
//! means parsing `cargo metadata` output, which means `serde_json`.
//! `zec-wallet-swap-near` already declares `serde_json` as a plain dependency;
//! `zec-wallet-core` does not, and adding one would be a manifest edit — the
//! implementer's artifact, which a test author may not touch (IT-6).
//!
//! **THE CALLER THIS FILE ONCE DID NOT HAVE — SUPERSEDED, KEPT AS THE RECORD
//! (IT-8, IT-15).** This paragraph used to read: *"They do **not** run under
//! `just sdk-gate-core` or `just sdk-fast`: both hand-list their test binaries
//! … Adding `--test supply_chain_policy` to those two lines is the
//! implementer's edit, not this file's."* That edit landed at the fold
//! (`Justfile:252` and `:313`), so the sentence is false about the tree today
//! and is quoted here only so the history reads straight — the superseded
//! number is named in order to disown it, not asserted.
//!
//! **The CLASS it described was not fixed then, and is fixed by the guard at
//! the bottom of this file (§4k, P0-7 R1(b)).** `sdk-gate-core` and `sdk-fast`
//! still name test binaries by hand, so an eighth binary would have been
//! invisible for exactly the reason four of the seven were until an earlier revision.
//! `every_sdk_test_target_is_named_by_a_live_carrier` enumerates the sdk
//! workspace's test targets **as cargo resolves them** and asserts each one is
//! named by a carrier that runs unbidden — including this file's own target,
//! which is why it needs no exemption for itself.
//!
//! THE TEST-ONLY OVERRIDES, AND WHY THEY CANNOT FAIL OPEN
//! -----------------------------------------------------
//! `P0_7_SDK_ROOT` and `P0_7_REPO_ROOT` retarget the two scans at a sandbox
//! tree, which is what lets `scripts/gate-proofs/p0-7-publish-guard.sh` mutate
//! their inputs and watch them go red — a source scan nobody mutated is the
//! next vacuous gate in this document (P0-9, P0-10 and P0-13 were all three).
//! An override that points at a tree without the file it names **panics**; it
//! never skips. The only skip in this file is the post-extraction one, and it
//! is keyed on a POSITIVE marker — `sdk/.extracted`, `EXTRACTION_MARKER` —
//! rather than on a missing file. (That sentence was FALSE of the code until
//! §4m G13: the skip fired whenever EITHER carrier file was absent, and
//! the override's own assert accepted either — see `repo_root_at`.)

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

// ─────────────────────────────────────────────────────────────────────────────
// Roots
// ─────────────────────────────────────────────────────────────────────────────

/// The `sdk/` workspace root. Override with `P0_7_SDK_ROOT` (proof harness).
fn sdk_root() -> PathBuf {
    if let Ok(p) = std::env::var("P0_7_SDK_ROOT") {
        let p = PathBuf::from(p);
        assert!(
            p.join("Cargo.toml").is_file(),
            "P0_7_SDK_ROOT={} carries no Cargo.toml. An override that points at a tree \
             without the manifest it names is a harness fault, and this test refuses to \
             report anything about it — it does NOT skip.",
            p.display()
        );
        return p;
    }
    // sdk/zec-wallet-swap-near -> sdk
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("sdk/ resolvable from CARGO_MANIFEST_DIR")
}

/// The file the extraction procedure writes, relative to the SDK workspace
/// root (`sdk_root()`), to say "this SDK now lives in its own repository".
///
/// **WHY A MARKER, AND WHY INSIDE `sdk/` (§4m Q-G13, decided).** The
/// only skip in this file must rest on POSITIVE evidence of extraction, never
/// on the absence of a carrier file: `repo_root()` used to return `None`
/// unless BOTH `lefthook.yml` and `Justfile` existed, so deleting either one
/// made every driven test in this binary return green without spawning `just`
/// (measured at 6524db0f: `5 passed … finished in 0.00s`, 8 ms wall, against
/// a copy holding only `lefthook.yml`). The spec's extraction is `git mv sdk/`
/// to a new repository (`docs/specs/wallet-sdk.md` §1.2), after which
/// `CARGO_MANIFEST_DIR/../..` may be OUTSIDE the repository altogether — so
/// the marker has to travel WITH `sdk/`, and it is read from `sdk_root()`.
/// Rejected candidates, both absences in disguise: `docs/plan/` (a directory
/// a doc sweep can move) and the root `Cargo.toml`'s `[patch]` (what
/// `pins_policy.rs::root_manifest` keys on — one deleted manifest disarms it
/// the same way).
///
/// **The cost, stated (IT-1b):** the extraction procedure must also `touch
/// sdk/.extracted`. If it forgets, this binary PANICS in the new repository,
/// naming the two missing carriers and this marker — loud, one line from a
/// fix — rather than skipping. And a marker committed to the monorepo by
/// mistake, beside both carriers, is a contradiction that also panics; it does
/// not disarm the battery (the row-8 defect with its sign flipped — the
/// planted G13 case).
const EXTRACTION_MARKER: &str = ".extracted";

/// The monorepo root — `Some` when this IS the monorepo; `None` ONLY when the
/// SDK declares itself extracted (`EXTRACTION_MARKER`). A monorepo that has
/// lost `lefthook.yml` or `Justfile` is a HARNESS FAULT that panics naming the
/// file; it never skips (§4m G13). `P0_7_REPO_ROOT` (proof harness) goes
/// through the same rule.
fn repo_root() -> Option<PathBuf> {
    let p = match std::env::var("P0_7_REPO_ROOT") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
    };
    repo_root_at(&p, &sdk_root())
}

/// The rule behind `repo_root()`, over explicit paths: `repo` is where the
/// carriers live, `sdk` is where the marker lives.
fn repo_root_at(repo: &Path, sdk: &Path) -> Option<PathBuf> {
    const CARRIERS: [&str; 2] = ["Justfile", "lefthook.yml"];
    let marker = sdk.join(EXTRACTION_MARKER);
    let present: Vec<&str> = CARRIERS
        .iter()
        .copied()
        .filter(|f| repo.join(f).is_file())
        .collect();
    if marker.is_file() {
        assert!(
            present.is_empty(),
            "HARNESS FAULT: {} says this SDK is extracted, yet {} still carries {present:?}. A \
             marker beside the carriers it claims are gone is a contradiction, and this test \
             refuses to skip on it — a stray `{EXTRACTION_MARKER}` in the monorepo would \
             otherwise switch off every driven gate test in this binary. Delete whichever one \
             is the lie.",
            marker.display(),
            repo.display()
        );
        return None;
    }
    let missing: Vec<&str> = CARRIERS
        .iter()
        .copied()
        .filter(|f| !repo.join(f).is_file())
        .collect();
    assert!(
        missing.is_empty(),
        "HARNESS FAULT: {} is a monorepo — there is no {} — and it is missing {missing:?}. The \
         driven gate tests copy those files; without them every `just_*` test in this binary \
         would return green having spawned nothing (measured at 6524db0f: `5 passed … 0.00s`). \
         If this SDK HAS been extracted into its own repository, the extraction procedure \
         writes `{EXTRACTION_MARKER}` at the SDK workspace root, and that is the only honest \
         skip.",
        repo.display(),
        marker.display()
    );
    Some(repo.to_path_buf())
}

// ─────────────────────────────────────────────────────────────────────────────
// A4 — every member with a wildcard path dep declares `publish = false`
// ─────────────────────────────────────────────────────────────────────────────

/// One workspace member, as cargo resolved it.
struct Member {
    name: String,
    manifest_path: PathBuf,
    /// `true` when the manifest says `publish = false` (cargo reports the
    /// allow-list as an empty array; absent/`None` means "publishable").
    publish_false: bool,
    /// Dependencies declared as a bare `{ path = … }` — cargo reports the
    /// version requirement as `*`, which is exactly what cargo-deny's
    /// `[bans]` wildcard check fires on.
    wildcard_path_deps: Vec<String>,
}

fn cargo_members(sdk: &Path) -> Vec<Member> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(&cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .arg("--manifest-path")
        .arg(sdk.join("Cargo.toml"))
        .output()
        .unwrap_or_else(|e| panic!("could not run `{cargo} metadata` on {}: {e}", sdk.display()));
    assert!(
        out.status.success(),
        "`cargo metadata --no-deps` failed on {} (exit {:?}).\nstderr:\n{}\n\
         This test enumerates the workspace members AS CARGO RESOLVES THEM on purpose \
         (A4): a hand-listed pair passes forever and cannot see a fifth member. If cargo \
         cannot answer, nothing was checked, and that is a failure rather than a pass.",
        sdk.display(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("`cargo metadata` emitted valid JSON");
    let ws: BTreeSet<&str> = meta["workspace_members"]
        .as_array()
        .expect("workspace_members is an array")
        .iter()
        .map(|v| v.as_str().expect("workspace member id is a string"))
        .collect();

    let mut members = Vec::new();
    for pkg in meta["packages"].as_array().expect("packages is an array") {
        let id = pkg["id"].as_str().expect("package id is a string");
        if !ws.contains(id) {
            continue;
        }
        // `publish` is `null` when publishing is allowed anywhere, and an
        // (empty) array when the manifest restricts it. `publish = false` is
        // the empty array; a non-empty array is a registry allow-list, which is
        // NOT `publish = false` and does NOT earn cargo-deny's exemption.
        let publish_false =
            matches!(pkg.get("publish"), Some(v) if v.as_array().is_some_and(|a| a.is_empty()));
        let wildcard_path_deps = pkg["dependencies"]
            .as_array()
            .expect("dependencies is an array")
            .iter()
            .filter(|d| !d["path"].is_null() && d["req"].as_str() == Some("*"))
            .map(|d| d["name"].as_str().unwrap_or("<unnamed>").to_string())
            .collect();
        members.push(Member {
            name: pkg["name"].as_str().expect("package name").to_string(),
            manifest_path: PathBuf::from(
                pkg["manifest_path"]
                    .as_str()
                    .expect("manifest_path is a string"),
            ),
            publish_false,
            wildcard_path_deps,
        });
    }
    members
}

/// A4. `sdk/deny.toml`'s `[bans] allow-wildcard-paths = true` exempts a bare
/// `{ path = … }` dependency from the wildcard ban — but cargo-deny applies
/// that exemption **only to a crate that declares `publish = false`**. So the
/// `publish = false` lines are a gate input: delete one and
/// `cd sdk && cargo deny check` goes red. Measured on this tree, with the
/// line stripped from `zec-wallet-core/Cargo.toml`, `cargo deny check bans`
/// exits 2 with `error[wildcard]: found 1 wildcard dependency for crate
/// 'zec-wallet-core'. allow-wildcard-paths is enabled, but does not apply to
/// public crates as crates.io disallows path dependencies.`
///
/// **The enumeration is cargo's, not a list in this file.** That is the whole
/// point of the assertion: the failure it must catch is a *fifth* member added
/// later with a path dep and no `publish = false`, and a hand-listed pair
/// cannot see one. Watched failing four ways — three real manifests and a
/// synthetic fifth member — by
/// `scripts/gate-proofs/p0-7-publish-guard.sh`.
///
/// What it cannot see: whether `allow-wildcard-paths` is still `true` in
/// `sdk/deny.toml` (flip it to `false` and every path dep is denied again
/// regardless of `publish`), and whether cargo-deny is installed at all. The
/// proof script cross-checks every one of its mutations against real
/// `cargo deny check bans` output, so this test cannot be right for a reason
/// cargo-deny disagrees with.
#[test]
fn every_sdk_member_with_a_wildcard_path_dep_declares_publish_false() {
    let sdk = sdk_root();
    let members = cargo_members(&sdk);

    // ── anti-vacuity, because a scan that matches nothing passes silently ──
    assert!(
        members.len() >= 2,
        "cargo resolved {} workspace member(s) under {} — the enumeration has stopped \
         matching. A query that returns nothing satisfies every assertion below it.",
        members.len(),
        sdk.display()
    );
    for m in &members {
        assert!(
            m.manifest_path.is_file(),
            "cargo named a manifest that is not on disk: {} ({}). The enumeration is not \
             describing this tree.",
            m.manifest_path.display(),
            m.name
        );
    }
    let with_wildcards: Vec<&Member> = members
        .iter()
        .filter(|m| !m.wildcard_path_deps.is_empty())
        .collect();
    assert!(
        !with_wildcards.is_empty(),
        "no member of {} declares a bare `{{ path = … }}` dependency, so this assertion \
         is vacuous. Either the workspace stopped using path deps — in which case \
         `[bans] allow-wildcard-paths` in sdk/deny.toml is now dead configuration and \
         should go — or the `req == \"*\"` shape cargo reports has changed and this test \
         is no longer looking at anything. Members seen: {:?}",
        sdk.display(),
        members.iter().map(|m| &m.name).collect::<Vec<_>>()
    );

    // ── the assertion ──
    let offenders: Vec<String> = with_wildcards
        .iter()
        .filter(|m| !m.publish_false)
        .map(|m| {
            format!(
                "  `{}` ({}) has wildcard path dep(s) {:?} and does NOT declare `publish = false`",
                m.name,
                m.manifest_path.display(),
                m.wildcard_path_deps
            )
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "PUBLISH-FLAG DRIFT — `cd sdk && cargo deny check` will fail with \
         `error[wildcard]` on {} member(s):\n{}\n\n\
         `sdk/deny.toml` sets `[bans] allow-wildcard-paths = true`, and cargo-deny \
         applies that exemption ONLY to a crate that declares `publish = false` \
         (ADR-0541 Decision 1: none of these ship to crates.io). Add `publish = false` \
         to the manifest above, or give the dependency a `version` beside its `path`. \
         Checked over the members AS CARGO RESOLVES THEM, so a member added after this \
         test was written is covered.",
        offenders.len(),
        offenders.join("\n")
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// A3 — the INC-017 guard: the sdk supply chain has a carrier that runs unbidden
// ─────────────────────────────────────────────────────────────────────────────

/// Strip shell/YAML `#` comments from one line, honouring single and double
/// quotes so a `#` inside a quoted string is not mistaken for a comment.
fn strip_comment(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let (mut sq, mut dq) = (false, false);
    for c in line.chars() {
        match c {
            '\'' if !dq => {
                sq = !sq;
                out.push(c);
            }
            '"' if !sq => {
                dq = !dq;
                out.push(c);
            }
            '#' if !sq && !dq => break,
            _ => out.push(c),
        }
    }
    out
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// An executable unit: a lefthook command, or a Justfile recipe. `dir_is_sdk`
/// records a unit-level working-directory declaration (lefthook's `root:`,
/// just's `[working-directory: '…']`), because a lane can be scoped to `sdk/`
/// without the word `cd` appearing anywhere in its body.
#[derive(Default)]
struct Unit {
    name: String,
    dir_is_sdk: bool,
    /// Comment-stripped, whitespace-squashed body lines.
    lines: Vec<String>,
    /// Recipes this unit hands off to (`just <name>`, or a recipe's own deps).
    calls: Vec<String>,
    /// **(§4l), added beside `calls` rather than replacing it.** `calls`
    /// flattens two things `just` treats very differently — a recipe's declared
    /// dependencies and a `just <name>` written in its body — and the ORDER
    /// question §4l asks cannot be answered from the flattened list. The three
    /// fields below record the same hand-offs with their position kept, and
    /// `calls` is left byte-identical to what built so `closure`, A3
    /// and `every_sdk_test_target_is_named_by_a_live_carrier` see no change
    /// (G8: what the guard refuses today, it still refuses).
    ///
    /// The recipe's declared dependency list, in header order. `just` runs
    /// these left to right and ABORTS at the first one that fails (measured,
    /// just 1.49.0 — see `first_aborting_line`).
    deps: Vec<String>,
    /// `just <name>` hand-offs found in the BODY, each with the index into
    /// `lines` of the line that makes the call.
    line_calls: Vec<(usize, String)>,
    /// True when the body is executed as **one script** rather than one line at
    /// a time: a `just` shebang recipe (first body line starts with `#!`), or a
    /// lefthook `run:` block, which lefthook hands to a shell whole.
    ///
    /// The difference is the entire subject of §4l. `just` runs a non-shebang
    /// recipe's lines as separate `bash -uc` invocations and aborts the recipe
    /// at the first that fails, so only the FIRST line is guaranteed to run. A
    /// script body without `set -e` runs every line regardless of what failed
    /// above it. Which of the two shapes a carrier has decides whether the
    /// targets it names are actually reached.
    ///
    /// It has to be recorded HERE, at parse time, because `strip_comment`
    /// deletes the `#!` line before it ever reaches `lines`.
    script_body: bool,
}

/// Is this ONE line scoped at the `sdk/` workspace?
///
/// Factored out of `Unit::runs_sdk` (P0-7 R1(b)) because the target-name
/// scan at the bottom of this file asks the identical question of the identical
/// lines, and a second copy of this predicate is the duplicate R10 forbids.
/// Behaviour is unchanged from the form.
///
/// Deliberately LOOSE: `cd sdk/<member>` also matches. That is right for
/// `runs_sdk` (a tool run inside a member still runs against this workspace's
/// lockfile) and wrong for package selection, so the target scan pairs it with
/// `sdk_is_workspace_root` below rather than reusing it alone.
fn line_targets_sdk(dir_is_sdk: bool, line: &str) -> bool {
    dir_is_sdk
        || line.contains("cd sdk")
        || line.contains("--manifest-path sdk/Cargo.toml")
        || line.contains("--manifest-path=sdk/Cargo.toml")
}

impl Unit {
    /// Does this unit invoke `cargo <tool>` against the `sdk/` workspace?
    ///
    /// Scoped per LINE, not per body: today's `cargo-audit` lane runs the tool
    /// twice, once at the repo root and once in `sdk/`, so "the body mentions
    /// `cd sdk` somewhere and `cargo audit` somewhere" would also be satisfied
    /// by a lane that runs `cd sdk && cargo fmt` beside a root-only audit.
    fn runs_sdk(&self, tool: &str) -> bool {
        let needle = format!("cargo {tool}");
        let alt = format!("cargo-{tool}");
        self.lines.iter().any(|l| {
            let mentions = l.contains(&needle) || l.contains(&alt);
            if !mentions {
                return false;
            }
            line_targets_sdk(self.dir_is_sdk, l)
        })
    }
}

/// Parse `lefthook.yml` into (hook name -> its command units).
fn parse_lefthook(text: &str) -> BTreeMap<String, Vec<Unit>> {
    let mut hooks: BTreeMap<String, Vec<Unit>> = BTreeMap::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut hook: Option<String> = None;
    let mut in_commands = false;
    let mut i = 0usize;
    while i < lines.len() {
        let raw = lines[i];
        let stripped = strip_comment(raw);
        let trimmed = stripped.trim();
        // A top-level key at column 0 opens (or closes) a hook section.
        if !stripped.is_empty() && indent_of(&stripped) == 0 && trimmed.ends_with(':') {
            hook = Some(trimmed.trim_end_matches(':').to_string());
            in_commands = false;
            i += 1;
            continue;
        }
        let Some(h) = hook.clone() else {
            i += 1;
            continue;
        };
        if trimmed == "commands:" {
            in_commands = true;
            i += 1;
            continue;
        }
        if !in_commands || trimmed.is_empty() {
            i += 1;
            continue;
        }
        // A command block: `    <name>:` at the commands' child indent.
        let cmd_indent = indent_of(&stripped);
        if trimmed.ends_with(':') && !trimmed.contains(' ') && cmd_indent >= 4 {
            let mut unit = Unit {
                name: trimmed.trim_end_matches(':').to_string(),
                ..Default::default()
            };
            i += 1;
            // Consume the block's own keys.
            while i < lines.len() {
                let s = strip_comment(lines[i]);
                if !s.trim().is_empty() && indent_of(&s) <= cmd_indent {
                    break;
                }
                let t = s.trim();
                if let Some(v) = t.strip_prefix("root:") {
                    let v = v.trim().trim_matches(['"', '\'']).trim_end_matches('/');
                    unit.dir_is_sdk = v == "sdk";
                }
                if let Some(v) = t.strip_prefix("run:") {
                    let v = v.trim();
                    if v == "|" || v == ">" || v == "|-" || v == ">-" {
                        let key_indent = indent_of(&s);
                        i += 1;
                        while i < lines.len() {
                            let b = strip_comment(lines[i]);
                            if !b.trim().is_empty() && indent_of(&b) <= key_indent {
                                break;
                            }
                            let sq = squash(&b);
                            if !sq.is_empty() {
                                unit.lines.push(sq);
                            }
                            i += 1;
                        }
                        continue;
                    } else if !v.is_empty() {
                        unit.lines.push(squash(v));
                    }
                }
                i += 1;
            }
            // A lefthook `run:` block is handed to a shell as ONE script, so it
            // has script semantics whether it was written inline or as a block
            // scalar — the same shape a `just` shebang recipe has.
            unit.script_body = true;
            record_calls(&mut unit);
            hooks.entry(h).or_default().push(unit);
            continue;
        }
        i += 1;
    }
    hooks
}

/// Fill a unit's body-line hand-offs, keeping `calls` byte-identical to the
/// form (deps first, then body calls in line order) while ALSO
/// recording which line each body call sits on (§4l).
fn record_calls(unit: &mut Unit) {
    let mut found: Vec<(usize, String)> = Vec::new();
    for (idx, l) in unit.lines.iter().enumerate() {
        let mut got = Vec::new();
        collect_just_calls(l, &mut got);
        for n in got {
            found.push((idx, n));
        }
    }
    for (_, n) in &found {
        unit.calls.push(n.clone());
    }
    unit.line_calls = found;
}

/// `just foo`, `just --justfile X foo` → record `foo`. Deliberately loose: a
/// missed hand-off makes this scan stricter, never laxer.
fn collect_just_calls(line: &str, out: &mut Vec<String>) {
    let toks: Vec<&str> = line.split_whitespace().collect();
    let mut k = 0usize;
    while k < toks.len() {
        if toks[k].trim_matches(['(', '"', '\'']) == "just" {
            let mut j = k + 1;
            while j < toks.len() && toks[j].starts_with('-') {
                j += 2; // skip a flag and its value
            }
            if let Some(name) = toks.get(j) {
                let n = name.trim_matches(['"', '\'', ')', ';']);
                if !n.is_empty() && !n.starts_with('-') {
                    out.push(n.to_string());
                }
            }
        }
        k += 1;
    }
}

/// Parse a `Justfile` into (recipe name -> unit). Recipe deps land in `calls`.
fn parse_justfile(text: &str) -> BTreeMap<String, Unit> {
    let mut out: BTreeMap<String, Unit> = BTreeMap::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut pending_workdir_sdk = false;
    let mut i = 0usize;
    while i < lines.len() {
        let raw = lines[i];
        let s = strip_comment(raw);
        let t = s.trim();
        if t.is_empty() {
            i += 1;
            continue;
        }
        // `[working-directory: 'sdk']` attaches to the NEXT recipe.
        if indent_of(&s) == 0 && t.starts_with('[') {
            let inner = t.trim_matches(['[', ']']);
            if let Some(v) = inner.strip_prefix("working-directory:") {
                pending_workdir_sdk =
                    v.trim().trim_matches(['"', '\'']).trim_end_matches('/') == "sdk";
            }
            i += 1;
            continue;
        }
        // A recipe header at column 0: `name: dep1 dep2` (no `=`, no `:=`).
        if indent_of(&s) == 0 && !t.starts_with('#') && t.contains(':') && !t.contains(":=") {
            let (head, deps) = t.split_once(':').expect("contains ':'");
            let name = head.split_whitespace().next().unwrap_or("").to_string();
            let ok = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if ok {
                let mut unit = Unit {
                    name: name.clone(),
                    dir_is_sdk: pending_workdir_sdk,
                    ..Default::default()
                };
                pending_workdir_sdk = false;
                for d in deps.split_whitespace() {
                    let d = d.trim_matches(['(', ')', '"', '\'']);
                    if !d.is_empty() && !d.starts_with('+') && !d.starts_with('$') {
                        unit.calls.push(d.to_string());
                        unit.deps.push(d.to_string());
                    }
                }
                i += 1;
                let mut seen_body_line = false;
                while i < lines.len() {
                    let b = lines[i];
                    if !b.trim().is_empty() && indent_of(b) == 0 {
                        break;
                    }
                    // The shebang is read from the RAW line: `strip_comment`
                    // treats `#!/usr/bin/env bash` as a comment and deletes it,
                    // so by the time a line reaches `lines` the one fact §4l
                    // needs most about a recipe is already gone.
                    if !seen_body_line && !b.trim().is_empty() {
                        seen_body_line = true;
                        unit.script_body = b.trim_start().starts_with("#!");
                    }
                    let sq = squash(&strip_comment(b));
                    if !sq.is_empty() {
                        unit.lines.push(sq);
                    }
                    i += 1;
                }
                record_calls(&mut unit);
                out.insert(name, unit);
                continue;
            }
        }
        pending_workdir_sdk = false;
        i += 1;
    }
    out
}

/// Walk a unit and everything it hands off to, collecting the reachable set.
fn closure<'a>(seed: &[&'a Unit], recipes: &'a BTreeMap<String, Unit>) -> Vec<&'a Unit> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<&Unit> = seed.to_vec();
    let mut out: Vec<&Unit> = Vec::new();
    while let Some(u) = stack.pop() {
        if !seen.insert(format!("{}@{}", u.name, u.lines.len())) {
            continue;
        }
        out.push(u);
        for c in &u.calls {
            if let Some(next) = recipes.get(c) {
                stack.push(next);
            }
        }
    }
    out
}

/// **THE DEFINITION OF "A LIVE CARRIER", IN ONE PLACE (R6).**
///
/// A carrier is live when it runs **without a human typing a command**. Exactly
/// two exist in this repository and this function returns both closures:
///
///   * **ARM P** — lefthook's `pre-commit` hook. Fires on every commit.
///   * **ARM N** — the launchd nightly, whose payload is `just sdk-gate`. Its
///     driver is out of tree (`~/.local/bin/sdk-gate-nightly.sh`), so a fresh
///     clone does not have it; the closure grades the recipe the driver calls.
///
/// **What is NOT a live carrier, and why** — the same ruling A3 already made at
/// its own assertion site, reused rather than re-argued (R6 asks this guard to
/// match the precedent or argue the divergence; it matches):
///   * `pre-push` — `.push-gate` means this repository never pushes;
///   * `just ci` / `just sdk-fast` — a human typing;
///   * `.github/workflows/ci.yml` — has never executed against any commit in
///     this history. Note what that costs: `just sdk-ci` runs
///     `cd sdk && cargo test --workspace`, which really does run every target,
///     and it is reachable from `just ci`. Counting it would make the R1(b)
///     guard **vacuous today** — every target is trivially covered and the
///     guard could never go red. Counting it the day CI actually runs would be
///     correct. The definition is therefore keyed to what executes unbidden,
///     not to what would be nice, and the day CI starts running this comment is
///     the thing to revisit.
///
/// Factored out of A3 so the R1(b) guard grades the SAME two arms rather than a
/// second hand-written idea of what is live (R10, and one source of truth for a
/// cross-cutting predicate).
fn carrier_arms<'a>(
    hooks: &'a BTreeMap<String, Vec<Unit>>,
    recipes: &'a BTreeMap<String, Unit>,
) -> (Vec<&'a Unit>, Vec<&'a Unit>) {
    let (seed_p, seed_n) = carrier_seeds(hooks, recipes);
    (closure(&seed_p, recipes), closure(&seed_n, recipes))
}

/// The two live arms' SEEDS, before any walk. Split out of `carrier_arms` at
/// so the §4l reachability walk starts from the same definition of "live"
/// the coverage walk uses — two walks over two hand-written ideas of what runs
/// unbidden is the duplicate R10 forbids, and it is how the two could silently
/// come to grade different carriers.
fn carrier_seeds<'a>(
    hooks: &'a BTreeMap<String, Vec<Unit>>,
    recipes: &'a BTreeMap<String, Unit>,
) -> (Vec<&'a Unit>, Vec<&'a Unit>) {
    let pre_commit = hooks.get("pre-commit").map(Vec::as_slice).unwrap_or(&[]);
    let seed_p: Vec<&Unit> = pre_commit.iter().collect();
    let seed_n: Vec<&Unit> = recipes.get("sdk-gate").into_iter().collect();
    (seed_p, seed_n)
}

/// A3 — the named guard for `evals/incidents.tsv` INC-017, owed part (1).
///
/// INC-017: *"The supply-chain gate has never once looked at the money-path
/// lockfile."* P0-6 made `cd sdk && cargo audit` / `cd sdk && cargo deny check`
/// blocking, and put them in lefthook's **`pre-push`** hook — and `.push-gate`
/// means this repository never pushes, so the blocking lane has no caller that
/// ever executes. This asserts the missing half: both money-path legs are
/// reachable from a carrier that runs **without a human typing a command** —
/// lefthook's `pre-commit` hook, or the launchd nightly's `just sdk-gate`
/// closure (the two carriers the contract's premise names, §4h).
///
/// **THIS IS A SOURCE SCAN, AND HERE IS WHAT IT CANNOT SEE.** It reads
/// `lefthook.yml` and the `Justfile` as text. It therefore cannot see:
///   * whether the hook is INSTALLED (`lefthook install` writes
///     `.git/hooks/pre-commit`; a fresh clone has no hook at all until then);
///   * whether `lefthook` is on PATH when git fires the hook;
///   * whether the launchd job exists — the nightly's driver is out of tree,
///     at `~/.local/bin/sdk-gate-nightly.sh`, so a fresh clone does not have it;
///   * whether the command **does** what it says. A lane whose body is
///     `echo "cd sdk && cargo audit"` satisfies this scan exactly as a lane
///     that runs it does. That blind spot is deliberate — stripping shell
///     string literals would also blind the scan to `sh -c "cd sdk && …"`,
///     which is a real invocation — and it is why this test is only half of
///     A1/A3. `scripts/gate-proofs/p0-7-carrier-reachability.sh` is the other
///     half: it plants a real RustSec advisory in `sdk/Cargo.lock` and drives a
///     real `git commit`, so it grades execution rather than configuration.
///     The proof runs the echo-only lane through BOTH and shows this test
///     passing on it while the carrier lets the advisory through.
///   * a hand-off through anything but `just` — `sh scripts/foo.sh` is not
///     followed.
///
/// Watched failing by `scripts/gate-proofs/p0-7-publish-guard.sh` against three
/// mutations of the real config: the legs left in `pre-push` only, the legs
/// present only as a `pre-commit` COMMENT, and one leg wired without the other.
#[test]
fn sdk_supply_chain_gate_is_reached_by_a_carrier_that_runs_unbidden() {
    let Some(root) = repo_root() else {
        return; // extracted to its own repo — there is no lefthook.yml/Justfile here
    };
    let lefthook = std::fs::read_to_string(root.join("lefthook.yml")).unwrap_or_default();
    let justfile = std::fs::read_to_string(root.join("Justfile")).unwrap_or_default();
    let recipes = parse_justfile(&justfile);
    let hooks = parse_lefthook(&lefthook);

    // ── anti-vacuity: the parsers must actually be seeing this tree ──
    assert!(
        recipes.contains_key("ci") && recipes.contains_key("sdk-gate"),
        "the Justfile parser resolved {} recipe(s) and found neither `ci` nor `sdk-gate` \
         under {}. It has stopped matching, and a parser that returns nothing satisfies \
         every assertion below it. Recipes seen: {:?}",
        recipes.len(),
        root.display(),
        recipes.keys().collect::<Vec<_>>()
    );
    let pre_commit = hooks.get("pre-commit").map(Vec::as_slice).unwrap_or(&[]);
    assert!(
        !pre_commit.is_empty(),
        "the lefthook parser found no `pre-commit` commands under {}. Hooks seen: {:?}",
        root.display(),
        hooks.keys().collect::<Vec<_>>()
    );

    // ── the two live arms: lefthook `pre-commit`, and the nightly's sdk-gate ──
    // Definition and its cost live on `carrier_arms`, one source of truth.
    let (reach_p, reach_n) = carrier_arms(&hooks, &recipes);
    let p_audit = reach_p.iter().any(|u| u.runs_sdk("audit"));
    let p_deny = reach_p.iter().any(|u| u.runs_sdk("deny"));
    let n_audit = reach_n.iter().any(|u| u.runs_sdk("audit"));
    let n_deny = reach_n.iter().any(|u| u.runs_sdk("deny"));

    let arm_p = p_audit && p_deny;
    let arm_n = n_audit && n_deny;

    assert!(
        arm_p || arm_n,
        "INC-017 IS STILL OPEN — nothing that runs by itself looks at sdk/Cargo.lock.\n\
         \n\
         lefthook `pre-commit` closure: cargo audit(sdk)={p_audit}, cargo deny(sdk)={p_deny}\n\
         `just sdk-gate` closure:       cargo audit(sdk)={n_audit}, cargo deny(sdk)={n_deny}\n\
         \n\
         Both money-path legs must be reachable from ONE of those two carriers. \
         `pre-push` does not count: `.push-gate` means this repository never pushes, so \
         a lane there has no caller that ever executes. `just ci` and the CI workflow do \
         not count either — `just ci` is a human typing a command, and \
         .github/workflows/ci.yml has never executed against any commit in this history \
         (contract §4h, the load-bearing premise). Wiring those two is still correct and \
         is part of the item; it just does not close it.\n\
         \n\
         This is a source scan: it can see the configuration and not the execution. \
         `scripts/gate-proofs/p0-7-carrier-reachability.sh` grades the other half by \
         planting a real advisory and driving a real `git commit`."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// P0-7 R1(b) — the class fix for the hand-list.
// Contract: `docs/plan/production-readiness-phase-0.md` §4k.
//
// THE PREMISE, RESTATED SO IT CAN BE ATTACKED: the four invisible binaries were
// invisible because two recipes name test targets BY HAND, and any guard that
// also names things by hand inherits the same defect one level up. So this guard
// asks cargo what exists and asks the carriers what they name, and compares the
// two sets. It contains no list of expected targets, on purpose.
//
// THE CONTRACT ASKS FOUR QUESTIONS. Answered here, in the code, with the cost of
// each answer (IT-1b), because a mechanism chosen without its cost written down
// is a decision nobody can review.
//
// Q1 — WHAT IS "NAMED BY A LIVE CARRIER"?
//   ANSWER: the precedent's definition, unchanged — see `carrier_arms`. Two
//   arms: lefthook `pre-commit` and the nightly's `just sdk-gate` closure.
//   COST: `just sdk-ci` really does run `cd sdk && cargo test --workspace`, so
//   in the ordinary sense every target IS run by something. Refusing to count it
//   is what keeps this guard capable of going red at all; counting it would make
//   it vacuous on arrival. The price is that the guard is silent about a world
//   where CI runs and the nightly does not — if `.push-gate` lifts and ci.yml
//   starts executing, `carrier_arms` is the one place to change.
//
// Q2 — HOW DOES THE GUARD SEE A NAME?
//   ANSWER: it reads the carrier files as TEXT (the existing `parse_justfile` /
//   `parse_lefthook` walk), finds `cargo test` invocations on lines scoped to
//   `sdk/`, and models cargo's own package/target selection from the flags —
//   `--workspace`, `--exclude`, `-p`, `--test`, `--tests`, `--all-targets`, and
//   the target flags that select something OTHER than integration tests.
//   WHAT IT CANNOT SEE, and every one of these FAILS CLOSED rather than passing:
//     * a name produced by shell expansion (`--test $NAME`, backticks) — the
//       whole invocation is refused, because the analogous blind spot in the
//       other direction (`--workspace $MAYBE_EXCLUDE`) would OVER-count;
//     * a `--` separator or a positional test-name filter — a filter can leave
//       a named binary running nothing;
//     * an unrecognised flag that swallows its value, and any short flag this
//       scan does not know;
//     * `-p 'zec-*'` or `-p a,b` — globs and comma lists are refused rather
//       than half-parsed;
//     * an implicit package scope that the scan cannot pin to the sdk workspace
//       ROOT (`cd sdk/<member> && cargo test` selects ONE package, not all).
//   The one blind spot that fails OPEN is inherited and deliberate: a body of
//   `echo "cargo test --workspace"` satisfies this scan. Stripping shell string
//   literals would also blind it to `sh -c "cd sdk && …"`, which is a real
//   invocation — the same trade A3 records above, made the same way (R6).
//   A hand-off through anything but `just` is still not followed; that direction
//   can only make a target look LESS covered, so it fails closed for free.
//
// Q3 — THE SELF-REFERENCE. A guard living in the sdk workspace is itself a test
//   target that must be named, or it accuses itself.
//   ANSWER: resolution 1 of the three the contract offers — wire it into a live
//   carrier — taken in its cheapest form: the guard is added to the test target
//   that `sdk-gate-core` ALREADY names (`Justfile:252`), so the eighth target
//   is never created and the paradox is dissolved rather than exempted. There
//   is no exemption list in this file, which is the strongest possible answer to
//   IT-15: no exemption exists, so nothing can hide in one. The guard checks its
//   own target exactly like the other six, so R7 is enforced BY the guard rather
//   than asserted beside it — if someone unwires `--test supply_chain_policy`
//   from `sdk-gate-core`, this test is the thing that goes red about it.
//   COST, stated: `supply_chain_policy.rs` now carries two subjects — the
//   money-path supply chain (A3/A4) and carrier coverage (R1(b)) — and it grows
//   past 900 lines. A future SDK extraction moves both together whether or not
//   that is wanted. The alternative shapes cost more: a new `tests/*.rs` needs
//   its own `sdk-gate-core` and `sdk-fast` lines plus a shared module for the
//   walker, and the `scripts/gate-proofs/`-shaped slot outside the workspace
//   would leave the `evals/mutants.tsv` named-test discipline entirely.
//
// Q4 — WHAT HAPPENS THE DAY THE HAND-LIST IS REPAIRED?
//   ANSWER (R9, decided here and asserted by
//   `a_workspace_wide_invocation_names_every_target`, not discovered later): a
//   `--workspace` (or `--all`) invocation on an sdk-scoped line in a live
//   carrier counts as naming every target that workspace resolves. Replace
//   `sdk-gate-core`'s four hand-written lines with `cd sdk && cargo test
//   --workspace` and this guard stays green. Without that carve-out the guard
//   would go red at exactly the moment the underlying defect was fixed, and the
//   first person to hit it would delete it.
//   COST: the carve-out is a hole the size of `--exclude`. `cargo test
//   --workspace --exclude zec_wallet` is a wildcard invocation that silently
//   drops three targets, two of which guard custody and the swap kill switch.
//   The contract does not name that case; it is planted and asserted below
//   (IT-1 +A), and it is the case this file would fail on first if the model of
//   cargo's selection is wrong.
//
// AND THE FIFTH QUESTION, the one the owed row did not ask: does this grade ALL
// carriers or only the Justfile? Both live ones. `carrier_arms` seeds from
// `lefthook.yml` AND the `Justfile` and follows `just` hand-offs across the two,
// so a target named only in lefthook counts and a target named in neither does
// not. `ci.yml` is excluded by Q1's ruling — by *what executes*, not by
// omission — and that exclusion is the thing to revisit, not a gap to widen.
// ─────────────────────────────────────────────────────────────────────────────

/// One integration-test target, as cargo resolved it. There is no list of these
/// anywhere in this file; that is the entire point of the item.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TestTarget {
    package: String,
    name: String,
    src_path: PathBuf,
}

impl TestTarget {
    fn key(&self) -> (String, String) {
        (self.package.clone(), self.name.clone())
    }
}

/// R1 — enumerate the sdk workspace's integration-test targets AS CARGO RESOLVES
/// THEM. Panics if cargo cannot answer: an enumeration that did not happen
/// checked nothing, and a guard that reports "nothing to check" as a pass is the
/// vacuous gate this whole phase exists to delete.
///
/// Watched refusing by `the_target_enumeration_fails_when_cargo_cannot_answer`.
fn cargo_test_targets(sdk: &Path) -> Vec<TestTarget> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(&cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .arg("--manifest-path")
        .arg(sdk.join("Cargo.toml"))
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "could not run `{cargo} metadata` on {}: {e}\n\
                 If cargo cannot answer, nothing was checked, and that is a failure rather \
                 than a pass.",
                sdk.display()
            )
        });
    assert!(
        out.status.success(),
        "`cargo metadata --no-deps` failed on {} (exit {:?}).\nstderr:\n{}\n\
         This test enumerates the sdk workspace's TEST TARGETS as cargo resolves them \
         (§4k R1): a hand-written list of expected targets passes forever and cannot see \
         an eighth one, which is the defect the item exists to remove. \
         If cargo cannot answer, nothing was checked, and that is a failure rather than \
         a pass.",
        sdk.display(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "`cargo metadata` emitted output this test could not parse ({e}). \
             If cargo cannot answer, nothing was checked, and that is a failure rather \
             than a pass."
        )
    });
    let ws: BTreeSet<&str> = meta["workspace_members"]
        .as_array()
        .expect("workspace_members is an array")
        .iter()
        .map(|v| v.as_str().expect("workspace member id is a string"))
        .collect();

    let mut targets = Vec::new();
    for pkg in meta["packages"].as_array().expect("packages is an array") {
        if !ws.contains(pkg["id"].as_str().expect("package id is a string")) {
            continue;
        }
        let package = pkg["name"].as_str().expect("package name").to_string();
        for t in pkg["targets"].as_array().expect("targets is an array") {
            // `kind: ["test"]` is cargo's own word for an integration test
            // binary. A lib target reports `kind: ["lib"]` with `test: true`,
            // and `--lib` is a different selection, so kind is the right filter.
            let is_test = t["kind"]
                .as_array()
                .is_some_and(|ks| ks.iter().any(|k| k.as_str() == Some("test")));
            if !is_test {
                continue;
            }
            targets.push(TestTarget {
                package: package.clone(),
                name: t["name"].as_str().expect("target name").to_string(),
                src_path: PathBuf::from(t["src_path"].as_str().expect("src_path is a string")),
            });
        }
    }
    targets.sort();
    targets
}

/// R2's floor, taken from DISK rather than from a constant.
///
/// The precedent's `members.len() >= 2` is a magic number that ages badly. This
/// asks a genuinely independent second source — the filesystem — how many
/// top-level `tests/*.rs` files each member has, and requires cargo to have
/// resolved at least that many targets. It cannot be satisfied by an enumeration
/// that silently stopped matching, and it is not a hand-list: nothing here names
/// a file, only counts them.
///
/// `>=`, not `==`, because a `[[test]]` section or a `tests/<dir>/main.rs`
/// legitimately adds a target with no matching top-level file.
fn tests_rs_files_on_disk(sdk: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for m in cargo_members(sdk) {
        let dir = m
            .manifest_path
            .parent()
            .expect("a manifest path has a parent")
            .join("tests");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue; // a member with no tests/ directory is normal
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() && p.extension().is_some_and(|x| x == "rs") {
                found.push(p);
            }
        }
    }
    found.sort();
    found
}

// ── modelling cargo's selection from a carrier's text ────────────────────────

/// Which test targets one invocation selects, in cargo's own terms.
#[derive(Debug, PartialEq, Eq)]
enum TargetSel {
    /// Every integration test of the selected packages.
    AllTests,
    /// Exactly these `--test` names.
    Only(BTreeSet<String>),
    /// A target selection that excludes integration tests (`--lib`, `--doc`, …).
    NoIntegrationTests,
}

/// One `cargo test` invocation the scan could read in full.
struct Selection {
    /// `None` = every workspace member (an explicit `--workspace`, or an
    /// implicit run pinned to the workspace root). `Some(set)` = these `-p`s.
    packages: Option<BTreeSet<String>>,
    exclude: BTreeSet<String>,
    targets: TargetSel,
    /// `<unit name>: <line>` — carried so a red can quote what it read.
    source: String,
    /// **§4l / G6.** `--no-fail-fast` was on the invocation. Without it, cargo
    /// stops after the FIRST target binary that fails and the rest of the
    /// targets this one call names never execute — measured, not assumed; see
    /// `cargo_test_runs_every_named_target_only_with_no_fail_fast`.
    no_fail_fast: bool,
    /// A target flag that selects something OTHER than integration tests
    /// (`--lib`, `--doc`, `--bins`, …) was on the invocation ALONGSIDE the
    /// `--test` names. That target runs in the same call and can starve them.
    also_non_test_targets: bool,
    /// No target flag at all, so cargo runs the lib unit tests, every
    /// integration test AND the doctests of every selected package in one call.
    all_target_kinds: bool,
}

/// One `cargo test` invocation the scan could NOT read. Never silently dropped:
/// an unreadable invocation is a failure of this guard, not a pass (Q2).
struct Blind {
    source: String,
    why: String,
}

/// Cargo flags that consume the following token as their value. An unknown flag
/// that also does so would make the scan read its value as a positional filter,
/// which is refused rather than guessed — see `read_invocation`.
const VALUE_FLAGS: &[&str] = &[
    "-p",
    "--package",
    "--exclude",
    "--test",
    "--bin",
    "--example",
    "--bench",
    "--features",
    "-F",
    "--target",
    "--manifest-path",
    "--target-dir",
    "--jobs",
    "-j",
    "--profile",
    "--message-format",
    "--color",
    "--config",
    "-Z",
];

/// Short flags this scan knows take no value. Anything else short is refused.
const NO_VALUE_SHORT: &[&str] = &["-v", "-vv", "-q", "-r"];

/// Split one carrier line into the argument list of every `cargo test` on it.
/// A line can carry more than one (`… && cargo test -p a && cargo test -p b`),
/// so each run stops at the next shell operator.
fn cargo_test_invocations(line: &str) -> Vec<Vec<String>> {
    const STOP: &[&str] = &["&&", "||", ";", "|", ">", ">>", "2>&1"];
    let toks: Vec<&str> = line.split_whitespace().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 1 < toks.len() {
        if toks[i].trim_start_matches(['(', '{']) == "cargo" && toks[i + 1] == "test" {
            let mut args = Vec::new();
            let mut j = i + 2;
            while j < toks.len() && !STOP.contains(&toks[j]) {
                args.push(toks[j].trim_end_matches([')', ';']).to_string());
                j += 1;
            }
            out.push(args);
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

/// Is this line's `cargo` invocation pinned to the sdk workspace ROOT?
///
/// `line_targets_sdk` is deliberately loose and matches `cd sdk/<member>` too.
/// That is fine for "does this touch the sdk lockfile" and wrong for "which
/// packages does this select": inside a member directory, a bare `cargo test`
/// selects ONE package. So an invocation with no `-p` and no `--workspace` is
/// only readable when the working directory is provably the workspace root.
fn sdk_is_workspace_root(dir_is_sdk: bool, line: &str) -> bool {
    if dir_is_sdk
        || line.contains("--manifest-path sdk/Cargo.toml")
        || line.contains("--manifest-path=sdk/Cargo.toml")
    {
        return true;
    }
    let toks: Vec<&str> = line.split_whitespace().collect();
    toks.windows(2).any(|w| {
        w[0].trim_start_matches(['(', '{']) == "cd"
            && w[1].trim_matches(['"', '\'']).trim_end_matches('/') == "sdk"
    })
}

/// Read one `cargo test` argument list. `Err(why)` means the scan cannot tell
/// what this invocation selects — which is a refusal, never a skip (Q2).
fn read_invocation(args: &[String]) -> Result<(Selection, bool), String> {
    let mut packages: BTreeSet<String> = BTreeSet::new();
    let mut exclude: BTreeSet<String> = BTreeSet::new();
    let mut tests: BTreeSet<String> = BTreeSet::new();
    let mut workspace = false;
    let mut all_tests = false;
    let mut non_test_target_flag = false;
    // §4l / G6: kept apart from `all_tests` because `--all-targets` also selects
    // the lib and the bins, and a failure in any of them stops the run.
    let mut all_targets_flag = false;
    let mut no_fail_fast = false;

    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            return Err(
                "a `--` separator — everything after it is a libtest filter, and a \
                        filter can leave a named binary running nothing"
                    .to_string(),
            );
        }
        // Normalise `--flag=value` and `--flag value` into one shape.
        let (flag, value) = if let Some((f, v)) = a.split_once('=') {
            if !a.starts_with('-') {
                return Err(format!(
                    "a positional argument `{a}` — cargo reads it as a test-NAME filter, and a \
                     filter can leave a named binary running nothing"
                ));
            }
            (f.to_string(), Some(v.to_string()))
        } else if a.starts_with("--") {
            if VALUE_FLAGS.contains(&a) {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| format!("`{a}` with no value after it"))?;
                if v.starts_with('-') {
                    return Err(format!(
                        "`{a}` followed by `{v}`, which reads as another flag"
                    ));
                }
                i += 1;
                (a.to_string(), Some(v.clone()))
            } else {
                (a.to_string(), None)
            }
        } else if a.starts_with('-') {
            if VALUE_FLAGS.contains(&a) {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| format!("`{a}` with no value after it"))?;
                i += 1;
                (a.to_string(), Some(v.clone()))
            } else if NO_VALUE_SHORT.contains(&a) {
                (a.to_string(), None)
            } else {
                return Err(format!(
                    "the short flag `{a}`, which this scan does not know. It refuses rather \
                     than guessing whether it swallows the next token — add it to \
                     VALUE_FLAGS or NO_VALUE_SHORT in supply_chain_policy.rs"
                ));
            }
        } else {
            return Err(format!(
                "a positional argument `{a}` — cargo reads it as a test-NAME filter, and a \
                 filter can leave a named binary running nothing"
            ));
        };
        i += 1;

        match flag.as_str() {
            "--workspace" | "--all" => workspace = true,
            "-p" | "--package" => {
                let v = value.expect("-p carries a value");
                if v.contains('*') || v.contains(',') {
                    return Err(format!(
                        "`-p {v}` — a glob or comma list. Refused rather than half-parsed"
                    ));
                }
                packages.insert(v);
            }
            "--exclude" => {
                exclude.insert(value.expect("--exclude carries a value"));
            }
            "--test" => {
                tests.insert(value.expect("--test carries a value"));
            }
            "--tests" => all_tests = true,
            "--all-targets" => {
                all_tests = true;
                all_targets_flag = true;
            }
            "--no-fail-fast" => no_fail_fast = true,
            "--lib" | "--bins" | "--bin" | "--examples" | "--example" | "--benches" | "--bench"
            | "--doc" | "--doctests" => non_test_target_flag = true,
            _ => {}
        }
    }

    // Cargo's own precedence: an explicit `--tests`/`--all-targets` selects every
    // integration test; explicit `--test NAME`s select exactly those; a target
    // flag that is not about tests selects none of them; and with no target flag
    // at all, integration tests are included.
    let no_target_flag = !all_tests && tests.is_empty() && !non_test_target_flag;
    let targets = if all_tests {
        TargetSel::AllTests
    } else if !tests.is_empty() {
        TargetSel::Only(tests)
    } else if non_test_target_flag {
        TargetSel::NoIntegrationTests
    } else {
        TargetSel::AllTests
    };

    let implicit_package_scope = !workspace && packages.is_empty();
    let sel = Selection {
        packages: if workspace || packages.is_empty() {
            None
        } else {
            Some(packages)
        },
        exclude,
        targets,
        source: String::new(),
        no_fail_fast,
        also_non_test_targets: non_test_target_flag,
        all_target_kinds: no_target_flag || all_targets_flag,
    };
    Ok((sel, implicit_package_scope))
}

/// Walk the live carriers' lines and read every sdk-scoped `cargo test` on them.
fn read_carriers(units: &[&Unit]) -> (Vec<Selection>, Vec<Blind>) {
    let mut sels = Vec::new();
    let mut blind = Vec::new();
    for u in units {
        read_unit_lines(u, u.lines.len(), &mut sels, &mut blind);
    }
    (sels, blind)
}

/// The body of `read_carriers`, for ONE unit and only its first `upto` lines.
///
/// Split out so the §4l reachability walk — which must stop at the line
/// where the unit can abort — reads a carrier through the SAME code path the
/// coverage walk uses. A second copy of this reader is how the two scans would
/// come to disagree about what a line says while both looked correct (R10).
fn read_unit_lines(u: &Unit, upto: usize, sels: &mut Vec<Selection>, blind: &mut Vec<Blind>) {
    for line in u.lines.iter().take(upto) {
        if !line_targets_sdk(u.dir_is_sdk, line) {
            continue;
        }
        for args in cargo_test_invocations(line) {
            let source = format!("{}: {}", u.name, line);
            // Shell expansion: refused for the whole invocation. Ignoring it
            // would under-count a `--test $NAME` (harmless, fails closed) but
            // OVER-count a `--workspace $MAYBE_EXCLUDE`, and the second is a
            // silent loss of coverage.
            if line.contains('$') || line.contains('`') {
                blind.push(Blind {
                    source,
                    why: "shell expansion on the line (`$…` or a backtick). This scan reads \
                          text and cannot evaluate it, so it cannot know what the \
                          invocation selects"
                        .to_string(),
                });
                continue;
            }
            match read_invocation(&args) {
                Err(why) => blind.push(Blind { source, why }),
                Ok((mut sel, implicit)) => {
                    if implicit && !sdk_is_workspace_root(u.dir_is_sdk, line) {
                        blind.push(Blind {
                            source,
                            why: "no `-p` and no `--workspace`, and the scan cannot prove the \
                                  working directory is the sdk workspace ROOT — inside a \
                                  member directory a bare `cargo test` selects one package"
                                .to_string(),
                        });
                        continue;
                    }
                    sel.source = source;
                    sels.push(sel);
                }
            }
        }
    }
}

/// Which of `targets` the carriers name, and which invocation named each.
fn covered(targets: &[TestTarget], sels: &[Selection]) -> BTreeMap<(String, String), String> {
    let mut out: BTreeMap<(String, String), String> = BTreeMap::new();
    for sel in sels {
        // The selection predicate itself lives in `sel_covers`: the §4l
        // reachability walk asks the identical question of the identical
        // selections, and a second copy of it is the duplicate R10 forbids.
        for t in sel_covers(sel, targets) {
            out.entry(t.key()).or_insert_with(|| sel.source.clone());
        }
    }
    out
}

// ── the three named tests ────────────────────────────────────────────────────

/// §4k's guard. Every test target the sdk workspace resolves is named by a
/// carrier that runs unbidden.
///
/// R3/R4 — watched failing both ways by `scripts/gate-proofs/p0-7-r1b-hand-list.sh`
/// and recorded in `evals/mutants.tsv`: with a `--test` name deleted from
/// `sdk-gate-core` (removal), and with a new `tests/*.rs` added to an sdk member
/// that no carrier names (addition). The second is the case the item exists for
/// and the one a hand-list cannot see.
#[test]
fn every_sdk_test_target_is_named_by_a_live_carrier() {
    let sdk = sdk_root();
    let targets = cargo_test_targets(&sdk);

    // ── R2: the enumeration's anti-vacuity floor, taken from disk ──
    let on_disk = tests_rs_files_on_disk(&sdk);
    assert!(
        on_disk.len() >= 2,
        "the `tests/*.rs` disk scan found {} file(s) under {} — the FLOOR itself has stopped \
         matching, so it cannot vouch for anything below it.",
        on_disk.len(),
        sdk.display()
    );
    assert!(
        targets.len() >= on_disk.len(),
        "cargo resolved {} integration-test target(s) but {} `tests/*.rs` file(s) exist on \
         disk. Either the enumeration has stopped matching, or a member has switched off \
         autodiscovery (`autotests = false`) — which hides a test binary from cargo exactly \
         the way a hand-list hides it from a carrier.\n  cargo: {:?}\n  disk:  {:?}",
        targets.len(),
        on_disk.len(),
        targets
            .iter()
            .map(|t| format!("{}::{}", t.package, t.name))
            .collect::<Vec<_>>(),
        on_disk
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
    );
    for t in &targets {
        assert!(
            t.src_path.is_file(),
            "cargo named a test source that is not on disk: {} ({}::{}). The enumeration is \
             not describing this tree.",
            t.src_path.display(),
            t.package,
            t.name
        );
    }

    // Post-extraction (the SDK in its own repo) there is no lefthook.yml and no
    // Justfile, so there is no carrier to grade. Keyed on the repo-root marker,
    // exactly as A3 above is, and never on a missing file.
    let Some(root) = repo_root() else {
        return;
    };
    let lefthook = std::fs::read_to_string(root.join("lefthook.yml")).unwrap_or_default();
    let justfile = std::fs::read_to_string(root.join("Justfile")).unwrap_or_default();
    let recipes = parse_justfile(&justfile);
    let hooks = parse_lefthook(&lefthook);

    // ── anti-vacuity on the parsers, same shape as A3 ──
    assert!(
        recipes.contains_key("sdk-gate") && recipes.contains_key("sdk-gate-core"),
        "the Justfile parser resolved {} recipe(s) under {} and found neither `sdk-gate` nor \
         `sdk-gate-core`. A parser that returns nothing satisfies every assertion below it. \
         Recipes seen: {:?}",
        recipes.len(),
        root.display(),
        recipes.keys().collect::<Vec<_>>()
    );

    let (arm_p, arm_n) = carrier_arms(&hooks, &recipes);
    let live: Vec<&Unit> = arm_p.into_iter().chain(arm_n).collect();
    let (sels, blind) = read_carriers(&live);

    // ── Q2's fail-closed leg: an invocation the scan cannot read is a RED ──
    assert!(
        blind.is_empty(),
        "THE CARRIER SCAN CANNOT READ {} `cargo test` INVOCATION(S), so it cannot say what is \
         covered — and an unreadable carrier is a failure of this guard, not a pass:\n{}\n\n\
         Rewrite the invocation so the flags are literal (`--test <name>`, `-p <pkg>`, or \
         `--workspace`), or teach `read_invocation` in \
         sdk/zec-wallet-swap-near/tests/supply_chain_policy.rs about the flag it choked on.",
        blind.len(),
        blind
            .iter()
            .map(|b| format!("  {}\n    ↳ {}", b.source, b.why))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // ── anti-vacuity on the scan itself: it must have READ something ──
    assert!(
        !sels.is_empty(),
        "the live carriers contain no sdk-scoped `cargo test` invocation at all. Either the \
         nightly lost its test leg — in which case NOTHING in this workspace runs unbidden — \
         or `cargo_test_invocations` has stopped matching this file's syntax. Units walked: \
         {:?}",
        live.iter().map(|u| &u.name).collect::<Vec<_>>()
    );

    let named = covered(&targets, &sels);
    let orphans: Vec<&TestTarget> = targets
        .iter()
        .filter(|t| !named.contains_key(&t.key()))
        .collect();

    // ── R8: the red names the target AND the fix ──
    assert!(
        orphans.is_empty(),
        "{} SDK TEST TARGET(S) ARE NAMED BY NO CARRIER THAT RUNS UNBIDDEN:\n{}\n\n\
         WHAT THIS MEANS. `just sdk-gate-core` and `just sdk-fast` name their test binaries \
         BY HAND. A binary they do not name runs only under `cargo test --workspace`, i.e. \
         `just ci` (a human typing) and CI's sdk-gate job (never executed against any commit \
         in this history) — so it can pass for months without the nightly ever building it. \
         That is not hypothetical: four of this workspace's seven targets were in exactly \
         that state until S259, two of them guarding seed custody and the swap kill switch.\n\n\
         THE FIX, either one:\n\
         {}\n\
         or replace the hand-list with a wildcard — a `cd sdk && cargo test --workspace` line \
         in a live carrier names every target this workspace resolves, and this guard accepts \
         it (§4k R9).\n\n\
         Add the same line to `sdk-fast` too, or the fast lane keeps its own blind spot.\n\n\
         What the scan read as naming things:\n{}",
        orphans.len(),
        orphans
            .iter()
            .map(|t| format!("  {}::{}  ({})", t.package, t.name, t.src_path.display()))
            .collect::<Vec<_>>()
            .join("\n"),
        orphans
            .iter()
            .map(|t| format!(
                "  add to Justfile `sdk-gate-core`:  cd sdk && CARGO_INCREMENTAL=0 cargo test \
                 -p {} --test {}",
                t.package, t.name
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        if named.is_empty() {
            "  (nothing)".to_string()
        } else {
            named
                .iter()
                .map(|((p, n), src)| format!("  {p}::{n}  ←  {src}"))
                .collect::<Vec<_>>()
                .join("\n")
        }
    );
}

/// R1's other half, asserted rather than assumed: when cargo cannot answer, the
/// enumeration FAILS. A guard whose enumeration silently returns an empty set
/// passes on every tree in the world.
///
/// IT-10 — which mechanism refuses this? The `out.status.success()` assertion in
/// `cargo_test_targets`, and the test proves it by the sentence it prints. A
/// broken manifest makes `cargo metadata` exit 101 with empty stdout, which is
/// the shape a toolchain-light or mid-edit checkout produces.
#[test]
fn the_target_enumeration_fails_when_cargo_cannot_answer() {
    let dir = std::env::temp_dir().join(format!(
        "p0-7-r1b-unanswerable-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir creatable");
    std::fs::write(dir.join("Cargo.toml"), "this is not a manifest\n[[[\n")
        .expect("scratch manifest writable");

    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {})); // the panic below is the assertion
    let outcome = std::panic::catch_unwind(|| cargo_test_targets(&dir));
    std::panic::set_hook(prev);
    let _ = std::fs::remove_dir_all(&dir);

    let payload = outcome.err().unwrap_or_else(|| {
        panic!(
            "`cargo metadata` was pointed at a manifest it cannot parse and the enumeration \
             RETURNED instead of failing. An enumeration that reports an empty set on an \
             unanswerable tree makes every assertion above it vacuous."
        )
    });
    let msg = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default();
    assert!(
        msg.contains(
            "If cargo cannot answer, nothing was checked, and that is a failure rather than a pass"
        ),
        "the enumeration failed, but not for the reason this test is named after. It printed:\n{msg}"
    );
}

/// R9 — the `--workspace` carve-out, decided in the contract and asserted here
/// so it is a named test rather than a discovery.
///
/// Five cases, and the last three are the anti-vacuity of the first two (IT-10:
/// for every case, which mechanism refuses it?):
///   1. `--workspace` names every target                      → `TargetSel::AllTests` + `packages: None`
///   2. `-p <one> --lib` names none of them                   → `TargetSel::NoIntegrationTests`
///   3. `--workspace --exclude <pkg>` drops that package      → `Selection::exclude`
///   4. a root-workspace `cargo test --workspace` names none  → `line_targets_sdk`
///   5. `cd sdk/<member> && cargo test` is unreadable         → `sdk_is_workspace_root`
///
/// **Case 3 is the case the contract does not name (IT-1 +A).** R9 says a
/// wildcard invocation names every target; it says nothing about `--exclude`,
/// and `cargo test --workspace --exclude zec_wallet` is a wildcard invocation
/// that silently drops three targets — two of which guard seed custody and the
/// swap kill switch. Without this case the carve-out written to save the guard
/// would be the hole that empties it.
#[test]
fn a_workspace_wide_invocation_names_every_target() {
    let sdk = sdk_root();
    let targets = cargo_test_targets(&sdk);
    assert!(
        targets.len() >= 2,
        "the enumeration resolved {} target(s); this test cannot discriminate on fewer than 2",
        targets.len()
    );
    let all: BTreeSet<(String, String)> = targets.iter().map(TestTarget::key).collect();

    // A synthetic carrier, parsed by the SAME parser the live scan uses.
    let scan = |body: &str| -> (BTreeMap<(String, String), String>, usize) {
        let text =
            format!("sdk-gate: sdk-gate-core\n    @echo gate\n\nsdk-gate-core:\n    {body}\n");
        let recipes = parse_justfile(&text);
        let seed: Vec<&Unit> = recipes.get("sdk-gate").into_iter().collect();
        assert!(
            !seed.is_empty(),
            "the synthetic Justfile did not parse: {text}"
        );
        let live = closure(&seed, &recipes);
        let (sels, blind) = read_carriers(&live);
        (covered(&targets, &sels), blind.len())
    };

    // CASE 1 — the carve-out itself.
    let (c1, b1) = scan("cd sdk && cargo test --workspace");
    assert_eq!(b1, 0, "case 1 was unreadable, so it proves nothing");
    assert_eq!(
        c1.keys().cloned().collect::<BTreeSet<_>>(),
        all,
        "R9: a `cd sdk && cargo test --workspace` line in a live carrier must name EVERY \
         target the workspace resolves. Without this the guard goes red on the day the \
         hand-list is repaired, and the first person to hit it deletes it."
    );

    // CASE 2 — the control. If this also passed, case 1 would mean nothing.
    let first_pkg = targets[0].package.clone();
    let (c2, b2) = scan(&format!("cd sdk && cargo test -p {first_pkg} --lib"));
    assert_eq!(b2, 0, "case 2 was unreadable, so it proves nothing");
    assert!(
        c2.is_empty(),
        "`--lib` selects the library target, not the integration tests, so this line names no \
         test target. It named: {:?}",
        c2.keys().collect::<Vec<_>>()
    );

    // CASE 3 — PLANTED, and the contract does not name it (IT-1 +A).
    let excluded = "zec_wallet";
    assert!(
        targets.iter().any(|t| t.package == excluded),
        "case 3's planted package `{excluded}` is no longer a member — re-aim the case at a \
         member that exists rather than deleting it. Members with tests: {:?}",
        targets.iter().map(|t| &t.package).collect::<BTreeSet<_>>()
    );
    let (c3, b3) = scan(&format!(
        "cd sdk && cargo test --workspace --exclude {excluded}"
    ));
    assert_eq!(b3, 0, "case 3 was unreadable, so it proves nothing");
    assert!(
        c3.keys().all(|(p, _)| p != excluded) && !c3.is_empty(),
        "`--workspace --exclude {excluded}` is a wildcard invocation that DROPS a package. R9's \
         carve-out must not treat it as naming everything, or the flag written to keep this \
         guard alive becomes the flag that empties it. It named: {:?}",
        c3.keys().collect::<Vec<_>>()
    );

    // CASE 4 — a ROOT-workspace wildcard is a different workspace.
    let (c4, b4) = scan("cargo test --workspace");
    assert_eq!(
        b4, 0,
        "case 4 must be skipped as out-of-scope, not read as blind"
    );
    assert!(
        c4.is_empty(),
        "a `cargo test --workspace` with no sdk scoping runs the ROOT workspace and names no \
         sdk target. It named: {:?}",
        c4.keys().collect::<Vec<_>>()
    );

    // CASE 5 — implicit package scope inside a member directory is unreadable.
    let (c5, b5) = scan(&format!("cd sdk/{first_pkg} && cargo test"));
    assert_eq!(
        b5, 1,
        "`cd sdk/<member> && cargo test` selects ONE package, so reading it as a workspace-wide \
         run would over-count. It must be refused as unreadable, not accepted."
    );
    assert!(
        c5.is_empty(),
        "an unreadable invocation must contribute no coverage. It named: {:?}",
        c5.keys().collect::<Vec<_>>()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// P0-7 R1(b) REPAIRS 6 AND 1 — §4l. NAMED IS NOT REACHED.
// Contract: `docs/plan/production-readiness-phase-0.md` §4l, rows G1–G9.
//
// THE ONE SENTENCE THIS SECTION EXISTS FOR. Everything above grades what the
// carriers SAY. §4k-review's security angle then showed that saying is not
// running: restore the pre-line order and
// `every_sdk_test_target_is_named_by_a_live_carrier` stays GREEN, while the
// nightly log for that same order shows one test run in the entire file. The
// four binaries the fold wired in were named by a live carrier and reached
// by nothing.
//
// THREE LEVELS OF THE SAME FAIL-FAST CLASS, AND ONLY TWO HAD EVER BEEN NAMED.
// Each one has been repaired at least once by moving something, and moving
// things is what does not work:
//
//   1. `just`'s DEPENDENCY list — `sdk-gate: a b c d e`. Measured (just 1.49.0,
//      four-recipe toy, §4k-review): a failing `b` means `c`, `d`, `e` and the
//      body never run. Only the FIRST dependency is guaranteed.
//   2. A RECIPE BODY's lines — `just` runs a non-shebang recipe one line at a
//      time and stops at the first that fails (measured, §4k). Only the FIRST
//      line is guaranteed.
//   3. **A SINGLE `cargo test` CALL's targets** — and this level had never been
//      named anywhere in the tree. Measured on a two-target toy
//      (`cargo_test_runs_every_named_target_only_with_no_fail_fast` re-measures
//      it on every run rather than quoting this comment):
//      `cargo test --lib --test alpha --test beta` with a failing `--lib` prints
//      exactly ONE `Running` line. `alpha` and `beta` never execute. Reordering
//      the flags does NOT help — cargo runs the lib target first whatever order
//      they appear in — so the "policy binaries first" convention that repaired
//      levels 1 and 2 has no counterpart here. `--no-fail-fast` is the only fix,
//      and it keeps the run RED (exit 101, `error: 1 target failed`).
//
// SO WHAT PROPERTY MAKES NAMING IMPLY REACHING (§4l G-Q2)? Not "recognise the
// knowingly-red leg" — that cannot be computed from carrier text. This:
// **every line, leg and target must be unable to starve its siblings.** That IS
// computable, it is shape-agnostic (a `-`-prefixed line, a shebang body without
// `set -e`, a `|| status=1` per line and a `--no-fail-fast` per multi-target
// call all satisfy it), and it is what `every_sdk_test_target_is_reached_and_
// not_merely_named` grades.
//
// WHAT THE STATIC HALF STILL CANNOT SEE, and why the driven half exists. It
// reads text, so it cannot see a `just` upgrade that changes abort semantics
// (owed row 7), a recipe body handed to a script this scan does not follow, or
// a leg that runs and does nothing. `just_sdk_gate_*` below drive the REAL
// recipes of this tree's own Justfile with one leg forced red at a time and
// grade what executed — the same method §4k-review's most valuable finding came
// from, which was the one produced by running the recipe rather than reading it.
// ─────────────────────────────────────────────────────────────────────────────

/// The legs `sdk-gate` must have. A FLOOR, not a definition: the leg list is
/// read out of the Justfile at run time (whatever shape the recipe has), and
/// these five must be in it. Anchored on names, never on positions — this
/// recipe's ordering has been rewritten three times in three sessions.
const SDK_GATE_LEGS_FLOOR: &[&str] = &[
    "sdk-audit",
    "sdk-lint",
    "l10n-drift-check",
    "sdk-gate-core",
    "flutter-ci",
];

/// The two binaries §4l calls the money legs: `zec_wallet::custody_policy`
/// guards the §4.3a seed-custody/keychain surface, `zec_wallet::swap_kill_door`
/// guards that a "Hard kill" actually stops NEAR-swap traffic. Neither has ever
/// executed in a recorded nightly (§4l evidence 1).
const MONEY_TEST_TARGETS: &[&str] = &["custody_policy", "swap_kill_door"];

/// The Dart packages `flutter-ci` must be seen grading, by name. A FLOOR in the
/// same sense as `SDK_GATE_LEGS_FLOOR`: the list is READ from the directories
/// the recipe `cd`s into, and these three must be among them.
///
/// `zec_wallet_ui` is the reason the constant exists. §4l evidence 3: on
/// 6 September the nightly's `flutter-ci` failed on ONE Dart test in
/// `sdk/zec_wallet_ui/test/features/wallet/swap/swap_screen_test.dart`, its last
/// output line was `All tests passed!` from a LATER leg, and no gate in this
/// tree said which leg had failed. A scan that reads zero packages passes every
/// loop under it, which is the defect `assert_legs_floor` was written against
/// one level up.
const FLUTTER_CI_PACKAGE_FLOOR: &[&str] = &["example", "zec_wallet", "zec_wallet_ui"];

// ── the model: which lines of a unit are guaranteed to run ───────────────────

/// Line heads that cannot, on their own, stop the lines after them. Everything
/// NOT here — every external command — is treated as able to abort. That is the
/// fail-closed direction: an unrecognised line makes what follows it
/// un-guaranteed rather than silently guaranteed.
const CANNOT_ABORT_HEAD: &[&str] = &[
    "if", "elif", "else", "then", "fi", "while", "until", "for", "do", "done", "case", "esac", "{",
    "}", "set", "echo", "printf", ":", "true", "local", "export", "declare", "readonly", "!",
];

/// `just`'s per-line prefixes. `-` means "ignore this line's failure" — and it
/// also SUPPRESSES the `error: Recipe … failed` message (measured, just 1.49.0),
/// which is why a `-`-prefixed leg satisfies reachability and fails G2/G3.
fn just_ignores_failure(line: &str) -> bool {
    let mut t = line.trim_start();
    let mut ignores = false;
    loop {
        if let Some(r) = t.strip_prefix('-') {
            ignores = true;
            t = r;
            continue;
        }
        if let Some(r) = t.strip_prefix('@') {
            t = r;
            continue;
        }
        break;
    }
    ignores
}

/// `name=value` with no command substitution. Such a line always succeeds.
/// `name=$(cmd)` takes the command's exit status and is NOT this.
fn is_plain_assignment(line: &str) -> bool {
    let t = line.trim();
    if t.contains("$(") || t.contains('`') {
        return false;
    }
    let Some((lhs, _)) = t.split_once('=') else {
        return false;
    };
    !lhs.is_empty()
        && !lhs.chars().next().is_some_and(|c| c.is_ascii_digit())
        && lhs.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Can this line `exit` the script? Nothing after such a line is guaranteed,
/// whatever else the line does — checked BEFORE the `||` rule, because
/// `cmd || exit 1` ends the script exactly as `exit 1` does.
///
/// It asks whether `exit` stands at a COMMAND position, not whether the word
/// appears. The first version asked the second question and cut `sdk-audit` at
/// `echo " … This is exit 2 (could not evaluate),"` — a sentence, in a string,
/// in a message about exit codes. A model that reports the wrong line is a model
/// nobody can act on, even when its verdict happens to be right.
fn has_exit_token(line: &str) -> bool {
    line.split(|c: char| ";&|{}()".contains(c))
        .any(|seg| seg.split_whitespace().next() == Some("exit"))
}

/// The first line of `u` that can stop the lines after it, and why. `None`
/// means every line runs whenever the unit runs — the property §4l is after.
///
/// Two semantics, because `just` has two:
///   * a NON-shebang recipe is run one line per `bash -uc`, and `just` aborts
///     the recipe at the first line that fails. One unprefixed line is enough;
///   * a script body (a shebang recipe, or a lefthook `run:` block) is one
///     shell script, so the answer turns on `set -e` and on `exit`.
fn first_aborting_line(u: &Unit) -> Option<(usize, String)> {
    let mut errexit = false;
    for (i, l) in u.lines.iter().enumerate() {
        if !u.script_body {
            if just_ignores_failure(l) {
                continue;
            }
            return Some((
                i,
                format!(
                    "`just` runs a non-shebang recipe ONE LINE AT A TIME and aborts the recipe at \
                     the first line that fails, so nothing below this line is guaranteed to run: \
                     `{l}`"
                ),
            ));
        }
        let t = l.trim();
        if let Some(rest) = t.strip_prefix("set ") {
            for o in rest.split_whitespace() {
                if o.strip_prefix('-').is_some_and(|f| f.contains('e')) {
                    errexit = true;
                }
                if o.strip_prefix('+').is_some_and(|f| f.contains('e')) {
                    errexit = false;
                }
            }
            if rest.contains("-o errexit") {
                errexit = true;
            }
            if rest.contains("+o errexit") {
                errexit = false;
            }
            continue;
        }
        if has_exit_token(t) {
            return Some((
                i,
                format!(
                    "this line can `exit` the script, so nothing below it is guaranteed to run: \
                     `{t}`"
                ),
            ));
        }
        if !errexit {
            continue;
        }
        // A `||` list never trips errexit: the shell takes the list's status
        // from the right-hand side, which in every aggregation idiom is an
        // assignment. That assumption is stated rather than hidden — a `cmd ||
        // other-command` whose right side also fails WOULD abort, and this scan
        // reads it as safe.
        if t.contains("||") || is_plain_assignment(t) {
            continue;
        }
        let head = t.split_whitespace().next().unwrap_or("");
        if CANNOT_ABORT_HEAD.contains(&head) {
            continue;
        }
        return Some((
            i,
            format!(
                "`set -e` is in force and this line is an unprotected command, so nothing below it \
                 is guaranteed to run: `{t}`"
            ),
        ));
    }
    None
}

/// A unit the carrier is guaranteed to enter, with how far into it the
/// guarantee reaches.
struct Reached<'a> {
    unit: &'a Unit,
    /// `unit.lines[..upto]` run whenever the carrier runs.
    upto: usize,
    /// Why the guarantee stops there. `None` = it does not stop.
    cut: Option<String>,
    /// The path the walk took to get here, for the failure message.
    via: String,
}

/// `closure`'s order-aware sibling: walk only the hand-offs a carrier is
/// GUARANTEED to make.
///
/// The one rule that does all the work: **`just` runs a recipe's dependency
/// list left to right and aborts at the first one that fails**, so only the
/// FIRST dependency is guaranteed. That is not a modelling shortcut — it is
/// §4k-review owed row 6's entire content, and it is why `sdk-gate`'s five-leg
/// dependency list cannot satisfy G1 no matter how it is ordered.
fn reached_walk<'a>(seeds: &[&'a Unit], recipes: &'a BTreeMap<String, Unit>) -> Vec<Reached<'a>> {
    let mut out: Vec<Reached<'a>> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<(&'a Unit, String)> = seeds.iter().map(|u| (*u, u.name.clone())).collect();
    while let Some((u, via)) = stack.pop() {
        if !seen.insert(format!("{}@{}", u.name, u.lines.len())) {
            continue;
        }
        let cut = first_aborting_line(u);
        let upto = cut.as_ref().map_or(u.lines.len(), |(i, _)| i + 1);
        if let Some(next) = u.deps.first().and_then(|d| recipes.get(d)) {
            stack.push((next, format!("{via} -> first dependency `{}`", next.name)));
        }
        for (idx, name) in &u.line_calls {
            if *idx >= upto {
                continue;
            }
            if let Some(next) = recipes.get(name) {
                stack.push((
                    next,
                    format!("{via} -> body line {} `just {name}`", idx + 1),
                ));
            }
        }
        out.push(Reached {
            unit: u,
            upto,
            cut: cut.map(|(_, why)| why),
            via,
        });
    }
    out
}

/// Which of `targets` ONE invocation selects. Factored out of `covered`
/// so the coverage question and the fail-fast question are answered from one
/// predicate rather than two that can drift (R10).
fn sel_covers<'a>(sel: &Selection, targets: &'a [TestTarget]) -> Vec<&'a TestTarget> {
    targets
        .iter()
        .filter(|t| {
            let in_scope = sel
                .packages
                .as_ref()
                .is_none_or(|ps| ps.contains(&t.package))
                && !sel.exclude.contains(&t.package);
            if !in_scope {
                return false;
            }
            match &sel.targets {
                TargetSel::AllTests => true,
                TargetSel::Only(names) => names.contains(&t.name),
                TargetSel::NoIntegrationTests => false,
            }
        })
        .collect()
}

/// §4l G6, applied. Does this ONE `cargo test` call let a sibling target starve
/// the targets it names? `Some(why)` = yes.
///
/// MEASURED, not inferred from the nightly logs (§4l G6 says so explicitly, and
/// `cargo_test_runs_every_named_target_only_with_no_fail_fast` re-measures it):
/// cargo runs the selected target binaries one after another and stops at the
/// first that fails. `--no-fail-fast` is the only flag that changes that, and it
/// leaves the exit code non-zero, so nothing is laundered by adding it.
fn starves_its_own_targets(sel: &Selection, n_named: usize) -> Option<String> {
    if sel.no_fail_fast {
        return None;
    }
    let other_kinds = sel.all_target_kinds || sel.also_non_test_targets;
    if n_named <= 1 && !other_kinds {
        return None;
    }
    let extra = if other_kinds {
        " (and a `--lib`/`--doc`/`--bins` target in the same call, which cargo runs FIRST \
          whatever order the flags appear in)"
    } else {
        ""
    };
    Some(format!(
        "this ONE `cargo test` call selects {n_named} integration-test target(s){extra} and does \
         not pass `--no-fail-fast`, so cargo stops after the first target binary that fails and \
         the rest never execute. Split the call, or add `--no-fail-fast` — it keeps the run RED"
    ))
}

/// §4l G7 — the guard grades REACHING.
///
/// Every test target the sdk workspace resolves must be *guaranteed to run*
/// whenever a live carrier runs: reached through hand-offs the carrier cannot
/// skip, on a line the carrier cannot abort before, in a `cargo test` call whose
/// other targets cannot starve it.
///
/// **This does not replace `every_sdk_test_target_is_named_by_a_live_carrier`
/// and does not weaken it** (G8). That test asks whether a carrier NAMES each
/// target and still refuses everything it refused before; this one asks whether
/// the naming means anything. Both run.
///
/// IT-10 — which mechanism refuses each case?
///   * a leg buried in a dependency list  -> `reached_walk`'s first-dependency rule
///   * a line below an aborting line      -> `first_aborting_line` + `Reached::upto`
///   * a target sharing a fail-fast call  -> `starves_its_own_targets`
///
/// Watched failing against all three by `scripts/gate-proofs/p0-7-r1b-reaching.sh`.
#[test]
fn every_sdk_test_target_is_reached_and_not_merely_named() {
    let sdk = sdk_root();
    let targets = cargo_test_targets(&sdk);
    assert!(
        targets.len() >= 2,
        "cargo resolved {} test target(s) in {}. The enumeration this test rests on has stopped \
         describing the tree, and a guard over an empty set passes on every tree in the world. \
         (`every_sdk_test_target_is_named_by_a_live_carrier` carries the full disk-backed floor.)",
        targets.len(),
        sdk.display()
    );

    let Some(root) = repo_root() else {
        return; // post-extraction: no Justfile, no carrier to grade
    };
    let lefthook = std::fs::read_to_string(root.join("lefthook.yml")).unwrap_or_default();
    let justfile = std::fs::read_to_string(root.join("Justfile")).unwrap_or_default();
    let recipes = parse_justfile(&justfile);
    let hooks = parse_lefthook(&lefthook);
    assert!(
        recipes.contains_key("sdk-gate") && recipes.contains_key("sdk-gate-core"),
        "the Justfile parser resolved {} recipe(s) under {} and found neither `sdk-gate` nor \
         `sdk-gate-core`. A parser that returns nothing satisfies every assertion below it. \
         Recipes seen: {:?}",
        recipes.len(),
        root.display(),
        recipes.keys().collect::<Vec<_>>()
    );

    // The SAME two live arms the coverage guard uses (`carrier_seeds`), walked
    // twice: once for what is named, once for what is guaranteed to run.
    let (seed_p, seed_n) = carrier_seeds(&hooks, &recipes);
    let seeds: Vec<&Unit> = seed_p.iter().chain(seed_n.iter()).copied().collect();
    let named_units = closure(&seeds, &recipes);
    let reached = reached_walk(&seeds, &recipes);

    let (named_sels, _named_blind) = read_carriers(&named_units);
    assert!(
        !named_sels.is_empty(),
        "the live carriers contain no sdk-scoped `cargo test` invocation at all, so this test \
         cannot tell a reachability defect from an empty tree. Units walked: {:?}",
        named_units.iter().map(|u| &u.name).collect::<Vec<_>>()
    );

    let mut reached_sels: Vec<Selection> = Vec::new();
    let mut reached_blind: Vec<Blind> = Vec::new();
    for r in &reached {
        read_unit_lines(r.unit, r.upto, &mut reached_sels, &mut reached_blind);
    }

    // Three verdicts per target, and the message turns on which one applies.
    let mut guaranteed: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut starved: BTreeMap<(String, String), String> = BTreeMap::new();
    for sel in &reached_sels {
        let hits = sel_covers(sel, &targets);
        match starves_its_own_targets(sel, hits.len()) {
            None => {
                for t in hits {
                    guaranteed
                        .entry(t.key())
                        .or_insert_with(|| sel.source.clone());
                }
            }
            Some(why) => {
                for t in hits {
                    starved
                        .entry(t.key())
                        .or_insert_with(|| format!("{}\n      ↳ {why}", sel.source));
                }
            }
        }
    }
    let named = covered(&targets, &named_sels);

    let orphans: Vec<&TestTarget> = targets
        .iter()
        .filter(|t| !guaranteed.contains_key(&t.key()))
        .collect();

    let mut table = String::new();
    for r in &reached {
        table.push_str(&format!(
            "  {} — {} of {} line(s) guaranteed  [{}]\n",
            r.unit.name,
            r.upto.min(r.unit.lines.len()),
            r.unit.lines.len(),
            r.via
        ));
        if let Some(why) = &r.cut {
            table.push_str(&format!("      cut: {why}\n"));
        }
    }
    let unreached: Vec<&str> = named_units
        .iter()
        .map(|u| u.name.as_str())
        .filter(|n| !reached.iter().any(|r| r.unit.name == *n))
        .collect();

    assert!(
        orphans.is_empty(),
        "{} SDK TEST TARGET(S) ARE NAMED BY A LIVE CARRIER AND NOT GUARANTEED TO RUN:\n{}\n\n\
         NAMED IS NOT REACHED, AND THIS IS THE DIFFERENCE. §4k-review proved it by building a \
         mutated carrier: restoring the pre-S260 line order left \
         `every_sdk_test_target_is_named_by_a_live_carrier` GREEN while the nightly log for that \
         order showed ONE test run in the whole file. §4l evidence 1: `custody_policy`, \
         `swap_kill_door`, `feature_policy`, `policy` and `supply_chain_policy` have never \
         appeared in ANY recorded nightly log.\n\n\
         THE THREE WAYS A NAMED TARGET FAILS TO RUN, all of them the same fail-fast class:\n\
         1. its leg is not the FIRST entry of a `just` dependency list — `just` aborts the chain \
         at the first dependency that fails, so only the first is guaranteed;\n\
         2. its line sits below a line that can abort the recipe — `just` runs a non-shebang \
         recipe one line at a time and stops at the first failure;\n\
         3. it shares one `cargo test` call with a target that can fail first — measured: \
         `cargo test --lib --test a --test b` with a failing `--lib` prints one `Running` line, \
         and flag order does not change it.\n\n\
         THE FIX, in the shape this tree already contains (`flutter-ci`): a `#!/usr/bin/env bash` \
         body, `status=0`, one `just <leg> || status=1` per leg, `exit $status` — every leg runs, \
         the recipe is still red if any failed. Same shape inside `sdk-gate-core` for its lines, \
         and `--no-fail-fast` on any `cargo test` that names more than one target.\n\
         WRITE `|| status=1`, NOT `|| status=$?`, ON A `cargo test` LINE: the S260 scan refuses \
         any invocation whose line carries a `$`, so `$?` turns \
         `every_sdk_test_target_is_named_by_a_live_carrier` red with a CANNOT READ.\n\n\
         WHAT REACHES WHAT, as this scan read it:\n{}\
         {}\n\
         Unreadable invocations on reached lines: {}\n\n\
         Targets a carrier NAMES (coverage, the S260 guard's question): {}",
        orphans.len(),
        orphans
            .iter()
            .map(|t| {
                let k = t.key();
                let why = if let Some(s) = starved.get(&k) {
                    format!("REACHED BUT STARVED BY A SIBLING TARGET\n      at {s}")
                } else if let Some(s) = named.get(&k) {
                    format!(
                        "NAMED BUT NOT REACHED — the naming line is `{s}`, and the walk above \
                             shows where the guarantee stops"
                    )
                } else {
                    "NOT NAMED BY ANY LIVE CARRIER AT ALL (this is also \
                     `every_sdk_test_target_is_named_by_a_live_carrier`'s finding)"
                        .to_string()
                };
                format!("  {}::{}\n      {why}", t.package, t.name)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        table,
        if unreached.is_empty() {
            String::new()
        } else {
            format!(
                "  UNITS THE CARRIER NAMES BUT IS NOT GUARANTEED TO ENTER AT ALL: {unreached:?}\n"
            )
        },
        if reached_blind.is_empty() {
            "none".to_string()
        } else {
            reached_blind
                .iter()
                .map(|b| format!("\n    {} ↳ {}", b.source, b.why))
                .collect::<Vec<_>>()
                .join("")
        },
        named.len(),
    );
}

/// §4l G9 — `sdk-fast` gets the same treatment.
///
/// It is deliberately NOT a live carrier (`carrier_arms`: a human typing a
/// command), and this test does not change that. It grades a different claim:
/// the recipe a developer types before committing must not silently skip the
/// money binaries it lists. Under `set -euo pipefail` one red line ends the
/// script with nothing in the output to say the rest were skipped rather than
/// passed — the defect `cdb59304` repaired in `sdk-gate-core` and left standing
/// here for four lines, which is why §4l names this recipe explicitly.
#[test]
fn sdk_fast_reaches_every_target_it_names() {
    let sdk = sdk_root();
    let targets = cargo_test_targets(&sdk);
    let Some(root) = repo_root() else {
        return;
    };
    let justfile = std::fs::read_to_string(root.join("Justfile")).unwrap_or_default();
    let recipes = parse_justfile(&justfile);
    let fast = recipes.get("sdk-fast").unwrap_or_else(|| {
        panic!(
            "no `sdk-fast` recipe in {}. §4l G9 names it as the money-path recipe a developer \
             types before committing; if it was renamed, re-aim this test rather than deleting \
             it. Recipes seen: {:?}",
            root.display(),
            recipes.keys().collect::<Vec<_>>()
        )
    });

    let cut = first_aborting_line(fast);
    let upto = cut.as_ref().map_or(fast.lines.len(), |(i, _)| i + 1);
    let mut all_sels = Vec::new();
    let mut all_blind = Vec::new();
    read_unit_lines(fast, fast.lines.len(), &mut all_sels, &mut all_blind);
    let mut ok_sels = Vec::new();
    let mut ok_blind = Vec::new();
    read_unit_lines(fast, upto, &mut ok_sels, &mut ok_blind);

    assert!(
        !all_sels.is_empty(),
        "`sdk-fast` contains no readable sdk-scoped `cargo test` invocation, so this test cannot \
         discriminate. Its lines: {:?}",
        fast.lines
    );

    let named: BTreeSet<(String, String)> = all_sels
        .iter()
        .flat_map(|s| sel_covers(s, &targets))
        .map(TestTarget::key)
        .collect();
    let mut guaranteed: BTreeSet<(String, String)> = BTreeSet::new();
    let mut why_starved: Vec<String> = Vec::new();
    for sel in &ok_sels {
        let hits = sel_covers(sel, &targets);
        match starves_its_own_targets(sel, hits.len()) {
            None => guaranteed.extend(hits.into_iter().map(TestTarget::key)),
            Some(why) => why_starved.push(format!("  {}\n      ↳ {why}", sel.source)),
        }
    }
    let lost: Vec<&(String, String)> = named.difference(&guaranteed).collect();
    assert!(
        lost.is_empty(),
        "`just sdk-fast` NAMES {} test target(s) it is not guaranteed to RUN:\n{}\n\n\
         The recipe stops being guaranteed here:\n  {}\n\n\
         Targets sharing a fail-fast `cargo test` call:\n{}\n\
         This is the money-path recipe a developer types before committing. §4l G9: fix the \
         class, not the instance — `sdk-fast` has already been the missed sibling once \
         (`cdb59304`), when `sdk-gate-core` was repaired and this recipe four lines away was not. \
         The fix is the same: drop `-e` and aggregate (`|| status=1` per line, `exit $status`), \
         and pass `--no-fail-fast` to any `cargo test` naming more than one target.",
        lost.len(),
        lost.iter()
            .map(|(p, n)| format!("  {p}::{n}"))
            .collect::<Vec<_>>()
            .join("\n"),
        cut.as_ref().map_or(
            "(nowhere — every line is guaranteed)".to_string(),
            |(_, w)| w.clone()
        ),
        if why_starved.is_empty() {
            "  (none)".to_string()
        } else {
            why_starved.join("\n")
        },
    );
}

/// §4l G6 — MEASURE the fail-fast behaviour of one `cargo test` call, do not
/// infer it from the nightly logs.
///
/// A two-target toy is enough and a toy is what this builds: a lib target whose
/// only unit test panics, plus two integration tests that pass. It is a standing
/// test rather than a note in a document because the whole reachability model
/// above rests on the answer — if a future cargo stopped failing fast, this
/// tree's `--no-fail-fast` requirement would become noise, and nothing else here
/// would notice.
///
/// Measured with cargo 1.97.1: **one `Running` line** without the flag,
/// **three** with it, exit 101 both times. No network (`--offline`, no deps), no
/// tool the nightly does not already invoke (G11).
#[test]
fn cargo_test_runs_every_named_target_only_with_no_fail_fast() {
    let dir = std::env::temp_dir().join(format!(
        "p0-7-g6-failfast-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).expect("toy src dir");
    std::fs::create_dir_all(dir.join("tests")).expect("toy tests dir");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"p0_7_g6_toy\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("toy manifest");
    std::fs::write(
        dir.join("src/lib.rs"),
        "#[cfg(test)]\nmod t {\n    #[test]\n    fn the_lib_target_fails() { panic!(\"on purpose\"); }\n}\n",
    )
    .expect("toy lib");
    std::fs::write(dir.join("tests/alpha.rs"), "#[test]\nfn alpha_ran() {}\n").expect("toy alpha");
    std::fs::write(dir.join("tests/beta.rs"), "#[test]\nfn beta_ran() {}\n").expect("toy beta");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let run = |extra: &[&str]| -> (Option<i32>, String) {
        let mut c = Command::new(&cargo);
        c.arg("test")
            .args(["--offline", "--lib", "--test", "alpha", "--test", "beta"])
            .args(extra)
            .current_dir(&dir)
            .env("CARGO_TARGET_DIR", dir.join("target"))
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_INCREMENTAL");
        let out = c.output().unwrap_or_else(|e| {
            panic!(
                "could not run `{cargo} test` on the toy crate at {}: {e}. If cargo cannot \
                 answer, nothing was measured, and that is a failure rather than a pass.",
                dir.display()
            )
        });
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.code(), text)
    };

    let (rc_ff, out_ff) = run(&[]);
    let (rc_nff, out_nff) = run(&["--no-fail-fast"]);
    let _ = std::fs::remove_dir_all(&dir);

    let running = |s: &str| s.matches("Running").count();
    let (n_ff, n_nff) = (running(&out_ff), running(&out_nff));

    assert!(
        n_nff >= 3,
        "the toy crate did not build or did not run its three targets even WITH \
         `--no-fail-fast` ({n_nff} `Running` line(s), exit {rc_nff:?}). Nothing was measured, so \
         nothing below is evidence.\n{out_nff}"
    );
    assert_eq!(
        n_ff, 1,
        "MEASURED CLAIM BROKEN. §4l's reachability model — and the `--no-fail-fast` this tree \
         now requires on every multi-target `cargo test` — rests on cargo stopping after the \
         FIRST target binary that fails. This run produced {n_ff} `Running` line(s) instead of 1 \
         (exit {rc_ff:?}), so either cargo changed or the toy stopped reproducing the shape. \
         Re-measure before changing the model; do not delete the requirement.\n{out_ff}"
    );
    assert!(
        rc_ff.is_some_and(|c| c != 0) && rc_nff.is_some_and(|c| c != 0),
        "`--no-fail-fast` must not launder a failing target into a green run: exit was \
         {rc_ff:?} without it and {rc_nff:?} with it. If the flag ever made the call exit 0, \
         requiring it would trade a starved binary for a silent one."
    );
}

// ── the driven half: run the REAL recipes, one leg forced red at a time ───────
//
// §4l G1/G2/G3/G4/G5 all say the same thing about method: *proved by execution
// — a driven run in which an early leg is forced red and the later legs are
// observed to have produced output — not by reading the recipe.* Everything
// above this line reads text. Everything below runs `just`.
//
// HOW THE SANDBOX IS FAITHFUL, AND WHERE IT IS NOT.
//   * The `Justfile` and `lefthook.yml` are copied BYTE FOR BYTE from the tree
//     under test. No recipe is rewritten, reordered or stubbed — the thing being
//     graded is the recipe's own text, run by the real `just`.
//   * What is replaced is the LEAF TOOLS: `cargo`, `flutter`, `dart`, `git`,
//     `cargo-audit`, `cargo-deny`, `python3` and the FRB codegen become one
//     stub script on a PATH that shadows the real ones. The stub logs its argv
//     and exits 0, or exits 1 when its argv matches a pattern this harness
//     supplies. So "which legs ran" is answered by which TOOLS were invoked,
//     never by a word in the output — a string a tool prints on every run
//     attributes nothing.
//   * NOT faithful: a leg's real duration, its real failure modes, and anything
//     that depends on the real `sdk/` sources (the sandbox scaffolds only the
//     paths the recipes probe with `[ -f … ]`). A scaffold gap makes a leg SKIP,
//     which shows up as a missing tool marker — a loud red, not a quiet pass.
//   * The green baseline is asserted FIRST in every test below. If the sandbox
//     cannot produce a clean run, nothing after it is evidence, and the test
//     says so instead of reporting on a broken harness.

/// A driven sandbox: the tree's own carrier files, a stubbed toolchain.
struct Driven {
    root: PathBuf,
    bin: PathBuf,
    log: PathBuf,
}

impl Drop for Driven {
    fn drop(&mut self) {
        if std::env::var("P0_7_KEEP_SANDBOX").is_err() {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

/// One driven run.
struct Run {
    code: Option<i32>,
    out: String,
    /// Every tool invocation, in order, as `<tool> <args>`.
    tools: Vec<String>,
}

impl Run {
    fn ran(&self, invocation: &str) -> bool {
        self.tools.iter().any(|t| t == invocation)
    }
}

/// The prefix every line the stub writes to the run's output begins with.
///
/// Kept as a constant because `legs_named_as_failed` must be able to tell the
/// harness's own voice from the carrier's, and `STUB` is a raw literal that
/// cannot interpolate it. `Driven::new` asserts the two still agree, so a reword
/// of the stub's `printf` breaks a named assertion instead of quietly making the
/// harness's chatter gradable output again.
const STUB_TAG: &str = "p0-7-stub:";

const STUB: &str = r#"#!/usr/bin/env bash
# P0-7 §4l stub. Logs its argv, then obeys P0_7_FORCE_RED (newline-separated
# substrings). Everything it prints goes to STDERR: a recipe that captures a
# tool's stdout in `$(…)` must not see this harness's own chatter.
tool="$(basename "$0")"
printf '%s %s\n' "$tool" "$*" >> "${P0_7_TOOL_LOG:-/dev/null}"
if [ -n "${P0_7_FORCE_RED:-}" ]; then
  while IFS= read -r p; do
    [ -n "$p" ] || continue
    case "$tool $*" in
      *"$p"*) printf 'p0-7-stub: FORCED-RED <%s %s>\n' "$tool" "$*" >&2; exit 1 ;;
    esac
  done <<< "$P0_7_FORCE_RED"
fi
# Batch D item 1 (S274): two `sdk-gate` legs are anti-vacuous BY DESIGN — the
# swap-off artifact guard refuses to grade without a real dep graph and a real
# archive, and the fuzz lane refuses an empty target list (COULD-NOT-RUN, exit
# 2, never a green skip). A stub that only says "ok" can therefore never make
# them green, and the sandbox's green baseline read 16/4 from the day those
# legs landed (`c8808486`, `ad8f4947`) until the first unbidden nightly showed
# it (2026-09-13). These answers are the minimum each query needs to be GRADED
# in the sandbox; everything they print goes to STDOUT on purpose, because
# the recipes capture it. The FORCED-RED check above still comes first, so a
# forced `cargo build` or `cargo +nightly fuzz run` is red as before.
case "$tool $*" in
  "cargo metadata"*)
    printf '{"target_directory":"%s/target"}\n' "$PWD"; exit 0 ;;
  "cargo tree "*)
    # ≥ 20 lines naming the bridge crate, none naming the swap adapter or its
    # HTTP subtree — the honest shape of a feature-off graph.
    printf 'zec_wallet v0.1.0 (%s)\n' "$PWD"
    for i in $(seq 1 24); do printf '├── p0-7-stub-dep-%s v0.0.0\n' "$i"; done
    exit 0 ;;
  "cargo build "*"--message-format=json"*)
    mkdir -p "$PWD/target/debug"
    printf 'p0-7-stub archive\n' > "$PWD/target/debug/libzec_wallet.a"
    printf '{"reason":"compiler-artifact","target":{"name":"zec_wallet"},"filenames":["%s/target/debug/libzec_wallet.a"]}\n' "$PWD"
    exit 0 ;;
  "cargo +nightly fuzz list"*)
    printf 'p0_7_stub_target\n'; exit 0 ;;
  "nm "*)  # FR-5 C2: P15 refuses a listing under MIN_SYMBOL_LINES (100 000, swap_near_off_no_symbols.sh), so: the floor plus one, naming no adapter or arti symbol. Raise the floor and this green baseline reds — the coupling working. One line on purpose: added lines here displace every registered citation below.
    seq 1 100001 | sed 's/^/0000000000000000 T _p0_7_stub_symbol_/'; exit 0 ;;
esac
printf 'p0-7-stub: ok <%s %s>\n' "$tool" "$*" >&2
exit 0
"#;

/// Tools the `sdk-gate` closure can invoke. Anything NOT here would run for
/// real, so the list is deliberately generous — and the green-baseline
/// assertion in every test is what catches an omission. `nm`, `cargo-fuzz` and
/// `rustup` joined at Batch D item 1: the swap-off guard reads the archive
/// with `nm`, and the fuzz lane checks `command -v cargo-fuzz` and
/// `rustup run nightly rustc --version` before it lists a target.
const STUBBED_TOOLS: &[&str] = &[
    "cargo",
    "cargo-audit",
    "cargo-deny",
    "cargo-fuzz",
    "flutter",
    "dart",
    "git",
    "nm",
    "python3",
    "rustup",
    "flutter_rust_bridge_codegen",
];

/// Recursive file copy. Directories and regular files only — a symlink is
/// skipped rather than followed, because following one is how a sandbox stops
/// being a sandbox.
fn copy_tree(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst)
        .unwrap_or_else(|e| panic!("could not create {}: {e}", dst.display()));
    let entries =
        std::fs::read_dir(src).unwrap_or_else(|e| panic!("could not read {}: {e}", src.display()));
    for entry in entries {
        let entry = entry.expect("directory entry readable");
        let kind = entry.file_type().expect("file type readable");
        let (from, to) = (entry.path(), dst.join(entry.file_name()));
        if kind.is_dir() {
            copy_tree(&from, &to);
        } else if kind.is_file() {
            std::fs::copy(&from, &to).unwrap_or_else(|e| {
                panic!("could not copy {} -> {}: {e}", from.display(), to.display())
            });
        }
    }
}

/// One whitespace-delimited Justfile word, reduced to the repo-relative path it
/// may be naming. `None` for anything that is not one, and — deliberately — for
/// everything that could point OUTSIDE the sandbox: an absolute path, a `..`
/// segment, or a word carrying a shell expansion whose value this scan cannot
/// know. The caller creates files from these, so the filter is a safety
/// property, not a tidiness one.
fn sandbox_path_word(raw: &str) -> Option<String> {
    // `--manifest-path=sdk/Cargo.toml` -> `sdk/Cargo.toml`
    let word = raw.rsplit('=').next().unwrap_or(raw);
    let word = word.trim_matches(|c: char| "'\"`(){}[]<>,;:&|!*?".contains(c));
    if word.is_empty()
        || word.starts_with('/')
        || word.starts_with('-')
        || word.starts_with('~')
        || word.contains('$')
        || word.contains("..")
    {
        return None;
    }
    Some(word.to_string())
}

/// Every repo-relative directory a recipe `cd`s into, and every repo-relative
/// file a recipe names, read out of the `Justfile` text.
///
/// This is what replaces `Driven::new`'s hand-written scaffold list. It is
/// deliberately GENEROUS — a word that looks like a path but names nothing is
/// dropped later by the "does it exist in the source tree?" test, so a false
/// positive costs nothing and a false negative costs an adjudication round.
///
/// Comment lines are skipped so a path mentioned only in prose does not get
/// scaffolded; a path inside an `echo` string still does, and that is the right
/// trade — `sdk-audit`'s failure text names `sdk/Cargo.lock`, and an empty file
/// of that name in a sandbox where cargo is a stub changes nothing.
fn justfile_referenced_paths(text: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut dirs, mut files) = (BTreeSet::new(), BTreeSet::new());
    // Batch D item 1: a recipe that `cd`s and then names a path on a LATER
    // line names it relative to that directory — `sdk-swap-off-guard` does
    // `cd sdk/zec_wallet/rust` and then `bash tests/swap_near_off_no_symbols.sh`,
    // and the script was never copied, so bash's own 127 stood in the sandbox
    // as the leg's verdict. The current recipe's `cd` target is carried across
    // its lines (a recipe header — a non-indented line ending in `:` — resets
    // it) and every file word is ALSO tried joined to it; `scaffold_files`
    // copies only what the source tree has, so a wrong join costs nothing.
    let mut recipe_cd: Option<String> = None;
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if !line.starts_with(char::is_whitespace) && line.trim_end().ends_with(':') {
            recipe_cd = None;
        }
        let cds = cd_targets(line);
        if let Some(last) = cds.last() {
            recipe_cd = Some(last.clone());
        }
        dirs.extend(cds);
        for raw in line.split_whitespace() {
            let Some(w) = sandbox_path_word(raw) else {
                continue;
            };
            // A file is a path with a directory part and a dotted last segment.
            if w.contains('/') && w.rsplit('/').next().is_some_and(|seg| seg.contains('.')) {
                if let Some(d) = &recipe_cd {
                    files.insert(format!("{d}/{w}"));
                }
                files.insert(w);
            } else if w.contains('/') {
                // A directory a recipe names without `cd`-ing into it on that
                // line — `for ws in sdk/zec-wallet-core/fuzz …; do (cd "$ws" …)`:
                // the `cd` word is a variable the scan cannot know, the literal
                // is here. `scaffold_dirs` creates only what the source tree
                // has as a directory.
                dirs.insert(w);
            }
        }
    }
    (dirs, files)
}

/// Every directory this ONE line `cd`s into. `(cd sdk/zec_wallet && …` counts.
///
/// Shared by the sandbox scaffolder above and by the `flutter-ci` test below,
/// which reads the packages a recipe grades out of the directories it enters
/// rather than out of a list in this file.
fn cd_targets(line: &str) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let mut out = Vec::new();
    for (i, raw) in words.iter().enumerate() {
        if raw.trim_matches(|c: char| "(`;&|".contains(c)) != "cd" {
            continue;
        }
        if let Some(d) = words.get(i + 1).and_then(|w| sandbox_path_word(w)) {
            out.push(d);
        }
    }
    out
}

/// Rule 4's directory half: create, under `root`, every directory the
/// `Justfile` `cd`s into that the source tree has.
///
/// Factored out of `Driven::new` (§4m G15) so the loop can be driven
/// with a word that BYPASSES `sandbox_path_word` — the only way to see whether
/// the loop itself refuses a write outside the sandbox, or merely trusts the
/// filter upstream of it.
fn scaffold_dirs(repo: &Path, root: &Path, dirs: &BTreeSet<String>) {
    for d in dirs {
        let dst = root.join(d);
        assert_inside_sandbox(root, &dst, d);
        if repo.join(d).is_dir() {
            std::fs::create_dir_all(&dst).expect("sandbox scaffold dir");
        }
    }
}

/// The belt behind `sandbox_path_word`'s braces, on BOTH loops (§4m G15):
/// `dst` must lie inside `root` LEXICALLY — a prefix match AND no `..` in the
/// relative part. `sandbox_path_word` already refuses absolute paths and
/// `..`; this is the assertion that keeps that refusal load-bearing, because
/// everything after it WRITES, and a scan that ever let one through would be
/// writing outside the sandbox.
///
/// **The form was `dst.starts_with(root)` alone, on the file loop only,
/// and it had a hole exactly where traversal goes through it.**
/// `Path::starts_with` is component-wise over the UN-normalised path, so
/// `root/../x` "starts with" `root` and the assert passed while the write
/// landed outside. Driven, not read: `scaffold_files` given `../<escape>/planted.txt`
/// with the source file real wrote `root/../<escape>/planted.txt` at 6524db0f
/// (the planted G15 case — the contract asked only that the directory loop
/// COPY that assert).
fn assert_inside_sandbox(root: &Path, dst: &Path, word: &str) {
    let inside = dst.strip_prefix(root).is_ok_and(|rel| {
        !rel.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    });
    assert!(
        inside,
        "the scaffold path {word:?} resolves to {} — outside the sandbox {}. Nothing is \
         written until `sandbox_path_word` refuses it.",
        dst.display(),
        root.display()
    );
}

/// Rules 3 and 4's file half: an empty placeholder (or, for a shell helper, a
/// verbatim copy) under `root` for every file the `Justfile` names that the
/// source tree has. Factored out with `scaffold_dirs`, for the same reason.
fn scaffold_files(repo: &Path, root: &Path, files: &BTreeSet<String>) {
    for f in files {
        let (src, dst) = (repo.join(f), root.join(f));
        assert_inside_sandbox(root, &dst, f);
        if dst.exists() || !src.is_file() {
            continue;
        }
        if let Some(p) = dst.parent() {
            std::fs::create_dir_all(p).expect("sandbox scaffold parent");
        }
        if f.ends_with(".sh") || f.ends_with(".bash") {
            std::fs::copy(&src, &dst).unwrap_or_else(|e| {
                panic!("could not copy the carrier helper {}: {e}", src.display())
            });
        } else {
            std::fs::write(&dst, "").expect("sandbox scaffold file");
        }
    }
}

impl Driven {
    /// Build a sandbox from `root`'s carrier files. `None` post-extraction.
    ///
    /// **WHAT IS COPIED, AND WHY IT IS NO LONGER A LIST OF FILE NAMES (
    /// repair).** The first version of this harness copied exactly `Justfile`
    /// and `lefthook.yml` and hand-scaffolded four empty paths. That allowlist
    /// was wrong the moment the other half of §4l's split wrote recipes that
    /// begin `. scripts/gate-aggregate.sh`: all four driven tests died on their
    /// own green baseline with `exit code 2` and `scripts/gate-aggregate.sh: No
    /// such file or directory`. The harness was HONEST — it refused to report on
    /// a sandbox it could not run — and it was still wrong, and no gate could
    /// have told anyone before the halves were joined. **A copy list somebody
    /// has to remember to extend will be wrong again the next time a recipe
    /// grows a dependency**, so it is derived here instead.
    ///
    /// Four rules replace the two lists, each with its own reason:
    ///
    /// 1. **`Justfile` + `lefthook.yml`** — the carriers under test.
    /// 2. **All of `scripts/`, verbatim.** §4l's split gives the implementer
    ///    "`Justfile` and, if it needs one, a new script under `scripts/`", so
    ///    that directory is the whole file surface the other half may add to.
    ///    Copying the DIRECTORY also covers a helper that sources a second
    ///    helper — a dependency no scan of the `Justfile` could see.
    /// 3. **Every `*.sh`/`*.bash` path the `Justfile` names, verbatim**, for a
    ///    helper placed outside `scripts/`. A shell script a recipe names is
    ///    carrier logic; an empty one would change what the recipe does.
    /// 4. **An EMPTY placeholder for each other repo-relative file the
    ///    `Justfile` names THAT THE SOURCE TREE HAS** (`src.is_file()`), plus
    ///    the directories it `cd`s into that the source tree has (`is_dir()`).
    ///    The word struck here was "every" (adjudication R2-Q5 item 3,
    ///    §4l-run owed row 3): a named path the source tree LACKS gets nothing,
    ///    so a sandbox cut from a tree without `sdk/zec_wallet_ui/` reproduces
    ///    that tree faithfully — and `remove_scaffolded` below is how a test on
    ///    a tree that HAS it reaches the same state (§4m G12). This replaces
    ///    the hand-written scaffold list, and empty is the CONTENT the sandbox
    ///    wants rather than a shortcut: `wallet-bridge-verify` reads
    ///    `flutter_rust_bridge:` out of `sdk/zec_wallet/pubspec.yaml` and
    ///    refuses a version skew against the stub's empty `--version`, so a
    ///    sandbox holding the REAL pubspec fails its own green baseline.
    ///    Measured while writing this, not assumed.
    ///
    /// **NOT `git archive`, and the reason is checkable rather than aesthetic:**
    /// `P0_7_REPO_ROOT` is pointed at a plain directory by
    /// `scripts/gate-proofs/p0-7-r1b-reaching.sh` — its sandboxes are tar
    /// extractions, not repositories — so a copy that needs a git repo cannot
    /// run in the place this harness is driven most. Nor the whole tree: rule 4
    /// says why the real `sdk/` would break the baseline.
    ///
    /// What survives of the old fragility, stated rather than hidden: a helper
    /// outside `scripts/` that the `Justfile` names without a `.sh` suffix gets
    /// an empty placeholder, and sourcing it yields `gate_leg: command not
    /// found` on the green baseline. Loud, named, and one line from a fix — not
    /// silence.
    fn new(tag: &str) -> Option<Driven> {
        let repo = repo_root()?;
        assert!(
            STUB.contains(STUB_TAG),
            "the stub no longer tags its output with `{STUB_TAG}`, so \
             `legs_named_as_failed` can no longer tell this harness's own chatter from what the \
             carrier printed. Every G3-shaped assertion would start grading the stub's argv."
        );
        let root = std::env::temp_dir().join(format!(
            "p0-7-g-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).expect("sandbox bin dir");
        // Rule 1.
        for f in ["Justfile", "lefthook.yml"] {
            let text = std::fs::read_to_string(repo.join(f))
                .unwrap_or_else(|e| panic!("could not read {}: {e}", repo.join(f).display()));
            std::fs::write(root.join(f), text).expect("sandbox carrier file");
        }
        // Rule 2.
        if repo.join("scripts").is_dir() {
            copy_tree(&repo.join("scripts"), &root.join("scripts"));
        }
        // Rules 3 and 4, derived from the sandbox's own copy of the Justfile.
        let justfile = std::fs::read_to_string(root.join("Justfile")).expect("sandbox Justfile");
        let (dirs, files) = justfile_referenced_paths(&justfile);
        scaffold_dirs(&repo, &root, &dirs);
        scaffold_files(&repo, &root, &files);
        for t in STUBBED_TOOLS {
            let p = bin.join(t);
            std::fs::write(&p, STUB).expect("stub written");
            let st = Command::new("chmod")
                .arg("+x")
                .arg(&p)
                .status()
                .expect("chmod runs");
            assert!(
                st.success(),
                "could not make the stub {} executable",
                p.display()
            );
        }
        Some(Driven {
            log: root.join("tools.log"),
            root,
            bin,
        })
    }

    fn justfile_text(&self) -> String {
        std::fs::read_to_string(self.root.join("Justfile")).expect("sandbox Justfile readable")
    }

    /// Run one recipe with zero or more forced-red tool patterns.
    fn run(&self, recipe: &str, force: &[&str]) -> Run {
        let _ = std::fs::write(&self.log, "");
        let path = format!(
            "{}:{}",
            self.bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = Command::new("just")
            .arg("--justfile")
            .arg(self.root.join("Justfile"))
            .arg("--working-directory")
            .arg(&self.root)
            .arg(recipe)
            .env("PATH", path)
            .env("P0_7_TOOL_LOG", &self.log)
            .env("P0_7_FORCE_RED", force.join("\n"))
            .output()
            .unwrap_or_else(|e| {
                panic!(
                    "could not run `just {recipe}` in the sandbox at {}: {e}\n\
                     §4l G11: the nightly runs `just sdk-gate`, so `just` is on the PATH wherever \
                     this gate runs. If cargo can reach this test but `just` cannot be spawned, \
                     nothing was driven, and that is a failure rather than a pass.",
                    self.root.display()
                )
            });
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        let tools = std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Run {
            code: out.status.code(),
            out: text,
            tools,
        }
    }

    /// `sdk-gate`'s legs, read out of the sandbox Justfile in the shape it
    /// actually has: declared dependencies first, then `just <name>` hand-offs
    /// in its body. §4l's "what this contract does NOT decide" #1 warns that
    /// moving the legs from the dependency list into the body changes what the
    /// R1(b) closure walks — this reads BOTH, so the test does not care which
    /// shape the repair took.
    fn sdk_gate_legs(&self) -> Vec<String> {
        let recipes = parse_justfile(&self.justfile_text());
        let gate = recipes
            .get("sdk-gate")
            .expect("the sandbox Justfile has an `sdk-gate` recipe");
        let mut legs: Vec<String> = Vec::new();
        for name in gate
            .deps
            .iter()
            .chain(gate.line_calls.iter().map(|(_, n)| n))
        {
            if recipes.contains_key(name) && !legs.contains(name) {
                legs.push(name.clone());
            }
        }
        legs
    }
}

/// Every leg's own tool footprint, and one invocation that is BOTH unique to
/// that leg and load-bearing for it.
///
/// **The second half is not decoration, and the first version of this harness
/// did not have it.** `flutter-ci`'s first unique invocation is
/// `flutter_rust_bridge_codegen --version`, whose failure `wallet-bridge-verify`
/// swallows on purpose — forcing it red left `just sdk-gate` GREEN, and two
/// tests below "failed" describing the harness rather than the tree. A mutation
/// nobody proved lethal is the inert-mutation defect (§4j D5): it reports on an
/// edit that never happened. So each candidate is now driven against its own
/// leg and kept only if the leg actually goes red.
fn leg_footprints(d: &Driven, legs: &[String]) -> BTreeMap<String, (Vec<String>, String)> {
    let mut solo: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for l in legs {
        let r = d.run(l, &[]);
        assert!(
            !r.tools.is_empty(),
            "`just {l}` invoked no tool at all in the sandbox, so this harness has no way to see \
             whether it ran. Either the leg no-ops without its toolchain (scaffold gap) or it \
             calls something {STUBBED_TOOLS:?} does not cover.\nOutput:\n{}",
            r.out
        );
        solo.insert(l.clone(), r.tools);
    }
    let mut out = BTreeMap::new();
    for l in legs {
        let mine = &solo[l];
        let others: BTreeSet<&String> = solo
            .iter()
            .filter(|(k, _)| *k != l)
            .flat_map(|(_, v)| v.iter())
            .collect();
        let mut candidates: Vec<&String> = mine.iter().filter(|t| !others.contains(*t)).collect();
        candidates.dedup();
        // A money invocation is a last resort: forcing a leg red must not mean
        // forcing the seed-custody or kill-door binary red as a side effect.
        candidates.sort_by_key(|t| MONEY_TEST_TARGETS.iter().any(|m| t.contains(m)));
        let pick = candidates
            .iter()
            .find(|c| d.run(l, &[c]).code.is_some_and(|x| x != 0))
            .unwrap_or_else(|| {
                panic!(
                    "no tool invocation is both unique to `{l}` and able to turn it red, so this \
                     harness cannot force exactly that leg to fail. Candidates tried: \
                     {candidates:?}"
                )
            });
        out.insert(l.clone(), (mine.clone(), (*pick).clone()));
    }
    out
}

/// Is `name` present in `line` as a whole token? `-` counts as a word
/// character, so `sdk-gate` does NOT match inside `sdk-gate-core`.
fn names_token(line: &str, name: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
    let mut from = 0usize;
    while let Some(rel) = line[from..].find(name) {
        let s = from + rel;
        let e = s + name.len();
        let before_ok = s == 0 || !line[..s].chars().next_back().is_some_and(word);
        let after_ok = e >= line.len() || !line[e..].chars().next().is_some_and(word);
        if before_ok && after_ok {
            return true;
        }
        from = s + 1;
    }
    false
}

/// Which legs the output names AS HAVING FAILED (§4l G3).
///
/// THE TRAP THIS AVOIDS. `just` echoes a non-shebang recipe's lines — comments
/// included — so a leg's name appears in the output of runs where it passed and
/// of runs where it never ran. Grepping for the name attributes nothing. A leg
/// counts as named-failed only when a line carries the name AND a failure
/// marker, and echoed comment lines are excluded. The two-sided check is what
/// makes it evidence: the GREEN baseline must name nobody.
///
/// **AND THE HARNESS'S OWN CHATTER IS EXCLUDED, which is the same rule applied
/// to the one writer nobody suspects.** The stub prints
/// `p0-7-stub: FORCED-RED <flutter test>` on every forced invocation. That line
/// carries the failing tool's whole argv, so a grader that reads it would find
/// a different string for every leg — and would report "this gate names its
/// failing leg" about a gate that printed nothing, on the strength of the
/// harness's own log. Today's marker list happens not to match `FORCED-RED`, so
/// nothing was wrong; one word added to `markers` would have made it wrong
/// silently. Grade what the RECIPE said.
fn legs_named_as_failed(out: &str, legs: &[String]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for t in failure_lines(out) {
        for l in legs {
            if names_token(&t, l) {
                found.insert(l.clone());
            }
        }
    }
    found
}

/// The lines of a driven run that REPORT A FAILURE — the one definition of that
/// predicate in this file.
///
/// Factored out of `legs_named_as_failed` when the `flutter-ci` test below
/// needed the same three exclusions (echoed comment, harness chatter, no failure
/// marker) to compare whole reports rather than leg names. Two copies of "what
/// counts as a failure report" is one copy too many: the copy that drifts is the
/// one grading the gate nobody re-reads.
fn failure_lines(out: &str) -> Vec<String> {
    let markers = ["error", "fail", "✗", "failed", "refused"];
    out.lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.starts_with('#') || t.starts_with(STUB_TAG) {
                return None;
            }
            let lower = t.to_lowercase();
            markers
                .iter()
                .any(|m| lower.contains(m))
                .then(|| t.to_string())
        })
        .collect()
}

/// EVERY driven `sdk-gate` test asserts this FIRST, and the reason is a defect
/// this harness had. `collect_just_calls` matches a token equal to `just`, so a
/// body of `-just sdk-audit` (just's ignore-failure prefix) resolves to NO legs
/// at all — and a loop over an empty leg list passes every assertion under it.
/// Measured while building the proof: that shape made two of the three driven
/// tests green on a carrier that reports nothing. An empty denominator is not a
/// clean bill of health.
fn assert_legs_floor(legs: &[String]) {
    for want in SDK_GATE_LEGS_FLOOR {
        assert!(
            legs.iter().any(|l| l == want),
            "`sdk-gate`'s legs, as this scan reads them, are {legs:?} and `{want}` is not among \
             them. Either the gate stopped running that leg, or it calls it in a form the scan \
             cannot see — `collect_just_calls` only recognises a bare `just <name>` token, so \
             `-just <name>` and `just \"$LEG\"` both vanish. Both readings are defects: §4l G4 \
             names the legs this gate must contain, and a leg the scan cannot see is also a leg \
             `every_sdk_test_target_is_named_by_a_live_carrier` drops from the live closure."
        );
    }
}

/// §4l G1 + G4 — `just sdk-gate` runs every leg, and a non-money leg cannot
/// starve the money binaries.
///
/// Driven: one leg forced red at a time, and every OTHER leg's own tool
/// footprint must still appear. IT-10 — what refuses this today? `just` aborts
/// a dependency chain at the first failing prerequisite, so forcing `sdk-audit`
/// red leaves `sdk-gate-core` unexecuted and `custody_policy` never runs.
#[test]
fn just_sdk_gate_runs_every_leg_and_the_money_binaries_when_one_leg_is_red() {
    let Some(d) = Driven::new("g1") else {
        return;
    };
    let legs = d.sdk_gate_legs();
    assert_legs_floor(&legs);

    let base = d.run("sdk-gate", &[]);
    assert_eq!(
        base.code,
        Some(0),
        "the GREEN baseline of `just sdk-gate` did not pass in the sandbox, so nothing below it \
         is evidence about starvation. Either a leg needs a real tool this harness stubs, or a \
         scaffolded path is missing.\nTools: {:?}\nOutput:\n{}",
        base.tools,
        base.out
    );

    let prints = leg_footprints(&d, &legs);
    for forced in &legs {
        let (_, pattern) = &prints[forced];
        let r = d.run("sdk-gate", &[pattern]);
        // Deliberately NOT asserted here: the exit code. G1 is "every leg ran"
        // and G2 is "and it is still red" — a carrier of `just <leg> || true`
        // satisfies the first and destroys the second, and keeping them in
        // separate tests is what makes that carrier produce one green and one
        // red instead of one indistinguishable failure. Proof case D4.
        for other in &legs {
            if other == forced {
                continue;
            }
            let (want, _) = &prints[other];
            let missing: Vec<&String> = want.iter().filter(|t| !r.ran(t)).collect();
            assert!(
                missing.is_empty(),
                "FORCING `{forced}` RED STARVED `{other}`.\n\
                 `just sdk-gate` must run EVERY leg and report red at the end (§4l G1); it must \
                 not stop at the first leg that fails. Missing invocations:\n{}\n\n\
                 Forced by: `{pattern}`\nWhat did run:\n{}\n\n\
                 The in-tree shape to copy is `flutter-ci`: a `#!/usr/bin/env bash` body, \
                 `status=0`, `just <leg> || status=1` per leg, `exit $status`.",
                missing
                    .iter()
                    .map(|t| format!("  {t}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                r.tools
                    .iter()
                    .map(|t| format!("  {t}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        // G4, asserted by name as well as by footprint: these two are the whole
        // point of the item, and a diff that renames a leg must not quietly stop
        // grading them.
        if forced == "sdk-gate-core" {
            continue; // forcing the core red is G5's subject, not G4's
        }
        for money in MONEY_TEST_TARGETS {
            assert!(
                r.tools.iter().any(|t| t.contains(money)),
                "FORCING THE NON-MONEY LEG `{forced}` RED STOPPED `{money}` FROM RUNNING.\n\
                 §4l G4: neither `sdk-audit`'s network dependency, nor a clippy regression in \
                 `sdk-lint`, nor a red `l10n-drift-check`, nor a Dart failure in `flutter-ci` may \
                 prevent the seed-custody and swap-kill-door binaries executing. §4l evidence 1: \
                 neither has appeared in ANY recorded nightly log.\nWhat ran:\n{}",
                r.tools
                    .iter()
                    .map(|t| format!("  {t}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    }
}

/// §4l G2 — aggregation must not launder a failure into a green.
///
/// Both directions, because only the pair is evidence: all-green ⇒ exit 0, and
/// any-red ⇒ non-zero. A recipe that runs every leg and swallows the result is
/// the `-`-prefix shape, which satisfies G1 and destroys the gate.
#[test]
fn just_sdk_gate_is_red_when_a_leg_failed_and_green_when_none_did() {
    let Some(d) = Driven::new("g2") else {
        return;
    };
    let legs = d.sdk_gate_legs();
    assert_legs_floor(&legs);
    let base = d.run("sdk-gate", &[]);
    assert_eq!(
        base.code,
        Some(0),
        "all legs green must mean exit 0. It exited {:?}.\nOutput:\n{}",
        base.code,
        base.out
    );
    let prints = leg_footprints(&d, &legs);
    for forced in &legs {
        let (_, pattern) = &prints[forced];
        let r = d.run("sdk-gate", &[pattern]);
        assert!(
            r.code.is_some_and(|c| c != 0),
            "`just sdk-gate` exited {:?} with `{forced}` forced red (pattern `{pattern}`). A gate \
             that runs every leg and then reports green is worse than one that stops early: it \
             says the money path was evaluated and clean. Note `just`'s `-` line prefix both \
             ignores the failure AND suppresses the `error: Recipe … failed` message, so it \
             satisfies G1 by destroying G2 and G3.\nOutput:\n{}",
            r.code,
            r.out
        );
    }
}

/// §4l G3 — the failing leg is NAMED, and two failing legs name both.
///
/// §4l evidence 3: `flutter-ci` failed on two nights, its last output line was
/// `All tests passed!` from a LATER leg, and the only signal was a one-line
/// `exit code 1` thousands of lines further down. Aggregation without naming is
/// a log that reads green.
#[test]
fn just_sdk_gate_names_every_leg_that_failed() {
    let Some(d) = Driven::new("g3") else {
        return;
    };
    let legs = d.sdk_gate_legs();
    assert_legs_floor(&legs);
    let base = d.run("sdk-gate", &[]);
    assert_eq!(
        base.code,
        Some(0),
        "the baseline must be green before naming means anything.\nOutput:\n{}",
        base.out
    );
    let named_on_green = legs_named_as_failed(&base.out, &legs);
    assert!(
        named_on_green.is_empty(),
        "the ALL-GREEN run already names {named_on_green:?} as failed. Whatever this test then \
         detected on a red run would attribute nothing — a string a tool prints on every run is \
         not evidence. Fix the detector or the recipe, not the expectation."
    );

    let prints = leg_footprints(&d, &legs);
    for forced in &legs {
        let (_, pattern) = &prints[forced];
        let r = d.run("sdk-gate", &[pattern]);
        let got = legs_named_as_failed(&r.out, &legs);
        assert_eq!(
            got,
            BTreeSet::from([forced.clone()]),
            "with `{forced}` forced red, the output names {got:?} as failed. It must name exactly \
             that leg: a run that fails without saying which leg failed costs the reader the \
             whole log, and the nightly writes ONE word.\nOutput:\n{}",
            r.out
        );
    }

    // Two at once — the case a serial abort chain cannot report, because the
    // second leg never runs.
    if legs.len() >= 2 {
        let (a, b) = (&legs[0], &legs[legs.len() - 1]);
        let pats = [prints[a].1.as_str(), prints[b].1.as_str()];
        let r = d.run("sdk-gate", &pats);
        let got = legs_named_as_failed(&r.out, &legs);
        assert_eq!(
            got,
            BTreeSet::from([a.clone(), b.clone()]),
            "with BOTH `{a}` and `{b}` forced red, the output names {got:?}. §4l G3: a run with \
             two failing legs names both. A chain that aborts at the first failure can never \
             satisfy this, which is the point of the row.\nOutput:\n{}",
            r.out
        );
    }
}

/// §4l G5 — the same property one level down, INSIDE `sdk-gate-core`.
///
/// Evidence 5's class: recipe-line order and dependency order have each been
/// repaired twice, and the third instance — a red line starving the lines below
/// it inside one recipe body — is graded here by driving the recipe rather than
/// by reading it.
#[test]
fn just_sdk_gate_core_runs_every_line_when_one_line_is_red() {
    let Some(d) = Driven::new("g5") else {
        return;
    };
    let base = d.run("sdk-gate-core", &[]);
    assert_eq!(
        base.code,
        Some(0),
        "the GREEN baseline of `just sdk-gate-core` did not pass in the sandbox, so nothing below \
         it is evidence.\nTools: {:?}\nOutput:\n{}",
        base.tools,
        base.out
    );
    // Only the `cargo test` legs are graded. A failing `cargo clean` is a
    // different question and this test does not
    // decide it — stated rather than silently skipped.
    let test_calls: Vec<String> = base
        .tools
        .iter()
        .filter(|t| t.starts_with("cargo test"))
        .cloned()
        .collect();
    assert!(
        test_calls.len() >= 2,
        "`just sdk-gate-core` made {} `cargo test` call(s) in the sandbox; this test cannot \
         discriminate on fewer than 2. Its recipe named the money binaries in five separate \
         invocations at S260.\nTools: {:?}",
        test_calls.len(),
        base.tools
    );

    for forced in &test_calls {
        let r = d.run("sdk-gate-core", &[forced]);
        assert!(
            r.code.is_some_and(|c| c != 0),
            "forcing `{forced}` red left `just sdk-gate-core` exiting {:?}.",
            r.code
        );
        let missing: Vec<&String> = test_calls
            .iter()
            .filter(|t| *t != forced && !r.ran(t))
            .collect();
        assert!(
            missing.is_empty(),
            "A RED LINE IN `sdk-gate-core` STARVED THE LINES AFTER IT.\n\
             Forced red: {forced}\n\
             Never ran:\n{}\n\n\
             §4l G5: whatever satisfies G1 for the dependency list must also satisfy it for the \
             recipe body, or the item has fixed one instance of the class and left the sibling \
             for the third time. `just` runs a non-shebang recipe one line at a time and stops at \
             the first failure; the core `--lib` leg is knowingly red until T0-4, so ordering \
             only chooses WHICH lines are starved.\n\
             What did run:\n{}",
            missing
                .iter()
                .map(|t| format!("  {t}"))
                .collect::<Vec<_>>()
                .join("\n"),
            r.tools
                .iter()
                .map(|t| format!("  {t}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

/// §4l G3, at the level its own evidence names — **INSIDE `flutter-ci`**.
///
/// WHY THIS TEST EXISTS AND `just_sdk_gate_names_every_leg_that_failed` IS NOT
/// ENOUGH. G3 is written at the `sdk-gate` level, and at that level the naming
/// is free: `just` itself prints `error: Recipe `flutter-ci` failed`, and the
/// gate's own accounting names `flutter-ci` too. So `legs_named_as_failed` sees
/// `{flutter-ci}` whether or not `flutter-ci` says one word about its own legs —
/// the row passes with its motivating defect fully present. §4l evidence 3 is
/// not about `flutter-ci`: it is about `zec_wallet_ui:test`, the Dart test that
/// failed in the nightly on 6 September behind a last output line reading
/// `All tests passed!`. **Nothing in this tree graded that until this test.**
///
/// WHAT IT MEASURES, AND WHY IT DOES NOT READ THE RECIPE'S NAMING CONVENTION.
/// It never looks for `gate_leg`, or for any particular label format — the
/// implementer owns that shape (§4l "does NOT decide" #4) and a test that
/// pattern-matched it would fail the day the format improved. It drives the
/// recipe and grades two properties of what came back:
///
///   **(A) IDENTITY** — with one inner phase forced red, at least one line that
///   reports a failure names a package the recipe entered, and across the phases
///   every package in the floor is named at least once. The all-green run must
///   name nobody, or the detector is matching a string printed on every run.
///
///   **(B) DISCRIMINATION** — two different forced phases must not produce the
///   SAME failure report. `flutter-ci` is one recipe with thirteen legs; a
///   report that cannot tell a format failure from a test failure costs the
///   reader the whole log, which is the cost §4l evidence 3 measured in days.
///
/// THE PHASES ARE DERIVED, NOT LISTED. Every distinct tool invocation of the
/// green run is a candidate; those a hand-off recipe makes on its own are
/// subtracted (forcing `git diff` red reds `wallet-bridge-verify`, and `just`
/// would name THAT for free — the very hole this test exists to close, one level
/// further down); and what remains is kept only if forcing it actually turns the
/// recipe red. A mutation nobody proved lethal reports on an edit that never
/// happened (§4j D5) — `flutter_rust_bridge_codegen --version` is exactly that
/// case here, swallowed on purpose by `wallet-bridge-verify`, and it drops out.
///
/// IT-10 — what refuses this today? The pre-`flutter-ci`: `status=1` per
/// leg, `exit $status`, naming nothing. It satisfies G1, G2 and the `sdk-gate`
/// level of G3, and it fails (A) on its first phase. Watched: proof case D8 of
/// `scripts/gate-proofs/p0-7-r1b-reaching.sh`.
#[test]
fn just_flutter_ci_names_the_inner_leg_that_failed() {
    let Some(d) = Driven::new("g3-inner") else {
        return;
    };
    let recipes = parse_justfile(&d.justfile_text());
    let ci = recipes.get("flutter-ci").unwrap_or_else(|| {
        panic!(
            "the sandbox Justfile has no `flutter-ci` recipe. §4l evidence 3 is about that recipe \
             failing without naming its leg; if it has been renamed, this test must be re-aimed \
             rather than left passing on a recipe that no longer exists. Recipes read: {:?}",
            recipes.keys().collect::<Vec<_>>()
        )
    });

    // The packages, read out of the directories the recipe enters.
    let mut packages: Vec<String> = Vec::new();
    for line in &ci.lines {
        for dir in cd_targets(line) {
            let leaf = dir.rsplit('/').next().unwrap_or(&dir).to_string();
            if !leaf.is_empty() && !packages.contains(&leaf) {
                packages.push(leaf);
            }
        }
    }
    for want in FLUTTER_CI_PACKAGE_FLOOR {
        assert!(
            packages.iter().any(|p| p == want),
            "`flutter-ci`'s packages, as this scan reads them, are {packages:?} and `{want}` is \
             not among them. Either the recipe stopped grading that package, or it enters it in a \
             form `cd_targets` cannot see — and an empty (or short) package list passes every \
             assertion below it. §4l evidence 3's failing leg was `zec_wallet_ui`'s."
        );
    }

    // The GREEN baseline first: nothing under it is evidence otherwise.
    let base = d.run("flutter-ci", &[]);
    assert_eq!(
        base.code,
        Some(0),
        "the GREEN baseline of `just flutter-ci` did not pass in the sandbox, so nothing below it \
         says anything about naming.\nTools: {:?}\nOutput:\n{}",
        base.tools,
        base.out
    );
    let named_on_green = legs_named_as_failed(&base.out, &packages);
    assert!(
        named_on_green.is_empty(),
        "the ALL-GREEN run of `just flutter-ci` already names {named_on_green:?} beside a failure \
         marker. Whatever this test then detected on a red run would attribute nothing.\n\
         Output:\n{}",
        base.out
    );

    // Candidate phases: this recipe's OWN work, with each hand-off recipe's solo
    // footprint subtracted.
    let mut handed_off: BTreeSet<String> = BTreeSet::new();
    for (_, name) in &ci.line_calls {
        if !recipes.contains_key(name) {
            continue;
        }
        handed_off.extend(d.run(name, &[]).tools);
    }
    let mut phases: Vec<String> = Vec::new();
    for t in &base.tools {
        if !handed_off.contains(t) && !phases.contains(t) {
            phases.push(t.clone());
        }
    }

    // Keep only the phases that really turn the recipe red, and record what each
    // one made it say.
    let mut reports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut named: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for phase in &phases {
        let r = d.run("flutter-ci", &[phase]);
        if r.code.is_none_or(|c| c == 0) {
            continue; // not lethal on its own — grading it would grade nothing
        }
        reports.insert(phase.clone(), failure_lines(&r.out));
        named.insert(phase.clone(), legs_named_as_failed(&r.out, &packages));
    }
    assert!(
        reports.len() >= 3,
        "only {} of {} candidate phase(s) in `flutter-ci` could be turned red one at a time, and \
         this test cannot discriminate on fewer than 3 — with fewer, (B) has nothing to compare \
         and (A) cannot cover {FLUTTER_CI_PACKAGE_FLOOR:?}. Candidates: {phases:?}",
        reports.len(),
        phases.len(),
    );

    // (A) IDENTITY.
    for (phase, got) in &named {
        assert!(
            !got.is_empty(),
            "FORCING `{phase}` RED LEFT `just flutter-ci` NAMING NO PACKAGE.\n\
             §4l evidence 3: on 6 September this recipe failed on `zec_wallet_ui`'s swap-screen \
             test, its LAST output line was `All tests passed!` from a later leg, and the only \
             signal was a one-line `exit code 1` thousands of lines further down. Aggregating a \
             leg's exit code without naming the leg produces a log whose tail reads green.\n\
             Packages it entered: {packages:?}\n\
             What it reported instead:\n{}",
            reports[phase]
                .iter()
                .map(|l| format!("  {l}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    let covered: BTreeSet<&String> = named.values().flatten().collect();
    for want in FLUTTER_CI_PACKAGE_FLOOR {
        assert!(
            covered.iter().any(|p| p.as_str() == *want),
            "no forced-red phase made `flutter-ci` name `{want}` as failing. Across {} phase(s) \
             the packages it named were {covered:?}. `zec_wallet_ui` is the one §4l evidence 3 is \
             about, and a naming scheme that can never mention a package grades it no better than \
             silence.",
            reports.len(),
        );
    }

    // (B) DISCRIMINATION.
    let ordered: Vec<(&String, &Vec<String>)> = reports.iter().collect();
    for i in 0..ordered.len() {
        for j in (i + 1)..ordered.len() {
            let (a, ra) = ordered[i];
            let (b, rb) = ordered[j];
            assert_ne!(
                ra,
                rb,
                "forcing `{a}` red and forcing `{b}` red made `just flutter-ci` print the SAME \
                 failure report, so the report does not say WHICH leg failed. §4l G3 at the level \
                 evidence 3 names: `flutter-ci` has {} leg(s) worth of work in one recipe, and a \
                 reader who cannot tell a format failure from a test failure has to read the whole \
                 log — which is the cost that let `zec_wallet_ui` stay red for a day.\n\
                 Both printed:\n{}",
                reports.len(),
                ra.iter()
                    .map(|l| format!("  {l}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §4m — the gate rows §4l-run FILED rather than patched: G12–G16.
//
// Contract: `docs/plan/production-readiness-phase-0.md` §4m. Every row below
// was watched RED first (IT-3 +A) — the driven ones against the `Justfile` at
// `6524db0f`, the harness ones against this file's own harness before the
// repair beneath them landed. The refusing mechanism is named per row (IT-10).
// ─────────────────────────────────────────────────────────────────────────────

impl Driven {
    /// Remove one scaffolded path — a file or a whole directory — from the
    /// sandbox AFTER `Driven::new` built it.
    ///
    /// This is the state a tree that LACKS a package is in (§4m row 7, Q-G12).
    /// Rule 4 scaffolds a placeholder for every path the `Justfile` names that
    /// the SOURCE tree has, so on `wallet-track` every sandbox has every
    /// `pubspec.yaml` and, until this method existed, no driven test could
    /// reach the missing state — `flutter-ci`'s `[ -f … ]` guards were true in
    /// every sandbox and their false branch was ungraded.
    ///
    /// Panics if the path was never scaffolded: a test that "removes" a thing
    /// the sandbox never had asserts about a sandbox that did not change, which
    /// is the inert-mutation defect (§4j D5) one level up.
    fn remove_scaffolded(&self, rel: &str) {
        let p = self.root.join(rel);
        // Same belt as the scaffold loops: a removal is a write too.
        assert_inside_sandbox(&self.root, &p, rel);
        assert!(
            p.exists(),
            "the sandbox never scaffolded {rel:?} (looked at {}), so removing it changes nothing \
             and whatever the test asserts next is about an unchanged sandbox. Either the \
             `Justfile` no longer names that path (re-aim the test) or the source tree lacks it \
             (rule 4 scaffolds only paths the source tree has).",
            p.display()
        );
        let res = if p.is_dir() {
            std::fs::remove_dir_all(&p)
        } else {
            std::fs::remove_file(&p)
        };
        res.unwrap_or_else(|e| panic!("could not remove {}: {e}", p.display()));
        assert!(!p.exists(), "{} still exists after removal", p.display());
    }

    /// Rewrite the SANDBOX `Justfile`: insert `echo laundered` between the
    /// first non-comment body line of `recipe` that contains `above` and the
    /// `gate_leg <leg> $?` line directly under it. Returns the two lines it
    /// separated — the command and the `gate_leg` — so the caller can force
    /// the command red and name the site in its message.
    ///
    /// Panics when no such adjacent pair exists: a site that has moved must be
    /// re-aimed, never silently passed.
    fn launder_gate_leg_after(&self, recipe: &str, above: &str) -> (String, String) {
        let text = self.justfile_text();
        let (mutated, cmd, leg) =
            insert_above_gate_leg_after(&text, recipe, above, "echo laundered");
        std::fs::write(self.root.join("Justfile"), mutated).expect("sandbox Justfile rewritten");
        (cmd, leg)
    }
}

/// The RAW body of every recipe, as `name -> (first body line, one past the
/// last)`, indices into `text.lines()`.
///
/// `parse_justfile` is the wrong reader for the two instruments that
/// need this: it comment-strips and whitespace-squashes every line before
/// storing it, and the G16 invariant is about ADJACENCY — a blank line, a
/// comment, or an `echo` between a command and the `gate_leg … $?` that
/// grades it. Three of those shapes are invisible after squashing. Same header
/// rule as `parse_justfile` (column 0, `name:` with a legal name, no `:=`), so
/// the two readers agree on what a recipe is.
fn raw_recipe_bodies(text: &str) -> BTreeMap<String, (usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = BTreeMap::new();
    let mut i = 0usize;
    while i < lines.len() {
        let raw = lines[i];
        let t = raw.trim();
        let header = indent_of(raw) == 0
            && !t.is_empty()
            && !t.starts_with('#')
            && !t.starts_with('[')
            && t.contains(':')
            && !t.contains(":=");
        if header {
            let name = t
                .split_once(':')
                .map(|(h, _)| h.split_whitespace().next().unwrap_or(""))
                .unwrap_or("");
            let legal = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if legal {
                let start = i + 1;
                let mut j = start;
                while j < lines.len() && (lines[j].trim().is_empty() || indent_of(lines[j]) > 0) {
                    j += 1;
                }
                out.insert(name.to_string(), (start, j));
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `gate_leg <leg> $?` and nothing else on the line — the shape whose `$?`
/// binds to the line above. `gate_leg <leg> 2` with a literal code (the
/// `[ -f … ] || gate_leg pkg:present 2` shape Q-G12 allows) has no such
/// binding and is not this instrument's subject.
fn is_gate_leg_status_line(t: &str) -> bool {
    let mut w = t.split_whitespace();
    w.next() == Some("gate_leg")
        && w.next().is_some()
        && w.next() == Some("$?")
        && w.next().is_none()
}

/// Why `line` cannot be the command a `gate_leg … $?` under it grades, or
/// `None` when it can. The rule is `CANNOT_ABORT_HEAD`'s, read in the other
/// direction: a line whose head cannot fail cannot carry a failure either, so
/// an `$?` read after it is the echo's, never the leg's.
///
/// **This is stricter than the contract's wording on purpose.** §4m G16(a)
/// says "a non-blank, non-comment command line" — and `echo laundered` IS
/// one. A guard written to that sentence is green on the exact mutant the
/// row exists for; measured before this was written, not assumed.
fn cannot_carry_a_failure(line: &str) -> Option<&'static str> {
    let t = line.trim();
    if t.is_empty() {
        return Some("blank");
    }
    if t.starts_with('#') {
        return Some("a comment");
    }
    let head = t
        .trim_start_matches('(')
        .split_whitespace()
        .next()
        .unwrap_or("");
    // The helpers in `scripts/gate-aggregate.sh` that can NOT carry a leg's
    // failure, BY NAME rather than by prefix (adjudication, the
    // `gate_present` seam): `gate_begin` only sets variables, `gate_leg`
    // returns 0 on both branches (it records, it never fails), and `gate_end`
    // returns the AGGREGATE, never one leg's code. `gate_present <path>` is
    // NOT here on purpose — it returns 2 when the path is absent, and that 2
    // is exactly what the `gate_leg <pkg>:present $?` under it grades. The
    // first form of this check excluded every `gate_*` head, and on the join
    // it named the three `:present` sites as launderers — a helper whose
    // return code IS the leg's answer is a command like any other here.
    if ["gate_begin", "gate_leg", "gate_end"].contains(&head) {
        return Some("a gate_ bookkeeping helper (begin/leg/end), not a command that can fail");
    }
    if CANNOT_ABORT_HEAD.contains(&head) {
        return Some(
            "a line whose head cannot fail (`CANNOT_ABORT_HEAD`), so its `$?` is never the leg's",
        );
    }
    None
}

/// Every `gate_leg <leg> $?` site in an aggregating recipe (one that calls
/// `gate_begin`) whose line DIRECTLY above cannot carry the failure the leg
/// names. Empty on a well-formed `Justfile`; the static half of §4m G16.
fn gate_leg_sites_not_grading_the_line_above(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut bad = Vec::new();
    for (name, (s, e)) in raw_recipe_bodies(text) {
        let body = &lines[s..e];
        if !body
            .iter()
            .any(|l| l.trim_start().starts_with("gate_begin"))
        {
            continue;
        }
        for (k, l) in body.iter().enumerate() {
            let t = l.trim();
            if !is_gate_leg_status_line(t) {
                continue;
            }
            let above = if k == 0 { "" } else { body[k - 1] };
            if let Some(why) = cannot_carry_a_failure(above) {
                bad.push(format!(
                    "{name}: line {} `{t}` — the line above it is `{}` ({why})",
                    s + k + 1,
                    above.trim()
                ));
            }
        }
    }
    bad
}

/// Insert `inserted` between the first non-comment body line of `recipe`
/// containing `above` and the `gate_leg <leg> $?` directly under it. Returns
/// (the mutated text, the command line, the `gate_leg` line). One function
/// for both G16 instruments: the static guard's anti-vacuity check and the
/// driven falsifier mutate the SAME way, so they cannot drift apart.
fn insert_above_gate_leg_after(
    text: &str,
    recipe: &str,
    above: &str,
    inserted: &str,
) -> (String, String, String) {
    let lines: Vec<&str> = text.lines().collect();
    let bodies = raw_recipe_bodies(text);
    let (s, e) = *bodies.get(recipe).unwrap_or_else(|| {
        panic!(
            "no recipe `{recipe}` in the Justfile; recipes read: {:?}",
            bodies.keys().collect::<Vec<_>>()
        )
    });
    let site = (s..e.saturating_sub(1))
        .find(|&i| {
            let t = lines[i].trim();
            !t.starts_with('#') && t.contains(above) && is_gate_leg_status_line(lines[i + 1].trim())
        })
        .unwrap_or_else(|| {
            panic!(
                "in `{recipe}`, no non-comment body line containing `{above}` is directly \
                 followed by a `gate_leg <leg> $?` line. The site this test aims at has moved; \
                 re-aim it rather than letting it pass on a site that no longer exists."
            )
        });
    let gate = lines[site + 1];
    let indent = &gate[..indent_of(gate)];
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    out.insert(site + 1, format!("{indent}{inserted}"));
    let mut joined = out.join("\n");
    joined.push('\n');
    (
        joined,
        lines[site].trim().to_string(),
        gate.trim().to_string(),
    )
}

/// The lines of a run that CLAIM every leg ran — the sentence G12 forbids
/// over a package the recipe did not grade.
fn claims_every_leg_ran(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| {
            let low = l.to_lowercase();
            low.contains("all of them ran") || low.contains("every leg ran")
        })
        .map(|l| l.trim().to_string())
        .collect()
}

/// §4m G12 — `flutter-ci` never prints "all of them ran" over a package it did
/// not grade.
///
/// The sandbox is cut from THIS tree, which has all three packages, so the
/// missing-package state is reached by removing one package's directory after
/// the scaffold — the state a tree that lacks it is actually in (`[ -f
/// pubspec.yaml ]` false AND `cd` failing), not only the file the guard tests.
/// Removing the file alone would make a recipe that DROPPED its guards report
/// green-and-true (the stubbed tools do not read the pubspec), and this test
/// would then red a correct recipe for a wrong reason.
///
/// Both dispositions Q-G12 allows pass, and the message says which was seen:
///   A. RED, with a failure-marked row naming the package (COULD-NOT-RUN or
///      FAILED — `legs_named_as_failed`, the file's one definition);
///   B. a visible SKIPPED-class row naming the package AND no closing line
///      claiming every leg ran.
///
/// IT-10 — what refuses this today? Twelve of `flutter-ci`'s thirteen
/// `gate_leg` calls sit inside `if [ -f sdk/<pkg>/pubspec.yaml ]`; a false
/// condition skips the legs silently, `_gate_count` never sees them, and
/// `gate_end` prints "(N leg(s), all of them ran)" — exit 0. What would make it
/// pass for the wrong reason: a recipe that never graded `zec_wallet_ui` at all
/// (caught by the `FLUTTER_CI_PACKAGE_FLOOR` check and by the footprint
/// precondition — removing the package must SHRINK the tool footprint, or the
/// package was not being graded and "did not grade" is vacuous), or a baseline
/// that was already red (asserted green first).
#[test]
fn just_flutter_ci_does_not_report_green_over_a_package_it_did_not_grade() {
    let Some(d) = Driven::new("g12") else {
        return;
    };
    let recipes = parse_justfile(&d.justfile_text());
    let ci = recipes
        .get("flutter-ci")
        .expect("the sandbox Justfile has a `flutter-ci` recipe (§4m row 7 is about it)");
    let mut package_dirs: Vec<String> = Vec::new();
    for line in &ci.lines {
        for dir in cd_targets(line) {
            if !package_dirs.contains(&dir) {
                package_dirs.push(dir);
            }
        }
    }
    let leaf = |d: &str| d.rsplit('/').next().unwrap_or(d).to_string();
    for want in FLUTTER_CI_PACKAGE_FLOOR {
        assert!(
            package_dirs.iter().any(|p| leaf(p) == *want),
            "`flutter-ci` no longer `cd`s into `{want}` (dirs read: {package_dirs:?}); a package \
             the recipe never enters cannot be 'not graded', so this test would pass on nothing."
        );
    }
    let package = "zec_wallet_ui";
    let package_dir = package_dirs
        .iter()
        .find(|p| leaf(p) == package)
        .cloned()
        .expect("zec_wallet_ui is in the floor, so it is in the dirs");

    // The baseline half: the unmodified sandbox stays GREEN.
    let base = d.run("flutter-ci", &[]);
    assert_eq!(
        base.code,
        Some(0),
        "the GREEN baseline of `just flutter-ci` did not pass in the sandbox, so nothing below \
         it is evidence.\nTools: {:?}\nOutput:\n{}",
        base.tools,
        base.out
    );

    // The tree that lacks the package.
    d.remove_scaffolded(&package_dir);
    let r = d.run("flutter-ci", &[]);
    assert!(
        r.tools.len() < base.tools.len(),
        "removing `{package_dir}` from the sandbox changed nothing: {} tool invocations before, \
         {} after. The recipe was not grading that package in the first place, so 'did not \
         grade' is vacuous here and this test must be re-aimed.\nAfter:\n{}",
        base.tools.len(),
        r.tools.len(),
        r.tools.join("\n")
    );

    let red = r.code.is_some_and(|c| c != 0);
    let named_failed = legs_named_as_failed(&r.out, &[package.to_string()]);
    let claims = claims_every_leg_ran(&r.out);
    let skip_rows: Vec<String> = r
        .out
        .lines()
        .map(str::trim)
        .filter(|t| !t.starts_with('#') && !t.starts_with(STUB_TAG) && names_token(t, package))
        .filter(|t| {
            let low = t.to_lowercase();
            [
                "skip",
                "not-run",
                "not run",
                "could-not-run",
                "absent",
                "missing",
            ]
            .iter()
            .any(|w| low.contains(w))
        })
        .map(str::to_string)
        .collect();
    let shape_a = red && !named_failed.is_empty();
    let shape_b = claims.is_empty() && !skip_rows.is_empty();
    assert!(
        shape_a || shape_b,
        "`just flutter-ci` REPORTED OVER A PACKAGE IT DID NOT GRADE.\n\
         `{package_dir}` was removed from the sandbox; {} of {} tool invocations ran (the \
         package's legs did not), and the recipe exited {:?}.\n\
         Neither disposition §4m Q-G12 allows was seen:\n\
           A. RED with a failure-marked row naming `{package}` — red: {red}, rows naming it: \
         {named_failed:?}\n\
           B. a visible SKIPPED-class row naming `{package}` and NO closing line claiming every \
         leg ran — skip rows: {skip_rows:?}, claims: {claims:?}\n\
         §4m row 7: twelve of thirteen `gate_leg` calls sit inside `[ -f sdk/<pkg>/pubspec.yaml \
         ]`, a false condition skips them silently, and `gate_end` prints \"all of them ran\". \
         P4: this is latent on `wallet-track` (all three packages exist) and live on any tree \
         missing one — main pre-merge, a cherry-pick, an extraction.\nOutput:\n{}",
        r.tools.len(),
        base.tools.len(),
        r.code,
        r.out
    );
    eprintln!(
        "G12 disposition seen: {} (exit {:?}; failure rows naming {package}: {named_failed:?}; \
         skip rows: {skip_rows:?}; every-leg-ran claims: {claims:?})",
        if shape_a {
            "A — RED with a failure-marked row naming the package"
        } else {
            "B — a visible skipped row, and no claim that every leg ran"
        },
        r.code
    );
}

/// §4m G14 — `sdk-ci` runs its test leg when lint is red, and names both.
///
/// `ci.yml:78` runs `just sdk-ci`. Driven: `cargo clippy` forced red, and the
/// `cargo test --workspace --no-fail-fast` invocation must still appear in the
/// tool log while the output names `sdk-lint`'s failure and the recipe is red.
///
/// IT-10 — what refuses this today? `sdk-ci: sdk-lint` is a `just`
/// PREREQUISITE chain: a red prerequisite aborts the recipe before its first
/// body line, so the workspace tests never run — the third sibling of the
/// class §4l fixed for `sdk-gate` and `sdk-lint`. Wrong-reason passes guarded
/// against: a recipe that dropped `sdk-lint` entirely (the green run must
/// invoke clippy, or forcing it red grades nothing) and one that runs the
/// tests but launders the lint failure (exit must be non-zero, and forcing the
/// test leg red must be red too — both directions, as §4l G2 asks).
#[test]
fn just_sdk_ci_runs_the_workspace_tests_when_lint_is_red() {
    let Some(d) = Driven::new("g14") else {
        return;
    };
    let base = d.run("sdk-ci", &[]);
    assert_eq!(
        base.code,
        Some(0),
        "the GREEN baseline of `just sdk-ci` did not pass in the sandbox.\nTools: {:?}\nOutput:\n{}",
        base.tools,
        base.out
    );
    let test_line = base
        .tools
        .iter()
        .find(|t| t.starts_with("cargo test") && t.contains("--workspace"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "`just sdk-ci`'s green run made no `cargo test … --workspace …` invocation, so \
                 the recipe no longer runs the workspace tests at all and this test must be \
                 re-aimed.\nTools: {:?}",
                base.tools
            )
        });
    assert!(
        test_line.contains("--no-fail-fast"),
        "`sdk-ci`'s workspace line `{test_line}` lost `--no-fail-fast` — §4l-run's class, one \
         invocation naming every target and cargo stopping at the first red binary."
    );
    assert!(
        base.tools.iter().any(|t| t.starts_with("cargo clippy")),
        "`just sdk-ci`'s green run never invoked clippy, so `sdk-lint` is no longer part of it \
         and forcing clippy red would grade nothing.\nTools: {:?}",
        base.tools
    );

    let r = d.run("sdk-ci", &["cargo clippy"]);
    assert!(
        r.code.is_some_and(|c| c != 0),
        "`just sdk-ci` exited {:?} with clippy forced red — a laundered lint failure.\nOutput:\n{}",
        r.code,
        r.out
    );
    assert!(
        r.ran(&test_line),
        "A RED LINT STARVED THE WORKSPACE TESTS IN `sdk-ci`.\n\
         With `cargo clippy` forced red, `{test_line}` never ran. `ci.yml:78` runs `just \
         sdk-ci`, so a fmt or clippy regression hides every test result in that lane — \
         `custody_policy` and `swap_kill_door` among them. §4m row 9: `sdk-ci: sdk-lint` is a \
         prerequisite chain, the third sibling of the class §4l fixed for `sdk-gate` and \
         `sdk-lint`.\nWhat ran:\n{}\nOutput:\n{}",
        r.tools
            .iter()
            .map(|t| format!("  {t}"))
            .collect::<Vec<_>>()
            .join("\n"),
        r.out
    );
    let named = legs_named_as_failed(&r.out, &["sdk-lint".to_string()]);
    assert!(
        !named.is_empty(),
        "`just sdk-ci` went red with clippy forced but no failure-marked line names `sdk-lint`; \
         a reader of the CI log cannot tell which leg failed.\nOutput:\n{}",
        r.out
    );
    // The other direction: a red test leg is not laundered by a green lint.
    let r2 = d.run("sdk-ci", &[&test_line]);
    assert!(
        r2.code.is_some_and(|c| c != 0),
        "`just sdk-ci` exited {:?} with the workspace tests forced red.\nOutput:\n{}",
        r2.code,
        r2.out
    );
}

/// `just ci` runs every leg when one is red, and names the red one — the
/// 2026-09-20 review's `just ci` finding (remediation plan §2a; had
/// found it the session before, so it is two receipts). As the dependency
/// chain `ci: evals sdk-audit root-ci sdk-ci flutter-ci` it aborted at the
/// first red prerequisite, and `sdk-ci` carries the maintainer's Q3 plant, so
/// `flutter-ci` — and `wallet-bridge-verify`, its first leg — never ran under
/// `just ci` for as long as the plant has stood. G14's shape, minus the green
/// baseline: `ci` CANNOT be green in this sandbox, because `root-ci`'s
/// `dep-boundary-check` reads the Relim workspace, which the sandbox scaffolds
/// empty, and refuses (measured "Recipe `dep-boundary-check` failed with
/// exit code 1"). That is itself an earlier leg going red, so the unforced run
/// is the first specimen rather than a baseline: it must have invoked
/// `flutter test` and `cargo clippy` at all (or forcing grades nothing) and
/// must have exited non-zero (no laundering). Then with clippy forced red —
/// `root-ci`'s lint leg, the earliest cargo leg after `evals` — `flutter test`
/// must still have run, the recipe must be red, and a failure-marked line must
/// name `root-ci`.
///
/// IT-10 — what refused this before the repair? The dependency chain: driven
/// against it, `just` stops at the first red prerequisite and no `flutter`
/// line reaches the tool log — the exact silence the resume recorded as "zero
/// occurrences of flutter-ci in a complete `just ci` log". Watched RED against
/// the old `ci: evals sdk-audit root-ci sdk-ci flutter-ci` line before this
/// test was believed.
#[test]
fn just_ci_runs_flutter_ci_when_an_earlier_leg_is_red() {
    let Some(d) = Driven::new("ci-legs") else {
        return;
    };
    let base = d.run("ci", &[]);
    let flutter_test = base
        .tools
        .iter()
        .find(|t| t.starts_with("flutter test"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "`just ci`'s unforced run made no `flutter test` invocation. Either `flutter-ci` \
                 is no longer part of it, or an earlier leg's red stopped the recipe before the \
                 Dart legs — the dependency-chain shape this test exists to refuse.\nExit: {:?}\n\
                 Tools: {:?}\nOutput:\n{}",
                base.code, base.tools, base.out
            )
        });
    assert!(
        base.tools.iter().any(|t| t.starts_with("cargo clippy")),
        "`just ci`'s unforced run never invoked clippy, so `root-ci`'s lint leg is no longer \
         part of it and forcing clippy red would grade nothing.\nTools: {:?}",
        base.tools
    );
    assert!(
        base.code.is_some_and(|c| c != 0),
        "`just ci` exited {:?} in a sandbox where `dep-boundary-check` cannot pass — a red leg \
         laundered into a green gate (if that leg now passes here, re-aim this assertion at \
         another leg the sandbox reds, and say which).\nOutput:\n{}",
        base.code,
        base.out
    );

    let r = d.run("ci", &["cargo clippy"]);
    assert!(
        r.code.is_some_and(|c| c != 0),
        "`just ci` exited {:?} with clippy forced red — a laundered lint failure.\nOutput:\n{}",
        r.code,
        r.out
    );
    assert!(
        r.ran(&flutter_test),
        "A RED CARGO LEG STARVED `flutter-ci` IN `just ci`.\n\
         With `cargo clippy` forced red, `{flutter_test}` never ran — the dependency-chain \
         shape the 2026-09-20 review and S291 both found: `just` aborts `ci: … root-ci sdk-ci \
         flutter-ci` at the first red prerequisite, and `sdk-ci` is red by ruling, so the \
         Dart legs and `wallet-bridge-verify` never execute under `just ci`.\nWhat ran:\n{}\n\
         Output:\n{}",
        r.tools
            .iter()
            .map(|t| format!("  {t}"))
            .collect::<Vec<_>>()
            .join("\n"),
        r.out
    );
    let named = legs_named_as_failed(&r.out, &["root-ci".to_string()]);
    assert!(
        !named.is_empty(),
        "`just ci` went red with clippy forced but no failure-marked line names `root-ci`; a \
         reader of the log cannot tell which leg failed.\nOutput:\n{}",
        r.out
    );
}

/// §4m G13's named assertion: on THIS tree the driven battery RAN. Every
/// `just_*` test above begins `let Some(d) = Driven::new(..) else { return; }`,
/// and a `None` there is a green test that spawned nothing. This is the one
/// test that turns that `None` into a red — and the probe the driver below
/// re-spawns in a subprocess against temp copies, so each of the five states
/// is read through the real `P0_7_REPO_ROOT` / `P0_7_SDK_ROOT` path rather
/// than through an in-process env write that would race the other tests.
#[test]
fn repo_root_is_some_so_the_driven_battery_ran() {
    let root = repo_root();
    assert!(
        root.is_some(),
        "REPO_ROOT_NONE: `repo_root()` returned None, so every `just_*` test in this binary \
         returned green without spawning `just`. That is the honest answer only in an EXTRACTED \
         SDK repository, and this tree is not one."
    );
    let root = root.expect("checked above");
    for f in ["Justfile", "lefthook.yml"] {
        assert!(
            root.join(f).is_file(),
            "repo_root() = {} but {f} is not there — the driven battery would copy nothing",
            root.display()
        );
    }
}

/// What the probe above should say when re-spawned against one temp state.
enum ProbeExpect {
    /// The monorepo, both carriers present: `Some`, the probe passes.
    Ran,
    /// A monorepo missing a carrier: a harness-fault panic NAMING the file.
    HarnessFault(&'static str),
    /// An extracted SDK: `None`, so the probe's own REPO_ROOT_NONE fires.
    Skipped,
}

/// §4m G13 — the driven battery cannot be disarmed by deleting one file.
///
/// Five states, each a temp copy driven through `P0_7_REPO_ROOT` (the carrier
/// dir) and `P0_7_SDK_ROOT` (where the extraction marker lives — inside
/// `sdk/`, because the spec's extraction is `git mv sdk/` to a new repo and
/// after it `CARGO_MANIFEST_DIR/../..` may be OUTSIDE the repository):
///
///   control        both carriers, no marker      → Ran
///   missing-Justfile  lefthook.yml only          → HarnessFault("Justfile")
///   missing-lefthook  Justfile only              → HarnessFault("lefthook.yml")
///   extracted      no carriers, marker present   → Skipped (the one honest None)
///   contradiction  both carriers AND the marker  → HarnessFault(marker)  [planted]
///
/// The last is the case the contract does not name (IT-1 +A): "None only on
/// the positive marker", implemented literally, is disarmed by ADDING one file
/// to the monorepo — the row-8 defect with the sign flipped. Predicted miss: a
/// repair that checks the marker and stops reading.
///
/// IT-10 — what refuses this today? Nothing: `repo_root()` returns `None`
/// unless BOTH files exist, the override assert accepts EITHER, and the four
/// non-control states above all come back as a quiet skip or the wrong panic.
#[test]
fn the_driven_battery_refuses_to_skip_on_a_monorepo_missing_a_carrier() {
    let Some(repo) = repo_root() else {
        return;
    };
    let exe = std::env::current_exe().expect("this test binary's own path");
    let base = std::env::temp_dir().join(format!("p0-7-g13-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let cases: [(&str, &[&str], bool, ProbeExpect); 5] = [
        (
            "control",
            &["Justfile", "lefthook.yml"],
            false,
            ProbeExpect::Ran,
        ),
        (
            "missing-Justfile",
            &["lefthook.yml"],
            false,
            ProbeExpect::HarnessFault("Justfile"),
        ),
        (
            "missing-lefthook",
            &["Justfile"],
            false,
            ProbeExpect::HarnessFault("lefthook.yml"),
        ),
        ("extracted", &[], true, ProbeExpect::Skipped),
        (
            "contradiction",
            &["Justfile", "lefthook.yml"],
            true,
            ProbeExpect::HarnessFault(EXTRACTION_MARKER),
        ),
    ];
    for (name, carriers, marker, expect) in cases {
        let repo_copy = base.join(name).join("repo");
        let sdk_copy = base.join(name).join("sdk");
        std::fs::create_dir_all(&repo_copy).expect("temp repo copy");
        std::fs::create_dir_all(&sdk_copy).expect("temp sdk copy");
        for f in carriers {
            std::fs::copy(repo.join(f), repo_copy.join(f))
                .unwrap_or_else(|e| panic!("could not copy {f} into the {name} copy: {e}"));
        }
        // `sdk_root()`'s own override assert wants a manifest; presence is all it reads.
        std::fs::write(sdk_copy.join("Cargo.toml"), "").expect("temp sdk manifest");
        if marker {
            std::fs::write(sdk_copy.join(EXTRACTION_MARKER), "").expect("temp marker");
        }
        let out = Command::new(&exe)
            .args([
                "--exact",
                "repo_root_is_some_so_the_driven_battery_ran",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("P0_7_REPO_ROOT", &repo_copy)
            .env("P0_7_SDK_ROOT", &sdk_copy)
            .output()
            .unwrap_or_else(|e| {
                panic!("could not re-spawn this test binary {}: {e}", exe.display())
            });
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        let code = out.status.code();
        let fault = text.contains("HARNESS FAULT");
        let none = text.contains("REPO_ROOT_NONE");
        let ok = match expect {
            ProbeExpect::Ran => code == Some(0),
            ProbeExpect::HarnessFault(file) => {
                code != Some(0) && fault && !none && text.contains(file)
            }
            ProbeExpect::Skipped => code != Some(0) && none && !fault,
        };
        assert!(
            ok,
            "G13 state `{name}` (carriers {carriers:?}, marker {marker}): the probe exited \
             {code:?}, harness-fault named: {fault}, quiet None: {none}.\n\
             Expected: {}.\n\
             §4m row 8: a monorepo that has LOST a carrier must be a harness fault naming the \
             file, never a quiet `None` — a `None` here is every driven test returning green \
             while spawning nothing. `None` is honest only on the positive extraction marker \
             `{EXTRACTION_MARKER}` under `sdk/`, and a marker beside both carriers is a \
             contradiction, not a skip.\nProbe output:\n{text}",
            match expect {
                ProbeExpect::Ran => "Some — the probe passes (exit 0)".to_string(),
                ProbeExpect::HarnessFault(f) =>
                    format!("a HARNESS FAULT panic naming `{f}` (non-zero exit, no REPO_ROOT_NONE)"),
                ProbeExpect::Skipped =>
                    "None — the probe's own REPO_ROOT_NONE fires (no HARNESS FAULT)".to_string(),
            }
        );
    }
    let _ = std::fs::remove_dir_all(&base);
}

/// §4m G15 — the sandbox never writes outside its root, on either loop.
///
/// Two mechanisms, driven separately (IT-10):
///   * the BRACES — `sandbox_path_word` refuses `..`, `/`, `~` and `$` words,
///     so a hostile `cd` in the `Justfile` never reaches either loop;
///   * the BELT — each loop's own assert, driven with a word that BYPASSES the
///     braces (as if `sandbox_path_word` were mutated to accept it), with the
///     source-side directory/file made real so the loops' `is_dir()` /
///     `is_file()` gates let the write proceed. Then the assert is the only
///     thing left between the loop and a write outside the sandbox.
///
/// The planted case (IT-1 +A): `..` traversal against the FILE loop's existing
/// assert. `Path::starts_with` is component-wise and un-normalised, so
/// `root/../x` "starts with" `root` — the belt the contract tells the directory
/// loop to copy has a hole exactly where traversal goes through it. Predicted
/// miss: a repair that copies the assert verbatim and calls the class closed.
#[test]
fn the_sandbox_scaffold_never_writes_outside_its_root() {
    let pid = std::process::id();
    let base = std::env::temp_dir().join(format!("p0-7-g15-{pid}"));
    let _ = std::fs::remove_dir_all(&base);
    // Different depths on purpose: `repo/../x` and `root/../x` must be
    // different places, or a write outside `root` lands on the fixture.
    let repo = base.join("r").join("repo");
    let root = base.join("s").join("t").join("root");
    std::fs::create_dir_all(repo.join("sdk")).expect("fixture repo");
    std::fs::create_dir_all(&root).expect("fixture root");
    let escape = format!("p0-7-escape-{pid}");

    // (1) The braces: hostile `cd` words through the scan.
    let hostile = [
        format!("cd ../../{escape}"),
        format!("cd /tmp/{escape}"),
        format!("cd $HOME/{escape}"),
        format!("cd ~/{escape}"),
        format!("cd \"sdk/../../{escape}\""),
    ];
    let mut text = String::from("hostile:\n    #!/usr/bin/env bash\n");
    for w in &hostile {
        text.push_str(&format!("    ({w} && true)\n"));
    }
    text.push_str("    (cd sdk && cargo test)\n");
    let (dirs, files) = justfile_referenced_paths(&text);
    for w in &hostile {
        assert!(
            !dirs.iter().any(|d| d.contains(&escape)),
            "the scan let `{w}` through as a scaffold target: {dirs:?}. `sandbox_path_word` is \
             the mechanism that refuses it, and it did not."
        );
    }
    assert!(
        dirs.contains("sdk"),
        "the scan read no `sdk` from `(cd sdk && cargo test)` — it refuses everything, which \
         would make the hostile-word assertions above pass on nothing. Dirs: {dirs:?}"
    );
    scaffold_dirs(&repo, &root, &dirs);
    scaffold_files(&repo, &root, &files);
    assert!(
        root.join("sdk").is_dir(),
        "the honest word was not scaffolded"
    );
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base.clone());
    for p in [
        base.join("s").join(&escape),
        base.join(&escape),
        std::env::temp_dir().join(&escape),
        PathBuf::from("/tmp").join(&escape),
        home.join(&escape),
    ] {
        assert!(
            !p.exists(),
            "the scaffold wrote {} — outside the sandbox",
            p.display()
        );
    }

    // (2) The belt, directory loop: a traversal word that bypassed the braces,
    // with `repo/../<escape>` real so `is_dir()` is true.
    let word = format!("../{escape}");
    std::fs::create_dir_all(repo.join(&word)).expect("fixture sibling dir");
    let outside_dir = root.join(&word);
    let set = BTreeSet::from([word.clone()]);
    let dir_panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scaffold_dirs(&repo, &root, &set)
    }))
    .is_err();
    let dir_escaped = outside_dir.exists();

    // (3) The belt, file loop: the same word, with the file real.
    let fword = format!("../{escape}/planted.txt");
    std::fs::write(repo.join(&fword), "").expect("fixture sibling file");
    let outside_file = root.join(&fword);
    let fset = BTreeSet::from([fword.clone()]);
    let file_panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scaffold_files(&repo, &root, &fset)
    }))
    .is_err();
    let file_escaped = outside_file.exists();
    let _ = std::fs::remove_dir_all(&base);

    // Both observed before either is judged, so one run reports both loops.
    assert!(
        dir_panicked && !dir_escaped && file_panicked && !file_escaped,
        "A SCAFFOLD LOOP WROTE OUTSIDE THE SANDBOX given a traversal word that bypassed \
         `sandbox_path_word`.\n\
         DIRECTORY LOOP, word `{word}`: `scaffold_dirs` {} — {} exists: {dir_escaped}\n\
         FILE LOOP, word `{fword}`: `scaffold_files` {} — {} exists: {file_escaped}\n\
         §4m row 10: the belt behind the braces must be on BOTH loops. And the planted case: \
         the S261 belt was `dst.starts_with(root)` on an un-normalised path, and `root/../x` \
         starts with `root` component-wise — so `..` went straight through the file loop's \
         assert, which is the one the contract tells the directory loop to copy.",
        if dir_panicked {
            "panicked"
        } else {
            "returned normally"
        },
        outside_dir.display(),
        if file_panicked {
            "panicked"
        } else {
            "returned normally"
        },
        outside_file.display(),
    );
}

/// §4m G16(a) — STATIC: in every aggregating recipe, each `gate_leg <leg> $?`
/// is directly under a line that can carry the failure the leg names.
///
/// GREEN on a well-formed `Justfile`, and that is the expected state, not a
/// vacuous one: the instrument's mutant is an `echo` between a command and
/// its `gate_leg`, and this test mutates the real text three ways (an `echo`,
/// a comment, a blank) and asserts the scanner names the site each time. The
/// aggregating set is READ (every recipe that calls `gate_begin`), so `sdk-ci`
/// joins it the day it aggregates; the five §4m names are a floor.
#[test]
fn every_gate_leg_grades_the_line_directly_above_it() {
    let Some(repo) = repo_root() else {
        return;
    };
    let text = std::fs::read_to_string(repo.join("Justfile")).expect("Justfile readable");
    let lines: Vec<&str> = text.lines().collect();
    let bodies = raw_recipe_bodies(&text);
    let aggregating: Vec<&String> = bodies
        .iter()
        .filter(|(_, span)| {
            let (s, e) = **span;
            lines[s..e]
                .iter()
                .any(|l| l.trim_start().starts_with("gate_begin"))
        })
        .map(|(n, _)| n)
        .collect();
    for want in [
        "sdk-gate",
        "sdk-gate-core",
        "sdk-fast",
        "sdk-lint",
        "flutter-ci",
    ] {
        assert!(
            aggregating.iter().any(|n| n.as_str() == want),
            "`{want}` no longer calls `gate_begin`, so this scan does not grade it. Aggregating \
             recipes read: {aggregating:?}. §4m G16(a) names it; if it stopped aggregating, that \
             is the finding, not a reason to drop it here."
        );
    }
    let sites: usize = aggregating
        .iter()
        .map(|n| {
            let (s, e) = bodies[n.as_str()];
            lines[s..e]
                .iter()
                .filter(|l| is_gate_leg_status_line(l.trim()))
                .count()
        })
        .sum();
    assert!(
        sites >= 20,
        "only {sites} `gate_leg <leg> $?` site(s) across {aggregating:?} (39 at 6524db0f). A \
         scan that reads few sites grades little: either the recipes changed shape or \
         `is_gate_leg_status_line` no longer recognises them."
    );
    let bad = gate_leg_sites_not_grading_the_line_above(&text);
    assert!(
        bad.is_empty(),
        "A `gate_leg … $?` IS NOT GRADING THE LINE ABOVE IT.\n{}\n\n\
         §4m review 1: `$?` is the exit status of the LAST command, so anything between a \
         command and the `gate_leg` that grades it launders the failure — driven at S261 with \
         one `echo`, `just sdk-gate` returned 0 over a real failing leg. Move the `gate_leg` \
         to the line directly under the command it names.",
        bad.iter()
            .map(|b| format!("  {b}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Anti-vacuity, in the same test: the scanner fires on its mutants. The
    // fourth — a `gate_leg` whose `$?` a second `gate_leg` then reads — keeps
    // the by-name helper exclusion in `cannot_carry_a_failure` observed, now
    // that it no longer covers every `gate_*` head.
    for mutant in [
        "echo laundered",
        "# a comment between",
        "",
        "gate_leg phantom $?",
    ] {
        let (mutated, cmd, leg) =
            insert_above_gate_leg_after(&text, "flutter-ci", "flutter test", mutant);
        let bad = gate_leg_sites_not_grading_the_line_above(&mutated);
        assert!(
            bad.iter()
                .any(|b| b.starts_with("flutter-ci") && b.contains(&leg)),
            "the scanner did NOT name `{leg}` after `{mutant:?}` was inserted between it and \
             `{cmd}` in `flutter-ci`. Sites it named: {bad:?}. A static guard that cannot see \
             its own mutant is the vacuous gate this document is about."
        );
    }
}

/// The driven G16 sites: (recipe, the body line to launder after, the tool
/// pattern that turns exactly that leg red). The pattern must match ONE
/// invocation of the recipe's green run — asserted — or a sibling leg's honest
/// red would hide the laundering. `flutter test` is invoked three times with
/// identical argv (the stub cannot see the cwd), which is why the example's
/// `dart format … lib test integration_test` is the leg used there.
const LAUNDER_SITES: &[(&str, &str, &str)] = &[
    (
        "flutter-ci",
        "lib test integration_test",
        "lib test integration_test",
    ),
    (
        "sdk-fast",
        "lib test integration_test",
        "lib test integration_test",
    ),
    // RE-AIMED at FR-5 C1 (2026-09-18): the force pattern was the bare `cargo
    // clippy`, which matched ONE invocation until C0 (`c0d0ad49`) gave
    // `sdk-lint` a second workspace to lint — `sdk/dialer-tor`, its own cargo
    // workspace (ADR-0551) — and the row went red on its own `hits == 1`
    // check, exactly as designed. `--workspace` is what distinguishes them:
    // the SDK's leg is `cargo clippy --workspace --all-targets`, the crate's
    // is `cargo clippy --all-targets`.
    //
    // THE `above` ANCHOR CARRIES THE SAME STRING, and that is load-bearing:
    // `insert_above_gate_leg_after` takes the FIRST body line containing
    // `above`, so a bare `cargo clippy` there selects the SDK leg only by
    // SOURCE ORDER — the day the dialer-tor legs move above it, the echo is
    // inserted at a leg the force pattern does not red, `refused_by_recipe`
    // is true for the wrong reason, and the row grades nothing while staying
    // green. That is the vacuity class this instrument exists to catch, so it
    // must not be reintroduced by the instrument's own table. (The earlier
    // comment here claimed the anchor had to stay the bare text "because that
    // is what appears in the recipe body"; refuted at `justfile:304`, which
    // contains `cargo clippy --workspace --all-targets` verbatim.)
    (
        "sdk-lint",
        "cargo clippy --workspace",
        "cargo clippy --workspace",
    ),
    ("sdk-gate", "just sdk-lint", "cargo clippy --workspace"),
];

/// §4m G16(b) — DRIVEN: an `echo` between a command and its `gate_leg … $?`
/// does not launder the failure, in the four recipes left ungraded.
///
/// Per site: green baseline; the force pattern hits exactly one invocation;
/// CONTROL — forced red without the echo, the recipe is red (the mutation is
/// lethal, §4j D5); then the sandbox `Justfile` is rewritten with `echo
/// laundered` above the site and the same forced red must STILL be red.
///
/// IT-10 — what refuses this today? Nothing at runtime: `$?` is the echo's,
/// `gate_leg` records 0, the recipe exits 0 — measured in
/// `sdk-gate-core` only. A recipe that turns this row green for the wrong
/// reason: one whose force pattern reds a SECOND leg (the `hits == 1` check).
#[test]
fn an_echo_between_a_command_and_its_gate_leg_does_not_launder_the_failure() {
    let mut laundered: Vec<String> = Vec::new();
    let mut uncaught: Vec<String> = Vec::new();
    for (recipe, above, force) in LAUNDER_SITES {
        let Some(d) = Driven::new(&format!("g16-{recipe}")) else {
            return;
        };
        let base = d.run(recipe, &[]);
        assert_eq!(
            base.code,
            Some(0),
            "the GREEN baseline of `just {recipe}` did not pass in the sandbox.\nTools: {:?}\n\
             Output:\n{}",
            base.tools,
            base.out
        );
        let hits = base.tools.iter().filter(|t| t.contains(force)).count();
        assert_eq!(
            hits, 1,
            "the force pattern `{force}` matches {hits} invocation(s) in `just {recipe}`'s green \
             run; it must match exactly one, so the laundered leg is the ONLY red leg. Re-aim \
             the site.\nTools: {:?}",
            base.tools
        );
        let control = d.run(recipe, &[force]);
        assert!(
            control.code.is_some_and(|c| c != 0),
            "CONTROL: forcing `{force}` red left `just {recipe}` at {:?} with NO echo inserted, \
             so the mutation is inert (§4j D5) and this site grades nothing.\nOutput:\n{}",
            control.code,
            control.out
        );
        let (cmd, leg) = d.launder_gate_leg_after(recipe, above);
        let r = d.run(recipe, &[force]);
        let refused_by_recipe = r.code.is_some_and(|c| c != 0);
        // The contract allows EITHER refusal — the recipe itself, or the
        // static guard at (a) run over the sandbox's mutated `Justfile`. At
        // runtime nothing can tell an echo's `$?` from the command's, so the
        // guard is the mechanism that actually refuses this shape; the
        // recipe's own exit code is recorded beside it so a reader sees that
        // the laundering is REAL (the mutant is not inert) and what caught it.
        let guard = gate_leg_sites_not_grading_the_line_above(&d.justfile_text());
        let refused_by_guard = guard
            .iter()
            .any(|b| b.starts_with(recipe) && b.contains(&leg));
        laundered.push(format!(
            "{recipe}: with `echo laundered` between `{cmd}` and `{leg}`, exit {:?} (without the \
             echo: {:?}) — refused by the recipe: {refused_by_recipe}; refused by the static \
             guard: {refused_by_guard}",
            r.code, control.code
        ));
        if !(refused_by_recipe || refused_by_guard) {
            uncaught.push(format!("{recipe}\nOutput:\n{}", r.out));
        }
    }
    for l in &laundered {
        eprintln!("G16(b) {l}");
    }
    assert!(
        uncaught.is_empty(),
        "LAUNDERED AND UNCAUGHT in {} recipe(s): a leg forced red, one `echo` above its \
         `gate_leg … $?`, and neither the recipe nor `every_gate_leg_grades_the_line_directly_above_it`'s \
         scanner refused it. §4m review 1: `$?` is the echo's, the leg is recorded ok, a real \
         failing leg is a green gate — caught once in `sdk-gate-core`, ungraded in these until \
         now.\nPer site:\n{}\n\nUncaught:\n{}",
        uncaught.len(),
        laundered
            .iter()
            .map(|l| format!("  {l}"))
            .collect::<Vec<_>>()
            .join("\n"),
        uncaught.join("\n\n")
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase-3 P3-8 — INC-010: the lefthook Dart lanes RAN.
//
// Contract: `docs/plan/production-readiness-phase-3.md` P3-8; the incident is
// `evals/incidents.tsv` INC-010. For months `dart-analyze`, `dart-fmt` and
// `flutter-test` read `cd app && … 2>/dev/null || true`; there is no `app/` in
// this tree, the `cd` failed, `|| true` swallowed it, and every Dart commit was
// "green in 0.02s" over a lane that invoked no tool. The remedy (`64ad0c75`)
// pointed the lanes at `sdk/` and dropped the laundering; nothing had driven it
// since, and the row stayed OWED. This is its guard.
//
// THE PREDICATE, as the incident names it: the lane RAN — not "exited 0".
// Three conjuncts, each driven against the lane's RAW `run:` block executed
// the way lefthook executes it (`sh -c`, from the repo root):
//   1. GREEN: exit 0 AND the lane invoked its tool INSIDE every package it
//      enters — the stub records `$PWD`, because a wrong directory is what the
//      incident was, so "which tool" without "where" grades nothing; and the
//      packages must cover a per-lane FLOOR (the scope `lefthook.yml` says it
//      mirrors from `sdk-fast`), so a lane that quietly drops a package cannot
//      pass by entering fewer.
//   2. A package directory removed → the lane is RED. A `cd` that fails must
//      fail the lane; `|| true` is the laundering that turned it green.
//   3. The tool itself forced red → the lane is RED. `2>/dev/null || true`
//      swallowed real failures too, not only the missing directory.
// "Runtime > 0" — the OWED row's own wording — measures the symptom; the
// invocation log with its cwd measures the cause, and is deterministic where a
// clock is not.
//
// WHAT IS RUN. `lefthook_run_block` returns the block VERBATIM (dedented), not
// `parse_lefthook`'s comment-stripped, whitespace-squashed lines: a script
// executed after squashing is not the script the hook runs, and `strip_comment`
// would delete the `#` lines a block scalar carries as SHELL comments.
// `flutter` and `dart` are rebound in the sandbox to `P3_8_STUB`; every other
// tool stays the shared P0-7 stub. No `git` runs anywhere in this section.
// ─────────────────────────────────────────────────────────────────────────────

/// The Dart lanes: the hook each lives in, the tool a forced red must hit, and
/// the packages each MUST enter — the scope `lefthook.yml`'s own comment says
/// mirrors `sdk-fast` (format on all three Dart packages, analyze on the two
/// SDK packages; the example's `flutter test` EXCLUDED on purpose because it
/// uninstalls the on-device app). A floor, not a list: a lane may enter more.
const DART_LANE_FLOOR: &[(&str, &str, &str, &[&str])] = &[
    (
        "pre-commit",
        "dart-analyze",
        "flutter analyze",
        &["sdk/zec_wallet", "sdk/zec_wallet_ui"],
    ),
    (
        "pre-commit",
        "dart-fmt",
        "dart format",
        &[
            "sdk/zec_wallet",
            "sdk/zec_wallet_ui",
            "sdk/zec_wallet/example",
        ],
    ),
    (
        "pre-push",
        "flutter-test",
        "flutter test",
        &["sdk/zec_wallet", "sdk/zec_wallet_ui"],
    ),
];

/// The prefix every line the P3-8 stub prints begins with — its own voice,
/// apart from the carrier's, for the same reason `STUB_TAG` exists.
const P3_8_STUB_TAG: &str = "p3-8-stub:";

/// A `flutter`/`dart` stand-in that records the tool, its argv AND the
/// directory it ran in (TAB-separated), and obeys `P0_7_FORCE_RED` exactly as
/// the P0-7 stub does.
const P3_8_STUB: &str = r#"#!/usr/bin/env bash
# P3-8 (INC-010) stub. The incident was a lane whose `cd` failed, so this
# records WHERE the tool ran, not only that it did.
tool="$(basename "$0")"
printf '%s %s\t%s\n' "$tool" "$*" "$PWD" >> "${P3_8_TOOL_LOG:-/dev/null}"
if [ -n "${P0_7_FORCE_RED:-}" ]; then
  while IFS= read -r p; do
    [ -n "$p" ] || continue
    case "$tool $*" in
      *"$p"*) printf 'p3-8-stub: FORCED-RED <%s %s>\n' "$tool" "$*" >&2; exit 1 ;;
    esac
  done <<< "$P0_7_FORCE_RED"
fi
printf 'p3-8-stub: ok <%s %s>\n' "$tool" "$*" >&2
exit 0
"#;

/// The RAW `run:` body of one lefthook lane: a block scalar's lines dedented to
/// their common indent (blank lines kept, `#` lines kept — they are shell), or
/// the inline value. `None` when the hook or the lane is absent, or the lane
/// carries no `run:`.
fn lefthook_run_block(text: &str, hook: &str, lane: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut in_hook = false;
    let mut in_commands = false; // the same gate `parse_lefthook` keeps
    let mut i = 0usize;
    while i < lines.len() {
        let stripped = strip_comment(lines[i]);
        let trimmed = stripped.trim();
        if !trimmed.is_empty() && indent_of(&stripped) == 0 && trimmed.ends_with(':') {
            in_hook = trimmed.trim_end_matches(':') == hook;
            in_commands = false;
            i += 1;
            continue;
        }
        if in_hook && trimmed == "commands:" {
            in_commands = true;
            i += 1;
            continue;
        }
        if !in_hook || !in_commands || trimmed != format!("{lane}:") || indent_of(&stripped) < 4 {
            i += 1;
            continue;
        }
        let lane_indent = indent_of(&stripped);
        i += 1;
        while i < lines.len() {
            let s = strip_comment(lines[i]);
            if !s.trim().is_empty() && indent_of(&s) <= lane_indent {
                return None;
            }
            let Some(v) = s.trim().strip_prefix("run:") else {
                i += 1;
                continue;
            };
            let v = v.trim();
            if !matches!(v, "|" | "|-" | ">" | ">-") {
                return Some(v.to_string());
            }
            let key_indent = indent_of(&s);
            let mut body: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() {
                let b = lines[i];
                if !b.trim().is_empty() && indent_of(b) <= key_indent {
                    break;
                }
                body.push(b);
                i += 1;
            }
            let dedent = body
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| indent_of(l))
                .min()
                .unwrap_or(0);
            return Some(
                body.iter()
                    .map(|l| {
                        if l.trim().is_empty() {
                            ""
                        } else {
                            &l[dedent..]
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        return None;
    }
    None
}

/// Two paths name the same directory: canonicalised when both exist (macOS puts
/// the temp dir behind a `/var` → `/private/var` symlink and `cd` keeps the
/// logical path), string-equal otherwise.
fn same_dir(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// One driven run of a lefthook lane's script.
struct LaneRun {
    code: Option<i32>,
    out: String,
    /// Every `flutter`/`dart` invocation as (`<tool> <args>`, cwd), in order.
    calls: Vec<(String, String)>,
}

impl Driven {
    /// Rebind `flutter` and `dart` in the sandbox to the P3-8 stub.
    fn rebind_dart_tools_to_the_p3_8_stub(&self) {
        assert!(
            P3_8_STUB.contains(P3_8_STUB_TAG),
            "the P3-8 stub no longer tags its output with `{P3_8_STUB_TAG}`"
        );
        for t in ["flutter", "dart"] {
            let p = self.bin.join(t);
            std::fs::write(&p, P3_8_STUB).expect("p3-8 stub written");
            let st = Command::new("chmod")
                .arg("+x")
                .arg(&p)
                .status()
                .expect("chmod runs");
            assert!(
                st.success(),
                "could not make the P3-8 stub {} executable",
                p.display()
            );
        }
    }

    /// Run one lefthook lane's script the way lefthook runs it: `sh -c`, from
    /// the repo root (here the sandbox root), the stubs first on the PATH.
    fn run_lefthook_lane(&self, script: &str, force: &[&str]) -> LaneRun {
        let log = self.root.join("p3-8-tools.log");
        let _ = std::fs::write(&log, "");
        let path = format!(
            "{}:{}",
            self.bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(script)
            .current_dir(&self.root)
            .env("PATH", path)
            .env("P3_8_TOOL_LOG", &log)
            .env("P0_7_TOOL_LOG", &self.log)
            .env("P0_7_FORCE_RED", force.join("\n"));
        // INC-026: a hook exports `GIT_*` to everything it runs, and this runs a
        // script taken verbatim out of `lefthook.yml` — the one file where a
        // `git` word appears next. None of today's lanes runs git; the child
        // still does not inherit a repository (the security review's row 5).
        for (k, _) in std::env::vars_os() {
            if k.to_string_lossy().starts_with("GIT_") {
                cmd.env_remove(k);
            }
        }
        let out = cmd.output().unwrap_or_else(|e| {
            panic!(
                "could not run `sh -c` in the sandbox at {}: {e}",
                self.root.display()
            )
        });
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        let calls = std::fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                l.split_once('\t')
                    .map(|(inv, cwd)| (inv.trim().to_string(), cwd.trim().to_string()))
            })
            .collect();
        LaneRun {
            code: out.status.code(),
            out: text,
            calls,
        }
    }
}

/// INC-010 — DRIVEN: each lefthook Dart lane invokes its tool inside every
/// package it enters, and is RED with a package directory missing or with its
/// tool failing. Contract at the head of this section.
///
/// IT-10 — what refuses each shape? (1) a lane entering too few packages: the
/// floor; (2) `|| true` back on a lane: conjuncts 2 and 3; (3) a `cd` into a
/// directory this tree lacks (the original `app`): the scaffold assertion,
/// before anything runs. **What it does NOT reach, stated (the security
/// review):** the lane's own `command -v flutter … || { echo "SKIP …"; exit 0; }`
/// arm — the stub is on the PATH for every conjunct here, so `command -v`
/// never fails in this harness, and a machine with no flutter/dart on its PATH
/// still commits Dart under a green hook. That arm is P0-2's decided three-state
/// shape (absent tool ⇒ SKIP, exit 0), not this row's; the INC-010 row names it
/// as the residual.
#[test]
fn lefthook_dart_lanes_run_their_tool_inside_each_package_and_are_red_when_one_is_missing() {
    let Some(d) = Driven::new("inc-010") else {
        return;
    };
    let repo = repo_root().expect("Driven::new found the repo root");
    d.rebind_dart_tools_to_the_p3_8_stub();
    let text = std::fs::read_to_string(d.root.join("lefthook.yml")).expect("sandbox lefthook.yml");
    for (hook, lane, tool, floor) in DART_LANE_FLOOR {
        let script = lefthook_run_block(&text, hook, lane).unwrap_or_else(|| {
            panic!(
                "`lefthook.yml` has no `{lane}` lane with a `run:` under `{hook}`. INC-010 is about \
                 that lane; if it was renamed or moved, re-aim this row rather than leave it \
                 passing over a lane that no longer exists."
            )
        });
        // The packages, read out of the directories the lane enters (comment
        // lines are prose, not `cd`s).
        let mut packages: Vec<String> = Vec::new();
        for line in script.lines().filter(|l| !l.trim_start().starts_with('#')) {
            for dir in cd_targets(line) {
                if !packages.contains(&dir) {
                    packages.push(dir);
                }
            }
        }
        for want in *floor {
            assert!(
                packages.iter().any(|p| p == want),
                "`{hook}/{lane}` enters {packages:?} and `{want}` is not among them. The lane's \
                 scope mirrors `sdk-fast` (its own comment says so); a lane that stopped entering \
                 a package grades nothing there, and a shorter list passes every assertion below."
            );
        }
        let dirs: BTreeSet<String> = packages.iter().cloned().collect();
        scaffold_dirs(&repo, &d.root, &dirs);
        for p in &packages {
            assert!(
                d.root.join(p).is_dir(),
                "`{hook}/{lane}` enters `{p}`, which the source tree does not have as a directory, \
                 so the sandbox could not scaffold it. That is INC-010's own shape (`cd app`): a \
                 lane entering a directory that does not exist."
            );
        }

        // 1. GREEN: exit 0, and the tool ran INSIDE every package.
        let green = d.run_lefthook_lane(&script, &[]);
        assert_eq!(
            green.code,
            Some(0),
            "the GREEN baseline of `{hook}/{lane}` did not pass in the sandbox, so nothing below \
             it says anything.\nCalls: {:?}\nOutput:\n{}",
            green.calls,
            green.out
        );
        for p in &packages {
            let want = d.root.join(p);
            let ran_inside = green.calls.iter().any(|(inv, cwd)| {
                same_dir(Path::new(cwd), &want)
                    && (inv.starts_with("flutter ") || inv.starts_with("dart "))
            });
            assert!(
                ran_inside,
                "INC-010: `{hook}/{lane}` exited 0 and invoked NO flutter/dart tool inside `{p}` — \
                 green over a lane that never ran there. That is the incident's exact shape \
                 (green in 0.02s, for months). Every call the lane made: {:?}\nOutput:\n{}",
                green.calls, green.out
            );
        }

        // 2. A package directory missing → RED.
        for p in &packages {
            d.remove_scaffolded(p);
            let r = d.run_lefthook_lane(&script, &[]);
            // Removing `sdk/zec_wallet` also removed `sdk/zec_wallet/example`;
            // re-scaffold every package, not only the one removed.
            scaffold_dirs(&repo, &d.root, &dirs);
            assert!(
                r.code.is_some_and(|c| c != 0),
                "INC-010: with `{p}` REMOVED, `{hook}/{lane}` exited {:?} — a `cd` that failed \
                 was laundered into a green hook. This is the incident: `cd app && … || true` \
                 with no `app/`.\nCalls: {:?}\nOutput:\n{}",
                r.code,
                r.calls,
                r.out
            );
        }

        // 3. The tool itself fails → RED.
        let hits = green
            .calls
            .iter()
            .filter(|(inv, _)| inv.starts_with(tool))
            .count();
        assert!(
            hits >= 1,
            "the force pattern `{tool}` matches no invocation of `{hook}/{lane}`'s green run \
             ({:?}), so forcing it red would be inert (§4j D5). Re-aim the row.",
            green.calls
        );
        let r = d.run_lefthook_lane(&script, &[tool]);
        assert!(
            r.code.is_some_and(|c| c != 0),
            "INC-010: `{tool}` forced red left `{hook}/{lane}` at {:?} — the lane does not \
             propagate its own tool's failure.\nOutput:\n{}",
            r.code,
            r.out
        );
        eprintln!(
            "P3-8 {hook}/{lane}: green exit 0 with {} call(s) inside {} package(s); red with each \
             package removed; red with `{tool}` forced",
            green.calls.len(),
            packages.len()
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase-3 P3-11 — INC-026's test-fn carrier.
//
// `evals/selftest_index_reader.py` is INC-026's guard: it strips every `GIT_*`
// variable from the git it runs and refuses to write until `--absolute-git-dir`
// inside its synthetic dir names THAT dir (its mutant was watched against a
// throwaway victim repo). It is a Python script, and `check.py`'s predicate can
// name only a test fn, so the incident stayed OWED. This is the carrier: it runs
// the selftest as a child — from the real checkout, with `GIT_*` stripped from
// its OWN environment too (a hook exports them to everything it runs, which is
// the incident) — and asserts the script's verdict line and exit 0. What it
// proves: the guard runs and passes under `cargo test`, so a deleted or broken
// guard is a red test with a name. What it does NOT prove: the guard's own
// discrimination — its registry row does that.
// ─────────────────────────────────────────────────────────────────────────────

/// INC-026 — the index-reader selftest runs and passes from a Rust carrier.
#[test]
fn the_index_reader_selftest_passes_from_a_rust_carrier_with_git_env_stripped() {
    if repo_root().is_none() {
        return; // post-extraction: no carriers, no evals/ (the file's own convention)
    }
    // The REAL checkout, not a `P0_7_REPO_ROOT` sandbox: the guard lives here.
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = repo.join("evals").join("selftest_index_reader.py");
    assert!(
        script.is_file(),
        "INC-026's guard is missing: {} — the incident row names it and this carrier runs it",
        script.display()
    );
    let mut cmd = Command::new("python3");
    cmd.arg(&script).current_dir(&repo);
    for (k, _) in std::env::vars_os() {
        // `vars_os`, not `vars`: a non-UTF-8 variable must not panic the loop
        // whose whole job is to leave no `GIT_*` behind.
        if k.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(k);
        }
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("could not run python3 {}: {e}", script.display()));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "INC-026's guard did not pass (exit {:?}):\n{text}",
        out.status.code()
    );
    assert!(
        text.contains("✓ index reader selftest"),
        "the guard exited 0 without its verdict line — a silent pass is the shape INC-017 \
         and P0-2 are about:\n{text}"
    );
}

/// The `exclude = [...]` array of `sdk/Cargo.toml`'s `[workspace]` table, as
/// written (one line; a multi-line array is refused so a partial read can
/// never pass for the whole list).
fn sdk_workspace_exclude(manifest: &str) -> Vec<String> {
    let mut in_workspace = false;
    for raw in manifest.lines() {
        let line = strip_comment(raw);
        let line = line.trim();
        if line.starts_with('[') {
            in_workspace = line == "[workspace]";
            continue;
        }
        if !in_workspace {
            continue;
        }
        let Some(rest) = line.strip_prefix("exclude") else {
            continue;
        };
        let rest = rest.trim_start().strip_prefix('=').unwrap_or(rest).trim();
        let (Some(open), Some(close)) = (rest.find('['), rest.rfind(']')) else {
            panic!(
                "sdk/Cargo.toml's `exclude` must be a ONE-LINE array so this guard reads it \
                 whole; found `{line}`"
            );
        };
        return rest[open + 1..close]
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    Vec::new()
}

/// **— the arch angle on FR-5 C0 (MEDIUM); ADR-0551 Decision 2.**
/// `sdk/Cargo.toml` now `exclude`s cargo workspaces that live UNDER `sdk/`
/// (`dialer-tor`; the Tor plugin's crate ahead), because arti's RustCrypto
/// line and the ZEC stack's pre-release pins cannot share one lock. Every
/// `--workspace` gate and the live-carrier guard above are blind to an
/// excluded workspace by construction — its targets are not in this
/// workspace's `cargo metadata` — so its carriers are hand-written legs, and
/// nothing above notices a THIRD excluded workspace landing with no legs at
/// all. This test is that notice. The `exclude` array is the SOLE input; each
/// entry that is a workspace root on disk (a `Cargo.toml` carrying
/// `[workspace]`) must be named, with its own `cd sdk/<dir> &&`, by a `cargo
/// test`, `cargo fmt --check`, `cargo clippy`, `cargo doc`, `cargo audit` and
/// `cargo deny check` line in the Justfile, and by a `cargo clippy` and a
/// `cargo fmt --check` line in lefthook.yml. An entry whose directory does
/// not exist yet is reported and skipped (it cannot be carried before it
/// exists); at least one entry must be graded, or the test refuses — never
/// vacuous. Post-extraction there are no carriers to read and the test
/// returns, exactly as its sibling above does.
#[test]
fn every_excluded_workspace_under_sdk_has_its_own_carriers() {
    let sdk = sdk_root();
    let Some(root) = repo_root() else {
        return;
    };
    let manifest =
        std::fs::read_to_string(sdk.join("Cargo.toml")).expect("sdk/Cargo.toml readable");
    let excluded = sdk_workspace_exclude(&manifest);
    assert!(
        !excluded.is_empty(),
        "sdk/Cargo.toml carries no `[workspace] exclude` — this guard has nothing to grade. If \
         the excluded workspaces were folded back into sdk/, delete this test with them; if \
         the array moved, teach `sdk_workspace_exclude` its new shape."
    );
    let clean = |text: &str| -> Vec<String> {
        text.lines()
            .map(|l| {
                strip_comment(l)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    };
    let just_lines =
        clean(&std::fs::read_to_string(root.join("Justfile")).expect("Justfile readable"));
    let hook_lines =
        clean(&std::fs::read_to_string(root.join("lefthook.yml")).expect("lefthook.yml readable"));

    let mut graded = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for dir in &excluded {
        let manifest_path = sdk.join(dir).join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest_path) else {
            eprintln!(
                "excluded entry `{dir}` has no manifest on disk yet ({}) — not graded; it is \
                 graded the day it lands",
                manifest_path.display()
            );
            continue;
        };
        if !text.lines().any(|l| l.trim() == "[workspace]") {
            // An excluded plain package (not a workspace root) is somebody
            // else's member; nothing here to carry.
            continue;
        }
        graded += 1;
        let prefix = format!("cd sdk/{dir} &&");
        let names = |lines: &[String], needle: &str| {
            lines
                .iter()
                .any(|l| l.contains(&prefix) && l.contains(needle))
        };
        for needle in [
            "cargo test",
            "cargo fmt --check",
            "cargo clippy",
            "cargo doc",
            "cargo audit",
            "cargo deny check",
        ] {
            if !names(&just_lines, needle) {
                missing.push(format!("Justfile: a line `{prefix} … {needle} …`"));
            }
        }
        for needle in ["cargo clippy", "cargo fmt --check"] {
            if !names(&hook_lines, needle) {
                missing.push(format!("lefthook.yml: a line `{prefix} … {needle} …`"));
            }
        }
    }
    assert!(
        graded > 0,
        "none of sdk/Cargo.toml's excluded entries {excluded:?} is a workspace root on disk — \
         nothing was graded, and a guard that grades nothing is not a guard."
    );
    assert!(
        missing.is_empty(),
        "AN EXCLUDED WORKSPACE UNDER sdk/ IS NOT CARRIED BY EVERY GATE:\n  {}\n\nEvery \
         `--workspace` gate and the live-carrier guard cannot see an excluded workspace; each \
         of its carriers is a hand-written line, and this is the only check that the line \
         exists. Add the missing leg beside its sibling (`sdk/dialer-tor` is the model: \
         `sdk-gate-core`, `sdk-audit` ×2, `sdk-lint` ×3, lefthook `rust-fmt` + a \
         `<name>-clippy` lane).",
        missing.join("\n  ")
    );
}
