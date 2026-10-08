//! Crate-level policy tests (spec §8): extraction-readiness (A1/ADR-0013),
//! the G2 enum-extensibility lint, and the gate-7 constant pins. These are
//! the tests that keep the SDK publishable — they run in `just ci` like any
//! other test and need no special tooling.

use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The FRB bridge crate (part of the extraction unit; spec §1.2).
fn bridge_dir() -> PathBuf {
    manifest_dir().join("../zec_wallet/rust")
}

/// **The `G2-WAIVER`'s premise, guarded rather than asserted in prose.**
/// `SwapOutcome` is the one `pub enum` allowed to stay exhaustive (P0-9,
/// `public_enums_non_exhaustive`'s pinned waiver set) BECAUSE the bridge mirrors
/// it exhaustively in both directions, so a new core variant is compile-loud
/// (#367). `convert.rs` carries twenty `_ => Self::Unknown` arms, the nearest
/// seventeen lines above the waived impl: one copied line would silently
/// falsify the waiver, the next variant would compile clean, and Dart would
/// render it as `Unknown` with no `#[non_exhaustive]` protecting a Rust host
/// either. This row reads BOTH `From` impl bodies and refuses a wildcard in
/// them. Mutant: add `_ => Self::Failed,` to the core→bridge impl → red naming
/// the impl.
#[test]
fn swap_outcomes_mirror_carries_no_wildcard_arm() {
    let convert = fs::read_to_string(bridge_dir().join("src/convert.rs"))
        .expect("the bridge's convert.rs is readable");
    let impls = [
        "impl From<rw::SwapOutcome> for api_swap::SwapOutcome {",
        "impl From<api_swap::SwapOutcome> for rw::SwapOutcome {",
    ];
    for header in impls {
        let start = convert
            .find(header)
            .unwrap_or_else(|| panic!("`{header}` no longer exists in convert.rs — the waiver's premise (an exhaustive mirror both ways) has no impl to rest on; strip the `// G2-WAIVER:` from SwapOutcome or restore the impl"));
        // The impl body ends at the first line that is a bare `}` at column 0.
        let body_end = convert[start..]
            .find("\n}\n")
            .map(|i| start + i)
            .expect("the impl closes");
        let body = &convert[start..body_end];
        assert!(
            !body.contains("_ =>"),
            "`{header}` carries a wildcard arm. SwapOutcome's G2 waiver rests on this \
             mirror being exhaustive in BOTH directions (#367); a `_ =>` here means the \
             next variant compiles clean and renders as Unknown — either name the variant \
             or withdraw the waiver in `public_enums_non_exhaustive`. Body:\n{body}"
        );
        assert!(
            body.matches("SwapOutcome::").count() >= 3,
            "`{header}` names fewer than three variants — the scanner is not reading the \
             impl it thinks it is. Body:\n{body}"
        );
    }
}

/// §8 architecture gate (A1/ADR-0013): the extraction unit carries no other
/// `relim-*` crate and no path/git dependency — every dep must be publicly
/// resolvable (crates.io). The sanctioned exceptions are the unit's OWN
/// internal edges (extraction-unit-INTERNAL — core, bridge, swap adapter and
/// Dart package move to a new repo together, §1.2): the bridge's path deps on
/// `zec-wallet-core` and the optional `zec-wallet-swap-near` (ADR-0525).
#[test]
fn core_and_bridge_have_no_relim_deps() {
    let manifest =
        fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("crate manifest readable");
    // `apple-secure-enclave` (FR-14 chunk 2) is the unit's OWN leaf crate — the minimal
    // `unsafe` Secure-Enclave key lifecycle, so core stays #![forbid(unsafe_code)]. It is
    // extraction-unit-INTERNAL (it moves to the new repo WITH the SDK and is published
    // alongside at ADR-0013), exactly like the bridge's path dep on `zec-wallet-core`. Its
    // name is neutral (not `relim-*`), so only the path-dep assertion needs the waiver.
    let mut inherited = scan_extraction_manifest(
        &manifest,
        "zec-wallet-core (core)",
        &["apple-secure-enclave"],
    );

    let bridge_manifest = fs::read_to_string(bridge_dir().join("Cargo.toml"))
        .expect("bridge manifest readable (sdk/zec_wallet/rust — part of the extraction unit)");
    inherited.extend(scan_extraction_manifest(
        &bridge_manifest,
        "zec_wallet (bridge)",
        // the sanctioned relim-*/path deps: the unit's own core + the optional
        // swap network adapter (both extraction-unit-internal, §1.2/ADR-0525)
        &["zec-wallet-core", "zec-wallet-swap-near"],
    ));

    // Workspace-inherited deps resolve in the ROOT manifest — verify none of
    // them is a path/git dep there (the workspace holds path deps for OTHER
    // crates; none may be ours). Post-extraction there is no workspace root;
    // the manifest becomes self-contained and this half is vacuous.
    let root = manifest_dir().join("../../Cargo.toml");
    if let Ok(root_manifest) = fs::read_to_string(&root) {
        let mut in_workspace_deps = false;
        let mut in_patch = false;
        for raw in root_manifest.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.starts_with('[') {
                in_workspace_deps = line == "[workspace.dependencies]";
                in_patch = line.starts_with("[patch");
                continue;
            }
            if line.is_empty() {
                continue;
            }
            let dep_name = line.split(['=', ' ']).next().unwrap_or("").trim();
            // a [patch] override could silently redirect a crates.io dep to a
            // path/git source while the version pin still looks clean (W2
            // arch review fold) — any patch on one of OUR deps fails the policy
            if in_patch && inherited.iter().any(|d| d == dep_name) {
                panic!(
                    "extraction policy violated: `{dep_name}` is [patch]-overridden in the \
                     workspace root — the extraction unit must resolve purely from crates.io"
                );
            }
            if in_workspace_deps && inherited.iter().any(|d| d == dep_name) {
                assert!(
                    !line.contains("path =") && !line.contains("git ="),
                    "extraction policy violated: inherited dep `{dep_name}` resolves to a \
                     path/git source in the workspace root"
                );
            }
        }
    }
}

/// One extraction-unit manifest: no `relim-*` dep, no path/git dep (publicly
/// resolvable only), except the names in `sanctioned` (which may be relim-*
/// AND a path dep — the unit's own internal edges). Returns the
/// workspace-inherited dep names for the root-manifest half of the check.
fn scan_extraction_manifest(manifest: &str, who: &str, sanctioned: &[&str]) -> Vec<String> {
    let mut section = String::new();
    let mut inherited: Vec<String> = Vec::new();
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            section = line.to_string();
            continue;
        }
        if line.is_empty() || !section.contains("dependencies") {
            continue;
        }
        let dep_name = line.split(['=', ' ']).next().unwrap_or("").trim();
        if sanctioned.contains(&dep_name) {
            continue;
        }
        assert!(
            !dep_name.starts_with("relim-"),
            "extraction policy violated ({who}): `{dep_name}` is a relim-* dependency (ADR-0013)"
        );
        assert!(
            !line.contains("path =") && !line.contains("path="),
            "extraction policy violated ({who}): `{dep_name}` is a path dependency"
        );
        assert!(
            !line.contains("git =") && !line.contains("git="),
            "extraction policy violated ({who}): `{dep_name}` is a git dependency"
        );
        if line.contains("workspace = true") || line.contains("workspace=true") {
            inherited.push(dep_name.to_string());
        }
    }
    inherited
}

/// `ironwood-nu63-support.md` §8 gate 4 — THE ONE-PREDICATE GATE.
///
/// §3.2 promises `consensus::consensus_compatibility` is the single source of
/// truth for "can this binary transact on this network", and §1 says a second
/// implementation is a bug. That promise rested on review alone until this
/// test: the post-build review found a third reader re-deriving the answer
/// with an inline `matches!`, in a module whose own doc says that cannot
/// happen.
///
/// The mechanism, mirroring the `MAP`/`POLICED` idiom its siblings use: the
/// consensus VERDICT ENUM may only be pattern-matched inside `consensus.rs`
/// (which owns the predicate and its three questions) and `consensus_stamp.rs`
/// (which serialises it). Anywhere else must ASK the type —
/// `permits_signing()`, `blocks_interpretation()`, `is_signing_capable_check()`
/// — so a new variant cannot be silently missed by one reader out of three.
///
/// Scope, stated honestly: this catches a reader that DESTRUCTURES the enum. It
/// cannot catch a second predicate written from scratch over `ServerIdentity`;
/// that one is caught by `no_second_branch_comparison` below.
#[test]
fn consensus_compatibility_is_the_only_staleness_predicate() {
    /// The two files that legitimately match on the verdict's variants.
    const OWNERS: &[&str] = &["consensus.rs", "consensus_stamp.rs"];
    let mut violations = Vec::new();
    scan(&manifest_dir().join("src"), &mut violations);

    assert!(
        violations.is_empty(),
        "these files pattern-match `ConsensusCompatibility` variants directly instead of \
         asking the type (§3.2 one-predicate rule — add a method on the enum and call it): \
         {violations:?}"
    );

    // ANTI-VACUITY: the scan must actually be looking at files. A refactor that
    // renames the module or moves the enum would otherwise leave this test
    // passing over nothing at all.
    let owners_seen = count_owner_matches(&manifest_dir().join("src"));
    assert!(
        owners_seen >= 4,
        "expected the owning modules to contain several variant matches; saw {owners_seen} — \
         the scan is not engaging with the code it is meant to police"
    );

    fn scan(dir: &Path, violations: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("src readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                scan(&path, violations);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if OWNERS.contains(&name) {
                continue;
            }
            let text = fs::read_to_string(&path).expect("source readable");
            // Test-only HELPERS may construct a verdict to stage a fixture —
            // that is not a second predicate. Exempted by NAME (`*_for_test`),
            // so adding one is a deliberate, greppable act rather than a hole.
            let mut in_test_helper = false;
            for (i, line) in text.lines().enumerate() {
                let code = line.trim();
                if let Some(rest) = code
                    .strip_prefix("pub(crate) fn ")
                    .or_else(|| code.strip_prefix("fn "))
                {
                    in_test_helper = rest.split('(').next().unwrap_or("").ends_with("_for_test");
                }
                if in_test_helper {
                    continue;
                }
                // Stop at the test MODULE — NOT at any `#[cfg(test)]`, which
                // also marks test-only HELPERS that sit mid-file. Breaking on
                // the attribute made this gate vacuous over `wallet.rs`, whose
                // `seed_*_for_test` helpers precede the code being policed:
                // a planted violation passed. (Caught by mutating the gate,
                // not by reading it.)
                if code.starts_with("mod tests {")
                    || code.starts_with("pub(crate) mod testing {")
                    || code.starts_with("mod testing {")
                {
                    break;
                }
                // Prose is not a second implementation: a doc comment naming a
                // variant is documentation, and the rule is about code.
                if code.starts_with("//") || code.starts_with("*") {
                    continue;
                }
                if line.contains("ConsensusCompatibility::") {
                    violations.push(format!("{name}:{}", i + 1));
                }
            }
        }
    }

    fn count_owner_matches(dir: &Path) -> usize {
        let mut n = 0;
        for entry in fs::read_dir(dir).expect("src readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                n += count_owner_matches(&path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if OWNERS.contains(&name) {
                n += fs::read_to_string(&path)
                    .expect("source readable")
                    .matches("ConsensusCompatibility::")
                    .count();
            }
        }
        n
    }
}

/// `ironwood-nu63-support.md` §1.2 / §8 gate 3 — the SECOND half of the
/// one-predicate rule, and the regression guard for the outage's own shape.
///
/// The endpoint's `consensus_branch_id` may be compared to our compiled params
/// in exactly ONE place. A well-meaning future change that adds its own
/// comparison — in `provision.rs` beside the chain guard, say — recreates the
/// two-mechanisms-one-question state that let the branch half of the
/// network-match sit specified-but-unbuilt for months.
#[test]
fn no_second_branch_comparison() {
    let mut offenders = Vec::new();
    scan(&manifest_dir().join("src"), &mut offenders);
    assert!(
        offenders.is_empty(),
        "`consensus_branch_id` is read outside the predicate + its boundary parse — a second \
         staleness comparison is the §1.2 defect recurring: {offenders:?}"
    );

    fn scan(dir: &Path, offenders: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("src readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                scan(&path, offenders);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            // consensus.rs owns the comparison; provision.rs owns the boundary
            // parse into `ServerIdentity` and the struct field itself.
            if matches!(name, "consensus.rs" | "provision.rs") {
                continue;
            }
            let text = fs::read_to_string(&path).expect("source readable");
            for (i, line) in text.lines().enumerate() {
                let code = line.trim();
                // The test MODULES, not any `#[cfg(test)]` attribute — see the
                // sibling above for why that distinction is load-bearing.
                // `testing` is the `#[cfg(test)] pub(crate) mod testing` that
                // holds the shared fakes (e.g. `sync::testing::FakeChain`,
                // which must synthesise a `ServerIdentity` to be an honest
                // endpoint at all).
                if code.starts_with("mod tests {")
                    || code.starts_with("pub(crate) mod testing {")
                    || code.starts_with("mod testing {")
                {
                    break;
                }
                if code.starts_with("//") || code.starts_with("*") {
                    continue;
                }
                if line.contains("consensus_branch_id") {
                    offenders.push(format!("{name}:{}", i + 1));
                }
            }
        }
    }
}

/// §8 G2 lint: every `pub enum` in the crate is `#[non_exhaustive]` — the
/// one API mistake that is unfixable post-1.0 (a host's exhaustive match
/// breaks on every SDK upgrade that adds a variant).
#[test]
fn public_enums_non_exhaustive() {
    let mut violations = Vec::new();
    let mut seen = Vec::new();
    let mut waived = Vec::new();
    scan_dir(
        &manifest_dir().join("src"),
        &mut violations,
        &mut seen,
        &mut waived,
    );
    assert!(
        violations.is_empty(),
        "pub enums missing #[non_exhaustive] (G2 policy, spec §2): {violations:?}"
    );
    // The waiver set is PINNED. `SwapOutcome` is the policy's one documented
    // exception (#367: the FFI mirror matches it exhaustively in both directions on
    // purpose, so a new variant is compile-loud rather than a silent wildcard
    // mis-render). Until that exception passed this gate by ACCIDENT — its
    // rationale paragraph contained the word `non_exhaustive` and the scan accepted
    // any comment that did. A waiver is now an exact marker line the scan reads, and
    // adding one anywhere else fails HERE until this list is edited deliberately.
    // Sorted before comparing: `seen`/`waived` fill in `read_dir` order, which is
    // unordered — inert at one waiver, flaky the day a second is added.
    waived.sort();
    assert_eq!(
        waived,
        vec!["SwapOutcome"],
        "the set of `// G2-WAIVER:` enums changed. A waiver is a decision about the \
         public API, not a comment: record the reason beside the enum AND add it to \
         this list in the same change"
    );
    // NOT VACUOUS (P0-9). This scan once stopped at the first `#[cfg(test)]` in a
    // file, and `send.rs` carries one at a mid-file test helper ABOVE
    // `LargeSendReason`, `keychain/mod.rs` one ABOVE `VaultTier` — both enums were
    // invisible to it, and removing either's attribute shipped green. The two are
    // named here so the scan is known to reach past that shape, and the floor is
    // the population's order of magnitude (44 when written), not its exact count.
    for must_see in ["LargeSendReason", "VaultTier"] {
        assert!(
            seen.iter().any(|name| name == must_see),
            "the enum scan never reached `pub enum {must_see}` — it sits below a \
             mid-file `#[cfg(test)]`, which is exactly the shape this scan went blind \
             on (P0-9); seen: {seen:?}"
        );
    }
    // 44 when written; the floor sits just under it on purpose — the largest
    // single file carries 11, so no ONE file going dark can pass. A deliberate
    // consolidation below 40 lowers this number in the same change, with the reason.
    assert!(
        seen.len() >= 40,
        "the enum scan saw only {} `pub enum`s — the crate has 44; a scan that \
         examines less than it should returns a shorter violation list, which is \
         indistinguishable from compliance (REVIEW.md §6). Seen: {seen:?}",
        seen.len()
    );

    fn scan_dir(
        dir: &Path,
        violations: &mut Vec<String>,
        seen: &mut Vec<String>,
        waived: &mut Vec<String>,
    ) {
        for entry in fs::read_dir(dir).expect("src readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                scan_dir(&path, violations, seen, waived);
            } else if path.extension().is_some_and(|e| e == "rs") {
                scan_file(&path, violations, seen, waived);
            }
        }
    }

    fn scan_file(
        path: &Path,
        violations: &mut Vec<String>,
        seen: &mut Vec<String>,
        waived: &mut Vec<String>,
    ) {
        let text = fs::read_to_string(path).expect("source readable");
        let lines: Vec<&str> = text.lines().collect();
        // Inside a test module: a brace depth, counted on code (never on a comment
        // line, never inside a string literal). SKIPPED, not `break`-ed on: the
        // Batch B review found `sync.rs`'s `pub(crate) mod testing {` sits
        // ~1,200 lines above that file's real test module, so a `break` there moved
        // the blind spot rather than closing it (P0-9, the third hole).
        let mut skip_depth: usize = 0;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if skip_depth > 0 {
                if !trimmed.starts_with("//") {
                    let (opens, closes) = brace_delta(trimmed);
                    skip_depth = skip_depth.saturating_add(opens).saturating_sub(closes);
                }
                continue;
            }
            // Enter (and skip) a test MODULE — NOT any `#[cfg(test)]`, which also
            // marks test-only helpers that sit mid-file, above the enums this gate
            // polices (`send.rs:143` above `LargeSendReason`, `keychain/mod.rs:37`
            // above `VaultTier`). The identical defect the two sibling scans in this
            // file already fixed; this was the third of three (P0-9).
            if trimmed.starts_with("mod tests {")
                || trimmed.starts_with("pub(crate) mod testing {")
                || trimmed.starts_with("mod testing {")
            {
                skip_depth = 1;
                continue;
            }
            let Some(rest) = trimmed.strip_prefix("pub enum ") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            seen.push(name.clone());
            // Walk back over the attributes and comments above the enum. Only the
            // EXACT attribute `#[non_exhaustive]`, or the exact `// G2-WAIVER:` line,
            // satisfies the gate — a `///` or `//` line that merely names
            // `non_exhaustive` is prose, and an attribute that merely CONTAINS the
            // word (`#[allow(non_exhaustive_omitted_patterns)]`, a `#[doc = …]`, a
            // `cfg_attr` that is exhaustive in the default build) is not the policy
            // either (P0-9's second hole, found twice: comments, attribute
            // substrings by the Batch B review).
            let mut covered = false;
            for back in lines[..i].iter().rev() {
                let b = back.trim();
                if b.starts_with("#[") {
                    // The attribute exactly, with a trailing `// …` comment allowed
                    // (four of the crate's enums annotate the attribute in place).
                    let attribute = b.split("//").next().unwrap_or("").trim();
                    if attribute == "#[non_exhaustive]" {
                        covered = true;
                        break;
                    }
                } else if b.starts_with("// G2-WAIVER:") {
                    waived.push(name.clone());
                    covered = true;
                    break;
                } else if !(b.starts_with("///") || b.starts_with("//")) {
                    break;
                }
            }
            if !covered {
                violations.push(format!("{}: {trimmed}", path.display()));
            }
        }
    }

    /// `{` and `}` counts on one line of CODE, ignoring string literals (a `"…"`
    /// with `\"` escapes honoured) and a trailing `//` comment. Character
    /// literals like `'{'` are not handled — none of the crate's test modules
    /// open or close a brace inside one, and an imbalance would only make the skip
    /// run long, never scan a test module as production.
    fn brace_delta(code: &str) -> (usize, usize) {
        let (mut opens, mut closes) = (0usize, 0usize);
        let mut in_string = false;
        let mut escaped = false;
        let mut prev = '\0';
        for c in code.chars() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
            } else {
                match c {
                    '"' => in_string = true,
                    '/' if prev == '/' => break,
                    '{' => opens += 1,
                    '}' => closes += 1,
                    _ => {}
                }
            }
            prev = c;
        }
        (opens, closes)
    }
}

/// §8 gate-7 pin: every named constant exists at its reviewed value — a
/// drive-by change fails HERE and comes to review with its WHY (the comment
/// in `constants.rs`).
#[test]
fn named_constants_at_boundary() {
    use zec_wallet_core::constants::*;

    // seed bounds (§2.2 — upstream panic guard)
    assert_eq!(SEED_MIN_BYTES, 32);
    assert_eq!(SEED_MAX_BYTES, 252);
    // money (§2.1)
    assert_eq!(MAX_MONEY_ZAT, 2_100_000_000_000_000);
    // memo / URI (§2.4, §4.6)
    assert_eq!(MEMO_TEXT_MAX_BYTES, 512);
    assert_eq!(MEMO_ARBITRARY_MAX_BYTES, 511);
    assert_eq!(PAYMENT_URI_MAX_BYTES, 8_192);
    // FR-27 read scope (§2.4 amendment). Value-pinned like their neighbours
    // because every FR-27 test uses the SYMBOLS: widening 32 → 511 would let a
    // "prefix" be the whole memo, and widening the count would unbound the
    // per-memo comparison work — both ship green without these two rows
    // (FR-27 security review).
    assert_eq!(MACHINE_MEMO_PREFIX_MAX_BYTES, 32);
    assert_eq!(MACHINE_MEMO_PREFIX_MAX_COUNT, 8);
    // sync (§7)
    assert_eq!(GRPC_MAX_MESSAGE_BYTES, 8 * 1024 * 1024);
    assert_eq!(GRPC_UNARY_TIMEOUT_SECS, 30);
    assert_eq!(GRPC_STREAMING_TIMEOUT_SECS, 100);
    // §3.2a hang-guard AND the budget the host reads as ZW_DIAL_BUDGET_SECS.
    // 25 since ADR-0552 stage 1b: STRICTLY below GRPC_UNARY_TIMEOUT_SECS above,
    // or a unary call cancels the dial at the instant the dial bound fires and
    // the `Preferred` switch is never evaluated on the money path.
    assert_eq!(DIAL_TIMEOUT_SECS, 25);
    // ADR-0552, the maintainer's minute: how long `Preferred` INSISTS on the private
    // path before it may switch to a direct connection. It is a PRIVACY promise,
    // not a liveness bound — shortening it moves every censored user to clearnet
    // sooner, which is why it is pinned here beside the dial bound it must exceed.
    assert_eq!(TOR_PATIENCE_SECS, 60);
    // ADR-0552 phase 2: what the clearnet leg gets to establish (TCP + TLS, one
    // deadline) after a switch. With the dial bound it must fit ONE unary budget
    // (asserted in `constants.rs`); raising it alone breaks that at compile time.
    assert_eq!(FALLBACK_ESTABLISH_BUDGET_SECS, 5);
    assert_eq!(DIAL_ATTEMPT_STAGGER_MS, 250); // FR-21 happy-eyeballs stagger (RFC 8305 §5)
    assert_eq!(DIAL_MAX_ADDRS_PER_FAMILY, 4); // FR-21 race fan-out cap (8 sockets max)
    // (#316): 1_000 → 100 — one batch's scan is the sync loop's only
    // uninterruptible span, and a cancelled sparse 1_000-block batch held the
    // wallet lock for minutes on-device, defeating the rescan/close quiesce.
    assert_eq!(SYNC_BATCH_BLOCKS, 100);
    assert_eq!(SYNC_BATCH_BLOCKS_DENSE, 100);
    // Scaled ×10 with the batch shrink so the O(scanned-set) summary read keeps
    // its ~8_000-block cadence (O(n²) guard).
    assert_eq!(SYNC_SUMMARY_REFRESH_BATCHES, 80);
    assert_eq!(POLL_INTERVAL_SECS, 20);
    assert_eq!(PROGRESS_REPORT_BLOCKS, 5); // §3.2g iv-d-3-b — intra-batch watchdog re-arm cadence
    // Any idle-timeout-alive link re-arms the watchdog before it fires (no false
    // restart), and ≥ 1 sample lands strictly within the dense batch — static
    // invariants, enforced at compile time.
    const {
        assert!(
            PROGRESS_REPORT_BLOCKS as u64 * GRPC_STREAMING_TIMEOUT_SECS < SYNC_STUCK_WATCHDOG_SECS
        )
    };
    const { assert!(PROGRESS_REPORT_BLOCKS > 0 && PROGRESS_REPORT_BLOCKS < SYNC_BATCH_BLOCKS_DENSE) };
    // ironwood-nu63-support.md §6.3 — the "degrade with age" grace for an
    // endpoint that withholds `consensus_branch_id`. ~1 day at the 75-second
    // target. Widening it lengthens exactly the silence the Ironwood outage was
    // made of, so a change comes here first.
    assert_eq!(UNKNOWN_BRANCH_GRACE_BLOCKS, 1_152);
    assert_eq!(SYNC_BACKOFF_INITIAL_SECS, 1); // §6.2 d-3 controller — first retry interval
    assert_eq!(SYNC_BACKOFF_MAX_SECS, 600);
    assert_eq!(SYNC_STUCK_WATCHDOG_SECS, 600);
    // §6.2 inc-2d-3-b-i aux-connection busy-timeout (a BLOCKING wait held under the db
    // Mutex) — pinned value + the load-bearing ordering: it MUST stay an order of
    // magnitude below the stuck-sync watchdog so a contended intent write can never
    // trip it.
    assert_eq!(SQLITE_BUSY_TIMEOUT_MS, 5_000);
    const { assert!(SQLITE_BUSY_TIMEOUT_MS < SYNC_STUCK_WATCHDOG_SECS * 1_000 / 10) };
    // §6.2 inc-2d-3-b-ii-A durable queued-send cap — pinned value + the load-bearing
    // ordering: a generous bound for legitimate offline batching that still exceeds the
    // in-memory propose-token cap (durable intents outlive a kill, so they warrant more
    // headroom than the transient proposal registry).
    assert_eq!(QUEUED_SEND_INTENTS_MAX, 128);
    const { assert!(QUEUED_SEND_INTENTS_MAX >= PROPOSAL_REGISTRY_MAX_LIVE) };
    // inc-2d-3-b-ii-B resubmission broadcast-phase time budget — pinned value + the two
    // load-bearing relationships: > one unary timeout (≥1 broadcast always fits ⇒ forward
    // progress, the queue can't wedge) and ≪ the watchdog window (it yields the pass guard so a
    // budgeted resubmission never reads as a wedged scan).
    assert_eq!(RESUBMIT_BROADCAST_BUDGET_SECS, 60);
    const { assert!(RESUBMIT_BROADCAST_BUDGET_SECS > GRPC_UNARY_TIMEOUT_SECS) };
    const { assert!(RESUBMIT_BROADCAST_BUDGET_SECS < SYNC_STUCK_WATCHDOG_SECS) };
    // #400 R1 — the prompt kick's bounded retry. This is the ONLY broadcast path for a host
    // running with sync off (the §6.1 `ReBroadcast` fallback rides `after_synced`), so both
    // values are money-relevant: > 1 attempt or a transient miss strands a signed transaction
    // forever, and an unbounded retry would hold a task plus a clone of the raw group alive on
    // a device that is simply offline.
    assert_eq!(KICK_BROADCAST_MAX_ATTEMPTS, 3);
    assert_eq!(KICK_BROADCAST_RETRY_BASE_SECS, 5);
    const { assert!(KICK_BROADCAST_MAX_ATTEMPTS > 1) };
    // the backoff floor never exceeds the cap (a degenerate config would hot-loop) —
    // a static invariant, enforced at compile time.
    const { assert!(SYNC_BACKOFF_INITIAL_SECS <= SYNC_BACKOFF_MAX_SECS) };
    assert_eq!(REORG_MAX_BLOCKS, 100);
    assert_eq!(REWIND_DISTANCE_BLOCKS, 10);
    // §3.2g iv-d-2b-ii: the per-pass consecutive-reorg bound = REORG_MAX_BLOCKS /
    // REWIND_DISTANCE_BLOCKS BY CONSTRUCTION (each reorg rewinds REWIND_DISTANCE_BLOCKS,
    // so this many consecutive rewinds cover the deepest auto-recovered reorg). Pin
    // the value AND the derivation so a change to either input comes to review.
    assert_eq!(MAX_SCAN_REORGS_PER_PASS, 10);
    assert_eq!(
        MAX_SCAN_REORGS_PER_PASS,
        REORG_MAX_BLOCKS / REWIND_DISTANCE_BLOCKS
    );
    // §4q-R P-RR3: the ABSOLUTE per-pass rewind cap = three epochs of the bound above BY
    // CONSTRUCTION — pin the value and the derivation. §4u RW-6 — the byte ceiling the cap
    // states (its doc in `constants.rs` derives it): a fork met in the batch above the
    // frontier un-scans under REWIND_DISTANCE_BLOCKS + SYNC_BATCH_BLOCKS blocks (the rewind
    // target, then the nearest checkpointed row — checkpoints at most one batch apart).
    //
    // §4u-run review row 5 CORRECTED this derivation. It used to say that span
    // "re-downloads in at most two batches" and price the rewind at three batches of
    // DOWNLOAD_BATCH_MAX_BYTES = 384 MiB, for 11.25 GiB per pass. Both steps were wrong:
    // the re-queue puts a `Verify` range first and a `ChainTip` range after, so the span
    // arrives as three ranges or more, and DOWNLOAD_BATCH_MAX_BYTES is a PER-BATCH
    // allowance — splitting a span into more batches RAISES `batches × allowance`, and
    // nothing bounds the batch count. For a span of S blocks in any number of batches the
    // cost of a split is Σ min(b_i × GRPC_MAX_MESSAGE_BYTES, DOWNLOAD_BATCH_MAX_BYTES), and a
    // CEILING is the supremum over splits: the per-batch allowance binds only for b_i > 16
    // (16 × 8 MiB = 128 MiB), so any split into ≤16-block batches reaches S × GRPC_MAX and no
    // split exceeds it. (The correction said "batches ≤ S makes the per-block term the
    // smaller" — false in the direction that matters: two 55-block batches cost 256 MiB, far
    // less than 880 MiB. Right number, wrong reason; corrected at the review.) A change
    // to ANY input re-derives the number here, and comes to review with the ceiling's new
    // value. §4u: the streak threshold is PASSES, not rewinds.
    assert_eq!(MAX_TOTAL_REORGS_PER_PASS, 30);
    assert_eq!(MAX_TOTAL_REORGS_PER_PASS, 3 * MAX_SCAN_REORGS_PER_PASS);
    let rewind_span_blocks = u64::from(REWIND_DISTANCE_BLOCKS + SYNC_BATCH_BLOCKS);
    assert_eq!(rewind_span_blocks, 110);
    // The re-downloaded span: per BLOCK, because the batch count is unbounded.
    let respan_bytes_max = rewind_span_blocks * GRPC_MAX_MESSAGE_BYTES as u64;
    assert_eq!(respan_bytes_max, 880 * 1024 * 1024);
    // The batch that forked: one batch, so the tighter of the two per-batch bounds — PLUS the
    // one message that TRIPS the cap, which has already crossed the wire when it is rejected
    // (`download_range_folding` adds a block to the tally, then checks).
    let forked_batch_bytes_max = DOWNLOAD_BATCH_MAX_BYTES
        .min(u64::from(SYNC_BATCH_BLOCKS) * GRPC_MAX_MESSAGE_BYTES as u64)
        + GRPC_MAX_MESSAGE_BYTES as u64;
    assert_eq!(forked_batch_bytes_max, 136 * 1024 * 1024);
    let rewind_bytes_max = respan_bytes_max + forked_batch_bytes_max;
    assert_eq!(rewind_bytes_max, 1_016 * 1024 * 1024);
    assert_eq!(
        u64::from(MAX_TOTAL_REORGS_PER_PASS) * rewind_bytes_max,
        30_480 * 1024 * 1024, // 29.77 GiB
    );
    assert_eq!(MAX_CONSECUTIVE_REWINDING_PASSES, 6);
    // §3.3 tx-enhancement per-pass bound (memo recovery converges over passes, not one burst).
    assert_eq!(MAX_ENHANCEMENTS_PER_PASS, 50);
    assert_eq!(MIN_CONFIRMATIONS, 10);
    // §3.2f: the new-wallet birthday lag is the reorg depth BY CONSTRUCTION —
    // pin the alias so an independent change to either comes to review.
    assert_eq!(NEW_WALLET_BIRTHDAY_LAG_BLOCKS, 100);
    assert_eq!(NEW_WALLET_BIRTHDAY_LAG_BLOCKS, REORG_MAX_BLOCKS);
    assert_eq!(CACHE_MAX_BYTES, 256 * 1024 * 1024);
    // §3.2g: the per-batch download byte ceiling (the byte-dimension DoS cap) is set
    // below the cache bound so a batch always fits the disposable cache with margin.
    assert_eq!(DOWNLOAD_BATCH_MAX_BYTES, 128 * 1024 * 1024);
    const { assert!(DOWNLOAD_BATCH_MAX_BYTES < CACHE_MAX_BYTES) };
    // §3.2g: the subtree-root count ceiling is the consensus-fixed max number of
    // completable subtrees per pool = 2^(tree depth 32 − shard height 16) = 2^16.
    assert_eq!(MAX_SUBTREE_ROOTS_PER_POOL, 1 << 16);
    assert_eq!(MAX_SUBTREE_ROOTS_PER_POOL, 65_536);
    // S15-F1 (ADR-0569): the incremental subtree-root fetch's full-verification
    // bounds, its start rounding, and the drops-in-a-row that turn it off.
    assert_eq!(SUBTREE_ROOTS_FULL_VERIFY_PASSES, 180);
    assert_eq!(SUBTREE_ROOTS_FULL_VERIFY_SECS, 3_600);
    assert_eq!(SUBTREE_ROOTS_START_GRANULARITY, 64);
    assert_eq!(SUBTREE_ROOTS_DROPS_OFF, 2);
    // send path (§3.1, §5.3, §7)
    assert_eq!(PROPOSAL_TTL_SECS, 600);
    assert_eq!(BROADCAST_JITTER_MAX_MS_DEFAULT, 10_000);
    assert_eq!(SHIELDING_THRESHOLD_ZAT, 100_000);
    assert_eq!(HISTORY_PAGE_MAX, 100);
    // §3 send money-safety (large-amount confirm) — the relative is a fraction (bps);
    // the absolute is a tunable policy backstop. Both stay valid: bps ≤ 10_000 (≤100%),
    // and the relative trip must sit below "the whole balance" so it can actually fire.
    assert_eq!(NEAR_TOTAL_SPENDABLE_BPS, 9_000);
    assert_eq!(LARGE_SEND_ABSOLUTE_ZAT, 100_000_000); // 1 ZEC
    // STRICTLY below 100%: at exactly 10_000 bps the relative trigger only fires when
    // total == available — the state `InsufficientFunds` already rejects — so it would be a
    // dead path. < 10_000 guarantees it can fire BEFORE a full drain.
    const { assert!(NEAR_TOTAL_SPENDABLE_BPS > 0 && NEAR_TOTAL_SPENDABLE_BPS < 10_000) };
    const { assert!(LARGE_SEND_ABSOLUTE_ZAT > 0 && LARGE_SEND_ABSOLUTE_ZAT <= MAX_MONEY_ZAT) };
    // swap (§2.6, §7)
    assert_eq!(SLIPPAGE_DEFAULT_BPS, 200);
    assert_eq!(SLIPPAGE_MAX_BPS, 1_000); // default must sit under this ceiling
    assert_eq!(SWAP_POLL_INITIAL_SECS, 5);
    assert_eq!(SWAP_POLL_MAX_SECS, 60);
    assert_eq!(SWAP_DEADLINE_DEFAULT_SECS, 86_400);
    assert_eq!(DEADLINE_SAFETY_MARGIN_SECS, 60);
    // inc-2d-swap-a: the unsynced-clock floor for the §4.4 deposit-deadline gate. Must sit safely
    // in the past (any real swap is quoted after this) yet far above every reset/epoch value.
    assert_eq!(CLOCK_PLAUSIBILITY_FLOOR_SECS, 1_700_000_000);
    const { assert!(CLOCK_PLAUSIBILITY_FLOOR_SECS > 0) };
    assert_eq!(PROVIDER_STR_MAX_BYTES, 256);
    assert_eq!(SWAP_DECIMAL_MAX_DIGITS, 27); // the M1 overflow-guard digit cap
    assert_eq!(MAX_ISSUED_QUOTES, 16); // in-flight quote registry cap
    // hostile-input caps (§4.6)
    assert_eq!(ADDRESS_MAX_BYTES, 512);
    // §3.2i-2 2e-2b ephemeral / TEX-reclaim family (#294/#300/#301/#315) — pin every value; the
    // ENGINE-SYMBOL derivations (the deadness margin vs `DEFAULT_TX_EXPIRY_DELTA`, the mint amount vs
    // the ZIP-317 fee floor) are owned by `reclaim::tests`, where the engine crate is importable. Here
    // the value pins + the load-bearing SDK-internal orderings a drive-by change must come to review.
    assert_eq!(EPHEMERAL_GAP_LIMIT, 10); // the engine's ephemeral gap window
    assert_eq!(EPHEMERAL_DETECT_LOOKBACK_BLOCKS, 2_880);
    assert_eq!(EPHEMERAL_SWEEP_MAX_ADDRS, 64);
    assert_eq!(EPHEMERAL_RECOGNISE_MAX_UTXOS, 64);
    assert_eq!(EPHEMERAL_SWEEP_THRESHOLD_ZAT, 1);
    assert_eq!(
        EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX,
        "wallet-ephemeral-detect"
    );
    assert_eq!(EPHEMERAL_RECLAIM_MINT_ZAT, 50_000);
    assert_eq!(EPHEMERAL_RECLAIM_DEADNESS_BLOCKS, 141);
    // The deadness margin buries the tx0's expiry beyond the deepest auto-recovered reorg, so it
    // ALWAYS exceeds the reorg horizon BY CONSTRUCTION (margin = expiry-delta + 1 + reorg) — static.
    const { assert!(EPHEMERAL_RECLAIM_DEADNESS_BLOCKS > REORG_MAX_BLOCKS) };
    assert_eq!(MAX_TEX_REPROPOSE_ATTEMPTS, 3);
    // The per-intent re-propose cap MUST stay below the gap window so one stubborn intent can never
    // exhaust it alone (slice-1's whole purpose; slice-2's reclaim is the aggregate remedy). If this
    // ever inverts, a single intent could brick the window and the cap would be decorative.
    const {
        assert!(
            MAX_TEX_REPROPOSE_ATTEMPTS > 0
                && (MAX_TEX_REPROPOSE_ATTEMPTS as u32) < EPHEMERAL_GAP_LIMIT
        )
    };
    // seal (§4.2a) — the compile-time u16 asserts guard the CAST, this pins
    // the VALUE
    assert_eq!(SEAL_MNEMONIC_MAX_BYTES, 1_024);
    // config bounds (§2.3 — FFI-boundary cap, W3 unstable-env fold)
    assert_eq!(SOCKS_ADDR_MAX_BYTES, 256);
    // lifecycle (§3.3)
    assert_eq!(WALLET_LOCK_FILE_NAME, ".wallet.lock");
}

/// §8 gate-1 security (spec §3.3/§4.1): the bridge's FRB-scanned surface
/// (`api/*.rs`) exposes NO key material — no seed/key/zeroizing types, no
/// raw byte buffers, no mnemonic-bearing names. The two SANCTIONED mnemonic
/// crossings (`restore` inbound, `reveal_mnemonic` outbound — spec §3.3)
/// must be added to `SANCTIONED` *explicitly* when they land, which is
/// exactly the review gate this test exists to force.
#[test]
fn ffi_surface_exposes_no_key_types() {
    // material-bearing TYPE tokens — none may appear in api code at all
    const DENY_TYPES: &[&str] = &[
        "Zeroizing",
        "SeedSource",
        "SealKey",
        "SeedPayload",
        "SpendingKey", // also catches UnifiedSpendingKey/ExtendedSpendingKey
        "ViewingKey",  // also catches UnifiedFullViewingKey
        "SecretKey",
        "Usk",
        "Ufvk",
        "ExtendedPrivKey",
    ];
    // raw byte buffers: the wallet api has NO legitimate one (relax
    // deliberately, per-case, if e.g. ciphertext export ever lands)
    const DENY_BUFFERS: &[&str] = &["Vec<u8>", "&[u8]", "[u8;"];
    // material-bearing NAME patterns (fields/params/fns)
    const DENY_NAMES: &[&str] = &[
        "mnemonic:",
        "mnemonic_",
        "reveal_mnemonic",
        "seed:",
        "seed_bytes",
        "passphrase",
        // material under a synonym must not slip the scan (W3 review fold)
        "recovery_phrase",
        "backup_words",
        "entropy",
        "secret",
        "key_bytes",
        "viewing_key",
        "spending_key",
        "usk",
        "ufvk",
        // viewing-key SHORT names: the TYPE net already catches
        // `UnifiedIncomingViewingKey`/`UnifiedFullViewingKey` (via "ViewingKey"),
        // but a bare String param named `uivk`/`ivk`/`fvk` would have slipped the
        // NAME scan — a future watch-only/key-export surface (spec §1.6 post-1.0)
        // must land here as a SANCTIONED entry, never as an unmatched name.
        "uivk",
        "ivk",
        "fvk",
        "xprv",
        "xpriv",
        "wif:",
        "derivation_path",
        // FR-12 host-custodied seed seam (spec §4.2): the WalletSeedPort is Rust-ONLY
        // — a `Uint8List` can't be zeroized, so the seed-supply port must never reach
        // the bridge. Pin the symbols so a future `open_with_seed_port`/`provide_seed`
        // crossing fails CI (the `Arc<dyn WalletSeedPort>` param type matches no
        // DENY_TYPES token, so the NAME deny is the guard).
        "WalletSeedPort",
        "SeedSupplyError",
        "seed_port",
        "provide_seed",
    ];
    // (file suffix, allowed substring) — stripped from a line BEFORE token
    // search. Append-only, reviewed: this is the sanctioned-crossings list.
    //
    // GRANULARITY (reviewed limitation): the strip is file-wide, so an allowed
    // NAME like `passphrase` is neutralised on EVERY line of that file, not only
    // the sanctioned signature/call. That is acceptable because the DENY_TYPES /
    // DENY_BUFFERS scan is NOT name-anchored and stays in full force after the
    // strip — a key TYPE (`Zeroizing`, `SeedSource`, a `Vec<u8>` buffer, …) can
    // NEVER hide behind a name strip, only the English param name itself. So a
    // hypothetical future `passphrase_hint: Vec<u8>` still trips on `Vec<u8>`.
    // The type/buffer net is the real guarantee; the name strips only suppress
    // the deliberate, reviewed param names of these crossings.
    //
    // ANCHORING (security-N1): the strip is identifier-boundary-anchored,
    // NOT raw-substring — `strip_sanctioned` removes a match only when it is not
    // embedded in a longer identifier on either side. A raw `str::replace` would
    // let a FUTURE `refund_ufvk: String` field hide inside the sanctioned
    // `ufvk: String` entry (the exact escape the review's M4 review rejected at
    // the entry level); anchored, such a field still trips the `ufvk` NAME net
    // until it is explicitly sanctioned here. Pinned by
    // `sanctioned_strip_is_identifier_anchored` below.
    const SANCTIONED: &[(&str, &str)] = &[
        // the persistence POLICY enum (no material) — its field name would
        // otherwise trip "seed:"-class patterns via `seed_persistence:`
        ("config.rs", "seed_persistence"),
        ("config.rs", "SeedPersistence"),
        // the SANCTIONED outbound mnemonic crossing (spec §3.3, §10): the
        // `WalletHandle::reveal_mnemonic` backup read. The METHOD NAME is
        // stripped here (the deliberate review gate); the surface still carries
        // NO key TYPE (it returns a plain `Vec<String>`, and the zeroizing→String
        // conversion lives in convert.rs, off this scanned api/ surface), so the
        // DENY_TYPES / DENY_BUFFERS scan stays fully in force on this file.
        ("wallet.rs", "reveal_mnemonic"),
        // the closed-handle regression TEST for that crossing: its fn name
        // embeds `reveal_mnemonic` as a prefix, which the anchored strip (N1)
        // correctly refuses to auto-sanction — listed exactly, so any OTHER
        // embedding (a new `reveal_mnemonic_*` surface) stays caught until
        // reviewed here.
        (
            "wallet.rs",
            "reveal_mnemonic_on_a_closed_handle_is_typed_not_a_panic",
        ),
        // the SANCTIONED *inbound* mnemonic crossing (spec §3.3, §10): the
        // `WalletHandle::restore` recovery-from-phrase entry. ONLY its two
        // key-bearing PARAM NAMES are stripped (the deliberate review gate) —
        // the surface still carries NO key TYPE: the words arrive as a plain
        // `Vec<String>` and the `SeedSource::Mnemonic` construction lives in
        // convert.rs (off this scanned api/ surface), so DENY_TYPES/DENY_BUFFERS
        // stay fully in force on the file. (The method name `restore` matches no
        // DENY token, so it needs no entry.)
        ("wallet.rs", "mnemonic_words"),
        ("wallet.rs", "passphrase"),
        // the SANCTIONED FR-17 spend-binding crossings (#396; spec wallet-sdk.md
        // "FR-17 + FR-18"): an SDK-minted random NONCE, explicitly NOT key
        // material — the spec sanctions it crossing Dart on display DTOs (its
        // only power is to make a review↔sign mismatch host-DETECTABLE; a
        // hostile reader gains nothing, a substituting writer can only cause a
        // fail-closed refusal). ONLY the exact field declarations are stripped
        // — any OTHER `Vec<u8>` in these files still trips the buffer net.
        ("payments.rs", "binding: Vec<u8>"),
        ("swap.rs", "binding: Option<Vec<u8>>"),
        ("state.rs", "binding: Option<Vec<u8>>"),
        // The SANCTIONED FR-28 compose-side machine memo (spec wallet-sdk.md
        // inc-2d-ui-prefill): OPAQUE HOST BYTES going OUT to a ZIP-321 URI,
        // explicitly not key material and not wallet-derived — the caller
        // already holds them, and a Dart host could already put the same bytes
        // on-chain by writing its own URI and handing it to `propose`. What
        // this crossing buys is that it can use the audited encoder instead.
        // INBOUND machine memo bytes are NOT sanctioned and must not be: those
        // are hostile on-chain data (principle 7), and the parse direction
        // stays a length-only display projection (`memo_to_parsed`).
        // EXACT declaration only, per the `binding:` precedent — any OTHER
        // `Vec<u8>` in this file still trips the buffer net.
        ("payments.rs", "memo_bytes: Option<Vec<u8>>"),
        // The SANCTIONED FR-27 machine-memo READ scope + verb (spec
        // wallet-sdk.md §2.4 amendment; the SDK RULING's four binding
        // conditions). This is the INBOUND crossing the FR-28 entry above
        // deliberately refused — granted separately, on its own conditions,
        // because the posture was asymmetric in the UNSAFE direction: the SDK
        // helps a host WRITE an envelope onto a permanent public ledger and
        // then makes that same envelope unreadable to it.
        //
        // What makes it reviewable rather than a hole: it is OPT-IN (absent by
        // default), PREFIX-SCOPED, `Arbitrary`-only (never `Reserved`),
        // length-capped, and it is a DIFFERENT verb from the display read —
        // `ParsedMemo` stays length-only, which
        // `inbound_parsed_memo_never_carries_raw_bytes` still enforces.
        //
        // The bytes are NOT key material and not wallet-derived: they are
        // public on-chain data anyone with the txid's block can see, and the
        // host is being handed the copy addressed to it.
        //
        // EXACT declarations only, per the `binding:` precedent. The return
        // type's text is generic enough that a SECOND verb with the identical
        // signature would be auto-sanctioned by this file-wide strip — which is
        // the escape the crypto audit built against the FR-28 entry — so
        // `fr27_machine_memo_waiver_covers_exactly_one_declaration` pins the
        // count that a strip cannot.
        ("config.rs", "machine_memo_prefixes: Vec<Vec<u8>>"),
        ("wallet.rs", "Result<Vec<Vec<u8>>, WalletApiError>"),
        // the SANCTIONED #397 watch-only refusal NAME (spec §3.7 D3): the
        // `WalletErrorKind::InvalidViewingKey` variant — a typed ERROR NAME
        // carrying NO material (payload-free by design; the offending string
        // is never echoed, §5.4). Its name necessarily contains the
        // "ViewingKey" TYPE-net token, which is exactly the comment's
        // predicted landing: sanctioned, never unmatched. The strip is the
        // variant name only — a real `UnifiedFullViewingKey`/`Ufvk` TYPE or
        // any `ufvk`-named param on this file still trips the scan in full
        // force.
        ("error.rs", "InvalidViewingKey"),
        // the SANCTIONED #397 UFVK crossings (spec §3.7 D1/D2, ADR-0538 —
        // the HARD-D re-cast's ONE deliberate egress + its import twin):
        // `WalletHandle::export_ufvk` (outbound String — the reference UI
        // gates it at the backup-phrase bar) and `create_watch_only`'s
        // `ufvk: String` param (inbound; decoded network-bound inside the
        // core, typed reject, never echoed). EXACT declarations only (the
        // `binding:` precedent; review security M4 rejected a bare-`ufvk`
        // file-wide strip — it would auto-sanction any FUTURE ufvk-named
        // surface in this file, e.g. a Q13 QR-chunk DTO): the method name,
        // the param declaration, and the one delegation call. The surface
        // carries NO key TYPE (plain Strings; `UnifiedFullViewingKey` lives
        // in core, off this scanned surface), so DENY_TYPES / DENY_BUFFERS
        // stay fully in force — and any OTHER `ufvk` name here still trips.
        ("wallet.rs", "export_ufvk"),
        ("wallet.rs", "ufvk: String"),
        ("wallet.rs", "(config, ufvk, birthday_height)"),
    ];

    let api_dir = bridge_dir().join("src/api");
    let mut violations = Vec::new();
    for entry in fs::read_dir(&api_dir).expect("bridge api dir readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let text = fs::read_to_string(&path).expect("api source readable");
        for (lineno, raw) in text.lines().enumerate() {
            // comments (incl. docs) can SAY "seed"/"mnemonic" — only code counts
            let mut code = raw.split("//").next().unwrap_or("").to_string();
            for (file, allowed) in SANCTIONED {
                if file_name.ends_with(file) {
                    code = strip_sanctioned(&code, allowed);
                }
            }
            for token in DENY_TYPES.iter().chain(DENY_BUFFERS).chain(DENY_NAMES) {
                if code.contains(token) {
                    violations.push(format!(
                        "{file_name}:{}: `{token}` in FRB-scanned api code",
                        lineno + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "key-material tokens on the FFI surface (spec §3.3 non-surface; add \
         to SANCTIONED only for a spec-sanctioned crossing): {violations:#?}"
    );
}

/// The FR-28 waiver's SCOPE, which the waiver itself cannot enforce.
///
/// `ffi_surface_exposes_no_key_types` strips a sanctioned declaration
/// FILE-WIDE, and `ParsedPayment` — the INBOUND display projection — lives in
/// the same `payments.rs` as the outbound `PaymentDraft`. So adding
/// `memo_bytes: Option<Vec<u8>>` to `ParsedPayment` would be auto-sanctioned by
/// the entry granted for the compose direction, and the buffer net would stay
/// green while the exact crossing that entry's comment forbids landed. Probed,
/// not theorised: the crypto audit built that escape.
///
/// The asymmetry is the whole point of the waiver — OUTBOUND bytes are the
/// caller's own; INBOUND ones are hostile on-chain data (principle 7), and the
/// parse direction stays a length-only projection (`memo_to_parsed`). This
/// asserts the half a file-wide strip cannot.
#[test]
fn inbound_parsed_memo_never_carries_raw_bytes() {
    let src = fs::read_to_string(bridge_dir().join("src/api/payments.rs"))
        .expect("bridge payments api readable");
    // The two INBOUND types: the parsed leg and its memo projection.
    for ty in ["pub struct ParsedPayment", "pub enum ParsedMemo"] {
        let start = src.find(ty).unwrap_or_else(|| panic!("{ty} still exists"));
        let decl = &src[start..];
        let end = decl.find("\n}").expect("declaration is brace-terminated");
        let body = &decl[..end];
        for token in ["Vec<u8>", "&[u8]", "[u8;"] {
            assert!(
                !body.contains(token),
                "{ty} carries `{token}`: machine-memo bytes must NOT cross \
                 INBOUND. The FR-28 SANCTIONED entry covers the compose \
                 direction only, and its file-wide strip would hide this \
                 (spec wallet-sdk.md inc-2d-ui-prefill; FR-27 is the read side \
                 and has its own conditions)."
            );
        }
    }
}

/// The FR-27 waiver's SCOPE, which the waiver itself cannot enforce — the same
/// shape as [`inbound_parsed_memo_never_carries_raw_bytes`] one entry up, at a
/// different seam.
///
/// `ffi_surface_exposes_no_key_types` strips a sanctioned declaration
/// FILE-WIDE, and the FR-27 entry's needle is a bare RETURN TYPE
/// (`Result<Vec<Vec<u8>>, WalletApiError>`) rather than a named field. So a
/// SECOND verb on `wallet.rs` returning exactly that would be auto-sanctioned
/// by the entry granted for the machine-memo read, and the buffer net would
/// stay green while an unreviewed byte crossing landed. Probed, not theorised:
/// the crypto audit built precisely that escape against the FR-28 entry,
/// which is why this test exists BEFORE anyone tries it here.
///
/// The remedy a file-wide strip cannot express: assert the COUNT.
#[test]
fn fr27_machine_memo_waiver_covers_exactly_one_declaration() {
    let src = fs::read_to_string(bridge_dir().join("src/api/wallet.rs"))
        .expect("bridge wallet api readable");
    let needle = "Result<Vec<Vec<u8>>, WalletApiError>";
    let count = src.matches(needle).count();
    assert_eq!(
        count, 1,
        "the FR-27 SANCTIONED entry waives `{needle}` FILE-WIDE, so it must cover \
         exactly ONE declaration — found {count}. A second verb returning raw \
         bytes to Dart is a new crossing and needs its own review + its own \
         entry, not a free ride on the machine-memo waiver (spec wallet-sdk.md \
         §2.4 amendment; FR-27's four binding conditions)."
    );
    // The SIBLING waiver needs the same pin, and did not have it — my own
    // remedy covered one of the two entries I added (FR-27 crypto audit). The
    // config.rs needle is a named field rather than a bare return type, so the
    // escape is narrower, but "narrower" is not "closed": a future
    // `WalletConfigPatch { machine_memo_prefixes: Vec<Vec<u8>> }` in the same
    // file would ride this strip with the buffer gate still green.
    let cfg_src = fs::read_to_string(bridge_dir().join("src/api/config.rs"))
        .expect("bridge config api readable");
    let cfg_needle = "machine_memo_prefixes: Vec<Vec<u8>>";
    let cfg_count = cfg_src.matches(cfg_needle).count();
    assert_eq!(
        cfg_count, 1,
        "the FR-27 SANCTIONED entry waives `{cfg_needle}` FILE-WIDE in api/config.rs, \
         so it must cover exactly ONE declaration — found {cfg_count}."
    );
    // …and it is the verb we sanctioned, not some other function that happens
    // to share the signature.
    assert!(
        src.contains("pub async fn machine_memos(&self, txid_hex: String)"),
        "the one sanctioned declaration is `machine_memos` — if it was renamed, \
         re-review the crossing and update the entry deliberately"
    );
}

/// Remove `needle` from `hay` only where the match stands on IDENTIFIER
/// boundaries — i.e. neither the character before the match nor the one after
/// it is `[A-Za-z0-9_]`. This is what keeps a sanctioned strip EXACT (
/// security-N1): `ufvk: String` must neutralise the one reviewed param
/// declaration, never a future `refund_ufvk: String` embedding it.
fn strip_sanctioned(hay: &str, needle: &str) -> String {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    // A side needs a boundary check only when the needle's own EDGE is an
    // identifier char — a needle edge like `(` or `>` is itself the boundary
    // (e.g. the `(config, ufvk, birthday_height)` call entry legitimately
    // follows the method name it belongs to).
    let check_before = needle.chars().next().is_some_and(is_ident);
    let check_after = needle.chars().next_back().is_some_and(is_ident);
    let mut out = String::with_capacity(hay.len());
    let mut rest = hay;
    while let Some(pos) = rest.find(needle) {
        // The true preceding char may live in `out` (when a KEPT embedded
        // occurrence immediately precedes this one), not only in `rest`.
        let prev = rest[..pos]
            .chars()
            .next_back()
            .or_else(|| out.chars().next_back());
        let bounded_before = !check_before || prev.is_none_or(|c| !is_ident(c));
        let after = &rest[pos + needle.len()..];
        let bounded_after = !check_after || after.chars().next().is_none_or(|c| !is_ident(c));
        out.push_str(&rest[..pos]);
        if !(bounded_before && bounded_after) {
            // Embedded in a longer identifier — NOT the sanctioned crossing;
            // keep it so the deny scan still sees it.
            out.push_str(needle);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Pins the N1 anchoring (see [`strip_sanctioned`]): the sanctioned
/// `ufvk: String` entry strips exactly the reviewed declaration and nothing
/// embedding it — the escape a raw `str::replace` would have opened.
#[test]
fn sanctioned_strip_is_identifier_anchored() {
    // The predicted future escape: a `*_ufvk`-named field must KEEP its name
    // so the `ufvk` deny token still trips on it.
    let kept = strip_sanctioned("        refund_ufvk: String,", "ufvk: String");
    assert!(kept.contains("ufvk"), "embedded match must not be stripped");
    // …while the reviewed declaration itself still strips clean.
    let stripped = strip_sanctioned("        ufvk: String,", "ufvk: String");
    assert!(
        !stripped.contains("ufvk"),
        "sanctioned declaration must strip"
    );
    // Suffix embedding is an escape too (`export_ufvk_v2` ⊃ `export_ufvk`).
    let kept = strip_sanctioned("    pub fn export_ufvk_v2(", "export_ufvk");
    assert!(
        kept.contains("export_ufvk"),
        "suffix embedding must not strip"
    );
    // Repeated bounded occurrences all strip (the file-wide contract holds).
    let stripped = strip_sanctioned("ufvk: String, ufvk: String", "ufvk: String");
    assert!(!stripped.contains("ufvk"));
    // Adjacency: an occurrence abutting a KEPT embedded one is itself embedded
    // (its true preceding char is the kept needle's trailing ident char, which
    // lives in the already-emitted output, not the remainder).
    let kept = strip_sanctioned("refund_ufvk: Stringufvk: String", "ufvk: String");
    assert_eq!(kept, "refund_ufvk: Stringufvk: String");
}

/// §8 gate-4/G2 drift guard: every variant of every core enum the bridge
/// re-exposes appears in the BRIDGE ENUM's declaration (the bridge enum is a
/// superset — it adds `Unknown`). Add a core variant and CI fails HERE until
/// the bridge, and thus the Dart surface, learns it.
///
/// **What this does NOT do, corrected (arch review MAJOR).** It reads
/// the two enum DECLARATIONS and nothing else. It used to claim it "makes the
/// wildcard `_ => Unknown` arms in `convert.rs` structurally unreachable",
/// which is false: declare the variant on both sides and forget the
/// `convert.rs` arm, and the wildcard swallows it — a real core state arriving
/// in Dart as `Unknown`, which every Dart switch renders as the
/// cannot-be-verified sentence. The declarations agree, so this gate is green.
///
/// Nor can the wildcard simply be banned. `TorState` and `TorRuntimeKind` are
/// `#[non_exhaustive]`, so rustc REQUIRES a catch-all in the downstream bridge
/// crate; the `SwapOutcome` technique (assert no `_ =>` arm) works only because
/// that enum is deliberately exhaustive and carries a written waiver saying so.
/// The checkable guarantee for a `#[non_exhaustive]` pair is therefore NAME
/// PARITY INSIDE THE ARM BODY, which [`bridge_conversion_arms_name_every_core_variant`]
/// now enforces for the privacy-bearing pair this stage shipped.
#[test]
fn bridge_enums_cover_core_variants() {
    // (core file, core enum) → (bridge file, bridge enum)
    // TorRuntime → TorRuntimeConfig is DELIBERATELY not listed: the Dart
    // side is a subset by design (`Dialer` — a Rust trait object — is not
    // Dart-expressible, spec §2.3; it is the ONE non-Dart runtime since the
    // SDK-owned `BuiltIn` was removed at FR-5 C1). `HostDialer` IS
    // Dart-expressible, as the unit `TorRuntimeConfig::HostDialer` (FR-29
    // spec §2.1 / §0 A5): the dialer behind it is the one the host's native
    // library registered through the C contract, so the Dart value carries
    // no payload. `TorRuntimeKind`'s host-dialer arm carries the host's name (a
    // bounded `String` on the bridge) and two closed enums, listed below as rows.
    const MAP: &[(&str, &str, &str, &str)] = &[
        // FR-29 / ADR-0547: the host transport descriptor's two DART-VISIBLE
        // closed value sets, carried by `TorRuntimeKind::HostDialer` beside the
        // host's own name and rendered by the host ("connections can be linked
        // by the proxy" from isolation; "not private" from exposure). A new
        // core value MUST fail CI here, never fold to the bridge's cautious
        // `Unknown`. The descriptor's THIRD closed set, `TransportHealth`
        // (ABI v3, ADR-0549), is deliberately absent: it never crosses to Dart
        // — a host declares it through the C contract and the SDK consumes it
        // in `live_tor_state`, which renders `Unavailable` (T15 pins that set
        // against the header instead, which is where its consumer reads it).
        (
            "src/net/host_dialer.rs",
            "IsolationSupport",
            "src/api/state.rs",
            "IsolationSupport",
        ),
        (
            "src/net/host_dialer.rs",
            "TransportExposure",
            "src/api/state.rs",
            "TransportExposure",
        ),
        (
            "src/state.rs",
            "SyncStatus",
            "src/api/state.rs",
            "SyncStatus",
        ),
        (
            "src/state.rs",
            "StallReason",
            "src/api/state.rs",
            "StallReason",
        ),
        (
            "src/state.rs",
            "PoolService",
            "src/api/state.rs",
            "PoolService",
        ),
        ("src/state.rs", "TxStatus", "src/api/state.rs", "TxStatus"),
        // Stage S8 `obligation` (R02): the per-transaction delivery reading. A
        // fifth core reading MUST fail CI here — folded to the bridge's `Unknown`
        // it would render as "saved" with no promise over a payment the wallet
        // is in fact retrying.
        (
            "src/state.rs",
            "DeliveryState",
            "src/api/state.rs",
            "DeliveryState",
        ),
        (
            "src/state.rs",
            "TxSubmitResult",
            "src/api/state.rs",
            "TxSubmitResult",
        ),
        ("src/state.rs", "TorState", "src/api/state.rs", "TorState"),
        (
            "src/state.rs",
            "TorRuntimeKind",
            "src/api/state.rs",
            "TorRuntimeKind",
        ),
        (
            "src/error.rs",
            "WalletError",
            "src/api/error.rs",
            "WalletErrorKind",
        ),
        (
            "src/error.rs",
            "SwapError",
            "src/api/error.rs",
            "SwapErrorKind",
        ),
        (
            "src/error.rs",
            "QuoteBoundSide",
            "src/api/error.rs",
            "QuoteBoundSide",
        ),
        (
            "src/error.rs",
            "DestinationInvalidReason",
            "src/api/error.rs",
            "DestinationInvalidReason",
        ),
        (
            "src/error.rs",
            "ProviderProtocolReason",
            "src/api/error.rs",
            "ProviderProtocolReason",
        ),
        (
            "src/error.rs",
            "SwapAddressCheckRefusal",
            "src/api/error.rs",
            "SwapAddressCheckRefusal",
        ),
        (
            "src/lifecycle.rs",
            "LifecyclePhase",
            "src/api/error.rs",
            "LifecyclePhase",
        ),
        // P3-13, the picker's two public enums (`sync-server-picker.md` §3.3).
        (
            "src/sync_server.rs",
            "SyncServerChoice",
            "src/api/config.rs",
            "SyncServerChoice",
        ),
        (
            "src/sync_server.rs",
            "SyncServerFallback",
            "src/api/config.rs",
            "SyncServerFallback",
        ),
        (
            "src/swap/types.rs",
            "SwapDirection",
            "src/api/swap.rs",
            "SwapDirection",
        ),
        (
            "src/swap/types.rs",
            "SwapAmount",
            "src/api/swap.rs",
            "SwapAmount",
        ),
        (
            "src/swap/types.rs",
            "ExactSide",
            "src/api/swap.rs",
            "ExactSide",
        ),
        (
            "src/swap/types.rs",
            "SwapStatus",
            "src/api/swap.rs",
            "SwapStatus",
        ),
        (
            "src/swap/types.rs",
            "SwapFailureCode",
            "src/api/swap.rs",
            "SwapFailureCode",
        ),
        (
            "src/swap/types.rs",
            "DisclosureItem",
            "src/api/swap.rs",
            "DisclosureItem",
        ),
        // SwapKill is `#[non_exhaustive]` + designed-to-grow (a future stricter severity
        // is inserted more-off); the lockstep guard makes a new core variant trip CI so
        // the bridge enum (and thus the host's expressible kill set) can't silently fall
        // behind. The bridge superset carries the extra `Unknown` arm (inbound-rejected).
        (
            "src/swap/types.rs",
            "SwapKill",
            "src/api/swap.rs",
            "SwapKill",
        ),
        ("src/money.rs", "Network", "src/api/config.rs", "Network"),
        (
            "src/memo.rs",
            "AddressKind",
            "src/api/payments.rs",
            "AddressKind",
        ),
        // core Memo's display projection: Text/Arbitrary/Reserved cross as
        // their bridge peers (Reserved is parse-unreachable today but bridged
        // for the incoming-memo lane — receivable on-chain reality)
        ("src/memo.rs", "Memo", "src/api/payments.rs", "ParsedMemo"),
        // the §5.1 de-shield disclosure enum (inc-2d-ffi): a new core pool MUST
        // fail CI here, never silently fold to the bridge's `Unknown` arm — the
        // host renders this to warn "funds leave the shielded set".
        (
            "src/send.rs",
            "OutputPool",
            "src/api/payments.rs",
            "OutputPool",
        ),
        // §3 money-safety large-amount reason (#226): a new core reason MUST fail CI here so the
        // bridge enum (and the host's large-amount confirm copy) can't silently fall behind. The
        // bridge superset carries the extra `Unknown` arm (a future reason still triggers a confirm).
        (
            "src/send.rs",
            "LargeSendReason",
            "src/api/payments.rs",
            "LargeSendReason",
        ),
        // the parked-send shape (#331): a new core kind MUST fail CI here, never silently fold to
        // the bridge's `Unknown` arm — the kind drives the host's "saved & pending" phrasing. The
        // bridge superset carries the extra `Unknown` (a future shape still renders the generic,
        // still-cancellable row).
        (
            "src/parked.rs",
            "ParkedSendKind",
            "src/api/state.rs",
            "ParkedSendKind",
        ),
        // FR-23-b parked authorization (#361): a new core outcome MUST fail CI here, never
        // silently fold to the bridge's `Unknown` arm — these arms drive whether the host says
        // "sending now", "still waiting", or nothing at all, and getting that wrong is either a
        // false money claim or a DOUBLE-PAY invitation. The bridge superset carries the extra
        // `Unknown` (pinned to the stillQueued phrasing, the money-silent direction).
        (
            "src/parked.rs",
            "ParkedAuthorization",
            "src/api/state.rs",
            "ParkedAuthorization",
        ),
        // the #315 reclaim outcome ladder: a new core outcome MUST fail CI here, never silently
        // fold to the bridge's `Unknown` arm — a `Minted`/`NotBroadcast` split drives the host's
        // recovery disclosure copy. The bridge superset carries the extra `Unknown` (a future
        // outcome still renders a neutral "recovery finished" row).
        (
            "src/reclaim.rs",
            "ReclaimOutcome",
            "src/api/state.rs",
            "ReclaimOutcome",
        ),
        // GRACE-1 (§4p G-1 / item 2; added by §4p-run row 2's arch review): the three
        // grace enums. Each crosses as a REFUSAL the host renders as its own sentence —
        // "update the app" (`NetworkUpgrade`), "switch servers" (`Blocks`), "check the
        // device's date and time" (`Clock`), "never confirmed" (`NeverConfirmed`) — and
        // rendering any two alike WAS the GRACE-1 defect. All three core enums are
        // `#[non_exhaustive]` and the bridge's `From` impls end in `_ => Self::Unknown`,
        // so without these rows a new core reason folds to `Unknown` in Dart with no CI
        // failure: the host shows the generic ended-grace copy for a reason it was never
        // told. A fold-to-Unknown here MUST fail CI.
        (
            "src/state.rs",
            "GraceExpiry",
            "src/api/state.rs",
            "GraceExpiry",
        ),
        (
            "src/state.rs",
            "UnknownBranchGrace",
            "src/api/state.rs",
            "UnknownBranchGrace",
        ),
        (
            "src/state.rs",
            "SigningBlock",
            "src/api/state.rs",
            "SigningBlock",
        ),
        (
            "src/config.rs",
            "TorPolicy",
            "src/api/config.rs",
            "TorPolicy",
        ),
        (
            "src/config.rs",
            "JitterPolicy",
            "src/api/config.rs",
            "JitterPolicy",
        ),
        (
            "src/seed.rs",
            "SeedPersistence",
            "src/api/config.rs",
            "SeedPersistence",
        ),
        // Stage S16 `bridge` (§3.3 assertion 4): the duress sever's report. A
        // host reads `severed`, not the absence of an exception, so a new core
        // outcome, cause or holder MUST fail CI here, never fold to the bridge's
        // `Unknown` — "the custody may be live" and "severed" rendered alike is
        // the whole failure this verb exists to prevent.
        (
            "src/sever.rs",
            "SeverOutcome",
            "src/api/state.rs",
            "SeverOutcome",
        ),
        (
            "src/sever.rs",
            "UnprovenReason",
            "src/api/state.rs",
            "UnprovenReason",
        ),
        (
            "src/sever.rs",
            "NotSeveredCause",
            "src/api/state.rs",
            "NotSeveredCause",
        ),
        (
            "src/sever.rs",
            "HolderSeen",
            "src/api/state.rs",
            "HolderSeen",
        ),
        (
            "src/sever.rs",
            "FilesOutcome",
            "src/api/state.rs",
            "FilesOutcome",
        ),
        // The ONE `// G2-WAIVER:` enum (P0-9): `SwapOutcome` is deliberately
        // exhaustive because the bridge mirrors it exhaustively in BOTH directions
        // (#367) — so a new variant is compile-loud rather than a silent `Unknown`.
        // That premise is what the waiver rests on, and until the Batch B review it
        // was asserted in a comment and guarded by nothing: this row makes a new
        // core variant fail CI here, and `swap_outcomes_mirror_carries_no_wildcard_arm`
        // below refuses the `_ =>` that would silently falsify the waiver.
        (
            "src/swap_record_store.rs",
            "SwapOutcome",
            "src/api/swap.rs",
            "SwapOutcome",
        ),
    ];

    // Core variants DOCUMENTED as never crossing the wallet FFI: consumed and
    // re-mapped INSIDE the core, so the bridge deliberately carries no kind.
    // Each entry must cite the core-side doc pinning the internal-only
    // contract; an undocumented miss still fails below. (arch review:
    // this gate had been silently RED since W-swap-4-a-2 because no allowlist
    // existed for the deliberate case.)
    const FFI_INTERNAL: &[(&str, &str)] = &[
        // error.rs `SwapDepositInFlight` doc: "Raised ONLY on the deposit
        // enqueue … so it never crosses the wallet FFI — the `DepositSender`
        // impl maps it to the swap-namespaced `SwapAlreadyInFlight`".
        ("WalletError", "SwapDepositInFlight"),
    ];

    // The allowlist is ENFORCED, not honor-system (W-swap-4-a-6 / security
    // MED-2 — a bare (enum, variant) pair was a working escape hatch for ANY
    // lockstep enum above, including the §5.1 de-shield disclosure ones). Each
    // entry must: (i) target `WalletError` — the only enum with a documented
    // internal-remap seam today; widening to another enum is a reviewed
    // decision HERE, not an allowlist edit; (ii) name a variant that still
    // parses from the core enum — a stale entry dies with its variant; and
    // (iii) sit on a variant whose core doc carries the "never crosses the
    // wallet FFI" marker phrase — the doc IS the contract the entry cites.
    // HONEST SCOPE: a DOC gate, not a REMAP gate — nothing here asserts the
    // internal remap exists (a marker-phrased entry without one would cross
    // folded to the bridge's `Unknown`, stable `code` intact). Accepted at
    // one entry; per-entry remap assertions are the hardening if this grows
    // (spec §4.4 W-swap-4-a-6 (e)).
    for (enum_name, variant) in FFI_INTERNAL {
        assert_eq!(
            *enum_name, "WalletError",
            "FFI_INTERNAL ({enum_name}, {variant}): only WalletError has a \
             documented internal-remap seam (see this loop's policy comment)"
        );
        let (core_file, ..) = MAP
            .iter()
            .find(|(_, e, _, _)| e == enum_name)
            .unwrap_or_else(|| panic!("FFI_INTERNAL names unmapped enum {enum_name}"));
        let core_src = fs::read_to_string(manifest_dir().join(core_file))
            .unwrap_or_else(|_| panic!("core source {core_file} readable"));
        assert!(
            enum_variants(&core_src, enum_name)
                .iter()
                .any(|v| v == variant),
            "stale FFI_INTERNAL entry: {enum_name}::{variant} no longer exists in the core enum"
        );
        assert!(
            variant_doc(&core_src, enum_name, variant).contains("never crosses the wallet FFI"),
            "{enum_name}::{variant} is allowlisted as FFI-internal, but its core \
             doc does not pin the contract — the variant's `///` doc must carry \
             the marker phrase \"never crosses the wallet FFI\""
        );
    }

    // Core `WalletError` variants that DO cross the FFI, FOLDED into an
    // EXISTING bridge kind at a convert arm, with their own stable `code`
    // riding `WalletApiError.code` (S9 / FR-47, contract revision 3: the
    // keychain timeout answers `keystoreUnavailable` with `RW-KEY-008`, no
    // new kind, no ABI bump). Unlike FFI_INTERNAL this is a REMAP gate, the
    // hardening that list's policy comment names: each entry must (i) name
    // a core variant that still exists, (ii) have its convert arm map it to
    // EXACTLY the named bridge kind, and (iii) have the core `code()` table
    // return EXACTLY the named code. A fold without its arm, or onto a
    // different kind, fails here. Widening this list is a reviewed decision.
    const FOLDED_INTO_KIND: &[(&str, &str, &str)] =
        &[("KeychainTimeout", "KeystoreUnavailable", "RW-KEY-008")];
    {
        let core_src = fs::read_to_string(manifest_dir().join("src/error.rs"))
            .expect("core src/error.rs readable");
        let convert_src = fs::read_to_string(bridge_dir().join("src/convert.rs"))
            .expect("bridge src/convert.rs readable");
        let core_variants = enum_variants(&core_src, "WalletError");
        for (variant, kind, code) in FOLDED_INTO_KIND {
            assert!(
                core_variants.iter().any(|v| v == variant),
                "stale FOLDED_INTO_KIND entry: WalletError::{variant} no longer exists"
            );
            let arm = format!("rw::WalletError::{variant} {{ .. }} => K::{kind},");
            assert!(
                convert_src.contains(&arm),
                "FOLDED_INTO_KIND ({variant} → {kind}): the bridge's convert.rs \
                 has no arm `{arm}` — the fold this entry exempts does not exist"
            );
            let code_arm = format!("Self::{variant} {{ .. }} => \"{code}\",");
            assert!(
                core_src.contains(&code_arm),
                "FOLDED_INTO_KIND ({variant}): the core code() table does not \
                 return `{code}` (`{code_arm}`) — the host could not tell it \
                 from the kind it is folded into"
            );
        }
    }

    for (core_file, core_enum, bridge_file, bridge_enum) in MAP {
        let core_src = fs::read_to_string(manifest_dir().join(core_file))
            .unwrap_or_else(|_| panic!("core source {core_file} readable"));
        let bridge_src = fs::read_to_string(bridge_dir().join(bridge_file))
            .unwrap_or_else(|_| panic!("bridge source {bridge_file} readable"));
        let core_variants = enum_variants(&core_src, core_enum);
        let bridge_variants = enum_variants(&bridge_src, bridge_enum);
        assert!(
            !core_variants.is_empty(),
            "no variants parsed for core {core_enum} — scanner broken?"
        );
        for v in &core_variants {
            if FFI_INTERNAL.contains(&(core_enum, v.as_str())) {
                continue;
            }
            if *core_enum == "WalletError" && FOLDED_INTO_KIND.iter().any(|(fv, ..)| fv == v) {
                continue;
            }
            assert!(
                bridge_variants.contains(v),
                "bridge enum {bridge_enum} ({bridge_file}) is missing core \
                 {core_enum}'s variant `{v}` — add the variant + conversion \
                 arm (and regenerate the Dart bindings) in the same change, \
                 or (ONLY for a variant whose core doc pins it as never \
                 crossing the FFI) add it to FFI_INTERNAL above"
            );
        }
    }

    /// The `///` doc text attached to `variant` inside `pub enum {name}`,
    /// joined with single spaces so a marker phrase matches across rustfmt's
    /// line wrap. Attribute lines (`#[…]`) do not break a doc run; any other
    /// non-doc line resets it (a `// ──` section header between variants must
    /// not leak one variant's doc onto the next). Same house-style text-scan
    /// pragmatics as [`enum_variants`].
    fn variant_doc(src: &str, name: &str, variant: &str) -> String {
        let needle = format!("pub enum {name} ");
        let start = src
            .find(&needle)
            .unwrap_or_else(|| panic!("pub enum {name} not found"));
        let mut doc: Vec<&str> = Vec::new();
        for raw in src[start..].lines() {
            let code = raw.trim();
            if let Some(text) = code.strip_prefix("///") {
                doc.push(text.trim());
                continue;
            }
            if code.starts_with("#[") || code.is_empty() {
                continue;
            }
            let ident: String = code
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if ident == variant {
                return doc.join(" ");
            }
            doc.clear();
        }
        String::new()
    }
}

/// Variant names of `pub enum {name}` in `src` — a text scan, good
/// enough because both crates follow the house style (one variant per
/// line, attributes on their own lines).
///
/// THE ONE PARSER both bridge gates read variants with. wrote a second
/// one for `bridge_conversion_arms_name_every_core_variant` and inverted a
/// single step — it updated the brace depth BEFORE reading the identifier, so
/// every MULTI-LINE struct variant (`Unanswered { runtime }` and three of its
/// siblings) was skipped and the new gate asserted nothing about them. A watch
/// caught it: deleting the `Unanswered` conversion arm SURVIVED. Two parsers
/// for one predicate is the duplicate-is-a-bug rule, and this is what it costs.
fn enum_variants(src: &str, name: &str) -> Vec<String> {
    let needle = format!("pub enum {name} ");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("pub enum {name} not found"));
    let mut depth = 0usize;
    let mut entered = false;
    let mut variants = Vec::new();
    for raw in src[start..].lines() {
        let code = raw.split("//").next().unwrap_or("").trim();
        // attribute lines can carry braces inside strings
        // (`#[error("…{reason}")]`) — never count those
        if code.starts_with('#') || code.is_empty() {
            continue;
        }
        if entered && depth == 1 {
            let ident: String = code
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if ident.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                variants.push(ident);
            }
        }
        for ch in code.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    entered = true;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if entered && depth == 0 {
                        return variants;
                    }
                }
                _ => {}
            }
        }
    }
    variants
}

/// The half [`bridge_enums_cover_core_variants`] could not do, for the pair
/// this stage's privacy claim rides on (arch review MAJOR).
///
/// Declaring a variant on both sides is not conversion. `convert.rs` must carry
/// a NAMED arm for each core variant, or the `_ => Self::Unknown` the compiler
/// forces on a `#[non_exhaustive]` enum swallows a real state and the Dart
/// switch renders it as the cannot-be-verified sentence — a state the core
/// deliberately publishes, arriving at the user as "unknown", with every
/// declaration gate green.
///
/// Scoped to `TorState` and `TorRuntimeKind` ON PURPOSE rather than applied to
/// all ~30 rows of the enum MAP: these two are the ones a privacy claim is read
/// off, they are the two this stage changed, and a gate that greps ~30 impl
/// bodies for a naming convention is a gate whose own failures need debugging.
/// Widen it when another enum earns it, not before.
#[test]
fn bridge_conversion_arms_name_every_core_variant() {
    // (core file, core enum, the OUTBOUND impl whose body must name them all).
    // The body, not the file: the first cut of this gate searched all of
    // convert.rs, and a watch showed it SURVIVED deleting the `Unanswered` arm
    // — the inbound impl names the same variant, so the needle was satisfied by
    // the conversion going the other way. Same shape as `SwapOutcome`'s row
    // above, which reads its two impl bodies for exactly this reason.
    const PAIR: &[(&str, &str, &str)] = &[
        (
            "src/state.rs",
            "TorState",
            "impl From<rw::TorState> for api_state::TorState {",
        ),
        (
            "src/state.rs",
            "TorRuntimeKind",
            "impl From<rw::TorRuntimeKind> for api_state::TorRuntimeKind {",
        ),
    ];
    let convert = fs::read_to_string(bridge_dir().join("src/convert.rs"))
        .expect("bridge source src/convert.rs readable");
    for (core_file, core_enum, header) in PAIR {
        let core_src = fs::read_to_string(manifest_dir().join(core_file))
            .unwrap_or_else(|_| panic!("core source {core_file} readable"));
        let variants = enum_variants(&core_src, core_enum);
        assert!(
            !variants.is_empty(),
            "no variants parsed for core {core_enum} — scanner broken?"
        );
        let start = convert.find(header).unwrap_or_else(|| {
            panic!("`{header}` no longer exists in convert.rs — this gate has no impl to read")
        });
        let body_end = convert[start..]
            .find("\n}\n")
            .map(|i| start + i)
            .expect("the impl closes");
        let body = &convert[start..body_end];
        for variant in &variants {
            let arm = format!("rw::{core_enum}::{variant}");
            assert!(
                body.contains(&arm),
                "`{header}` has no `{arm}` arm: the bridge DECLARES this variant (the \
                 declaration gate is green) but converts it through the `_ => Self::Unknown` \
                 wildcard that `#[non_exhaustive]` forces, so a real {core_enum} reaches Dart \
                 as `Unknown` and renders as the cannot-be-verified sentence"
            );
        }
        // Anti-vacuity: an impl body that names nothing would pass every
        // assertion above by having nothing to fail.
        assert!(
            body.matches(&format!("rw::{core_enum}::")).count() >= variants.len(),
            "`{header}` names fewer arms than {core_enum} has variants — the scanner is not \
             reading the impl it thinks it is. Body:\n{body}"
        );
    }
}

/// The SIBLING of [`bridge_enums_cover_core_variants`] for DTO STRUCTS — the
/// one the `truth` contract (§3.2a) said "applies" and that did not exist:
/// every row of the enum gate's `MAP` is an ENUM, and nothing in the tree
/// compared a bridge struct's fields with its core original.
///
/// What that left open, at the DTO whose whole promise is that it cannot
/// disagree with a log line: `ArmCounts` is one field per `outcome` value
/// `wallet.dial` prints. A ninth `DialError` variant forces a ninth core
/// field (the exhaustive matches in `log_dial` and `ArmTally::record` see to
/// that), and `convert.rs` builds the bridge struct from eight NAMED field
/// reads — so the conversion would still compile, the bridge DTO would keep
/// eight fields, `just ci` would be green, and the host's counters would
/// quietly stop summing to the lines beside them. A compile error at the
/// bridge is NOT available: the core structs are `#[non_exhaustive]`, so the
/// bridge crate cannot destructure them exhaustively even if it wanted to.
/// This is the gate instead.
///
/// It is a text scan, like its enum sibling and for the same reason (both
/// crates follow the house style: one `pub` field per line). The comparison
/// is by NAME, in order, and it is BOTH ways — a bridge field with no core
/// original is as much a lie as a core field the bridge never learned. TYPES
/// are deliberately not compared: `u64` → `i64` is the sanctioned Dart-int
/// crossing (`an_out_of_range_dial_count_saturates_instead_of_going_negative`
/// owns its correctness).
#[test]
fn bridge_structs_mirror_core_fields() {
    // (core file, core struct, bridge file, bridge struct)
    const MAP: &[(&str, &str, &str, &str)] = &[
        // FR-37 (stage S1 `truth`): the dial counters. The DTO's own doc says
        // the fields are "spelled the same, so the number and the line cannot
        // disagree" — this is what makes that a gate instead of a sentence.
        (
            "src/state.rs",
            "DialCounts",
            "src/api/state.rs",
            "DialCounts",
        ),
        ("src/state.rs", "ArmCounts", "src/api/state.rs", "ArmCounts"),
    ];

    for (core_file, core_struct, bridge_file, bridge_struct) in MAP {
        let core_src = fs::read_to_string(manifest_dir().join(core_file))
            .unwrap_or_else(|_| panic!("core source {core_file} readable"));
        let bridge_src = fs::read_to_string(bridge_dir().join(bridge_file))
            .unwrap_or_else(|_| panic!("bridge source {bridge_file} readable"));
        let core_fields = struct_fields(&core_src, core_struct);
        let bridge_fields = struct_fields(&bridge_src, bridge_struct);
        assert!(
            !core_fields.is_empty(),
            "no fields parsed for core {core_struct} — scanner broken?"
        );
        assert_eq!(
            core_fields, bridge_fields,
            "the bridge DTO {bridge_struct} ({bridge_file}) no longer mirrors core \
             {core_struct} ({core_file}) field for field, in order — add the field \
             and its conversion arm (and regenerate the Dart bindings) in the same \
             change. A DTO that promises the host its numbers match the SDK's own \
             log lines cannot quietly carry one fewer"
        );
    }

    /// Field names of `pub struct {name}` in `src`, in declaration order — a
    /// text scan, good enough because both crates follow the house style (one
    /// `pub` field per line, attributes and docs on their own lines). Only
    /// `pub` fields count: a private field cannot cross the FFI and has
    /// nothing to mirror.
    fn struct_fields(src: &str, name: &str) -> Vec<String> {
        let needle = format!("pub struct {name} ");
        let start = src
            .find(&needle)
            .unwrap_or_else(|| panic!("pub struct {name} not found"));
        let mut fields = Vec::new();
        for raw in src[start..].lines().skip(1) {
            let code = raw.split("//").next().unwrap_or("").trim();
            if code.is_empty() || code.starts_with('#') {
                continue;
            }
            if code.starts_with('}') {
                break;
            }
            if let Some(rest) = code.strip_prefix("pub ") {
                fields.push(
                    rest.chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect(),
                );
            }
        }
        fields
    }
}

/// T0-1b D11 (`docs/plan/production-readiness-phase-1.md` §4c, "The
/// Sapling/Orchard question, DECIDED"): whatever `StallReason` variant the
/// bridge mirrors for a required-pool height violation carries no endpoint
/// host, URL or index, no height, no note and no address. This is the SHAPE
/// half of D6/D11 — a payload field named `host` with an empty value passes
/// the value-side checks in `degraded_pool_proof.rs` and is still a D11
/// violation — and it is the only row in the crate that can see the bridge
/// enum, which is why it lives with the extraction gate (D11: cite
/// `extraction_policy.rs`, not §5.4). The refusal `code` MAY cross by the
/// contract's own wording, so it is deliberately not on the list.
///
/// Anti-vacuity: the extractor must find the six variants the bridge enum
/// carries today before its "no such field" claim counts — an extractor that
/// missed the enum body would pass on nothing.
///
/// GREEN at `a02a33e6` by construction (every variant is a unit variant); it
/// guards the shape of what the implementer adds. Test-author row (blind
/// half), append-only.
#[test]
fn stall_reason_bridge_variants_carry_no_endpoint_or_chain_datum() {
    let src = fs::read_to_string(bridge_dir().join("src/api/state.rs"))
        .expect("bridge state.rs readable");
    let start = src
        .find("pub enum StallReason")
        .expect("the bridge mirrors StallReason (see bridge_enums_cover_core_variants)");

    // The enum body, comments stripped so prose ("Endpoint unreachable.")
    // cannot trip a field check, cut at the brace that closes the enum.
    let mut depth = 0usize;
    let mut entered = false;
    let mut closed = false;
    let mut body = String::new();
    'lines: for raw in src[start..].lines() {
        let code = raw.split("//").next().unwrap_or("").trim();
        if code.is_empty() {
            continue;
        }
        body.push_str(code);
        body.push('\n');
        for ch in code.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    entered = true;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if entered && depth == 0 {
                        closed = true;
                        break 'lines;
                    }
                }
                _ => {}
            }
        }
    }
    assert!(
        closed,
        "extractor: the bridge StallReason body never closed:\n{body}"
    );
    for v in [
        "EndpointUnreachable",
        "TorUnavailable",
        "StorageFull",
        "ChainReorg",
        "Internal",
        "Unknown",
    ] {
        assert!(
            body.contains(v),
            "extractor anti-vacuity: the bridge StallReason body must carry `{v}` \
             before its no-such-field claim counts; got:\n{body}"
        );
    }

    // Payload field names: an identifier followed by a single `:` (not `::`).
    let chars: Vec<char> = body.chars().collect();
    let mut fields = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let s = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if j < chars.len() && chars[j] == ':' && chars.get(j + 1) != Some(&':') {
                fields.push(chars[s..i].iter().collect::<String>());
            }
        } else {
            i += 1;
        }
    }
    const FORBIDDEN: &[&str] = &[
        "host", "url", "uri", "endpoint", "index", "height", "note", "address", "addr",
    ];
    for f in &fields {
        let lower = f.to_ascii_lowercase();
        for tok in FORBIDDEN {
            assert!(
                !lower.contains(tok),
                "D11: the bridge StallReason carries a payload field `{f}` that names \
                 `{tok}` — no endpoint host/URL/index, no height, no note, no address \
                 may cross the bridge; body:\n{body}"
            );
        }
    }
}

/// T0-1c E10 (`docs/plan/production-readiness-phase-1.md` §4k): whatever the
/// implementer mints for "this endpoint is behind the binary's own data" — a
/// `SyncStatus` sibling, a `PoolService` arm, or a field on either — carries
/// no endpoint host, URL or index, no note, account or address across the
/// bridge. The SHAPE half of E10, for the two bridge enums the T0-1b row above
/// does not read, plus the report struct they carry.
///
/// A HEIGHT is deliberately not on this list, unlike the `StallReason` row's:
/// `UpToDate { tip }` already carries one, and E10 permits the newest bundled
/// row — a public constant of the signed binary — to cross if the implementer
/// cites this file. A text scan cannot tell a public constant from a served
/// completion height, so the VALUE half
/// (`degraded_pool_proof::assert_carries_no_fixture_datum`) polices those
/// against every T0-1c row's fixture; this row polices the field NAMES.
///
/// Anti-vacuity: each body must carry the variants (or field) it has today
/// before its no-such-field claim counts. The extractor is the sibling row's,
/// restated here rather than shared, because that row is append-only too; a
/// fold may lift both into one helper.
///
/// GREEN at `44a3496d` by construction; guards the shape of what the
/// implementer adds. Test-author row (blind half), append-only.
#[test]
fn sync_status_and_pool_service_bridge_variants_carry_no_endpoint_datum() {
    let src = fs::read_to_string(bridge_dir().join("src/api/state.rs"))
        .expect("bridge state.rs readable");
    const FORBIDDEN: &[&str] = &[
        "host", "url", "uri", "endpoint", "index", "note", "account", "address", "addr",
    ];
    let bodies: [(&str, &[&str]); 3] = [
        (
            "pub enum SyncStatus",
            // T0-1c-R2 (§4m code reviewer #15): `EndpointBehind` named, so a
            // rename or drop of the variant this row exists to police fails
            // the anti-vacuity clause instead of leaving the row green.
            &["UpToDateDegraded", "EndpointBehind", "Stalled", "Unknown"],
        ),
        ("pub enum PoolService", &["Withheld", "Served", "Unknown"]),
        ("pub struct PoolServiceReport", &["ironwood"]),
    ];
    for (needle, must_carry) in bodies {
        let body = brace_body(&src, needle);
        for v in must_carry {
            assert!(
                body.contains(v),
                "extractor anti-vacuity: the bridge `{needle}` body must carry `{v}` \
                 before its no-such-field claim counts; got:\n{body}"
            );
        }
        for f in payload_field_names(&body) {
            let lower = f.to_ascii_lowercase();
            for tok in FORBIDDEN {
                assert!(
                    !lower.contains(tok),
                    "E10: the bridge `{needle}` carries a payload field `{f}` that names \
                     `{tok}` — no endpoint host/URL/index, no note, account or address \
                     may cross the bridge for a 'behind' datum; body:\n{body}"
                );
            }
        }
    }

    /// The brace-delimited body that follows `needle`, comments stripped so
    /// prose cannot trip a field check, cut at the brace that closes it.
    fn brace_body(src: &str, needle: &str) -> String {
        let start = src.find(needle).unwrap_or_else(|| {
            panic!("the bridge mirrors `{needle}` (see bridge_enums_cover_core_variants)")
        });
        let mut depth = 0usize;
        let mut entered = false;
        let mut body = String::new();
        for raw in src[start..].lines() {
            let code = raw.split("//").next().unwrap_or("").trim();
            if code.is_empty() {
                continue;
            }
            body.push_str(code);
            body.push('\n');
            for ch in code.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        entered = true;
                    }
                    '}' => {
                        depth = depth.saturating_sub(1);
                        if entered && depth == 0 {
                            return body;
                        }
                    }
                    _ => {}
                }
            }
        }
        panic!("extractor: the bridge `{needle}` body never closed:\n{body}");
    }

    /// Payload field names: an identifier followed by a single `:` (not `::`).
    fn payload_field_names(body: &str) -> Vec<String> {
        let chars: Vec<char> = body.chars().collect();
        let mut fields = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
                let s = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let mut j = i;
                while j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                if j < chars.len() && chars[j] == ':' && chars.get(j + 1) != Some(&':') {
                    fields.push(chars[s..i].iter().collect::<String>());
                }
            } else {
                i += 1;
            }
        }
        fields
    }
}

/// T0-1d (a) (`docs/plan/production-readiness-phase-1.md` §4l, decision 1 —
/// both shapes): the reason the wallet publishes for a root conflict at a
/// recorded address names the exit that actually clears it, the RESCAN
/// rebuild.
///
/// `sync_bind_proof::a_first_contact_poison_is_not_blamed_on_every_later_honest_server_forever`
/// proves the exit works, and its planted sibling proves a server switch does
/// not; neither can see the sentence a host author reads. At the contract
/// commit `StallReason::EndpointMisbehaving`'s doc says the honest next step
/// IS "switch servers" and the enum body never mentions a rescan — and for the
/// first-contact-poison cell that remedy is measured false (the poison is a
/// root THIS wallet recorded from a previous server). Whether the exit rides
/// `EndpointMisbehaving`'s doc or a distinct reason's, §4l decision 1 has it
/// "written into the state doc"; this row polices that the `StallReason` body
/// carries the word, and nothing about the wording beyond the word.
///
/// Extractor anti-vacuity: the body must carry `EndpointMisbehaving` and
/// `Internal` before its claim counts. RED at `1498a91a` (no "rescan" in the
/// body). Test-author row (blind half), append-only. A join that names the
/// exit ONLY in UI copy and not in the state doc is the adjudicator's call
/// against decision 1's wording, not this row's to relax.
#[test]
fn the_published_reason_for_a_local_root_conflict_names_the_rescan_exit() {
    let src =
        fs::read_to_string(manifest_dir().join("src/state.rs")).expect("core state.rs readable");
    let start = src
        .find("pub enum StallReason")
        .expect("core state.rs declares StallReason");
    // The body: from the declaration to the brace that closes it at column 0,
    // doc comments KEPT — the doc is what is policed.
    let body: String = src[start..]
        .lines()
        .take_while(|l| l.trim_end() != "}")
        .collect::<Vec<_>>()
        .join("\n");
    for v in ["EndpointMisbehaving", "Internal"] {
        assert!(
            body.contains(v),
            "extractor anti-vacuity: the `StallReason` body must carry `{v}` before its \
             claim counts; got:\n{body}"
        );
    }
    assert!(
        body.to_ascii_lowercase().contains("rescan"),
        "T0-1d (a): a root conflict at a recorded address is refused because of a root THIS \
         wallet recorded from an earlier server, and the only operation that clears it is \
         the rescan rebuild — yet no reason on the `StallReason` body names a rescan, and \
         the reason published today says the honest next step is \"switch servers\", \
         which the switch plant measures false for this cell. Write the exit into the \
         doc of the reason that carries it (§4l decision 1). Body:\n{body}"
    );
}

/// T0-1c-R2 G7 (`docs/plan/production-readiness-phase-1.md` §4n): the
/// sentences M1–M4 falsified are gone from the docs, and the one §4n asks for
/// is there. Four scoped bodies, one clause each, none about wording beyond the
/// phrase:
///
/// 1. `SyncStatus::EndpointBehind`'s doc (core `state.rs`) no longer says the
///    variant is held "nowhere durable" — M2: `record_synced` runs before the
///    ranking, so a behind pass sets `ever_synced` and stamps the behind height.
/// 2. `sync::tip_standing`'s doc no longer says closing the residual needs "a
///    second oracle" — M1: the wallet's own scanned height is read on every
///    pass already — and no longer gives the equality rule the reason that
///    "every honest, current endpoint's tip is the row" — false by P3 (the tail
///    is ~17 k blocks below the chain at release).
/// 3. `Wallet::evaluate_consensus`'s doc + body name `capable_tip` — M4:
///    decision 5's sentence there enumerated only the verdict while the same
///    identity height built the persisted signing anchor. Its precheck token
///    is `grace_anchor` — where the anchor is BUILT: the implementer moved the
///    clamp `min(claimed, scanned + REORG_MAX_BLOCKS)` out of this fn into
///    `consensus::grace_anchor`, so the constant that stood here at the T0-1c-R2
///    contract commit is no longer a token of THIS body (`TEST_WRONG` in the
///    T0-1c-R2 ruling; re-aimed at T0-1c-R3, §4n-R R3-4) — and the margin is
///    asserted where it now lives.
/// 4. `net::grpc::transport_err`'s doc carries the contradiction ON THE RECORD
///    — §4m #7: a `PermissionDenied` from an absent credential pair renders
///    through it as a dead link, which contradicts the class sentence beside
///    it. The clause this row first wrote — that the fn "no longer says 'the
///    link demonstrably worked'" — was `CONTRACT_WRONG` in the T0-1c-R2 ruling
///    (§4n paraphrased §4m #7's *contradicts* into *falsified*): the sentence
///    is TRUE at source (`classify_status` routes every transport-origin status
///    to `Transport`, so a `Status` is by construction a server that answered)
///    and stays. What the doc owes instead is to say what the code DOES and
///    where the honest arm is owed (§4k-R-run owed 4); re-aimed at T0-1c-R3,
///    §4n-R R3-4.
///
/// **IT-15, scoped:** a correction that must QUOTE a forbidden phrase in order
/// to disown it is exempt when the same line carries a correction marker
/// (`false`, `wrong`, `no longer`, `superseded`, `refuted`, `used to`). Named
/// and scoped to the bodies `forbid` polices, never a blanket; a staleness
/// hiding behind a marker is the adjudicator's to read.
///
/// Extractor anti-vacuity: each body must carry a token it has at the contract
/// commit before its claim counts. RED at `1fee0f36` on clauses 1–4 as first
/// written (measured at source before this row was written: `state.rs:192`,
/// `sync.rs:2244` and `:2305`, no `capable_tip` in `evaluate_consensus`,
/// `grpc.rs:811`). Clauses 3 and 4 as re-aimed are measured against the
/// T0-1c-R2 join in the T0-1c-R3 test author's report.
/// Test-author row (blind half), append-only; the R3 edit is the one §4n-R
/// permits its author.
#[test]
fn the_sentences_t0_1c_r2_falsified_are_gone_from_the_docs() {
    const CORRECTION_MARKERS: &[&str] = &[
        "false",
        "wrong",
        "no longer",
        "superseded",
        "refuted",
        "used to",
    ];

    // (1) the behind variant's doc.
    let state = read_core("src/state.rs");
    let behind_doc = doc_block_above(&state, |l| l == "    EndpointBehind {");
    must_carry(
        &behind_doc,
        "newest_known",
        "SyncStatus::EndpointBehind's doc",
    );
    forbid(
        &behind_doc,
        "nowhere durable",
        "M2: `emit_synced` calls `record_synced(tip)` BEFORE the ranking, so a behind pass \
         durably sets `ever_synced` and stamps the behind height — \"nowhere durable\" is \
         false for this variant",
        CORRECTION_MARKERS,
    );

    // (2) the grade's doc.
    let sync = read_core("src/sync.rs");
    let grade_doc = doc_block_above(&sync, |l| l.starts_with("pub(crate) fn tip_standing"));
    must_carry(&grade_doc, "AtOrAboveBundle", "sync::tip_standing's doc");
    forbid(
        &grade_doc,
        "second oracle",
        "M1: the residual is closable against the wallet's own scanned height, read on every \
         pass already — no second oracle, no new state, no new RPC",
        CORRECTION_MARKERS,
    );
    forbid(
        &grade_doc,
        "every honest, current endpoint's tip is the row",
        "§4m #6: false by P3 — the tail is ~17 k blocks below the chain at release; the \
         decision is right, the reason is not, and the same reason would justify slack",
        CORRECTION_MARKERS,
    );

    // (3) decision 5's sentence names the signing anchor.
    let wallet = read_core("src/wallet.rs");
    let evaluate = fn_with_doc(&wallet, "async fn evaluate_consensus", "    }");
    // provenance: the T0-1c-R2 join (the implementer's M4, `1fa92990`), read at
    // the T0-1c-R3 base — `grace_anchor` is where the anchor is BUILT, the one
    // name that leaves this body only if the anchor does (§4n-R R3-4).
    must_carry(&evaluate, "grace_anchor", "Wallet::evaluate_consensus");
    assert!(
        evaluate.contains("capable_tip"),
        "M4: `evaluate_consensus` builds the persisted signing anchor from the same identity \
         height its decision-5 sentence calls harmless — the sentence enumerates only the \
         verdict. Name `capable_tip` where the height is read. Body:\n{evaluate}"
    );
    // …and the margin, asserted where it lives now: the two-sided clamp in
    // `consensus::grace_anchor` (the T0-1c-R2 ruling §2 G4).
    let consensus = read_core("src/consensus.rs");
    let anchor = fn_with_doc(&consensus, "pub(crate) fn grace_anchor(", "}");
    must_carry(&anchor, "REORG_MAX_BLOCKS", "consensus::grace_anchor");

    // (4) the transport mapper's comment says what the code DOES.
    let grpc = read_core("src/net/grpc.rs");
    let transport = fn_with_doc(&grpc, "fn transport_err", "}");
    must_carry(&transport, "TorUnavailable", "net::grpc::transport_err");
    // provenance: the T0-1c-R2 join (the implementer's G7 commit `66a0f9bf`),
    // read at the T0-1c-R3 base; the ruling verified the paragraph at source.
    // The "demonstrably worked" sentence beside it is TRUE and is not policed.
    assert!(
        transport.contains("PermissionDenied") && transport.contains("owed"),
        "G7 clause 4 (re-aimed, §4n-R R3-4): `transport_err`'s doc must carry the \
         contradiction ON THE RECORD — a `PermissionDenied` (an answered link) is mapped to \
         `EndpointUnreachable` (a dead one), and the honest arm is OWED to its own row — \
         rather than a comment that pretends the mapping is right. Body:\n{transport}"
    );

    fn read_core(rel: &str) -> String {
        fs::read_to_string(manifest_dir().join(rel))
            .unwrap_or_else(|e| panic!("core {rel} readable: {e}"))
    }

    /// The contiguous `///` block immediately above the first line `is_anchor`
    /// accepts, doc markers kept — the doc is what is policed.
    fn doc_block_above(src: &str, is_anchor: impl Fn(&str) -> bool) -> String {
        let lines: Vec<&str> = src.lines().collect();
        let at = lines
            .iter()
            .position(|l| is_anchor(l.trim_end()))
            .expect("the anchored declaration exists");
        let mut start = at;
        while start > 0 && lines[start - 1].trim_start().starts_with("///") {
            start -= 1;
        }
        assert!(
            start < at,
            "extractor: the declaration at line {} carries a doc block",
            at + 1
        );
        lines[start..at].join("\n")
    }

    /// From the doc block above the first line containing `needle` through the
    /// first later line that is exactly `end` (the fn's closing brace at its
    /// indent) — doc + signature + body, so a sentence may live in either.
    fn fn_with_doc(src: &str, needle: &str, end: &str) -> String {
        let lines: Vec<&str> = src.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("`{needle}` exists"));
        let mut start = at;
        while start > 0 && lines[start - 1].trim_start().starts_with("///") {
            start -= 1;
        }
        let close = (at..lines.len())
            .find(|&i| lines[i].trim_end() == end)
            .unwrap_or_else(|| panic!("`{needle}` closes with `{end}`"));
        lines[start..=close].join("\n")
    }

    fn must_carry(body: &str, token: &str, what: &str) {
        assert!(
            body.contains(token),
            "extractor anti-vacuity: {what} must carry `{token}` before its claim counts; \
             got:\n{body}"
        );
    }

    /// No line of `body` carries `phrase` unless it also carries a correction
    /// marker (IT-15, scoped).
    fn forbid(body: &str, phrase: &str, why: &str, markers: &[&str]) {
        for line in body.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains(phrase) && !markers.iter().any(|m| lower.contains(m)) {
                panic!(
                    "G7: the doc still claims \"{phrase}\" — {why}. Line:\n{line}\nBody:\n{body}"
                );
            }
        }
    }
}

/// SCAN-1 (§4o S5): the device-timing door — `cfg(zec_wallet_device_timing)`,
/// under which `init_app` installs the logcat layer in a RELEASE `.so` — is
/// CLOSED in every default build. Door (b) of the contract: the cfg is emitted
/// by the bridge's `build.rs` only inside a branch gated on
/// `ZEC_WALLET_DEVICE_TIMING`, once; it is declared to `check-cfg` and
/// re-evaluated on `rerun-if-env-changed`; no cargo feature opens it (so no
/// `default` list can); `init_app` has ONE layer-install site (the debug door
/// and the measurement door share it, never a copy); and the only recipe in the
/// tree that sets the variable is `wallet-device-timing-apk`. String-scan shape,
/// like `core_and_bridge_have_no_relim_deps`.
///
/// Mutants this row was watched against (S6): the `rustc-cfg` println moved
/// below the `if`'s closing brace (unconditional — the door open in every
/// build) → the brace-depth walk finds it at depth 0, red; the `if` removed →
/// red; a second unconditional emission added → the count is 2, red; a cargo
/// feature named for the timing → red; the variable set in a second recipe →
/// red. NOT caught, and said so: a condition rewritten to be always-true while
/// it still reads the variable (`|| true`) — a string scan cannot evaluate it;
/// the review reads that line. Also NOT caught here: an exported variable
/// arming a Flutter RELEASE build — that is the `CARGOKIT_CONFIGURATION` /
/// `PROFILE` refusal in `build.rs` (review item 2), which this scan only
/// checks is PRESENT between the gate and the emission; and whether a
/// door-closed artifact really carries no layer — that is the negative artifact
/// witness, `just wallet-device-timing-negative-witness`.
#[test]
fn the_device_timing_door_is_closed_in_every_default_build() {
    const CFG: &str = "zec_wallet_device_timing";
    const ENV: &str = "ZEC_WALLET_DEVICE_TIMING";

    // build.rs: emitted once, inside the env-gated `if`, declared, re-evaluated.
    let build_rs =
        fs::read_to_string(bridge_dir().join("build.rs")).expect("bridge build.rs readable");
    let lines: Vec<&str> = build_rs.lines().collect();
    let emit = format!("cargo:rustc-cfg={CFG}");
    let emits: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(&emit))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        emits.len(),
        1,
        "the door is emitted from exactly ONE line of build.rs; found at lines {:?}",
        emits.iter().map(|i| i + 1).collect::<Vec<_>>()
    );
    let i_emit = emits[0];
    assert!(
        build_rs.contains(&format!("cargo:rustc-check-cfg=cfg({CFG})")),
        "the cfg is declared to check-cfg (unexpected_cfgs stays quiet on every target)"
    );
    assert!(
        build_rs.contains(&format!("cargo:rerun-if-env-changed={ENV}")),
        "flipping the variable must rebuild the crate"
    );
    let i_if = (0..i_emit)
        .rev()
        .find(|&i| {
            let l = lines[i].trim_start();
            l.starts_with("if ") && l.contains("env::var") && l.contains(ENV)
        })
        .unwrap_or_else(|| {
            panic!(
                "no `if … env::var(\"{ENV}\") …` precedes the emission at line {}",
                i_emit + 1
            )
        });
    // Brace depth from the `if` line (inclusive) to the emission (exclusive): a
    // `}` in between would have closed the branch and made the door unconditional.
    let depth: i64 = lines[i_if..i_emit]
        .iter()
        .map(|l| l.matches('{').count() as i64 - l.matches('}').count() as i64)
        .sum();
    assert!(
        depth >= 1,
        "the emission at line {} is not inside the `if` at line {} (brace depth {depth})",
        i_emit + 1,
        i_if + 1
    );
    // The profile guard sits between the gate and the emission: a release
    // configuration with the variable set fails the build instead of arming it.
    let guarded = &lines[i_if..i_emit];
    assert!(
        guarded.iter().any(|l| l.contains("CARGOKIT_CONFIGURATION"))
            && guarded
                .iter()
                .any(|l| l.contains("Some(\"release\") => panic!"))
            && guarded
                .iter()
                .any(|l| l.contains("profile == \"release\" => panic!")),
        "the env-gated branch must refuse a release configuration (cargokit's and a bare cargo one) before emitting the cfg"
    );

    // Cargo.toml: no feature opens the door, so no `default` list can.
    let manifest =
        fs::read_to_string(bridge_dir().join("Cargo.toml")).expect("bridge manifest readable");
    let mut in_features = false;
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            in_features = line == "[features]";
            continue;
        }
        if in_features {
            assert!(
                !line.to_ascii_lowercase().contains("timing"),
                "a cargo feature must not open the device-timing door: {line:?}"
            );
        }
    }

    // meta.rs: one install site, gated on the cfg name.
    let meta = fs::read_to_string(bridge_dir().join("src/api/meta.rs")).expect("meta.rs readable");
    assert_eq!(
        meta.matches("paranoid_android::layer(").count(),
        1,
        "ONE logcat-layer install site, shared by the debug and measurement doors"
    );
    assert!(
        meta.contains(&format!(
            "cfg(all(target_os = \"android\", any(debug_assertions, {CFG})))"
        )),
        "init_app's measurement door is the cfg beside debug_assertions, on Android only"
    );

    // Justfile (pre-extraction only — the recipe leaves with it): the variable
    // is set by exactly one recipe, and it is the measurement one.
    let justfile = manifest_dir().join("../../Justfile");
    if let Ok(justfile) = fs::read_to_string(&justfile) {
        let jl: Vec<&str> = justfile.lines().collect();
        // a `#` comment naming the variable is documentation, not a setter
        let setters: Vec<usize> = jl
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.trim_start().starts_with('#') && l.contains(&format!("{ENV}=")))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            setters.len(),
            1,
            "exactly one Justfile line sets {ENV}; found at lines {:?}",
            setters.iter().map(|i| i + 1).collect::<Vec<_>>()
        );
        let recipe = (0..setters[0])
            .rev()
            .map(|i| jl[i])
            .find(|l| {
                !l.starts_with(' ') && !l.starts_with('#') && !l.is_empty() && l.ends_with(':')
            })
            .expect("the setter sits inside a recipe");
        assert_eq!(
            recipe, "wallet-device-timing-apk:",
            "the variable is set only by the measurement recipe"
        );
    }
}

/// Android 15+ / Play: every Rust `.so` this repo builds links with 16 KB LOAD
/// segments. The example pins NDK r27, which does not default to it, so each
/// crate's `build.rs` passes `-z max-page-size=16384` inside its Android branch.
/// the device walk found the Tor plugin without one (`0x1000`, flagged by
/// Android at launch) — a device found it, no gate did. The plugin sits beside
/// the SDK only in this repository (its own workspace, staged apart), so its
/// half is REQUIRED in a repository — the monorepo keyed on `evals/check.py`,
/// the public repo on the plugin's own directory, neither carried by a staged
/// package — and skipped only outside both: a moved or renamed plugin is a red
/// in the monorepo, never a silent pass. String scan, like the
/// device-timing door above; the artifact itself is measured at release
/// (checklist §B).
///
/// Mutants: the line removed from either build.rs → red naming it; the line
/// moved below the `if`'s closing brace → the brace-depth walk finds it outside
/// the branch → red. Only a `println!(` line counts (a comment quoting the arg
/// cannot stand in for it), and the gate must read `== Ok("android")` (an
/// inverted `!=` gate is refused). NOT caught: a gate rewritten to be never-true
/// by other means — the review reads the condition.
#[test]
fn every_android_library_this_repo_builds_links_16k_aligned() {
    const ARG: &str = "cargo:rustc-link-arg=-Wl,-z,max-page-size=16384";
    let check = |name: &str, build_rs: &str| {
        let lines: Vec<&str> = build_rs.lines().collect();
        let i_emit = lines
            .iter()
            .position(|l| l.trim_start().starts_with("println!(") && l.contains(ARG))
            .unwrap_or_else(|| panic!("{name}: build.rs no longer passes `{ARG}`"));
        let i_if = (0..i_emit)
            .rev()
            .find(|&i| {
                let l = lines[i].trim_start();
                l.starts_with("if ")
                    && l.contains("CARGO_CFG_TARGET_OS")
                    && l.contains("== Ok(\"android\")")
            })
            .unwrap_or_else(|| {
                panic!(
                    "{name}: no `if … CARGO_CFG_TARGET_OS … == Ok(\"android\")` gates the link arg"
                )
            });
        let depth: i64 = lines[i_if..i_emit]
            .iter()
            .map(|l| l.matches('{').count() as i64 - l.matches('}').count() as i64)
            .sum();
        assert!(
            depth >= 1,
            "{name}: the link arg at line {} is outside the Android branch at line {}",
            i_emit + 1,
            i_if + 1
        );
    };

    let bridge =
        fs::read_to_string(bridge_dir().join("build.rs")).expect("bridge build.rs readable");
    check("the bridge (libzec_wallet.so)", &bridge);
    // In a repository (never in a staged package) the plugin must be there:
    // the monorepo is known by `evals/check.py` (so a moved or renamed plugin
    // stays a red here), the public repo by the plugin directory beside the
    // SDK (it carries no `evals/`; stage-7-extraction §3). A staged package
    // has neither.
    let in_monorepo = manifest_dir().join("../../evals/check.py").is_file();
    let in_public_repo = manifest_dir().join("../zec_wallet_tor/rust").is_dir();
    if in_monorepo || in_public_repo {
        let plugin = manifest_dir().join("../zec_wallet_tor/rust/build.rs");
        let plugin = fs::read_to_string(&plugin).unwrap_or_else(|e| {
            panic!(
                "the Tor plugin's build.rs is unreadable at {} ({e}): moved, renamed, or gone — \
                 its .so would ship 4 KB-aligned",
                plugin.display()
            )
        });
        check("the Tor plugin (libzec_wallet_tor.so)", &plugin);
    }
}

/// GRACE-1 G-6 (`docs/plan/production-readiness-phase-1.md` §4p) — the copy
/// row, this file's first arb read. The grace strings the implementer mints
/// (expired by blocks, expired by clock, and the reading while it runs) never
/// say the network was "upgraded" or that the app needs an "update": for a
/// server that merely stopped reporting the network version both are false,
/// and the spec's own promise (`ironwood-nu63-support.md` §6.3, `:630-632`)
/// is "this server will not say what network it is on, so try another one".
/// The next step is "switch servers"; the clock expiry adds "check the device
/// time". `walletSendFaultNetworkUpgrade` / `walletParkedBlockedByNetworkUpgrade`
/// keep saying "update" — they are TRUE for `Unsupported` — and stay reachable
/// from exactly the arms that reach them at the contract commit
/// (`WalletErrorKind_NetworkUpgradeUnsupported` in `send_state.dart`,
/// `blockedByNetworkUpgrade` in `parked_sends_section.dart`), so a fold that
/// routes the new refusal kind or a new parked state onto them fails HERE.
///
/// The grace strings are found by DISTINGUISHABILITY, never by key: an entry
/// whose key carries `grace` or whose value carries "network version" (the
/// maintainer's phrase, G-6's own sentence). The arb precedent for a Dart-side
/// assertion is `zec_wallet_ui/test/features/wallet/send/wallet_send_request_test.dart`;
/// this row lives here because §4p G-6 puts it here (the test author says
/// which — this) and because the arb is read as TEXT, the way every other row
/// in this file reads its sources. E12 in full: every locale carries every
/// grace key (the non-English drafts are flagged for native review, not for
/// absence).
///
/// Anti-vacuity: RED at the contract commit — no such entry exists there, and
/// a pin over an empty set is no pin. The Dart-side shape checks are anchored
/// on counts measured at the contract commit (provenance inline), so a second
/// producer, not a reworded one, is what reds them.
#[test]
fn the_grace_copy_never_says_upgraded_or_update_the_app() {
    let ui = manifest_dir().join("../zec_wallet_ui");
    let read_ui = |rel: &str| {
        fs::read_to_string(ui.join(rel))
            .unwrap_or_else(|e| panic!("zec_wallet_ui {rel} readable: {e}"))
    };
    let arb_entries = |text: &str| -> Vec<(String, String)> {
        // One `"key": "value",` per line; a metadata object (`@key`) spans
        // lines and its inner `"description": …` line does not start with
        // `wallet`, so it is skipped by the prefix.
        text.lines()
            .filter_map(|l| {
                let t = l.trim().trim_end_matches(',');
                let rest = t.strip_prefix('"')?;
                let (key, rest) = rest.split_once('"')?;
                if !key.starts_with("wallet") {
                    return None;
                }
                let rest = rest.trim_start().strip_prefix(':')?.trim_start();
                let value = rest.strip_prefix('"')?.strip_suffix('"')?;
                Some((key.to_owned(), value.to_owned()))
            })
            .collect()
    };
    let is_grace = |k: &str, v: &str| {
        k.to_ascii_lowercase().contains("grace")
            || v.to_ascii_lowercase().contains("network version")
    };

    let en = arb_entries(&read_ui("lib/l10n/wallet_en.arb"));
    // provenance: contract commit — 724 `wallet*` entries parsed (measured).
    assert!(
        en.len() > 500,
        "extractor anti-vacuity: wallet_en.arb parsed ({} entries)",
        en.len()
    );
    // provenance: contract commit `dc78e688`, wallet_en.arb:1708 and :1736 —
    // the two "update" strings, TRUE for Unsupported, which must keep saying it.
    for key in [
        "walletSendFaultNetworkUpgrade",
        "walletParkedBlockedByNetworkUpgrade",
    ] {
        let (_, value) = en
            .iter()
            .find(|(k, _)| k == key)
            .unwrap_or_else(|| panic!("`{key}` still exists — the stale-build copy is kept"));
        assert!(
            value.to_ascii_lowercase().contains("update"),
            "`{key}` keeps saying 'update' — it renders for Unsupported, where that is true: {value}"
        );
        assert!(
            !is_grace(key, value),
            "`{key}` is the stale-build string and must not read as a grace string: {value}"
        );
    }

    let grace: Vec<&(String, String)> = en.iter().filter(|(k, v)| is_grace(k, v)).collect();
    assert!(
        grace.len() >= 2,
        "G-6: the grace copy exists — one reading for 'expired by blocks', one for 'by clock' \
         (and one while it runs); found {grace:?}. At the contract commit there is none: an \
         expired grace renders walletSendFaultNetworkUpgrade ('The Zcash network was upgraded \
         and this app needs an update…') and walletParkedBlockedByNetworkUpgrade ('Waiting for \
         an app update…'), both false for a server that merely went silent"
    );
    for (k, v) in &grace {
        let lower = v.to_ascii_lowercase();
        for forbidden in [
            "upgraded",
            "update the app",
            "app update",
            "needs an update",
            "needs updating",
            "update this app",
            "update your app",
        ] {
            assert!(
                !lower.contains(forbidden),
                "G-6: `{k}` says \"{forbidden}\" — for a server that stopped reporting the \
                 network version nothing was upgraded and an update fixes nothing: {v}"
            );
        }
    }
    let some_grace_says = |needles: &[&str]| {
        grace.iter().any(|(_, v)| {
            let l = v.to_ascii_lowercase();
            needles.iter().all(|n| l.contains(n))
        })
    };
    assert!(
        some_grace_says(&["switch", "server"]),
        "G-6: the next step is 'switch servers' — the spec's §6.3 promise, delivered; got {grace:?}"
    );
    assert!(
        some_grace_says(&["device", "time"]) || some_grace_says(&["device", "clock"]),
        "G-6: the clock expiry says 'check the device time' — the one next step a block count \
         cannot suggest; got {grace:?}"
    );
    // E12 in full: every locale carries every grace key.
    let l10n = ui.join("lib/l10n");
    let mut locales = 0;
    for entry in fs::read_dir(&l10n).expect("l10n readable") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if !name.starts_with("wallet_") || !name.ends_with(".arb") || name == "wallet_en.arb" {
            continue;
        }
        locales += 1;
        let keys: Vec<String> = arb_entries(&fs::read_to_string(&path).expect("locale readable"))
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        for (k, _) in &grace {
            assert!(
                keys.contains(k),
                "E12: `{name}` lacks the grace key `{k}` — every locale carries every grace string"
            );
        }
    }
    // provenance: contract commit, 16 locales beside en.
    assert!(
        locales >= 15,
        "extractor anti-vacuity: the locale sweep saw {locales} non-English arb files"
    );

    // send_state.dart: every production of the stale-build reason sits under the
    // stale-build kind's arm. provenance: contract commit — two productions
    // (:577 the propose map, :733 the sign-time arm); the enum member (:467)
    // carries no `SendFaultReason.` prefix and is not a production.
    let send_state = read_ui("lib/features/wallet/send/send_state.dart");
    let lines: Vec<&str> = send_state.lines().collect();
    let mut productions = 0;
    for (i, l) in lines.iter().enumerate() {
        // Adjudication repair (GRACE-1; charged to the test half): a
        // `//` or `///` comment that NAMES the reason is not a production of
        // it — the implementer's `SendServerSilentFault` doc says it "must
        // never fold into [SendFaultReason.networkUpgradeUnsupported]", and
        // this scanner read that sentence as the fold it forbids.
        if l.trim_start().starts_with("//") {
            continue;
        }
        if !l.contains("SendFaultReason.networkUpgradeUnsupported") {
            continue;
        }
        productions += 1;
        let window = lines[i.saturating_sub(3)..=i].join("\n");
        assert!(
            window.contains("WalletErrorKind_NetworkUpgradeUnsupported()"),
            "G-6: `SendFaultReason.networkUpgradeUnsupported` (→ walletSendFaultNetworkUpgrade, \
             'update the app') is produced at send_state.dart:{} from an arm other than \
             WalletErrorKind_NetworkUpgradeUnsupported — the grace refusal must not render as \
             the stale-build one:\n{window}",
            i + 1
        );
    }
    assert!(
        productions >= 1,
        "extractor anti-vacuity: send_state.dart produces the stale-build reason somewhere"
    );
    // form_fault_view.dart: the stale-build copy is selected by exactly one
    // reason arm. provenance: contract commit :196-197.
    let ffv = read_ui("lib/features/wallet/send/form_fault_view.dart");
    let ffv_lines: Vec<&str> = ffv.lines().collect();
    let uses: Vec<usize> = ffv_lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("l10n.walletSendFaultNetworkUpgrade"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        uses.len(),
        1,
        "G-6: form_fault_view.dart renders walletSendFaultNetworkUpgrade from exactly one arm; \
         found at lines {:?}",
        uses.iter().map(|i| i + 1).collect::<Vec<_>>()
    );
    let window = ffv_lines[uses[0].saturating_sub(2)..=uses[0]].join("\n");
    assert!(
        window.contains("SendFaultReason.networkUpgradeUnsupported =>"),
        "G-6: …and that arm is the stale-build reason's:\n{window}"
    );
    // parked_sends_section.dart: 'Waiting for an app update' is selected by the
    // network-upgrade blockage and nothing else. provenance: contract commit :652.
    let parked = read_ui("lib/features/wallet/parked_sends_section.dart");
    let parked_lines: Vec<&str> = parked.lines().collect();
    let uses: Vec<usize> = parked_lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("l10n.walletParkedBlockedByNetworkUpgrade"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        uses.len(),
        1,
        "G-6: parked_sends_section.dart renders walletParkedBlockedByNetworkUpgrade from exactly \
         one site; found at lines {:?}",
        uses.iter().map(|i| i + 1).collect::<Vec<_>>()
    );
    let window = parked_lines[uses[0].saturating_sub(1)..=uses[0]]
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        window.contains("networkupgrade"),
        "G-6: …selected by the network-upgrade blockage (the bool today, or a state named for \
         it), never by a grace state:\n{window}"
    );
}

/// `"key": "value",` one per line — the arb shape `flutter gen-l10n` writes.
/// A metadata object (`@key`) spans lines and its inner `"description": …`
/// line does not start with `wallet`, so the prefix filter skips it.
///
/// NOTE (owed): this mirrors the closure inside
/// [`the_grace_copy_never_says_upgraded_or_update_the_app`] verbatim. That row
/// is another round's and stays untouched here; the next change to it should
/// collapse its closure onto this function so the file parses arb ONE way.
fn arb_entries(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let t = l.trim().trim_end_matches(',');
            let rest = t.strip_prefix('"')?;
            let (key, rest) = rest.split_once('"')?;
            if !key.starts_with("wallet") {
                return None;
            }
            let rest = rest.trim_start().strip_prefix(':')?.trim_start();
            let value = rest.strip_prefix('"')?.strip_suffix('"')?;
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// The three keys UI-1 row T-5 re-words, in one place.
const T5_KEYS: [&str; 3] = [
    "walletStallBirthdayInFuture",
    "walletSyncGraceEndedClock",
    "walletParkedBlockedByServerSilentClock",
];

/// `wallet_en.arb`'s `wallet*` entries, LOWERCASED values, with the extractor
/// floored at the count measured at the contract commit — a copy pin over an
/// empty set is no pin. Shared by the three T-5 rows below.
fn en_arb_values() -> Vec<(String, String)> {
    let path = manifest_dir().join("../zec_wallet_ui/lib/l10n/wallet_en.arb");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    let en: Vec<(String, String)> = arb_entries(&text)
        .into_iter()
        .map(|(k, v)| (k, v.to_ascii_lowercase()))
        .collect();
    // provenance: contract commit `53b6993c` — 730 `wallet*` entries parsed.
    assert!(
        en.len() > 500,
        "extractor anti-vacuity: wallet_en.arb parsed ({} entries)",
        en.len()
    );
    en
}

/// Panics BY NAME if the key is gone or its value stopped parsing — a T-5 row
/// can never pass because it silently looked at nothing.
fn en_value(en: &[(String, String)], key: &str) -> String {
    en.iter()
        .find(|(k, _)| k == key)
        .unwrap_or_else(|| panic!("`{key}` exists in wallet_en.arb and parses"))
        .1
        .clone()
}

/// UI-1 row T-5a (`docs/plan/production-readiness-phase-1.md` §4r U-5, from
/// §4n-review row 7) — `StallReason::BirthdayInFuture` has TWO producers: a
/// restore's typed birthday AND `rescan_from(Some(h))`, a height the user
/// picked in the rescan sheet with no restore anywhere in the story. The copy
/// names only the first ("the starting block you entered when restoring"), so
/// the second producer's user is sent looking for a screen they never saw —
/// and the honest next step (change the height, or try another server) is
/// described in words that do not apply to them. The block this wallet is SET
/// TO is the one fact true of both.
///
/// Read as TEXT, the way every other row in this file reads its sources; the
/// sibling copy row `the_grace_copy_never_says_upgraded_or_update_the_app` is
/// the precedent. Findings are ACCUMULATED so one run names every violation.
#[test]
fn the_birthday_stall_names_the_block_the_wallet_is_set_to() {
    let en = en_arb_values();
    let value = en_value(&en, "walletStallBirthdayInFuture");
    let mut problems = Vec::new();

    if !value.contains("set to") {
        problems.push(
            "does not name the block this wallet is SET TO — the fact a \
             rescan-from-height user can act on too"
                .to_owned(),
        );
    }
    for forbidden in ["entered", "restor"] {
        if value.contains(forbidden) {
            problems.push(format!(
                "says \"{forbidden}\" — `rescan_from(Some(h))` is the second producer and that \
                 user entered nothing and restored nothing"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "§4r U-5: `walletStallBirthdayInFuture` names one of its two producers.\n  \
         value: {value}\n  {}",
        problems.join("\n  ")
    );
}

/// UI-1 row T-5b (§4r U-5, from the GRACE-1 fold review's row 6) — a wrong
/// device clock is a PRECONDITION of the grace's time rule, never an
/// alternative fix for it. "Switch servers, or check this device's date and
/// time" reads as a choice: a user who takes the second branch, finds the
/// clock right, and stops has followed the copy exactly and is still refused.
/// §4r U-5 orders them — "If this device's date and time are wrong, fix them
/// first — then switch to a server that reports the network version" — so
/// each sentence carries the server that REPORTS, and never the alternative
/// shape "or check". Both surfaces (the sync detail and the parked row) say
/// the same thing about the same state, by the GRACE-1 no-drift rule.
#[test]
fn the_clock_copy_makes_the_device_time_a_precondition_not_an_alternative() {
    let en = en_arb_values();
    let mut problems = Vec::new();

    for key in [
        "walletSyncGraceEndedClock",
        "walletParkedBlockedByServerSilentClock",
    ] {
        let value = en_value(&en, key);
        if !value.contains("server that reports") {
            problems.push(format!(
                "`{key}`: the next step is a server that REPORTS the network version — the one \
                 thing that lifts the refusal.\n    value: {value}"
            ));
        }
        if value.contains("or check") {
            problems.push(format!(
                "`{key}`: offers checking the clock as an ALTERNATIVE (\"or check\"), so a user \
                 whose clock is already right follows that branch and is still refused.\n    \
                 value: {value}"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "§4r U-5: the clock is a precondition, not a second remedy.\n  {}",
        problems.join("\n  ")
    );
}

/// UI-1 row T-5c (§4r U-5) — the CONTROL: all sixteen locales carry all three
/// re-worded keys. A locale that drops one renders the English fallback for a
/// sentence the other fifteen were re-worded for — the exact split the
/// non-English drafts exist to avoid (they are flagged for native review, not
/// for absence). The E12 shape of the sibling grace row.
/// The picker's copy (`sync-server-picker.md` §8 gate 8): every key the
/// `SyncServerSheet` and the sync sheet's Server row render.
const SYNC_SERVER_KEYS: &[&str] = &[
    "walletSyncServerSheetTitle",
    "walletSyncServerInUse",
    "walletSyncServerAppDefault",
    "walletSyncServerCustom",
    "walletSyncServerCustomHint",
    "walletSyncServerCheck",
    "walletSyncServerChecking",
    "walletSyncServerUse",
    "walletSyncServerSwitching",
    "walletSyncServerContinue",
    "walletSyncServerCancel",
    "walletSyncServerTrustTitle",
    "walletSyncServerTrustNotice",
    "walletSyncServerSwitchNotice",
    "walletSyncServerUnreachable",
    // Stage S1 `copy`: the unreachable arm SPLIT on whether the user typed the
    // address. Both halves are picker copy and both must reach every locale.
    "walletSyncServerUnreachableOffered",
    "walletSyncServerWrongNetwork",
    "walletSyncServerInvalidUrl",
    "walletSyncServerNotOffered",
    "walletSyncServerBusy",
    "walletSyncServerFallbackNotOffered",
    "walletSyncServerFallbackUnreadable",
    "walletSyncServerSwitchFailedRecovered",
    "walletSyncServerRowSemantics",
];

/// `sync-server-picker.md` §8 gate 8 — the `every_locale_carries_the_three_
/// reworded_keys` shape over the picker's 23 keys: every `wallet_*.arb`
/// carries every one (machine copy ships by the maintainer's waiver; a
/// missing key would fall back to English silently under the fallback
/// delegate, which is the drift this row refuses).
#[test]
fn every_locale_carries_the_sync_server_keys() {
    let l10n = manifest_dir().join("../zec_wallet_ui/lib/l10n");
    let mut locales = 0;
    let mut problems = Vec::new();
    for entry in fs::read_dir(&l10n).expect("l10n readable") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if !name.starts_with("wallet_") || !name.ends_with(".arb") {
            continue;
        }
        locales += 1;
        let keys: Vec<String> = arb_entries(&fs::read_to_string(&path).expect("locale readable"))
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        for key in SYNC_SERVER_KEYS {
            if !keys.iter().any(|k| k == key) {
                problems.push(format!("`{name}` lacks `{key}`"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "P3-13 gate 8: every locale carries all {} picker keys.\n  {}",
        SYNC_SERVER_KEYS.len(),
        problems.join("\n  ")
    );
    assert!(
        locales >= 16,
        "extractor anti-vacuity: the locale sweep saw {locales} arb files"
    );
}

/// `sync-server-picker.md` §3.3 — the Dart-visible surface of the picker's
/// DTOs never renders the key: no generated or hand-written `toString` in
/// the bridge's `config.dart` / `config.freezed.dart` mentions `authValue`.
/// (The crypto audit found the promised row had not landed and that the
/// property held only by FRB's default — a plain class with no `toString`.
/// Recorded residual: `SyncServerChoice.custom(url:)` IS freezed and its
/// `toString` interpolates the URL, so a host must never log a choice — the
/// §5.4 rule in Dart form; the row below pins the KEY half.)
#[test]
fn the_dart_sync_server_surface_never_prints_the_key() {
    let dir = bridge_dir().join("../lib/src/rust/api");
    let mut checked = 0;
    for file in ["config.dart", "config.freezed.dart"] {
        let src =
            fs::read_to_string(dir.join(file)).unwrap_or_else(|e| panic!("{file} readable: {e}"));
        assert!(
            src.contains("class SyncServer"),
            "{file} carries the picker's DTOs (anti-vacuity)"
        );
        // Every `toString` body up to its closing brace.
        let mut from = 0;
        while let Some(i) = src[from..].find("String toString(") {
            let start = from + i;
            let end = src[start..]
                .find('}')
                .map(|e| start + e)
                .unwrap_or(src.len());
            let body = &src[start..end];
            assert!(
                !body.contains("authValue"),
                "{file}: a toString renders the key: {body}"
            );
            checked += 1;
            from = end;
        }
    }
    // `SyncServerChoice`/`SyncServerFallback` are freezed and DO carry
    // toString bodies; the plain `SyncServer` class carries none.
    assert!(checked >= 2, "the scan saw {checked} toString bodies");
    // ADR-0568: the user's key rides `SyncServerChoice.custom(key:)` as a
    // PLAIN `SyncServerKey`, so the freezed `custom` toString renders the
    // object, never its fields — which holds only while the plain class has
    // no `toString` of its own.
    let config = fs::read_to_string(dir.join("config.dart")).expect("config.dart readable");
    let start = config
        .find("class SyncServerKey")
        .expect("config.dart carries SyncServerKey (anti-vacuity)");
    let end = config[start + 1..]
        .find("\nclass ")
        .map(|e| start + 1 + e)
        .unwrap_or(config.len());
    assert!(
        !config[start..end].contains("toString"),
        "SyncServerKey grew a toString — it would render the user's key: {}",
        &config[start..end]
    );
}

/// `sync-server-picker.md` §8 gate 4 — ONE source of truth for the reference
/// servers: the Dart reference config's two endpoint constants (the DEFAULT a
/// wallet dials when nothing is chosen) equal the SDK catalog's entries BYTE
/// FOR BYTE. The picker marks the row in use by URL equality, so `https://
/// zec.rocks` and `https://zec.rocks:443` — the same server — would leave the
/// default unmarked and an "App default" row showing beside it. (Amended from
/// the spec's "read from the catalog": the Dart builder must stay pure for the
/// host-VM tests, so it carries a copy and this row pins the copy.)
#[test]
fn the_dart_reference_endpoints_match_the_catalog() {
    use zec_wallet_core::{REFERENCE_SYNC_SERVERS_MAINNET, REFERENCE_SYNC_SERVERS_TESTNET};
    let src = fs::read_to_string(
        manifest_dir().join("../zec_wallet_ui/lib/features/wallet/wallet_config.dart"),
    )
    .expect("wallet_config.dart readable");
    let literal = |name: &str| -> String {
        let needle = format!("const String {name} = '");
        let start = src
            .find(&needle)
            .unwrap_or_else(|| panic!("`{name}` is declared in wallet_config.dart"))
            + needle.len();
        let end = src[start..].find('\'').expect("the literal closes") + start;
        src[start..end].to_owned()
    };
    assert_eq!(
        literal("referenceMainnetEndpoint"),
        REFERENCE_SYNC_SERVERS_MAINNET[0].url,
        "the Dart mainnet default must spell the catalog's zec-rocks entry exactly"
    );
    assert_eq!(
        literal("referenceTestnetEndpoint"),
        REFERENCE_SYNC_SERVERS_TESTNET[0].url,
        "the Dart testnet default must spell the catalog's testnet entry exactly"
    );
}

#[test]
fn every_locale_carries_the_three_reworded_keys() {
    let l10n = manifest_dir().join("../zec_wallet_ui/lib/l10n");
    let mut locales = 0;
    let mut problems = Vec::new();
    for entry in fs::read_dir(&l10n).expect("l10n readable") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if !name.starts_with("wallet_") || !name.ends_with(".arb") {
            continue;
        }
        locales += 1;
        let keys: Vec<String> = arb_entries(&fs::read_to_string(&path).expect("locale readable"))
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        for key in T5_KEYS {
            if !keys.iter().any(|k| k == key) {
                problems.push(format!("`{name}` lacks `{key}`"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "§4r U-5: every locale carries all three re-worded keys.\n  {}",
        problems.join("\n  ")
    );
    // provenance: contract commit `53b6993c`, 16 wallet_*.arb files (en + 15
    // drafts). A floor, not an equality: a seventeenth locale is a feature,
    // and it would have to carry the three keys to pass the loop above.
    assert!(
        locales >= 16,
        "extractor anti-vacuity: the locale sweep saw {locales} arb files"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// FR-5 C2 — the gates that need no plugin (plan §2 C2)
// ─────────────────────────────────────────────────────────────────────────────

/// The one source every non-member package in `sdk/Cargo.lock` may carry.
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// FR-5 spec §8 **P26**: every package in the SDK's lock resolves from a
/// REGISTRY. This is the lock-side twin of `core_and_bridge_have_no_relim_deps`
/// — that test reads the MANIFESTS and refuses a declared path/git dep; this
/// one reads the resolved LOCK, which is the artifact a publish actually
/// depends on and the only place a `[patch]`, a vendor directory or a
/// transitive git dep shows up. ADR-0550 made the plugin consume
/// `sdk/dialer-tor` by path, so the pressure to reach for a patch stanza is
/// real and the lock is where that would land silently.
///
/// The member list is read from `sdk/Cargo.toml`'s `[workspace] members` and
/// mapped path → package name through each member's own manifest (the members
/// are PATHS — `zec_wallet/rust` — while the lock keys on the NAME
/// `zec_wallet`), so a fifth member tomorrow is exempted tomorrow without an
/// edit here. Both anti-vacuity floors the plan asked for are asserted: the
/// member list is non-empty and the lock holds more than 100 packages.
///
/// Watched red (plan §2 C2): a scratch `sdk/.cargo/config.toml` with
/// `[patch.crates-io] hex = { path = … }` + `cargo update -p hex` drops `hex`'s
/// `source` line → this fails naming `hex`. Base restored with
/// `git checkout -- sdk/Cargo.lock` and the config deleted (`sdk/.cargo/` is
/// gitignored — plan §0.3).
#[test]
fn every_sdk_lock_package_resolves_from_a_registry() {
    let sdk = manifest_dir().join("..");
    let members = workspace_member_package_names(&sdk);
    assert!(
        members.len() >= 2 && members.iter().any(|m| m == "zec-wallet-core"),
        "sdk/Cargo.toml's `[workspace] members` did not resolve to package names \
         containing zec-wallet-core (got {members:?}) — every lock entry would then \
         look like a foreign package and this gate would be noise rather than a check"
    );

    let lock = fs::read_to_string(sdk.join("Cargo.lock")).expect("sdk/Cargo.lock readable");
    let mut packages = 0usize;
    let mut unsourced = Vec::new();
    let mut foreign = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let Some(name) = toml_value(block, "name") else {
            continue;
        };
        packages += 1;
        let source = toml_value(block, "source");
        if members.contains(&name) {
            // A workspace member is resolved from the tree by definition; it
            // carries no `source` and must not acquire one.
            if let Some(src) = source {
                foreign.push(format!(
                    "`{name}` is a workspace member yet carries `source = \"{src}\"`"
                ));
            }
            continue;
        }
        match source {
            None => unsourced.push(name),
            Some(src) if src != CRATES_IO => {
                foreign.push(format!("`{name}` resolves from `{src}`"))
            }
            Some(_) => {}
        }
    }

    assert!(
        packages > 100,
        "P26 anti-vacuity: sdk/Cargo.lock parsed to {packages} packages — the split \
         above is not reading the lock it thinks it is"
    );
    assert!(
        unsourced.is_empty(),
        "P26: {} lock package(s) carry NO `source` and are not workspace members, so they \
         resolve from this machine's disk (a `[patch]`, a vendor dir or a path dep). The \
         SDK must be publishable from the lock alone (A1/ADR-0013): {unsourced:?}",
        unsourced.len()
    );
    assert!(
        foreign.is_empty(),
        "P26: the SDK lock resolves {} package(s) from somewhere other than crates.io — a \
         published SDK cannot be built from this lock:\n  {}",
        foreign.len(),
        foreign.join("\n  ")
    );

    // Not vacuous in the other direction either: every member must actually
    // appear in the lock, or the exemption list above is exempting nothing and
    // a member renamed out of the lock would pass unnoticed.
    for member in &members {
        assert!(
            lock.contains(&format!("name = \"{member}\"")),
            "P26: workspace member `{member}` has no `[[package]]` entry in sdk/Cargo.lock"
        );
    }
}

/// The `[workspace] members` of `sdk/Cargo.toml`, as written (member PATHS).
/// Panics rather than returning an empty list: a guard that silently scanned
/// zero members is the shape, and the review's finding was that a
/// hardcoded member list means a fifth crate is policed by nothing.
fn workspace_member_paths(sdk: &Path) -> Vec<String> {
    let root = fs::read_to_string(sdk.join("Cargo.toml")).expect("sdk/Cargo.toml readable");
    let mut in_workspace = false;
    let mut paths: Vec<String> = Vec::new();
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
            paths = inner
                .split(',')
                .map(|m| m.trim().trim_matches('"').to_string())
                .filter(|m| !m.is_empty())
                .collect();
            break;
        }
    }
    assert!(
        !paths.is_empty(),
        "sdk/Cargo.toml has no `[workspace] members = [...]` line — nothing to scan"
    );
    paths
}

/// The members mapped from member PATH to the package NAME their own manifest
/// declares (the members are paths — `zec_wallet/rust` — while the lock keys
/// on the name `zec_wallet`).
fn workspace_member_package_names(sdk: &Path) -> Vec<String> {
    workspace_member_paths(sdk)
        .iter()
        .map(|p| {
            let manifest = fs::read_to_string(sdk.join(p).join("Cargo.toml"))
                .unwrap_or_else(|e| panic!("sdk/{p}/Cargo.toml readable: {e}"));
            toml_value_in_table(&manifest, "package", "name")
                .unwrap_or_else(|| panic!("sdk/{p}/Cargo.toml declares `[package] name`"))
        })
        .collect()
}

/// `key = "value"` anywhere in a lock `[[package]]` block, stopping at the
/// block's own nested tables so a `[package.metadata]` key never shadows it.
fn toml_value(block: &str, key: &str) -> Option<String> {
    for raw in block.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            break; // a nested table inside this block — the package's own keys are above
        }
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// `key = "value"` inside `[table]` of a manifest.
fn toml_value_in_table(manifest: &str, table: &str, key: &str) -> Option<String> {
    let mut in_table = false;
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            in_table = line == format!("[{table}]");
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// FR-5 spec §8 **P28, the source half** (the crypto audit MEDIUM on C1,
/// widened here per plan §2 C2). `wallet-sdk.md` §3.2a now asserts "ONE TLS
/// path for every runtime" unconditionally, and it holds at the code — but it
/// was GATED over one file: T13 (`net/grpc.rs`) pins its own builder by
/// `include_str!`, while `zec-wallet-swap-near/src/http_client.rs` carries a
/// byte-equivalent second `ClientConfig` with NO source pin, and nothing
/// anywhere forbade a third builder site or a `dangerous()` escape.
///
/// This scans every `src/**` file of every SDK workspace member and binds the
/// invariant at the tree, not per-file:
///   * EVERY `ClientConfig::builder*` constructor is read by its full name —
///     rustls 0.23 has four — and only `builder_with_provider` is the SDK's.
///     `builder()` and `builder_with_protocol_versions()` read the PROCESS
///     default (the first calls the second, which calls
///     `get_default_or_install_from_crate_features`); `builder_with_details()`
///     takes a provider but also a custom time source, which decides
///     certificate validity. Any of those three, anywhere, is refused;
///   * the same for `ServerConfig::builder*` (the SDK has no TLS server; only
///     a test helper would build one). This is the guard the two behavioural
///     P28 tests RELY on: each installs a broken process-default provider for
///     the rest of its shared test binary, and rustls has no uninstall — so a
///     test anywhere in `src/**` (test modules included, which this walk
///     reads) that built TLS from the default would fail for an unrelated
///     reason, in an order-dependent way (the code reviewer MINOR on C2);
///   * `ClientConfig::builder_with_provider` appears at EXACTLY the two known
///     sites;
///   * `dangerous(` — rustls's cert/verifier escape hatch — appears nowhere;
///   * the swap builder names `webpki_roots::TLS_SERVER_ROOTS` and
///     `with_no_client_auth()`, as T13 already requires of grpc.
///
/// A source scan has a known limit, stated: an import alias
/// (`use rustls::ClientConfig as Cfg;`) is not read. The behavioural half —
/// beside each builder (`net/grpc.rs`'s
/// `the_sdk_tls_never_reads_the_process_default_provider` and the swap
/// adapter's twin), a foreign provider installed as the process default and the
/// builder still standing up its config from `ring` — covers the two builders
/// whatever they are called.
///
/// Watched red: a third `builder_with_provider` site added in
/// `zec-wallet-core/src/net/grpc.rs`; a planted `dangerous(` in the swap
/// securer; a `builder_with_protocol_versions(` site, which the first cut of
/// this scan (a `builder(` needle) could not see; and a `builder(` call placed
/// after a `https://` string on the same line, which its first comment strip
/// (`split("//")`) cut away; and a `ServerConfig::builder()` test helper.
#[test]
fn the_sdk_tls_stack_has_exactly_two_explicit_builder_sites() {
    let sdk = manifest_dir().join("..");
    // Read from the workspace, never hardcoded: a fifth SDK crate is policed
    // the day it becomes a member (the finding, in pins_policy's words).
    let members = workspace_member_paths(&sdk);
    assert!(
        members.len() >= 2 && members.iter().any(|m| m == "zec-wallet-core"),
        "sdk/Cargo.toml's members did not parse to a list containing zec-wallet-core \
         (got {members:?}) — the walk below would then examine nothing"
    );

    let mut files: Vec<(String, String)> = Vec::new();
    for path in &members {
        collect_rs(
            &sdk.join(path).join("src"),
            &format!("{path}/src"),
            &mut files,
        );
    }
    assert!(
        files.len() > 20,
        "P28 anti-vacuity: the src walk found {} .rs files under the SDK members — it is \
         not reading the tree it thinks it is",
        files.len()
    );

    const CLIENT: &str = "ClientConfig::builder";
    const SERVER: &str = "ServerConfig::builder";
    let mut explicit = Vec::new();
    let mut other_forms = Vec::new();
    let mut dangerous = Vec::new();
    for (label, text) in &files {
        for (n, raw) in text.lines().enumerate() {
            // Line comments are not calls — the rationale comments beside these
            // builders name the forms they are about.
            let line = code_before_line_comment(raw);
            let at = format!("{label}:{}", n + 1);
            for ctor in [CLIENT, SERVER] {
                let mut rest = line;
                while let Some(i) = rest.find(ctor) {
                    let tail = &rest[i + ctor.len()..];
                    let suffix: String = tail
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    match (ctor, suffix.as_str()) {
                        (CLIENT, "_with_provider") => explicit.push(at.clone()),
                        // An explicit-provider SERVER config reads no global.
                        (SERVER, "_with_provider") => {}
                        _ => other_forms.push(format!("{at} (`{ctor}{suffix}`)")),
                    }
                    rest = tail;
                }
            }
            if line.contains("dangerous(") {
                dangerous.push(at);
            }
        }
    }

    assert!(
        other_forms.is_empty(),
        "P28: a `ClientConfig`/`ServerConfig` built by a constructor other than \
         `builder_with_provider`. `builder()` and `builder_with_protocol_versions()` read the \
         PROCESS-DEFAULT `CryptoProvider` — whatever some other crate in the final binary \
         installed first (and, in this crate's test binaries, the broken one the P28 \
         behavioural tests install); `builder_with_details()` adds a custom time source, which \
         decides certificate validity. The SDK's TLS is `builder_with_provider(ring)`, so a \
         host cannot swap the wallet's crypto by winning a race at startup. Found: \
         {other_forms:?}"
    );
    assert!(
        dangerous.is_empty(),
        "P28: rustls's `dangerous()` escape hatch appears in SDK source at {dangerous:?} — \
         that is the door to a custom certificate verifier. The SDK verifies against the \
         WebPKI bundle and nothing else (§3.2a)."
    );
    assert_eq!(
        explicit.len(),
        2,
        "P28: the SDK must have EXACTLY two TLS builder sites — sync (`net/grpc.rs`) and \
         swap (`http_client.rs`) — so \"one TLS path for every runtime\" (§3.2a) is a fact \
         about the tree and not about one pinned file. Found {}: {explicit:?}. A third TLS \
         stack is a deliberate decision: widen this count with the reason.",
        explicit.len()
    );
    assert!(
        explicit.iter().any(|s| s.contains("net/grpc.rs"))
            && explicit.iter().any(|s| s.contains("http_client.rs")),
        "P28: the two builder sites are not the two expected files: {explicit:?}"
    );

    // The swap builder gets the source pin T13 already gives grpc: the same
    // roots, the same no-client-auth, and the explicit ring provider.
    let swap = fs::read_to_string(sdk.join("zec-wallet-swap-near/src/http_client.rs"))
        .expect("the swap adapter's http_client.rs is readable");
    let start = swap
        .find("impl RustlsSecurer {")
        .expect("the swap TLS builder impl");
    let body = &swap[start..start + swap[start..].find("\n}\n").expect("the impl closes")];
    for needle in [
        "webpki_roots::TLS_SERVER_ROOTS",
        "with_no_client_auth()",
        "rustls::crypto::ring::default_provider()",
    ] {
        assert!(
            body.contains(needle),
            "P28: the swap TLS builder must name `{needle}` — it is the SAME stack the sync \
             path uses (§3.2a: WebPKI roots, the pinned `ring` provider, no client auth). \
             Body:\n{body}"
        );
    }
}

/// Every `.rs` file under `dir`, labelled `<member>/<relative path>`. A member
/// with no `src/` contributes nothing (the caller's file-count floor catches a
/// walk that found too little); but an entry or a file that EXISTS and cannot
/// be read panics — a scan that skipped it silently would grade a tree it never
/// read.
fn collect_rs(dir: &Path, label: &str, out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return; // a member with no src/ (or not present post-extraction)
    };
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("{label}: a directory entry is unreadable: {e}"))
            .path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();
        if path.is_dir() {
            collect_rs(&path, &format!("{label}/{name}"), out);
        } else if name.ends_with(".rs") {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{label}/{name} is unreadable: {e}"));
            out.push((format!("{label}/{name}"), text));
        }
    }
}

/// `raw` up to the `//` that opens a line comment. A `//` inside a string
/// literal — a `https://` URL — is not one, so the code after it on the same
/// line is still scanned (the first cut, `split("//")`, cut it away). A
/// heuristic over `"` parity with `\` escapes: a `'"'` char literal flips it,
/// and the failure that buys is a comment scanned as code — a false RED, the
/// loud direction.
fn code_before_line_comment(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1, // the escaped byte is not a delimiter
            b'"' => in_str = !in_str,
            b'/' if !in_str && bytes.get(i + 1) == Some(&b'/') => return &raw[..i],
            _ => {}
        }
        i += 1;
    }
    raw
}

/// (`docs/plan/on-device-log-layer-phase-1.md` §6 step 2): the device-log
/// layer is installed in a build NOBODY opened a build-time door in — so the
/// HOST's runtime switch (FR-35) has something to switch. `init_app` calls ONE
/// install function under the platform cfg and nothing else, so no
/// `debug_assertions` and no measurement cfg stands between a shipped Android
/// build and its connectivity log; inside it, the door-closed arm builds the
/// all-targets layer and the LOUD fmt layer is reachable only under the door's
/// cfg.
///
/// Installed CLOSED, and honest about it: both arms are handed the PROCESS gate
/// (the one `set_device_log` writes — a fresh gate here would be a switch wired
/// to nothing), nothing in the install opens it, and the log is marked
/// installed only when `set_global_default` succeeded — which is what lets
/// `device_log_level` answer `Off` in a process whose `tracing` default
/// another library owns.
///
/// And the layer's module never instantiates a `fmt` layer: that type's
/// `on_close` links `time.busy` / `time.idle` into the artifact, which are the
/// two strings `just wallet-device-timing-negative-witness` tells a
/// measurement build from a distributable by.
///
/// S5 (`docs/plan/stage-5-the-device-log-a-user-can-send.md` §3.1): every arm
/// builds ONE `FanOut` by struct literal, its host arm on the PROCESS host slot
/// (the one `watch_device_log` fills and the sever quiesces), its platform arm
/// at the arm's scope, and the layer's filter is `AllSdkTargets` in every build.
///
/// S5 `stderr` (FR-42): Linux and Windows get the Apple shape over stderr —
/// `init_app`'s cfg names them, and their install is checked like Apple's.
///
/// String-scan shape, like the door pin above it. Watched against: an arm
/// whose `FanOut` is built over a fresh `HostSlot::default()` (S5); the
/// Linux and Windows install handed a fresh `Gate::default()` instead of the
/// process gate (S5 `stderr`); the
/// `init_app` call re-gated to `cfg(all(target_os = "android",
/// any(debug_assertions, zec_wallet_device_timing)))` (the first assertion
/// reds); the door-closed arm deleted; `gate.set(DeviceLogLevel::Detailed)` added to the
/// install; `mark_installed()` moved out of the `is_ok()` branch;
/// `paranoid_android::layer(` used in `device_log.rs`. NOT caught, and said so:
/// whether the door-closed ARTIFACT really logs once switched on — that is the
/// device walk (plan §6 step 3).
#[test]
fn the_device_log_is_installed_closed_in_every_build() {
    let meta = fs::read_to_string(bridge_dir().join("src/api/meta.rs")).expect("meta.rs readable");
    let code_lines = |body: &str| -> Vec<String> {
        body.lines()
            .map(|l| code_before_line_comment(l).trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect()
    };
    let body_of = |header: &str| -> String {
        let start = meta
            .find(header)
            .unwrap_or_else(|| panic!("`{header}` exists in meta.rs"));
        let rest = &meta[start + header.len()..];
        rest[..rest.find("\n}").expect("the fn closes")].to_owned()
    };
    // Whitespace and trailing commas out, so rustfmt re-wrapping an install
    // cannot move a match.
    let unwrapped =
        |text: &str| -> String { text.replace(' ', "").replace(",}", "}").replace(",)", ")") };
    // S5 §3.1: the ONE layer over the fan-out, the platform arm at `scope`, the
    // host arm on the PROCESS slot (the one `watch_device_log` fills and the
    // sever quiesces — a fresh `HostSlot` here is a stream wired to nothing)
    // and the PROCESS gate.
    let fan_out_layer = |platform: &str, scope: &str| -> String {
        format!(
            "device_log_layer(FanOut{{platform:{platform},platform_scope:Scope::{scope},\
             host:process_host_slot().clone()}},gate)"
        )
    };

    // Joined, because rustfmt wraps an attribute this long over several lines.
    assert_eq!(
        unwrapped(&code_lines(&body_of("pub fn init_app() {")).join("")),
        "#[cfg(any(target_os=\"android\",target_vendor=\"apple\",target_os=\"linux\",\
         target_os=\"windows\"))]install_device_log();",
        "init_app installs the device log under the PLATFORM cfg alone — a build-profile gate \
         here is the blindness the layer exists to end (debug_assertions is never true in a \
         build anyone ships)"
    );

    // APPLE's install (the second `fn install_device_log`): the same gate, the
    // same "installed only on success", every SDK target, and NO loud layer —
    // checked first, because everything below reads the ANDROID install.
    let apple_at = meta
        .find("#[cfg(target_vendor = \"apple\")]\nfn install_device_log() {")
        .expect("meta.rs has an Apple install");
    let apple = &meta[apple_at..];
    let apple = code_lines(&apple[..apple.find("\n}").expect("the Apple install closes")]);
    let apple_text = unwrapped(&apple.join(""));
    assert!(
        apple_text.contains(&fan_out_layer("sink", "AllSdkTargets"))
            && apple.contains(&"let gate = process_gate().clone();".to_owned())
            && !apple_text.contains("loud_logcat_layer")
            && !apple_text.contains(".set(")
            && !apple_text.contains("set_level"),
        "the Apple install hands the PROCESS gate and the PROCESS host slot to one fan-out \
         whose platform arm takes every SDK target, builds no loud layer and sets nothing: \
         {apple:?}"
    );
    let marked = apple
        .iter()
        .position(|l| l == "mark_installed();")
        .expect("the Apple install marks itself");
    assert_eq!(
        apple[marked - 1],
        "if tracing::subscriber::set_global_default(subscriber).is_ok() {",
        "on Apple too, installed is marked ONLY when the global default was really taken"
    );
    assert!(
        apple.contains(&"let Some(sink) = OsLogSink::new() else {".to_owned()),
        "a missing OS log handle skips the install instead of failing the wallet's init: {apple:?}"
    );

    // LINUX and WINDOWS (S5 `stderr`, FR-42): the Apple shape over stderr.
    let desktop_at = meta
        .find("#[cfg(any(target_os = \"linux\", target_os = \"windows\"))]\nfn install_device_log() {")
        .expect("meta.rs has a Linux and Windows install");
    let desktop = &meta[desktop_at..];
    let desktop = code_lines(&desktop[..desktop.find("\n}").expect("the desktop install closes")]);
    let desktop_text = unwrapped(&desktop.join(""));
    assert!(
        desktop_text.contains(&fan_out_layer("StderrSink::new()", "AllSdkTargets"))
            && desktop.contains(&"let gate = process_gate().clone();".to_owned())
            && !desktop_text.contains("loud_logcat_layer")
            && !desktop_text.contains(".set(")
            && !desktop_text.contains("set_level"),
        "the Linux and Windows install hands the PROCESS gate and the PROCESS host slot to one \
         fan-out whose stderr arm takes every SDK target, builds no loud layer and sets \
         nothing: {desktop:?}"
    );
    let marked = desktop
        .iter()
        .position(|l| l == "mark_installed();")
        .expect("the desktop install marks itself");
    assert_eq!(
        desktop[marked - 1],
        "if tracing::subscriber::set_global_default(subscriber).is_ok() {",
        "on Linux and Windows too, installed is marked ONLY when the global default was taken"
    );

    let install = code_lines(&body_of("fn install_device_log() {"));
    let arm_after = |cfg: &str| -> String {
        let at = install
            .iter()
            .position(|l| l == cfg)
            .unwrap_or_else(|| panic!("install_device_log has a `{cfg}` arm; got {install:?}"));
        // the arm is the `let subscriber = …;` statement that follows the attribute
        let mut arm = String::new();
        for line in &install[at + 1..] {
            arm.push_str(line);
            if line.ends_with(';') {
                break;
            }
        }
        arm
    };
    let arm_after = |cfg: &str| unwrapped(&arm_after(cfg));
    let closed = arm_after("#[cfg(not(any(debug_assertions, zec_wallet_device_timing)))]");
    assert!(
        closed.contains(&fan_out_layer("LogcatSink::new()", "AllSdkTargets")),
        "the door-CLOSED arm installs the device log over every SDK target, fanned out to \
         logcat and the PROCESS host slot, behind the process gate: {closed}"
    );
    assert!(
        !closed.contains("loud_logcat_layer"),
        "the door-closed arm never builds the loud fmt layer: {closed}"
    );
    let open = arm_after("#[cfg(any(debug_assertions, zec_wallet_device_timing))]");
    assert!(
        open.contains("loud_logcat_layer()")
            && open.contains(&fan_out_layer("LogcatSink::new()", "BesideTheLoudLayer")),
        "the door-OPEN arm carries both layers: logcat's arm scoped beside the loud one, the \
         host arm on the PROCESS slot, behind the process gate: {open}"
    );
    // FR-35: installed CLOSED, behind the gate the host's verb writes.
    assert!(
        install.contains(&"let gate = process_gate().clone();".to_owned()),
        "both arms are handed the PROCESS gate — the one `set_device_log` writes: {install:?}"
    );
    assert!(
        !install
            .iter()
            .any(|l| l.contains(".set(") || l.contains("set_level")),
        "nothing in the install sets the gate — the host does, or nobody: {install:?}"
    );
    let marked = install
        .iter()
        .position(|l| l == "mark_installed();")
        .expect("the install marks itself");
    assert_eq!(
        install[marked - 1],
        "if tracing::subscriber::set_global_default(subscriber).is_ok() {",
        "the log is marked installed ONLY when the global default was really taken — \
         `device_log_level` must not claim a log another library's subscriber shadows"
    );
    assert_eq!(
        code_lines(&meta)
            .iter()
            .filter(|l| l.contains("loud_logcat_layer()"))
            .count(),
        1,
        "the loud layer is built at ONE site, the door-open arm"
    );

    let device_log =
        fs::read_to_string(bridge_dir().join("src/device_log.rs")).expect("device_log.rs readable");
    let device_log_code = code_lines(&device_log).join("\n");
    // The layer's FILTER takes every SDK target in every arm, the door-open one
    // included: the platform arm's narrower scope lives in the fan-out, so the
    // host stream still gets the core's module beside the loud layer.
    let device_log_text = unwrapped(&code_lines(&device_log).join(""));
    for filter in [
        "DeviceLogLayer::new(sink,gate).with_filter(device_log_filter())",
        "fndevice_log_filter()->FilterFn<implFn(&Metadata<'_>)->bool>{\
         filter_fn(|meta|prints(Scope::AllSdkTargets,meta))",
    ] {
        assert!(
            device_log_text.contains(filter),
            "the installed layer's filter is `AllSdkTargets` in every build, taking no scope \
             (`{filter}`)"
        );
    }
    for fmt_layer in [
        "paranoid_android::layer(",
        "fmt::layer(",
        "fmt::Layer",
        "FmtSpan",
    ] {
        assert!(
            !device_log_code.contains(fmt_layer),
            "device_log.rs must not instantiate a fmt layer (`{fmt_layer}`): it would link \
             `time.busy`/`time.idle` into every artifact and blind the negative witness"
        );
    }
}

/// One `tracing` event macro's argument list, read from source: the field
/// NAMES — everything before the message literal that is not a `target:` /
/// `parent:` / `name:` directive; `name = value`, the bare shorthand `name`, and
/// the sigil shorthands `%name` / `?name` all yield `name` — and the MESSAGE
/// literal's text, if the site has one (`\"` and `\\` unescaped; none of this
/// tree's messages uses another escape or a `\`-newline continuation, and one
/// that did would differ from its runtime value only outside the tokens graded).
fn tracing_event_fields_and_message(args: &str) -> (Vec<String>, Option<String>) {
    // split at top-level commas, outside strings and brackets
    let mut parts = Vec::new();
    let (mut depth, mut in_str, mut start) = (0i32, false, 0usize);
    let bytes = args.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'(' | b'[' | b'{' if !in_str => depth += 1,
            b')' | b']' | b'}' if !in_str => depth -= 1,
            b',' if !in_str && depth == 0 => {
                parts.push(&args[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(&args[start..]);

    let mut names = Vec::new();
    let mut message = None;
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(literal) = part.strip_prefix('"') {
            // the message; what follows are FORMAT arguments, not fields
            let literal = literal.strip_suffix('"').unwrap_or(literal);
            message = Some(literal.replace("\\\"", "\"").replace("\\\\", "\\"));
            break;
        }
        if ["target:", "parent:", "name:"]
            .iter()
            .any(|d| part.starts_with(d))
        {
            continue;
        }
        let name: String = part
            .trim_start_matches(['%', '?'])
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
            .collect();
        names.push(name);
    }
    (names, message)
}

/// The index just past the `close` that balances the `open` at `body[0]`, or
/// `None` if the text ends first. It reads Rust's LITERALS rather than counting
/// every delimiter it sees: a `"…"` string (with `\` escapes), a raw string
/// (`r"…"`, `r#"…"#`, `br##"…"##` — no escapes, closed by the quote and the same
/// number of `#`), and a CHAR literal (`'{'`, `'"'`, `'\n'`, `'\u{7b}'`) are
/// skipped whole — and a LIFETIME (`'a`, `'static`) is told from a char literal
/// by the closing quote it does not have. Line comments are the caller's to
/// strip; block comments are not read (none of the scanned trees puts a
/// delimiter in one).
fn matching_close(body: &str, open: u8, close: u8) -> Option<usize> {
    let b = body.as_bytes();
    let is_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut depth = 0i32;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                // a plain string: run to the unescaped closing quote
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
            }
            // `r…` or `br…`, and not the tail of an identifier (`for"` is not Rust,
            // but `bar#` inside a macro could be)
            b'r' if (i == 0
                || !is_ident(b[i - 1])
                || (b[i - 1] == b'b' && (i == 1 || !is_ident(b[i - 2]))))
                && b[i + 1..].iter().find(|c| **c != b'#') == Some(&b'"') =>
            {
                // a raw string: `r`, n hashes, a quote … a quote, n hashes
                let hashes = b[i + 1..].iter().take_while(|c| **c == b'#').count();
                let mut end = String::from("\"");
                end.push_str(&"#".repeat(hashes));
                let from = i + 1 + hashes + 1;
                i = from + body[from..].find(&end)? + end.len() - 1;
            }
            b'\'' => {
                // a char literal has a closing quote one char (or one escape)
                // later; a lifetime has none, and is left alone
                let rest = &body[i + 1..];
                if let Some(escaped) = rest.strip_prefix('\\') {
                    i += 2 + escaped[1.min(escaped.len())..].find('\'')? + 1;
                } else if let Some(c) = rest.chars().next()
                    && rest[c.len_utf8()..].starts_with('\'')
                {
                    i += c.len_utf8() + 1;
                }
            }
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// `code` with every `#[cfg(test)] mod … { … }` block blanked out — newlines
/// kept, so line numbers still point at the file. BRACE-MATCHED, and it goes on
/// after each block: the first cut of this scanner stopped at the FIRST test
/// module, and `send.rs` opens one at line 2,905 of 9,040 — nothing production
/// sat below it that day, but a house style that puts helper test modules
/// mid-file would have hidden everything after one (security review; the
/// exhibit `docs/REVIEW.md` §6 names).
fn without_test_modules(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut rest = code;
    while let Some(at) = rest.find("#[cfg(test)]") {
        let after_attr = &rest[at + "#[cfg(test)]".len()..];
        let item = after_attr.trim_start();
        let is_mod = ["mod ", "pub(crate) mod ", "pub mod "]
            .iter()
            .any(|m| item.starts_with(m));
        let Some(open) = after_attr.find('{').filter(|_| is_mod) else {
            // `#[cfg(test)]` on a fn / use / impl: not a module — keep scanning
            // past the attribute (a test-only FUNCTION's events are still read,
            // the loud direction).
            out.push_str(&rest[..at + "#[cfg(test)]".len()]);
            rest = after_attr;
            continue;
        };
        out.push_str(&rest[..at]);
        let body = &after_attr[open..];
        // A module the matcher could not CLOSE must never be read as "test code
        // to the end of the file": that is the silent direction — production
        // below it would go unscanned. Closing EARLY scans test code as
        // production: a false red, the loud direction. Closing LATE is the one
        // that is neither (the quality pass's MAJOR): a `'{'` char literal
        // over-counts, and a later stray `}` then closes the module somewhere
        // inside production code with nothing announcing it — so the matcher
        // reads literals properly (`matching_close`) instead of hoping a test
        // module never holds one, and `the_scanner_reads_past_a_test_module…`
        // plants exactly that.
        let end = matching_close(body, b'{', b'}').unwrap_or_else(|| {
            panic!(
                "a `#[cfg(test)] mod` block never closed for the brace matcher — it begins: {:?}",
                &item[..item.len().min(60)]
            )
        });
        let skipped = &rest[at..at + "#[cfg(test)]".len() + open + end];
        out.extend(skipped.chars().filter(|c| *c == '\n'));
        rest = &after_attr[open + end..];
    }
    out.push_str(rest);
    out
}

/// One INFO-and-above `tracing` EVENT site: its line, field names and message.
type EventSite = (usize, Vec<String>, Option<String>);

/// Every INFO-and-above `tracing` EVENT in the PRODUCTION source of `text` —
/// the file with its `#[cfg(test)]` modules blanked out.
fn info_and_above_event_sites(text: &str) -> Vec<EventSite> {
    // comments out, line numbers kept
    let code: String = text
        .lines()
        .map(code_before_line_comment)
        .collect::<Vec<_>>()
        .join("\n");
    let code = without_test_modules(&code);
    let mut sites = Vec::new();
    for mac in ["tracing::error!(", "tracing::warn!(", "tracing::info!("] {
        let mut from = 0;
        while let Some(found) = code[from..].find(mac) {
            // from the macro's own `(`, so the matcher starts at depth 0 → 1
            let open = from + found + mac.len();
            let i = open - 1
                + matching_close(&code[open - 1..], b'(', b')')
                    .unwrap_or_else(|| panic!("an unclosed `{mac}` — the scanner lost its place"));
            let line = code[..open].matches('\n').count() + 1;
            let (fields, message) = tracing_event_fields_and_message(&code[open..i - 1]);
            sites.push((line, fields, message));
            from = i;
        }
    }
    sites.sort();
    sites
}

/// The static scan's OWN instrument check: production code AFTER a test module
/// is still read, whatever the module holds. The fixture's test module carries
/// the three literals that defeat a naive brace counter — a lone `'{'` (which
/// over-counts, so the module closes LATE, at some later `}` inside production
/// code, with nothing announcing it), a `'"'` (which flips a string-parity
/// flag and blinds the counter to every brace after it) and a raw string
/// holding an unbalanced `}` — plus a lifetime, which must NOT be read as a char
/// literal. An INFO event INSIDE the module is not a production site; the WARN
/// after it is.
///
/// This is the row the quality pass asked for: every hole previously
/// found in this file's scanners was found by planting, not by reading.
///
/// Watched against: the char-literal arm removed from `matching_close`.
#[test]
fn the_scanner_reads_past_a_test_module_whatever_literals_it_holds() {
    const FIXTURE: &str = r##"
fn before() {
    tracing::info!(target: "zec_wallet_core", outcome = "ok", "before.the.module");
}

#[cfg(test)]
mod tests {
    fn literals<'a>(x: &'a str) -> (char, char, char, u8, &'static str, &'a str) {
        ('{', '{', '"', b'{', r#"{ " {{ "#, x)
    }

    #[test]
    fn inside() {
        tracing::info!(target: "zec_wallet_core", host = "in.a.test", "inside.the.module");
    }
}

fn after() {
    let brace = '}';
    tracing::warn!(target: "zec_wallet_core", outcome = "ok", "after.the.module");
}

#[cfg(test)]
fn a_test_only_fn_is_not_a_module() {
    tracing::error!(target: "zec_wallet_core", "test.only.fn");
}
"##;
    let messages: Vec<String> = info_and_above_event_sites(FIXTURE)
        .into_iter()
        .map(|(_, _, message)| message.expect("every fixture event has a message"))
        .collect();
    assert_eq!(
        messages,
        ["before.the.module", "after.the.module", "test.only.fn"],
        "the module's own event is skipped; everything after the module is still read (a \
         `#[cfg(test)]` FUNCTION is read too — the loud direction)"
    );
    // and the matcher itself, on the shapes one at a time
    for (text, close_at) in [
        ("{ '{' }", Some(7)),
        ("{ '\"' { } }", Some(11)),
        ("{ r#\"}\"# }", Some(10)),
        ("{ b'}' '\\'' '\\u{7d}' }", Some(22)),
        ("{ fn f<'a>(x: &'a str) {} }", Some(27)),
        ("{ \"}\\\"}\" }", Some(10)),
        ("{ '{' ", None),
    ] {
        assert_eq!(matching_close(text, b'{', b'}'), close_at, "{text:?}");
    }
}

/// — THE STATIC COMPLEMENT of the capture guard. `tracing_guard` grades
/// the fields on paths a test DRIVES, and says so in its header; this grades
/// the field NAMES on every INFO-and-above event in the production source of
/// the core and the bridge — every line the host-switched device-log layer can
/// print — driven or not. It exists because the gap was measured, not argued:
/// the scan this test is built from found `requeued` (the resubmit pass's INFO)
/// and `io_kind` (the wipe cleanup's WARN) shipping on paths no capture test
/// drives, neither ever allowlisted. Both are benign and are allowlisted now,
/// with their reasons; the next one gets a §5.4 review HERE instead of a
/// `withheld=1` on a tester's phone.
///
/// The ONE exemption is `transport_chain` on `net/grpc.rs`'s debug diagnostic,
/// a free-text error chain that must never be allowlisted — and the test
/// checks the reason it is exempt rather than trusting it: the site sits
/// inside a `#[cfg(debug_assertions)]` block, so it is compiled out of every
/// build that ships.
///
/// Field NAMES, and the one VALUE that can be graded from source: the MESSAGE.
/// It is a static literal at every site, and the runtime layer puts it through
/// the value scan like any field — so a message whose wording collides with a
/// never-log token (`seed`, `deposit`, `auth`) would print as `<message
/// withheld>`. Six did (the security review's MEDIUM); they are the closed
/// `tracing_guard::SANCTIONED_MESSAGES` vocabulary now, and a seventh fails
/// here. And a message must STAY static: one that interpolates (`"… {e}"`)
/// carries a value no name gate ever sees, so an INFO-and-above message with a
/// `{` in it is refused outright.
///
/// Every other value is the runtime layer's to scan (`device_log.rs`); this
/// makes its name gate complete.
///
/// Watched against: `host = host` added to the `wallet.dial` event (red, naming
/// the file and line); the `#[cfg(debug_assertions)]` removed from the grpc
/// diagnostic (red — the exemption's premise is gone); an entry removed from
/// `SANCTIONED_MESSAGES` (its site reds as a withheld message); a `{}` added to
/// an INFO message.
#[test]
fn every_info_and_above_field_name_in_the_sdk_is_allowlisted() {
    use zec_wallet_core::log_policy::field_is_loggable;

    let mut files = Vec::new();
    collect_rs(&manifest_dir().join("src"), "zec-wallet-core", &mut files);
    collect_rs(&bridge_dir().join("src"), "zec_wallet", &mut files);
    files.retain(|(label, _)| !label.ends_with("frb_generated.rs"));

    let mut sites = 0usize;
    let mut refused: Vec<(String, String)> = Vec::new();
    let mut bad_messages: Vec<(String, String)> = Vec::new();
    let mut seen_dial = 0usize;
    let mut seen_sanctioned = 0usize;
    for (label, text) in &files {
        for (line, fields, message) in info_and_above_event_sites(text) {
            sites += 1;
            if fields == ["dial_arm", "dial_class", "outcome"] {
                seen_dial += 1;
            }
            for field in fields {
                if !field_is_loggable(&field, "0") {
                    refused.push((format!("{label}:{line}"), field));
                }
            }
            let message = message.unwrap_or_else(|| {
                panic!("{label}:{line}: an INFO-and-above event with no message literal")
            });
            if message.contains('{') {
                bad_messages.push((
                    format!("{label}:{line}"),
                    format!("interpolates a value: {message:?}"),
                ));
            } else if !field_is_loggable("message", &message) {
                bad_messages.push((
                    format!("{label}:{line}"),
                    format!("would print as `<message withheld>`: {message:?}"),
                ));
            }
            if message == "wallet.swap.deposit_kick" || message == "wallet.parked.authorize_kick" {
                seen_sanctioned += 1;
            }
        }
    }
    assert!(
        bad_messages.is_empty(),
        "an INFO-and-above MESSAGE the device log cannot print as written — reword it, or (a \
         static name that merely collides with a never-log token) add it to \
         `tracing_guard::SANCTIONED_MESSAGES` with its review: {bad_messages:#?}"
    );
    assert_eq!(
        seen_sanctioned, 2,
        "extractor anti-vacuity: both resubmit-kick messages were read from source and passed \
         ONLY through the sanctioned vocabulary"
    );
    // provenance: measured at the commit that added this test — 84 sites.
    assert!(
        sites >= 70,
        "extractor anti-vacuity: {sites} INFO-and-above event sites found; the scan lost the tree"
    );
    assert_eq!(
        seen_dial, 2,
        "extractor anti-vacuity: the `wallet.dial` line's two callsites (its WARN and its INFO) \
         were each read as written, three fields apiece"
    );

    let (exempt, real): (Vec<_>, Vec<_>) = refused
        .into_iter()
        .partition(|(_, field)| field == "transport_chain");
    assert!(
        real.is_empty(),
        "an INFO-and-above tracing field outside the §5.4 allowlist would reach a user's device \
         log as `withheld` — review what it carries, then allowlist it in `tracing_guard` with \
         its reason, or drop it: {real:?}"
    );
    assert_eq!(
        exempt.len(),
        1,
        "exactly one exempt site (the grpc debug diagnostic): {exempt:?}"
    );
    let (site, _) = &exempt[0];
    assert!(
        site.starts_with("zec-wallet-core/net/grpc.rs:"),
        "the exemption is the grpc diagnostic's and nobody else's: {site}"
    );
    let grpc = &files
        .iter()
        .find(|(label, _)| label == "zec-wallet-core/net/grpc.rs")
        .expect("net/grpc.rs was walked")
        .1;
    let at = grpc
        .find("transport_chain = %chain")
        .expect("the diagnostic's field");
    let gate = grpc[..at]
        .rfind("#[cfg(debug_assertions)]")
        .expect("the diagnostic sits under a debug_assertions gate");
    let depth: i64 = grpc[gate..at]
        .lines()
        .map(code_before_line_comment)
        .map(|l| l.matches('{').count() as i64 - l.matches('}').count() as i64)
        .sum();
    assert!(
        depth >= 1 && grpc[gate..at].len() < 1500,
        "the `transport_chain` diagnostic must sit INSIDE the `#[cfg(debug_assertions)]` block \
         (brace depth {depth}): it is a free-text error chain, exempt from the allowlist only \
         because no shipped build compiles it"
    );
}

/// Stage S16 `bridge` (the diff review's fold): the sever report's enums may
/// carry variants the core does not have, and each is DECLARED here. They are
/// `Unknown` (forward compatibility) and the bridge's own overrun answer
/// (`NotSeveredCause::StillRunning`, `FilesOutcome::StillInUse`). `bridge_enums_cover_core_variants`
/// checks core ⊆ bridge. This row checks the other direction for these five
/// enums, so a bridge-only answer cannot appear without a declared producer
/// and a host doc.
#[test]
fn sever_bridge_only_variants_are_declared() {
    const DECLARED: &[(&str, &[&str])] = &[
        ("SeverOutcome", &["Unknown"]),
        ("UnprovenReason", &["Unknown"]),
        ("NotSeveredCause", &["StillRunning", "Unknown"]),
        ("HolderSeen", &["Unknown"]),
        ("FilesOutcome", &["StillInUse", "Unknown"]),
    ];
    let core = fs::read_to_string(manifest_dir().join("src/sever.rs")).expect("core sever.rs");
    let bridge =
        fs::read_to_string(bridge_dir().join("src/api/state.rs")).expect("bridge api/state.rs");
    for (name, declared) in DECLARED {
        let core_variants = enum_variants(&core, name);
        let mut extra: Vec<String> = enum_variants(&bridge, name)
            .into_iter()
            .filter(|v| !core_variants.contains(v))
            .collect();
        extra.sort();
        let mut want: Vec<String> = declared.iter().map(|s| (*s).to_string()).collect();
        want.sort();
        assert_eq!(
            extra, want,
            "{name}: the bridge-only variants must be exactly the declared ones"
        );
    }
}
