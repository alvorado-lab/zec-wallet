//! FR-29 spec §8 T14 — `no_sdk_network_io_outside_the_dialer` (gate 4).
//!
//! ADR-0526 froze ONE outbound seam: every byte the SDK sends rides a
//! `NetDialer`, and the SDK's own socket code lives in exactly one place,
//! `zec-wallet-core/src/net/dialer.rs` (`DirectTcpDialer`). The host
//! transport crossing (ADR-0543) REPLACES one implementation of that seam and
//! adds no path — so a second site that opens a socket, resolves a name or
//! binds a datagram would be a clearnet path the policy never sees
//! (`Required` could leak; `Preferred` could fall back silently). This scan
//! pins the claim at the source: the socket tokens below appear in NO
//! production line of the three SDK crates' `src/` trees except the dialer.
//!
//! THE RULE, stated once. Scanned trees: `zec-wallet-core/src`,
//! `zec-wallet-swap-near/src`, `zec_wallet/rust/src` (the bridge's generated
//! `frb_generated.rs` excluded — it is flutter_rust_bridge's output and opens
//! no socket). Tokens: `TcpStream`, `TcpSocket`, `UdpSocket`, `lookup_host`, `socket2::`, `mio::net::`, `std::os::unix::net::`, `UnixStream`, `UnixDatagram` (the raw-socket crates tokio already pulls in, and the Unix-domain sockets — the wave review's gap),
//! and a `std::net::` path whose item is a SOCKET type (`TcpListener`,
//! `TcpStream`, `UdpSocket`, `ToSocketAddrs`, `Shutdown`) — the pure address
//! types (`IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `SocketAddr*`, `AddrParseError`)
//! are data, not I/O, and stay admitted anywhere (`config.rs` parses a
//! loopback literal with one). Comments are stripped before matching (a
//! doc line that NAMES `TcpStream` is not a socket). Test code is excluded,
//! and the exclusion is a SPAN in every form it takes — never a file tail, and
//! never a list:
//!
//!   * the `tests/` directories are never scanned;
//!   * a `#[cfg(test)]` attribute that opens an INLINE module skips that
//!     module's braces and the scan RESUMES on the line after it;
//!   * a `#[cfg(test)]` attribute on an OUT-OF-LINE module (`mod x;`, with or
//!     without `#[path]`) skips the declaration, and the declared file is
//!     skipped whole — including a declaration NESTED in an inline module,
//!     which is gated by INHERITANCE when that inline module is `#[cfg(test)]`
//!     (rustc compiles it only under `cfg(test)` because its parent is) and is
//!     resolved as rustc resolves it: a non-mod-rs declarer's own name, then
//!     the inline module components, as directories (`src/wallet.rs`'s
//!     `mod tests { mod x; }` is `src/wallet/tests/x.rs`); a mod-rs declarer
//!     (`lib.rs`, `main.rs`, `mod.rs`) owns the directory it sits in. But only
//!     when EVERY declaration of that file, anywhere in the scanned trees, is
//!     gated. One gated declaration is not evidence a file is absent from
//!     production; a second ungated `mod` compiles it in, and the scan must
//!     see it. A `#[path]` containing `..` is refused outright.
//!   * a `#[cfg(test)]` on a single ITEM excludes nothing — it sits in
//!     production-adjacent code and is scanned.
//!
//! Only `net/dialer.rs` is exempt, whole.
//!
//! **And the resolver was wrong a third way (stage S8's repair).** A
//! declaration nested inside the inline `#[cfg(test)] mod tests { … }` of a
//! non-mod-rs file was resolved BESIDE the declarer (`src/x.rs` for
//! `src/wallet.rs`'s nested `mod x;`) — a path that does not exist, dropped at
//! `canonicalize` — and read as ungated unless its own line carried the
//! attribute; so the file it really declares (`src/wallet/tests/x.rs`) had no
//! declaration in the gate's eyes and was scanned as production. Five files in
//! the three trees sit in that shape (`wallet/tests/{truth_probes,
//! private_path_truth, delivery_obligation, delivery_obligation_named}.rs`,
//! `tor_status/tests/unanswered.rs`), the first two since an earlier revision, silently; the
//! first with a socket in it — S8's loopback lightwalletd fixture — made it
//! visible. A probe printed every resolved `(path, gated)` pair before the fix;
//! the measurement: 37,349 production lines across 96 files before, 91 after.
//!
//! **Both halves of this were wrong when found them, and the second was
//! the expensive one.** Stage S1 moved two test modules out of `net/grpc.rs`
//! into their own files, and the gate read a loopback `TcpListener` fixture in
//! one of them as a production socket — the visible failure. Repairing THAT
//! exposed the premise underneath: the scan used to `break` at the first
//! `#[cfg(test)]` module and never resume, "because this crate's test modules
//! sit at a file's tail". Measured across the three trees, that was false for
//! 64 files and 83,766 lines — `zec-wallet-swap-near/src/lib.rs` declares
//! `mod provider_tests;` at line 72 and its endpoint parser, its provider and
//! its `impl SwapPort` all sit below it, unscanned, in the crate this gate is
//! cited as pinning. The anti-vacuity guard counted FILES, and a truncated
//! file is still a file, so nothing reported it for as long as it stood.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The I/O tokens, matched as substrings of a comment-stripped code line.
const SOCKET_TOKENS: &[&str] = &[
    "TcpStream",
    "TcpSocket",
    "UdpSocket",
    "lookup_host",
    "socket2::",
    "mio::net::",
    "std::os::unix::net::",
    "UnixStream",
    "UnixDatagram",
];

/// `std::net::` items that are sockets (any other `std::net::X` is address
/// data and admitted).
const STD_NET_SOCKET_ITEMS: &[&str] = &[
    "TcpListener",
    "TcpStream",
    "UdpSocket",
    "ToSocketAddrs",
    "Shutdown",
];

/// The ONE file allowed to touch a socket, relative to the core crate's `src`.
const THE_DIALER: &str = "net/dialer.rs";

/// The floor the scan's PRODUCTION LINE count may not fall below. Measured at
/// 35,892 lines over 93 files. This guards VACUITY — a stripper that
/// starts returning nothing, a walk that stops early — and it is honest about
/// what it does NOT guard: the tail-cut defect it was written in response to
/// cost only 3,182 production lines tree-wide (6 %), which no floor loose
/// enough to survive ordinary deletions would ever catch. The guard for THAT
/// is [`the_scan_resumes_below_a_test_module_in_the_real_tree`].
const PRODUCTION_LINE_FLOOR: usize = 30_000;

/// A file in the real tree whose production code sits BELOW an out-of-line test
/// module, and a token from it that must be scanned. `zec-wallet-swap-near`
/// declares `mod provider_tests;` at line 73 and its `impl SwapPort` is at 486
/// — 399 production lines the tail-cut scanner never read, in the crate whose
/// ONE-WAY-OUT promise this gate is cited as pinning.
const RESUME_CANARY: (&str, &str) = (
    "zec-wallet-swap-near/src/lib.rs",
    "impl SwapPort for NearIntentsProvider",
);

fn sdk_root() -> PathBuf {
    // sdk/zec-wallet-core → sdk
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every `.rs` file under `dir`, recursively, in a stable order.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}

/// EVERY file this source file declares as a module of its own, each paired
/// with whether that declaration is `#[cfg(test)]`-gated — resolved as rustc
/// resolves it, gated as rustc gates it.
///
/// RESOLUTION (the Rust reference, "Module source filenames"): a mod-rs file
/// (`lib.rs`, `main.rs`, `mod.rs`) owns the directory it sits in; a non-mod-rs
/// file `x.rs` owns `x/`; each enclosing INLINE module adds a directory of its
/// name; a bare `mod y;` is then `<owned>/y.rs` or `<owned>/y/mod.rs`. A
/// `#[path]` on a top-level declaration is relative to the declarer's own
/// directory, mod-rs or not; inside an inline module it is relative to the
/// owned directory plus the inline components. An inline module declares no
/// file of its own. found this walk resolving every nested declaration
/// beside the declarer instead — a path that does not exist — so the file
/// really declared was scanned as production (the module doc).
///
/// GATING: an own-line `#[cfg(test)]` gates the declaration it precedes, and a
/// declaration nested in a `#[cfg(test)]` inline module INHERITS the gate —
/// rustc compiles it only under `cfg(test)` because its parent is.
///
/// Both halves matter, and the second is the one got wrong (three review
/// angles, independently). A `cfg(test)` declaration keeps a file out of every
/// production build — but only if it is the file's ONLY declaration. A file
/// declared `#[cfg(test)] mod x;` here and plain `mod x;` anywhere else IS
/// compiled into production, so the caller must see both and exclude nothing
/// that any ungated declaration claims.
///
/// A `#[path]` value containing `..` is refused outright: it can name a file
/// outside the declaring directory, and "some file somewhere is test-only" is
/// not a claim this scan will take from one attribute.
fn module_declarations(stripped: &[String], file: &Path) -> Vec<(PathBuf, bool)> {
    let Some(dir) = file.parent() else {
        return Vec::new();
    };
    let is_mod_rs = file
        .file_name()
        .is_some_and(|n| n == "mod.rs" || n == "lib.rs" || n == "main.rs");
    let owned = match (is_mod_rs, file.file_stem()) {
        (false, Some(stem)) => dir.join(stem),
        _ => dir.to_path_buf(),
    };
    let mut out = Vec::new();
    // The enclosing inline modules, innermost last: the brace depth each opened
    // at and its gate, with their names as the path components they add.
    let mut inline: Vec<(usize, bool)> = Vec::new();
    let mut components: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut gated = false;
    let mut declared: Option<String> = None;
    for line in stripped {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if l == "#[cfg(test)]" {
            gated = true;
            continue;
        }
        if let Some(rest) = l.strip_prefix("#[path = \"") {
            declared = rest.strip_suffix("\"]").map(str::to_owned);
            continue;
        }
        if l.starts_with("#[") || l.starts_with("#![") {
            continue;
        }
        if opens_module(l) {
            let is_gated = gated || inline.last().is_some_and(|(_, g)| *g);
            let name = l
                .rsplit_once("mod ")
                .map(|(_, rest)| rest.trim_end_matches([';', ' ', '{']).trim())
                .unwrap_or_default();
            if l.ends_with(';') {
                // The out-of-line declaration. THE resolution line — the
                // gate mutant resolves beside the declarer again (`dir`).
                let base: PathBuf = components.iter().fold(owned.clone(), |b, c| b.join(c));
                match &declared {
                    // `..` refused. A top-level `#[path]` is relative to the
                    // declarer's directory; a nested one to the inline components.
                    Some(path) => {
                        if !path.split(['/', '\\']).any(|part| part == "..") {
                            let from = if inline.is_empty() {
                                dir.to_path_buf()
                            } else {
                                base
                            };
                            out.push((from.join(path), is_gated));
                        }
                    }
                    // No `#[path]`: rustc looks for `y.rs` then `y/mod.rs`. Both
                    // are candidates; only one will ever match a file on disk,
                    // and a candidate that matches nothing costs nothing.
                    None => {
                        if !name.is_empty() {
                            out.push((base.join(format!("{name}.rs")), is_gated));
                            out.push((base.join(name).join("mod.rs"), is_gated));
                        }
                    }
                }
            } else if !name.is_empty() {
                // An inline module: a nesting level and a path component, no file.
                inline.push((depth, is_gated));
                components.push(name.to_string());
            }
        }
        for c in l.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        while inline.last().is_some_and(|(opened, _)| depth <= *opened) {
            inline.pop();
            components.pop();
        }
        gated = false;
        declared = None;
    }
    out
}

/// One line with its comments removed, carrying the block-comment state across
/// lines. THE one stripper: both the production scan and the declaration walk
/// read through it, so neither can disagree with the other about what is code.
///
/// Kept simple, and the premise is re-stated rather than inherited: these trees
/// carry no string or char literal that spells a socket token or an unbalanced
/// brace, so a literal is never mistaken for either.
fn strip_comments(raw: &str, in_block_comment: &mut bool) -> String {
    let mut code = String::new();
    let mut rest: &str = raw;
    loop {
        if *in_block_comment {
            match rest.find("*/") {
                Some(end) => {
                    *in_block_comment = false;
                    rest = &rest[end + 2..];
                }
                None => break,
            }
        } else {
            let line_comment = rest.find("//");
            let block_start = rest.find("/*");
            match (line_comment, block_start) {
                (Some(lc), Some(bs)) if lc < bs => {
                    code.push_str(&rest[..lc]);
                    break;
                }
                (Some(lc), None) => {
                    code.push_str(&rest[..lc]);
                    break;
                }
                (_, Some(bs)) => {
                    code.push_str(&rest[..bs]);
                    *in_block_comment = true;
                    rest = &rest[bs + 2..];
                }
                (None, None) => {
                    code.push_str(rest);
                    break;
                }
            }
        }
    }
    code
}

/// Every line of a source file, comments stripped, in order.
fn stripped_lines(src: &str) -> Vec<String> {
    let mut in_block_comment = false;
    src.lines()
        .map(|l| strip_comments(l, &mut in_block_comment))
        .collect()
}

/// Whether a stripped, trimmed line opens a module.
fn opens_module(line: &str) -> bool {
    line.starts_with("mod ") || line.starts_with("pub(crate) mod ") || line.starts_with("pub mod ")
}

/// The index just past the `#[cfg(test)]` MODULE whose attribute sits at
/// `start`, or `None` when that attribute is on something which is not a
/// module (a `#[cfg(test)]` ITEM is production-adjacent and stays scanned).
///
/// **This is a SPAN, not a tail** (docs consistency MAJOR). It used to
/// `break` the whole scan, on the premise that a crate's test module is the
/// last thing in its file. Measured across the three trees that premise was
/// false for 64 files and 83,766 lines: `zec-wallet-swap-near/src/lib.rs`
/// declares `mod provider_tests;` at line 72 and everything below it — the
/// endpoint parser, the provider, its `impl SwapPort` — was never scanned by
/// the gate that is cited as the pin for "one socket site in the whole SDK".
/// An out-of-line `mod x;` has no body here at all, so only its declaration is
/// skipped; an inline `mod x { … }` is brace-matched on stripped lines and the
/// scan RESUMES after its closing brace.
fn cfg_test_module_span(stripped: &[String], start: usize) -> Option<usize> {
    let decl = (start + 1..stripped.len()).find(|j| {
        let l = stripped[*j].trim();
        !l.is_empty() && !l.starts_with("#[")
    })?;
    if !opens_module(stripped[decl].trim()) {
        return None;
    }
    // Out-of-line (`mod x;`): the body is another file. Skip the declaration.
    if stripped[decl].trim().ends_with(';') {
        return Some(decl + 1);
    }
    let mut depth = 0usize;
    let mut opened = false;
    for (j, line) in stripped.iter().enumerate().skip(decl) {
        for c in line.chars() {
            match c {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if opened && depth == 0 {
            return Some(j + 1);
        }
    }
    // Unterminated — the rest of the file is the module. Skipping to EOF is the
    // conservative reading of a file that would not compile anyway.
    Some(stripped.len())
}

/// The production lines of a source file: comments stripped, each
/// `#[cfg(test)]` module's SPAN skipped, everything else scanned. Returns
/// `(line_number, code)` pairs.
fn production_lines(src: &str) -> Vec<(usize, String)> {
    let stripped = stripped_lines(src);
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < stripped.len() {
        if stripped[i].trim() == "#[cfg(test)]"
            && let Some(end) = cfg_test_module_span(&stripped, i)
        {
            i = end;
            continue;
        }
        if !stripped[i].trim().is_empty() {
            out.push((i + 1, stripped[i].clone()));
        }
        i += 1;
    }
    out
}

/// The socket tokens a production line carries, if any.
fn socket_hits(code: &str) -> Vec<String> {
    let mut hits: Vec<String> = SOCKET_TOKENS
        .iter()
        .filter(|t| code.contains(*t))
        .map(|t| (*t).to_string())
        .collect();
    for (idx, _) in code.match_indices("std::net::") {
        let item: String = code[idx + "std::net::".len()..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if STD_NET_SOCKET_ITEMS.contains(&item.as_str()) {
            hits.push(format!("std::net::{item}"));
        }
    }
    hits
}

#[test]
fn no_sdk_network_io_outside_the_dialer() {
    let sdk = sdk_root();
    let trees = [
        sdk.join("zec-wallet-core/src"),
        sdk.join("zec-wallet-swap-near/src"),
        sdk.join("zec_wallet/rust/src"),
    ];
    let dialer = sdk.join("zec-wallet-core/src").join(THE_DIALER);

    // Pass 1: every module declaration in the trees, gated and ungated. A file
    // is test-only when it is declared AND every declaration of it is
    // `cfg(test)`-gated — one gated declaration is not enough, because a second
    // ungated one compiles the same file into production.
    let mut gated_targets: HashSet<PathBuf> = HashSet::new();
    let mut ungated_targets: HashSet<PathBuf> = HashSet::new();
    for tree in &trees {
        assert!(tree.is_dir(), "scanned tree exists: {}", tree.display());
        for file in rust_files(tree) {
            let Ok(src) = fs::read_to_string(&file) else {
                continue;
            };
            for (declared, gated) in module_declarations(&stripped_lines(&src), &file) {
                let Ok(canonical) = declared.canonicalize() else {
                    continue;
                };
                if gated {
                    gated_targets.insert(canonical);
                } else {
                    ungated_targets.insert(canonical);
                }
            }
        }
    }
    let both: Vec<String> = gated_targets
        .intersection(&ungated_targets)
        .map(|p| p.display().to_string())
        .collect();
    // Loud, not silent: a file compiled both ways is a real defect (rustc builds
    // it into production; `clippy::duplicate_mod` catches the same-crate form).
    // It is SCANNED either way — the subtraction below is what protects the
    // policy — but it must not pass unremarked.
    assert!(
        both.is_empty(),
        "a file is declared both `cfg(test)`-gated and ungated, so it compiles into \
         production; it stays scanned, but resolve the duplicate declaration:\n{}",
        both.join("\n")
    );
    let test_only: HashSet<PathBuf> = gated_targets
        .difference(&ungated_targets)
        .cloned()
        .collect();
    // The exemption is for test code and must never reach the one production
    // file whose socket lines this gate is anchored on.
    assert!(
        !dialer.canonicalize().is_ok_and(|d| test_only.contains(&d)),
        "the cfg(test)-module exclusion never swallows {THE_DIALER}"
    );

    let mut production_line_count = 0usize;
    let mut scanned = 0usize;
    let mut violations = Vec::new();
    let mut dialer_seen = false;
    for tree in &trees {
        for file in rust_files(tree) {
            if file.file_name().is_some_and(|n| n == "frb_generated.rs") {
                continue;
            }
            if file
                .canonicalize()
                .is_ok_and(|canonical| test_only.contains(&canonical))
            {
                continue;
            }
            let src = fs::read_to_string(&file)
                .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
            scanned += 1;
            let is_dialer = file
                .canonicalize()
                .ok()
                .zip(dialer.canonicalize().ok())
                .is_some_and(|(a, b)| a == b);
            let lines = production_lines(&src);
            production_line_count += lines.len();
            for (line, code) in lines {
                let hits = socket_hits(&code);
                if hits.is_empty() {
                    continue;
                }
                if is_dialer {
                    dialer_seen = true;
                } else {
                    violations.push(format!(
                        "{}:{line}: {} — `{}`",
                        file.strip_prefix(&sdk).unwrap_or(&file).display(),
                        hits.join(", "),
                        code.trim()
                    ));
                }
            }
        }
    }
    // Anti-vacuity: the scanner must have read the trees AND found the dialer's
    // own socket code, or a broken matcher would pass with nothing looked at.
    //
    // The FILE count alone could not see the defect this gate shipped with: a
    // truncated file is still a file. The LINE floor below catches a scan that
    // collapses; the CANARY test catches a scan that truncates. Neither is the
    // other's guard, and saying so is the point — a floor loose enough to live
    // with ordinary deletions cannot see a 6 % loss.
    assert!(
        scanned >= 30,
        "the scan read {scanned} files — the trees were found"
    );
    // The measurement, printed so a change to the exclusion can be graded by
    // how many files it took out of the scan (`--nocapture`): the resolver
    // fix read 96 files before and 91 after — the five nested-declared test
    // files, no other.
    println!("the scan read {production_line_count} production lines across {scanned} files");
    assert!(
        production_line_count >= PRODUCTION_LINE_FLOOR,
        "the scan read {production_line_count} production lines across {scanned} files, \
         below the {PRODUCTION_LINE_FLOOR} floor — a skip is eating production code"
    );
    assert!(
        dialer_seen,
        "the scanner found the dialer's own socket tokens in {THE_DIALER} — the matcher works"
    );
    assert!(
        violations.is_empty(),
        "ADR-0526/ADR-0543: socket I/O outside `{THE_DIALER}` — every SDK byte rides a \
         NetDialer, and a second socket site is a path the policy never sees:\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_scanner_strips_comments_and_skips_a_test_modules_span_then_resumes() {
    // The instrument's own honesty: a comment naming a token is not a hit; a
    // cfg(test) ITEM does not start a skip; a cfg(test) MODULE does; the
    // address-only `std::net::` items are admitted, the socket ones are not.
    //
    // AND THE SPAN ENDS. Lines 12 and 17 are production code sitting
    // BELOW a test module — the first below an out-of-line `mod x;`, the second
    // below an inline `mod x { … }`. The scanner cut the file at the first of
    // these and read neither; across the three real trees that lost 3,182
    // production lines, 2,241 of them in this stage's own `net/grpc.rs`.
    let src = "\
use std::net::IpAddr; // TcpStream in a comment
/* UdpSocket in a block */ fn a() {}
#[cfg(test)]
fn item_under_cfg_test() { let _ = std::net::SocketAddr::from(([0, 0, 0, 0], 1)); }
fn b() { let _ = std::net::TcpListener::bind; }
#[cfg(test)]
mod tests {
    fn c() { let _ = tokio::net::TcpStream::connect; }
}
#[cfg(test)]
mod out_of_line_tests;
fn after_an_out_of_line_test_module() { let _ = std::net::UdpSocket::bind; }
#[cfg(test)]
mod more_tests {
    fn d() { let _ = tokio::net::TcpStream::connect; }
}
fn after_an_inline_test_module() { let _ = socket2::Socket::new; }
";
    let lines = production_lines(src);
    let hits: Vec<(usize, Vec<String>)> = lines
        .iter()
        .map(|(n, code)| (*n, socket_hits(code)))
        .filter(|(_, h)| !h.is_empty())
        .collect();
    assert_eq!(
        hits,
        vec![
            (5, vec!["std::net::TcpListener".to_string()]),
            (
                12,
                vec!["UdpSocket".to_string(), "std::net::UdpSocket".to_string()]
            ),
            (17, vec!["socket2::".to_string()]),
        ],
        "comments stripped, the cfg(test) item kept but address-only, each test module's \
         SPAN skipped — and the production lines after both of them still scanned"
    );
}

#[test]
fn the_scanner_takes_a_cfg_test_module_out_of_the_scan_only_when_cfg_test_declares_it() {
    // The exclusion's own honesty: `cfg(test)` is what carries it, not the
    // `#[path]` and not the file's name. A `#[path]` module WITHOUT
    // `#[cfg(test)]` is production code and stays scanned — that is the one
    // mistake here that would open a socket path the policy never sees.
    //
    // Three escapes the first cut left open, each refused here (found by
    // three review angles independently, none of them by its author):
    //   * a declaration inside a COMMENT — the walk read raw lines;
    //   * a `#[path]` reaching out of the directory with `..`;
    //   * the SECOND declaration: `gated` must be reported per declaration so
    //     the caller can refuse a file that an ungated `mod` also compiles.
    let src = "\
#[cfg(test)]
#[path = \"grpc_window_tests.rs\"]
mod window_tests;

#[path = \"grpc_retry.rs\"]
mod retry;

#[cfg(test)]
#[path = \"grpc_truth_tests.rs\"]
pub(crate) mod truth_tests;

// #[cfg(test)]
// #[path = \"commented_out_tests.rs\"]
// mod commented_out;

#[cfg(test)]
#[path = \"../elsewhere/reaches_out.rs\"]
mod reaches_out;

#[cfg(test)]
mod also_a_real_module;

#[cfg(test)]
mod tests {
    fn c() {}
}
";
    let declared = module_declarations(
        &stripped_lines(src),
        Path::new("zec-wallet-core/src/net/grpc.rs"),
    );
    assert_eq!(
        declared,
        vec![
            (
                PathBuf::from("zec-wallet-core/src/net/grpc_window_tests.rs"),
                true
            ),
            (
                PathBuf::from("zec-wallet-core/src/net/grpc_retry.rs"),
                false
            ),
            (
                PathBuf::from("zec-wallet-core/src/net/grpc_truth_tests.rs"),
                true
            ),
            (
                PathBuf::from("zec-wallet-core/src/net/grpc/also_a_real_module.rs"),
                true
            ),
            (
                PathBuf::from("zec-wallet-core/src/net/grpc/also_a_real_module/mod.rs"),
                true
            ),
        ],
        "every declaration is reported WITH its gating, resolved as rustc does — a top-level \
         `#[path]` beside the declaring file, a bare declaration under the non-mod-rs \
         declarer's own directory (`grpc/`), and an inline `mod tests {{ … }}` declares no \
         file: the ungated `#[path]` module is reported as ungated rather than dropped, the \
         commented-out declaration is not a declaration, and the `..` path is refused"
    );
}

#[test]
fn a_declaration_nested_in_a_cfg_test_inline_module_is_gated_by_inheritance_and_resolved_as_rustc_does()
 {
    // (stage S8's repair): the shape every `src/wallet/tests/*.rs` and
    // `src/tor_status/tests/unanswered.rs` is declared in — a bare `mod x;`
    // INSIDE the inline `#[cfg(test)] mod tests { … }` of a non-mod-rs file.
    // rustc puts that file at `src/wallet/tests/x.rs` (the declarer's own name,
    // then the inline components, as directories) and compiles it only under
    // cfg(test) because its parent is. The gate resolved it beside the declarer
    // and read it as ungated, so the file it really declares was scanned as
    // production — invisible until the first such file held a socket (the
    // loopback lightwalletd fixture in `delivery_obligation.rs`).
    let src = "\
mod top_level;
#[cfg(test)]
mod tests {
    mod bare_nested;
    #[cfg(test)]
    mod attributed_nested;
    mod inner {
        mod deeper;
    }
    #[path = \"fixtures/planted.rs\"]
    mod planted;
}
mod production {
    mod plain;
    #[cfg(test)]
    mod gated_in_production;
}
#[cfg(test)]
mod after;
";
    let want = |p: &str, g: bool| (PathBuf::from(p), g);
    let declared = module_declarations(
        &stripped_lines(src),
        Path::new("zec-wallet-core/src/wallet.rs"),
    );
    assert_eq!(
        declared,
        vec![
            want("zec-wallet-core/src/wallet/top_level.rs", false),
            want("zec-wallet-core/src/wallet/top_level/mod.rs", false),
            want("zec-wallet-core/src/wallet/tests/bare_nested.rs", true),
            want("zec-wallet-core/src/wallet/tests/bare_nested/mod.rs", true),
            want(
                "zec-wallet-core/src/wallet/tests/attributed_nested.rs",
                true
            ),
            want(
                "zec-wallet-core/src/wallet/tests/attributed_nested/mod.rs",
                true
            ),
            want("zec-wallet-core/src/wallet/tests/inner/deeper.rs", true),
            want("zec-wallet-core/src/wallet/tests/inner/deeper/mod.rs", true),
            want("zec-wallet-core/src/wallet/tests/fixtures/planted.rs", true),
            want("zec-wallet-core/src/wallet/production/plain.rs", false),
            want("zec-wallet-core/src/wallet/production/plain/mod.rs", false),
            want(
                "zec-wallet-core/src/wallet/production/gated_in_production.rs",
                true
            ),
            want(
                "zec-wallet-core/src/wallet/production/gated_in_production/mod.rs",
                true
            ),
            want("zec-wallet-core/src/wallet/after.rs", true),
            want("zec-wallet-core/src/wallet/after/mod.rs", true),
        ],
        "a non-mod-rs declarer owns `<stem>/`; each inline module adds a directory; a \
         declaration under a cfg(test) inline module is gated whether or not its own line \
         says so; one under an ungated inline module is gated only by its own line; a nested \
         `#[path]` is relative to the inline components; an inline module declares no file"
    );
    // A mod-rs declarer owns the directory it sits in.
    let declared = module_declarations(
        &stripped_lines("#[cfg(test)]\nmod tests {\n    mod nested;\n}\nmod sibling;\n"),
        Path::new("zec-wallet-core/src/net/mod.rs"),
    );
    assert_eq!(
        declared,
        vec![
            want("zec-wallet-core/src/net/tests/nested.rs", true),
            want("zec-wallet-core/src/net/tests/nested/mod.rs", true),
            want("zec-wallet-core/src/net/sibling.rs", false),
            want("zec-wallet-core/src/net/sibling/mod.rs", false),
        ],
        "a mod-rs file (`mod.rs`, `lib.rs`, `main.rs`) owns the directory it sits in"
    );
    // Against the real tree: two canaries in that shape — S1's, the oldest, and
    // S8's, the one with the socket — resolve to files that exist, gated.
    let sdk = sdk_root();
    for (declarer, file) in [
        (
            "zec-wallet-core/src/wallet.rs",
            "zec-wallet-core/src/wallet/tests/private_path_truth.rs",
        ),
        (
            "zec-wallet-core/src/wallet.rs",
            "zec-wallet-core/src/wallet/tests/delivery_obligation.rs",
        ),
    ] {
        let path = sdk.join(declarer);
        let src =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let target = sdk.join(file);
        assert!(target.exists(), "{file} exists on disk");
        let declared = module_declarations(&stripped_lines(&src), &path);
        assert!(
            declared.iter().any(|(p, g)| *g && p == &target),
            "{declarer} declares {file} gated (nested in its cfg(test) `mod tests`), got \
             {declared:?}"
        );
    }
}

#[test]
fn the_scan_resumes_below_a_test_module_in_the_real_tree() {
    // The synthetic fixture above proves the SPAN logic; this proves it against
    // the tree the gate actually defends, which is where the premise decayed.
    // A canary, not a count: it names one production line that the tail-cut
    // scanner could not see, so a regression to any truncating skip reds here
    // with the reason on its face.
    let (file, token) = RESUME_CANARY;
    let path = sdk_root().join(file);
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let lines = production_lines(&src);
    assert!(
        lines.iter().any(|(_, code)| code.contains(token)),
        "the scan stops before `{token}` in {file} — production code below a test module is \
         unscanned, which is the tail-cut defect S291 removed ({} production lines read)",
        lines.len()
    );
}

#[test]
fn a_file_an_ungated_module_also_declares_is_never_taken_out_of_the_scan() {
    // The rule the caller applies, stated where it can be read: ANY ungated
    // declaration of a file keeps that file in the scan, however many gated
    // ones there are. One `#[cfg(test)]` declaration is not evidence that a
    // file is absent from production — only that ONE of its declarations is.
    let one = module_declarations(
        &stripped_lines("#[cfg(test)]\n#[path = \"shared.rs\"]\nmod under_test;\n"),
        Path::new("crate/src/a.rs"),
    );
    let two = module_declarations(
        &stripped_lines("#[path = \"shared.rs\"]\nmod in_production;\n"),
        Path::new("crate/src/b.rs"),
    );
    let gated: HashSet<&PathBuf> = one
        .iter()
        .chain(&two)
        .filter(|(_, g)| *g)
        .map(|(p, _)| p)
        .collect();
    let ungated: HashSet<&PathBuf> = one
        .iter()
        .chain(&two)
        .filter(|(_, g)| !*g)
        .map(|(p, _)| p)
        .collect();
    assert_eq!(
        gated.intersection(&ungated).count(),
        1,
        "the same target is claimed by a gated and an ungated declaration — the scan must \
         see the conflict rather than trust the gated one"
    );
    assert!(
        gated.difference(&ungated).next().is_none(),
        "and nothing is left to exclude: `shared.rs` compiles into production"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The consensus parameter type is named by ONE alias (ADR-0579; the NU7 wave's
// step 1, `docs/plan/nu7-wave.md` §3). It sits in this file because this file
// holds the one resolver of which `src/` files are production code; the gate
// above is carried by every recipe that runs this binary.
// ─────────────────────────────────────────────────────────────────────────────

/// The names that spell the concrete consensus parameters: the variants of
/// `zcash_protocol::consensus::Network`, the `MainNetwork` / `TestNetwork` unit
/// types, and the `MAIN_NETWORK` / `TEST_NETWORK` singletons. Matched as whole
/// identifiers. The type itself, `consensus::Network`, is matched as a path and
/// inside a `consensus::{…}` import group (see [`concrete_params_hits`]).
const CONCRETE_PARAMS_IDENTS: &[&str] =
    &["MainNetwork", "TestNetwork", "MAIN_NETWORK", "TEST_NETWORK"];

/// The production items allowed to name the concrete type, as (file under the
/// core's `src/`, enclosing item). The alias's own definition and its mapping
/// (`money.rs`), and `derivation.rs`'s encoders, decoders and key derivation,
/// which read only the network type, never an activation height: no params
/// override would have anything to change there. `use` admits the import line
/// those functions need.
const CONCRETE_PARAMS_ALLOWED: &[(&str, &str)] = &[
    ("money.rs", "WalletParams"),
    ("money.rs", "consensus"),
    ("derivation.rs", "use"),
    ("derivation.rs", "derive_default_address"),
    ("derivation.rs", "encode_ufvk"),
    ("derivation.rs", "decode_ufvk"),
    ("derivation.rs", "derive_spending_key"),
    ("derivation.rs", "derive_transparent_refund_address"),
    ("derivation.rs", "default_address_from_ufvk"),
    ("derivation.rs", "transparent_receive_address_from_ufvk"),
    ("derivation.rs", "encode_transparent_receiver"),
    ("derivation.rs", "encode_unified_address"),
    ("derivation.rs", "transparent_receiver_from_stored_address"),
    ("derivation.rs", "derive_transparent_receive_address"),
];

/// The `src/` files of `trees` that compile only under `cfg(test)`: declared, and
/// every declaration of them gated. The same computation as pass 1 of
/// [`no_sdk_network_io_outside_the_dialer`], which keeps its inline copy so the
/// lines its registry rows cite stay where they are.
fn test_only_files(trees: &[PathBuf]) -> HashSet<PathBuf> {
    let mut gated_targets: HashSet<PathBuf> = HashSet::new();
    let mut ungated_targets: HashSet<PathBuf> = HashSet::new();
    for tree in trees {
        assert!(tree.is_dir(), "scanned tree exists: {}", tree.display());
        for file in rust_files(tree) {
            let Ok(src) = fs::read_to_string(&file) else {
                continue;
            };
            for (declared, gated) in module_declarations(&stripped_lines(&src), &file) {
                let Ok(canonical) = declared.canonicalize() else {
                    continue;
                };
                if gated {
                    gated_targets.insert(canonical);
                } else {
                    ungated_targets.insert(canonical);
                }
            }
        }
    }
    gated_targets
        .difference(&ungated_targets)
        .cloned()
        .collect()
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Every place `text` (comment-stripped production code, `\n`-joined) names the
/// concrete consensus parameters, as (byte offset, what was named).
fn concrete_params_hits(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let anchored = |at: usize, len: usize| {
        (at == 0 || !is_ident_byte(bytes[at - 1]))
            && bytes.get(at + len).is_none_or(|b| !is_ident_byte(*b))
    };
    let mut hits = Vec::new();
    for ident in CONCRETE_PARAMS_IDENTS {
        for (at, _) in text.match_indices(ident) {
            if anchored(at, ident.len()) {
                hits.push((at, (*ident).to_string()));
            }
        }
    }
    // The type by path: `consensus::Network`, not `consensus::NetworkType`.
    const PATH: &str = "consensus::Network";
    for (at, _) in text.match_indices(PATH) {
        if bytes
            .get(at + PATH.len())
            .is_none_or(|b| !is_ident_byte(*b))
        {
            hits.push((at, PATH.to_string()));
        }
    }
    // The type in an import group, `consensus::{BlockHeight, Network, …}`, where a
    // path match cannot see it. A group may span lines.
    for (at, _) in text.match_indices("consensus::{") {
        let open = at + "consensus::".len();
        let Some(len) = text[open..].find('}') else {
            continue;
        };
        let group = &text[open + 1..open + len];
        if group
            .split([',', ' ', '\n', '{', '}'])
            .any(|item| item.trim() == "Network")
        {
            hits.push((at, "consensus::{Network}".to_string()));
        }
    }
    hits.sort();
    hits
}

/// The item a production line belongs to: the nearest line at or above `line`
/// that opens an item, by its name (`use` and `impl` by their keyword).
fn enclosing_item(lines: &[String], line: usize) -> String {
    const VISIBILITY: &[&str] = &["pub(crate) ", "pub(super) ", "pub "];
    const QUALIFIERS: &[&str] = &["async ", "const ", "unsafe "];
    for code in lines[..=line].iter().rev() {
        let mut l = code.trim();
        if let Some(rest) = VISIBILITY.iter().find_map(|v| l.strip_prefix(v)) {
            l = rest;
        }
        if l.starts_with("use ") {
            return "use".to_string();
        }
        if l.starts_with("impl ") || l.starts_with("impl<") {
            return "impl".to_string();
        }
        let mut fn_line = l;
        while let Some(rest) = QUALIFIERS.iter().find_map(|q| fn_line.strip_prefix(q)) {
            fn_line = rest;
        }
        for keyword in [
            "fn ", "type ", "const ", "static ", "struct ", "enum ", "trait ", "mod ",
        ] {
            let candidate = if keyword == "fn " { fn_line } else { l };
            if let Some(rest) = candidate.strip_prefix(keyword) {
                return rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
            }
        }
    }
    String::new()
}

/// **The NU7 wave's step 1, its first named choice** (`docs/plan/nu7-wave.md` §3,
/// ADR-0579). The wallet's consensus parameters are `WalletParams`, an alias of
/// `zcash_protocol::consensus::Network`, so the concrete type is named in one place
/// and every reader of the params the signing gate reads goes through the alias.
///
/// Over the PRODUCTION code of the core's `src/` (test modules and test-only files
/// excluded by this file's resolver), the concrete type is named only by the
/// allowed items above. It prints how many files and lines it read and fails on
/// none (`docs/REVIEW.md` §6). Every allowed item must still be seen, so an entry
/// that no longer names the concrete type fails here instead of standing as a hole.
///
/// **The item reader's limit, stated.** [`enclosing_item`] takes the nearest line
/// that opens an item, so an item declared INSIDE a function body (a local `const`,
/// `static`, `fn` or `use`) becomes the owner of the lines after it. For a local
/// `const`, `static` or `fn` that is the loud direction: a hit after one, inside an
/// allowed function, reads as that local item and fails. A local `use` reads as
/// `use`, which `derivation.rs` is allowed, so a `use` of the singletons inside any
/// `derivation.rs` function is admitted: the same file the allowlist already trusts.
///
/// Mutant: `db.rs`'s `WalletConn` back to `WalletConnFor<zcash_protocol::consensus::Network>`.
#[test]
fn the_concrete_consensus_type_is_named_only_by_the_walletparams_alias() {
    let src = sdk_root().join("zec-wallet-core/src");
    let test_only = test_only_files(std::slice::from_ref(&src));

    let mut files_read = 0usize;
    let mut lines_read = 0usize;
    let mut allowed_seen: HashSet<(String, String)> = HashSet::new();
    let mut violations = Vec::new();
    for file in rust_files(&src) {
        if file
            .canonicalize()
            .is_ok_and(|canonical| test_only.contains(&canonical))
        {
            continue;
        }
        let text =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        let production = production_lines(&text);
        files_read += 1;
        lines_read += production.len();
        // Back into one text, line numbers kept, so an import group that spans
        // lines is read whole.
        let mut lines = vec![String::new(); text.lines().count() + 1];
        for (number, code) in production {
            lines[number] = code;
        }
        let joined = lines.join("\n");
        let relative = file
            .strip_prefix(&src)
            .unwrap_or(&file)
            .display()
            .to_string();
        for (at, named) in concrete_params_hits(&joined) {
            let line = joined[..at].matches('\n').count();
            let item = enclosing_item(&lines, line);
            if CONCRETE_PARAMS_ALLOWED.contains(&(relative.as_str(), item.as_str())) {
                allowed_seen.insert((relative.clone(), item));
            } else {
                violations.push(format!(
                    "{relative}:{line}: `{named}` in `{item}`: `{}`",
                    lines[line].trim()
                ));
            }
        }
    }
    println!(
        "the alias scan read {lines_read} production lines across {files_read} files; \
         {} allowed items named the concrete type",
        allowed_seen.len()
    );
    assert!(
        files_read >= 30 && lines_read >= 20_000,
        "the alias scan read {lines_read} production lines across {files_read} files: the \
         tree was not found, or the resolver is eating production code"
    );
    // Not vacuous: the alias and the mapping are read where they are written.
    for site in [("money.rs", "WalletParams"), ("money.rs", "consensus")] {
        assert!(
            allowed_seen.contains(&(site.0.to_string(), site.1.to_string())),
            "the scan did not see `{}` in {} name the concrete type: the matcher or the \
             item reader lost its place",
            site.1,
            site.0
        );
    }
    assert!(
        violations.is_empty(),
        "production code names the concrete consensus parameters outside the \
         `WalletParams` alias (`money.rs`) and `derivation.rs`'s encoders: read the \
         params through `WalletParams` / `Network::consensus`:\n{}",
        violations.join("\n")
    );
    // No stale entry: every allowed item still names the concrete type. One that no
    // longer does is a hole the next edit to that item could walk through unseen.
    let mut stale: Vec<String> = CONCRETE_PARAMS_ALLOWED
        .iter()
        .filter(|(file, item)| !allowed_seen.contains(&(file.to_string(), item.to_string())))
        .map(|(file, item)| format!("{file} `{item}`"))
        .collect();
    stale.sort();
    assert!(
        stale.is_empty(),
        "allowlist entries that no longer name the concrete consensus type: remove them \
         from CONCRETE_PARAMS_ALLOWED: {stale:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Every generator upstream proves and signs with, and the clock beside it, comes
// from ONE seam (ADR-0579 Decision 3; the NU7 wave's step 2b,
// `docs/plan/nu7-wave.md` §3). It sits in this file for the alias pin's reason:
// this file holds the one resolver of which `src/` files are production code.
// ─────────────────────────────────────────────────────────────────────────────

/// The seam's generator, as a gated position must spell it: by its full path and
/// no other way. A bare `wallet_rng()` is refused at any gated position, whatever
/// the file imports. A bare name can belong to something else in its scope (a
/// `let`, a parameter, a closure argument, an enum variant, a function-local
/// `use`), and no reading of one file's text tells those apart from the seam,
/// while `crate::rng::` is an absolute path that nothing in a file can shadow.
const SEAM_FN: &str = "wallet_rng";
const SEAM_CALLS: &[&str] = &["crate::rng::wallet_rng()"];

/// The seam's clock, spelled the same one way (contract item 2). `WalletDb` has
/// no clock accessor, so "the clock the same `WalletDb` holds" cannot be read
/// from the instance; it is held by construction, one function feeding both the
/// wallet DB and the create. Its identity is what the gate pins, not its type: a
/// fresh `SystemClock` at a create is refused too, since it is a second clock
/// source that a change to the seam's clock would not move.
const SEAM_CLOCK_FN: &str = "wallet_db_clock";
const SEAM_CLOCK_CALLS: &[&str] = &["crate::rng::wallet_db_clock()"];

/// An upstream entry point that takes a generator, with the positions of its
/// clock and generator arguments. A method's positions exclude its receiver, so
/// a method call (`receiver.name(…)`) is read at them. A path-form call
/// (`Type::name(receiver, …)`) puts the receiver at the generator's position,
/// which is never the seam's call, so it is refused rather than read. Any other
/// occurrence of an entry's name outside a `use` item (a bare word, a value, a
/// macro argument, an item of ours with the same name) is not read as a call,
/// and is refused for that: the generator it is eventually given is one this gate
/// never reads.
struct GeneratorEntry {
    name: &'static str,
    clock: Option<usize>,
    rng: usize,
}

/// The upstream entry points this gate reads, each signature re-read at the
/// pinned crates. They are the four public functions of
/// `zcash_client_backend-0.25.0`'s `data_api::wallet` whose signature takes
/// `rng: &mut impl CryptoRng` (`create_proposed_transactions` `:1442`,
/// `create_pczt_from_proposal` `:2963`, `extract_and_store_transaction_from_pczt`
/// `:3587`, `shield_transparent_funds` `:4107`); the `pczt-0.10.0` role methods
/// that take a generator as an argument (the signers, `roles/signer/mod.rs:366`,
/// `:435`, `:529`; the IO finalizer, `roles/io_finalizer/mod.rs:31`; the
/// transaction extractor, `roles/tx_extractor/mod.rs:74`; the provers,
/// `roles/prover/orchard.rs:10`, `:53` and `roles/prover/sapling.rs:10`); and
/// `zcash_client_sqlite-0.23.0`'s `PoolMigrations::take_transaction_for_broadcast`
/// (`pool_migration/orchard_ironwood.rs:476`, the ZIP 318 migration's broadcast
/// seam: the module is always compiled, `pool_migration.rs:87`, and the impl
/// holding this method is `cfg(feature = "orchard")`, which our `orchard` feature
/// turns on along with `zcash_pool_migration/wallet`). The contract names only
/// the first; the rest are added so that a PCZT, shielding or migration path
/// added later carries a generator this gate reads.
///
/// Matched by name, so a name names one position: `finalize_io` is read at
/// argument 0, `pczt`'s IO finalizer, while `orchard`'s and `sapling-crypto`'s own
/// bundle-level `finalize_io(sighash, rng)` (`pczt/io_finalizer.rs:24` and `:12`)
/// take it at 1; a seam call there is refused, the safe direction. It is not every
/// upstream API that takes a generator:
/// [`every_signing_generator_comes_from_the_one_seam`] names the KNOWN ones outside
/// it.
const GENERATOR_ENTRIES: &[GeneratorEntry] = &[
    GeneratorEntry {
        name: "create_proposed_transactions",
        clock: Some(2),
        rng: 3,
    },
    GeneratorEntry {
        name: "create_pczt_from_proposal",
        clock: Some(2),
        rng: 3,
    },
    GeneratorEntry {
        name: "shield_transparent_funds",
        clock: Some(2),
        rng: 3,
    },
    GeneratorEntry {
        name: "extract_and_store_transaction_from_pczt",
        clock: Some(1),
        rng: 2,
    },
    GeneratorEntry {
        name: "sign_sapling",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "sign_orchard",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "sign_ironwood",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "finalize_io",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "extract",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "create_orchard_proof",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "create_ironwood_proof",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "create_sapling_proofs",
        clock: None,
        rng: 0,
    },
    GeneratorEntry {
        name: "take_transaction_for_broadcast",
        clock: None,
        rng: 0,
    },
];

/// `WalletDb`'s two constructors, both `(conn_or_path, params, clock, rng)`
/// (`zcash_client_sqlite-0.23.0/src/lib.rs:571`, `:645`). Matched by name after
/// any `::`, so a construction through an alias of the type is read too; a call
/// of another arity is some other type's and is skipped.
const WALLET_DB_CONSTRUCTORS: &[&str] = &["from_connection", "for_path"];
const WALLET_DB_ARITY: usize = 4;
const WALLET_DB_CLOCK: usize = 2;
const WALLET_DB_RNG: usize = 3;

fn skip_ws(bytes: &[u8], mut at: usize) -> usize {
    while at < bytes.len() && bytes[at].is_ascii_whitespace() {
        at += 1;
    }
    at
}

/// The index just past a turbofish `::<…>` that starts at `at` (after
/// whitespace), or `at` itself when none does.
fn skip_turbofish(text: &str, at: usize) -> usize {
    let bytes = text.as_bytes();
    let start = skip_ws(bytes, at);
    if !text[start..].starts_with("::<") {
        return at;
    }
    let mut depth = 0usize;
    for (j, b) in bytes.iter().enumerate().skip(start + 2) {
        match b {
            b'<' => depth += 1,
            b'>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
    }
    bytes.len()
}

/// The arguments of the call whose `(` sits at `open`, split at the commas of
/// its own level and stripped of whitespace, or `None` when it never closes.
/// Strings are skipped whole; an angle bracket counts only after `::`, so a
/// comparison inside an argument cannot unbalance it.
fn call_args(text: &str, open: usize) -> Option<Vec<String>> {
    let bytes = text.as_bytes();
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut angle = 0usize;
    let mut in_string = false;
    let mut j = open;
    while j < bytes.len() {
        let c = bytes[j] as char;
        if in_string {
            current.push(c);
            if c == '\\' && j + 1 < bytes.len() {
                current.push(bytes[j + 1] as char);
                j += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            j += 1;
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                current.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                if depth > 1 {
                    current.push(c);
                }
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let last: String = current.split_whitespace().collect();
                    if !last.is_empty() {
                        args.push(last);
                    }
                    return Some(args);
                }
                current.push(c);
            }
            '<' if text[..j].ends_with("::") => {
                angle += 1;
                current.push(c);
            }
            '>' if angle > 0 => {
                angle -= 1;
                current.push(c);
            }
            ',' if depth == 1 && angle == 0 => {
                args.push(current.split_whitespace().collect());
                current.clear();
            }
            _ => current.push(c),
        }
        j += 1;
    }
    None
}

/// Whether a whitespace-free argument is the seam call, by value or `&mut`.
fn is_seam_call(arg: &str) -> bool {
    let bare = arg.strip_prefix("&mut").unwrap_or(arg);
    SEAM_CALLS.contains(&bare)
}

/// Whether a whitespace-free argument is the seam's clock call, by value or `&`.
fn is_seam_clock(arg: &str) -> bool {
    let bare = arg.strip_prefix('&').unwrap_or(arg);
    SEAM_CLOCK_CALLS.contains(&bare)
}

/// The index of the `{` that the `}` at `close` closes, or `None` when none does.
fn matching_open(bytes: &[u8], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for j in (0..close).rev() {
        match bytes[j] {
            b'}' => depth += 1,
            b'{' if depth == 0 => return Some(j),
            b'{' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Whether the occurrence at `at` sits inside a `use` item. THE one predicate
/// for "this is an import": the entry loop and the `WalletDb` rename check both
/// ask it, and nothing else in this gate answers the question.
///
/// It reads statements, not lines. Walking back from the occurrence, a use-tree
/// group (`::{ … }`) is stepped over whole; the nearest `;`, `{` or `}` left is
/// where the statement holding the occurrence begins. The occurrence is in a
/// `use` item when that statement is `use` (after any attributes and visibility)
/// followed by nothing but a use tree up to the occurrence: path segments, `::`,
/// groups, `,`, `*`, `as` and whitespace. So a `use` inside a function body
/// covers its own statement and never the lines after it, and a string that
/// spells `use` is not an import (its quote is not part of a use tree).
fn in_a_use_item(text: &str, at: usize) -> bool {
    let bytes = text.as_bytes();
    let mut end = at;
    let start = loop {
        let Some(b) = text[..end].rfind([';', '{', '}']) else {
            break 0;
        };
        match bytes[b] {
            b'{' if text[..b].trim_end().ends_with("::") => end = b,
            b'}' => match matching_open(bytes, b) {
                Some(open) if text[..open].trim_end().ends_with("::") => end = open,
                _ => break b + 1,
            },
            _ => break b + 1,
        }
    };
    let mut stmt = text[start..at].trim_start();
    while let Some(rest) = stmt.strip_prefix("#[") {
        let Some(close) = rest.find(']') else {
            return false;
        };
        stmt = rest[close + 1..].trim_start();
    }
    if let Some(rest) = stmt.strip_prefix("pub") {
        stmt = rest.trim_start();
        if let Some(scope) = stmt.strip_prefix('(') {
            let Some(close) = scope.find(')') else {
                return false;
            };
            stmt = scope[close + 1..].trim_start();
        }
    }
    stmt.strip_prefix("use").is_some_and(|tree| {
        tree.starts_with(char::is_whitespace)
            && tree.chars().all(|c| {
                c.is_alphanumeric()
                    || c.is_whitespace()
                    || matches!(c, '_' | ':' | '{' | '}' | ',' | '*')
            })
    })
}

/// Every anchored occurrence of the identifier `ident` in `text`.
fn ident_hits(text: &str, ident: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    text.match_indices(ident)
        .map(|(at, _)| at)
        .filter(|&at| {
            (at == 0 || !is_ident_byte(bytes[at - 1]))
                && bytes
                    .get(at + ident.len())
                    .is_none_or(|b| !is_ident_byte(*b))
        })
        .collect()
}

/// One production file's findings: the generator and clock sites it holds, and
/// every way it breaks the rule.
#[derive(Default)]
struct SeamFindings {
    /// (entry or constructor name, line) for every site whose generator is the
    /// seam's.
    sites: Vec<(String, usize)>,
    /// (entry or constructor name, line) for every site whose clock is the seam's.
    clock_sites: Vec<(String, usize)>,
    /// The entry or constructor name of every site whose generator was read,
    /// the seam's or refused: what the non-vacuity clause counts.
    found: Vec<String>,
    violations: Vec<String>,
}

/// The seam rule over one production file. `lines` is the file's production
/// code indexed by line number (a skipped line is empty); `relative` names the
/// file in messages.
fn seam_findings(lines: &[String], relative: &str) -> SeamFindings {
    let text = lines.join("\n");
    let bytes = text.as_bytes();
    let line_of = |at: usize| text[..at].matches('\n').count();
    let mut out = SeamFindings::default();
    let check_seam = |arg: &str, line: usize, what: &str, out: &mut SeamFindings| {
        out.found.push(what.to_string());
        if is_seam_call(arg) {
            out.sites.push((what.to_string(), line));
        } else {
            out.violations.push(format!(
                "{relative}:{line}: `{what}` takes its generator from `{arg}`, not the seam \
                 `crate::rng::{SEAM_FN}()`"
            ));
        }
    };
    // `refusal` opens the message for a clock that is not the seam's; the clock
    // follows it.
    let check_clock =
        |arg: &str, line: usize, what: &str, refusal: String, out: &mut SeamFindings| {
            if is_seam_clock(arg) {
                out.clock_sites.push((what.to_string(), line));
            } else {
                out.violations.push(format!(
                    "{relative}:{line}: {refusal} `{arg}`, not the seam's \
                     `crate::rng::{SEAM_CLOCK_FN}()`"
                ));
            }
        };

    for entry in GENERATOR_ENTRIES {
        for at in ident_hits(&text, entry.name) {
            let line = line_of(at);
            let after = skip_ws(bytes, at + entry.name.len());
            if in_a_use_item(&text, at) {
                if text[after..].starts_with("as ") {
                    out.violations.push(format!(
                        "{relative}:{line}: `{}` is renamed on import, so a call through the \
                         new name is a call this gate never reads",
                        entry.name
                    ));
                }
                continue;
            }
            let open = skip_ws(bytes, skip_turbofish(&text, at + entry.name.len()));
            if bytes.get(open) != Some(&b'(') {
                out.violations.push(format!(
                    "{relative}:{line}: `{}` is named but not called, so the generator it is \
                     eventually given is one this gate never reads",
                    entry.name
                ));
                continue;
            }
            let Some(args) = call_args(&text, open) else {
                out.violations.push(format!(
                    "{relative}:{line}: `{}`'s argument list does not close",
                    entry.name
                ));
                continue;
            };
            let Some(rng) = args.get(entry.rng) else {
                out.violations.push(format!(
                    "{relative}:{line}: `{}` has no argument at its generator's position {}",
                    entry.name, entry.rng
                ));
                continue;
            };
            check_seam(rng.as_str(), line, entry.name, &mut out);
            if let Some(clock) = entry.clock {
                check_clock(
                    args.get(clock).map(String::as_str).unwrap_or("(missing)"),
                    line,
                    entry.name,
                    format!(
                        "`{}` must take the clock the wallet DB holds; its clock is",
                        entry.name
                    ),
                    &mut out,
                );
            }
        }
    }

    for ctor in WALLET_DB_CONSTRUCTORS {
        for at in ident_hits(&text, ctor) {
            if !text[..at].ends_with("::") {
                continue;
            }
            let line = line_of(at);
            let open = skip_ws(bytes, skip_turbofish(&text, at + ctor.len()));
            if bytes.get(open) != Some(&b'(') {
                continue;
            }
            let Some(args) = call_args(&text, open) else {
                out.violations.push(format!(
                    "{relative}:{line}: `{ctor}`'s argument list does not close"
                ));
                continue;
            };
            if args.len() != WALLET_DB_ARITY {
                continue;
            }
            check_seam(args[WALLET_DB_RNG].as_str(), line, ctor, &mut out);
            check_clock(
                args[WALLET_DB_CLOCK].as_str(),
                line,
                ctor,
                format!("a wallet DB built by `{ctor}` holds the clock"),
                &mut out,
            );
        }
    }

    // `WalletDb` renamed on import: a construction through the new name would
    // still be read (constructors match by name), but a rename is the first
    // step of hiding one, so it is refused outright.
    for at in ident_hits(&text, "WalletDb") {
        if in_a_use_item(&text, at)
            && text[skip_ws(bytes, at + "WalletDb".len())..].starts_with("as ")
        {
            out.violations.push(format!(
                "{relative}:{}: `WalletDb` is renamed on import",
                line_of(at)
            ));
        }
    }

    // No rule here about the seam's bare names, by design: a gated position
    // admits only the full path (`SEAM_CALLS`, `SEAM_CLOCK_CALLS`), so a binding,
    // a variant or a function elsewhere named `wallet_rng` reaches no gated
    // position, and a bare call at one is refused as not the seam.
    out
}

/// One file's production code, indexed by line number (a skipped line is
/// empty), the shape [`seam_findings`] reads.
fn numbered_production_lines(text: &str) -> Vec<String> {
    let mut lines = vec![String::new(); text.lines().count() + 1];
    for (number, code) in production_lines(text) {
        lines[number] = code;
    }
    lines
}

/// What [`seam_scan`] read and found over one `src/` tree.
struct SeamScan {
    files_read: usize,
    lines_read: usize,
    /// (file, entry or constructor name, line) for every site whose generator is
    /// the seam's.
    sites: Vec<(String, String, usize)>,
    /// How many sites name the seam's clock.
    clock_sites: usize,
    /// The entry or constructor name of every site whose generator was read,
    /// the seam's or refused.
    found: Vec<String>,
    violations: Vec<String>,
}

/// [`seam_findings`] over every production file of `src` (test modules and
/// test-only files excluded by this file's resolver).
fn seam_scan(src: &Path) -> SeamScan {
    let test_only = test_only_files(&[src.to_path_buf()]);
    let mut scan = SeamScan {
        files_read: 0,
        lines_read: 0,
        sites: Vec::new(),
        clock_sites: 0,
        found: Vec::new(),
        violations: Vec::new(),
    };
    for file in rust_files(src) {
        if file
            .canonicalize()
            .is_ok_and(|canonical| test_only.contains(&canonical))
        {
            continue;
        }
        let text =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        scan.files_read += 1;
        scan.lines_read += production_lines(&text).len();
        let relative = file
            .strip_prefix(src)
            .unwrap_or(&file)
            .display()
            .to_string();
        let findings = seam_findings(&numbered_production_lines(&text), &relative);
        scan.clock_sites += findings.clock_sites.len();
        scan.found.extend(findings.found);
        scan.sites.extend(
            findings
                .sites
                .into_iter()
                .map(|(what, line)| (relative.clone(), what, line)),
        );
        scan.violations.extend(findings.violations);
    }
    scan
}

/// **The NU7 wave's step 2b, contract items 1 and 2** (`docs/plan/nu7-wave.md`
/// §3, ADR-0579 Decision 3). In the production code of the core's `src/`, every
/// call of an upstream entry point that takes a generator, and every wallet DB
/// construction, names the seam's generator `crate::rng::wallet_rng()` at the
/// generator's position and the seam's clock `crate::rng::wallet_db_clock()` at
/// the clock's, by their full paths (a bare spelling is refused, `SEAM_CALLS`).
/// It is the call site that names them, so the generator is drawn where the call
/// is, never handed in from elsewhere, and the create's clock is the one source
/// the wallet DB is built with. Every call of `create_proposed_transactions` is
/// spelled with a turbofish, so an identifier match finds each one (D-L7). An
/// import, at file level or inside a function, covers its own `use` statement
/// only ([`in_a_use_item`]), so a call after a function-local `use` is read.
///
/// **Its boundary.** It scans the production code of `zec-wallet-core/src` only:
/// the bridge (`zec_wallet/rust/src`) and `zec-wallet-swap-near` sign nothing
/// today and are not read. The entry points it reads are [`GENERATOR_ENTRIES`],
/// and the list is not every upstream API that takes a generator. These are the
/// KNOWN ones outside it; a production call of any of them would carry a
/// generator this gate never reads:
///
///   * upstream's transaction, Orchard and Sapling builders;
///   * `pczt`'s low-level signer (`roles/low_level_signer`, the `sign_*_with`
///     methods), whose generator lives in a closure the caller writes, so no
///     argument position holds it;
///   * the `zcash_pool_migration` crate's own entry points (its planner, prover
///     and scheduler take generators), which the ZIP 318 migration would call;
///   * what `orchard` and `sapling-crypto`, both direct dependencies, offer
///     themselves: `orchard-0.16.0`'s `SigningKey::sign`
///     (`src/primitives/redpallas.rs:73`) and its own PCZT roles (`pczt/signer.rs`
///     `sign`, `pczt/prover.rs` `create_proof`, `pczt/tx_extractor.rs`
///     `apply_binding_signature`); `sapling-crypto-0.9.0`'s PCZT roles
///     (`pczt/signer.rs:14` `sign`, `pczt/prover.rs` `create_proofs`,
///     `pczt/tx_extractor.rs` `apply_binding_signature`) and its note-randomness
///     helpers (`util.rs` `generate_random_rseed`, `note.rs`
///     `generate_or_derive_esk`, `value.rs` `random`). Their bundle-level
///     `finalize_io` is matched by name and refused (the list's doc).
///
/// The door this list cannot close, a generator BORN somewhere other than
/// `rng.rs`, is the plan's owed item (§8).
///
/// The clock half reads the clock arguments of the `data_api::wallet` functions
/// and `WalletDb`'s constructors only: `PoolMigrations::for_account`
/// (`zcash_client_sqlite-0.23.0/src/pool_migration/orchard_ironwood.rs:251`)
/// takes a clock too, for the migration transaction's record, and is not read.
/// None of these is called today.
///
/// There is no deny-list of generator types: the deny-list was the first
/// draft's gate, which the contract removed. `rand` is not a direct dependency
/// of this crate, so none of its generators is nameable here (ADR-0579 Decision
/// 3). It is in the shipped graph all the same (`rand 0.10.3`, through
/// `groth16` and `sapling-crypto`): what keeps it out of the DIRECT dependencies
/// is step 1g's pin row in `tests/pins_policy.rs`, not this gate.
///
/// Mutants (the contract's), each red: inline `UnwrapErr(SysRng)` at the
/// generator argument of `send.rs`'s create; plant a call in a file outside any
/// list (also watched by [`a_generator_planted_in_a_file_no_list_names_is_refused`]).
/// And for item 2: a fresh `SystemClock` at the create's clock argument.
#[test]
fn every_signing_generator_comes_from_the_one_seam() {
    let src = sdk_root().join("zec-wallet-core/src");
    let scan = seam_scan(&src);
    // Every site the matcher found, the seam's or refused, so a refused
    // generator reds the violations clause below with its own message, not the
    // non-vacuity clause.
    let count = |name: &str| scan.found.iter().filter(|what| *what == name).count();
    let creates = count("create_proposed_transactions");
    let wallet_dbs = count("from_connection") + count("for_path");
    println!(
        "the seam scan read {} production lines across {} files and found {creates} create \
         calls and {wallet_dbs} wallet DB constructions; {} sites name the seam, {} its clock",
        scan.lines_read,
        scan.files_read,
        scan.sites.len(),
        scan.clock_sites,
    );
    assert!(
        scan.files_read >= 30 && scan.lines_read >= 20_000,
        "the seam scan read {} production lines across {} files: the tree was not found, \
         or the resolver is eating production code",
        scan.lines_read,
        scan.files_read
    );
    // Not vacuous: the production create and the production wallet DB are read
    // where they stand. A matcher that lost its place finds neither.
    assert!(
        creates >= 1,
        "the scan found no production `create_proposed_transactions` call: the matcher \
         lost it (the wallet cannot sign without one)"
    );
    assert!(
        wallet_dbs >= 1,
        "the scan found no production wallet DB construction: the matcher lost it"
    );
    assert!(
        scan.violations.is_empty(),
        "ADR-0579 Decision 3: a generator reaches upstream from somewhere other than the \
         one seam, or with a clock other than the wallet DB's:\n{}",
        scan.violations.join("\n")
    );
}

/// The matcher's own honesty (IT-10: which mechanism refuses each case?). Each
/// planted case is refused for the reason named beside it, and the seam's own
/// shapes are admitted. Each case is its own file, so no finding can be another
/// case's.
#[test]
fn the_seam_scanner_refuses_each_planted_generator_and_admits_the_seam() {
    let findings = |src: &str| seam_findings(&numbered_production_lines(src), "planted.rs");
    // Refused for exactly the named reason, and for nothing else, so the case
    // cannot pass on a finding that belongs to another clause.
    let refused = |src: &str, because: &str| {
        let found = findings(src);
        assert!(
            found.violations.len() == 1 && found.violations[0].contains(because),
            "the planted case was not refused for `{because}` alone:\n{src}\nfindings: {:?}",
            found.violations
        );
    };

    // Contract item 2: the create's clock is the clock the wallet DB holds, the
    // seam's `wallet_db_clock()`. Any other clock is refused, a fixed one and a
    // fresh `SystemClock` alike: the second is a second clock source, which a
    // change to the seam's clock would not move.
    refused(
        "\
fn create_signed_core() {
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &FixedClock(0), &mut crate::rng::wallet_rng(), p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
}
",
        "clock is `&FixedClock(0)`",
    );
    refused(
        "\
fn create_signed_core() {
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &SystemClock, &mut crate::rng::wallet_rng(), p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
}
",
        "clock is `&SystemClock`",
    );
    refused(
        "\
fn open() {
    let wdb = WalletDb::from_connection(conn, params, zcash_client_sqlite::util::SystemClock, crate::rng::wallet_rng());
}
",
        "holds the clock `zcash_client_sqlite::util::SystemClock`",
    );

    // Admitted: the production create's shape, and the production wallet DB's.
    let admitted = findings(
        "\
use zcash_client_backend::data_api::wallet::{
    SpendingKeys, create_proposed_transactions, propose_transfer,
};
use crate::rng::{WalletDbClock, WalletRng};
fn create_signed_core() {
    let result = create_proposed_transactions::<
        _,
        _,
        std::convert::Infallible,
        StandardFeeRule,
        std::convert::Infallible,
        _,
    >(
        wdb,
        params,
        // the clock, in a comment that says SystemClock
        &crate::rng::wallet_db_clock(),
        // the generator, in a comment that says UnwrapErr(SysRng)
        &mut crate::rng::wallet_rng(),
        spend_prover,
        output_prover,
        &SpendingKeys::from_unified_spending_key(usk),
        OvkPolicy::Sender,
        proposal,
        None,
    );
}
fn open() {
    let mut wdb = WalletDb::from_connection(
        conn,
        network.consensus(),
        crate::rng::wallet_db_clock(),
        crate::rng::wallet_rng(),
    );
}
",
    );
    assert!(
        admitted.violations.is_empty(),
        "the seam's own shapes are admitted: {:?}",
        admitted.violations
    );
    assert_eq!(
        admitted.sites.len(),
        2,
        "both sites were read as naming the seam: {:?}",
        admitted.sites
    );
    assert_eq!(
        admitted.clock_sites.len(),
        2,
        "both sites were read as naming the seam's clock: {:?}",
        admitted.clock_sites
    );

    // The contract's first mutant: the OS generator inlined at the create.
    refused(
        "\
fn create_signed_core() {
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &crate::rng::wallet_db_clock(), &mut UnwrapErr(SysRng), p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
}
",
        "takes its generator from `&mutUnwrapErr(SysRng)`",
    );
    // A generator drawn before the call and handed in: not drawn at the create.
    refused(
        "\
use crate::rng::wallet_rng;
fn create_signed_core() {
    let mut rng = wallet_rng();
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &crate::rng::wallet_db_clock(), &mut rng, p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
}
",
        "takes its generator from `&mutrng`",
    );
    // The added item: a shielding entry point with a generator of its own.
    refused(
        "\
fn shield() {
    shield_transparent_funds(db, params, &crate::rng::wallet_db_clock(), &mut UnwrapErr(SysRng), p, p, &sel, &chg, t, &keys, &addrs, acct, policy);
}
",
        "`shield_transparent_funds` takes its generator",
    );
    // The added item: a PCZT signer method with a generator of its own.
    refused(
        "\
fn sign() {
    signer.sign_orchard(UnwrapErr(SysRng), index, &ask);
}
",
        "`sign_orchard` takes its generator",
    );
    // A method entry's name reaching the call through a macro that takes it as a
    // token: the name is not read as a call, so it is refused for that.
    refused(
        "\
macro_rules! call { ($o:expr, $m:ident, $r:expr) => { $o.$m($r) } }
fn sign() {
    call!(signer, sign_orchard, UnwrapErr(Fixed::seed(0)));
}
",
        "`sign_orchard` is named but not called",
    );
    // The rest of the list in one planted file: the PCZT IO finalizer, the three
    // provers, the extractor (its receiver on the line above, as a method chain
    // is laid out) and the migration's broadcast seam, each handed a generator
    // that is not the seam's. Each is refused once, by its own name, and nothing
    // else is.
    let pczt_roles = findings(
        "\
fn migrate() {
    let pczt = IoFinalizer::new(created).finalize_io(UnwrapErr(SysRng))?;
    let proved = Prover::new(pczt)
        .create_orchard_proof(UnwrapErr(SysRng), &orchard_pk)?
        .create_ironwood_proof(UnwrapErr(SysRng), &ironwood_pk)?
        .create_sapling_proofs(UnwrapErr(SysRng), &spend_prover, &output_prover)?
        .finish();
    let tx = TransactionExtractor::new(signed)
        .extract(UnwrapErr(SysRng))?;
    let tx = migrations.take_transaction_for_broadcast(UnwrapErr(SysRng), &state, id)?;
}
",
    );
    let planted_roles = [
        "finalize_io",
        "create_orchard_proof",
        "create_ironwood_proof",
        "create_sapling_proofs",
        "extract",
        "take_transaction_for_broadcast",
    ];
    for name in planted_roles {
        let refusal = format!("`{name}` takes its generator from `UnwrapErr(SysRng)`");
        let times = pczt_roles
            .violations
            .iter()
            .filter(|v| v.contains(&refusal))
            .count();
        assert_eq!(
            times, 1,
            "the planted `{name}` call was not refused exactly once: {:?}",
            pczt_roles.violations
        );
    }
    assert_eq!(
        pczt_roles.violations.len(),
        planted_roles.len(),
        "nothing but the six planted calls is refused: {:?}",
        pczt_roles.violations
    );
    // A wallet DB holding a generator that is not the seam's, built through an alias.
    refused(
        "\
fn open() {
    let wdb = WalletConn::for_path(path, params, crate::rng::wallet_db_clock(), UnwrapErr(SysRng));
}
",
        "`for_path` takes its generator from `UnwrapErr(SysRng)`",
    );
    // A wallet DB holding another clock.
    refused(
        "\
fn open() {
    let wdb = WalletDb::from_connection(conn, params, FixedClock(0), crate::rng::wallet_rng());
}
",
        "holds the clock `FixedClock(0)`",
    );
    // The entry renamed on import, then called by the new name.
    refused(
        "\
use zcash_client_backend::data_api::wallet::create_proposed_transactions as build;
fn create_signed_core() {
    build(wdb, params, &FixedClock(0), &mut UnwrapErr(SysRng), p, p, &keys, OvkPolicy::Sender, proposal, None);
}
",
        "is renamed on import",
    );
    // The entry taken as a value, to be called later.
    refused(
        "\
fn create_signed_core() {
    let build = create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>;
}
",
        "is named but not called",
    );
    // A function-local `use` covers its own statement, never the calls after it,
    // whatever it imports: the entry itself, the receiver's type, or nothing to
    // do with either.
    refused(
        "\
pub(crate) fn shield_now() {
    use zcash_client_backend::data_api::wallet::shield_transparent_funds;
    shield_transparent_funds(db, params, &crate::rng::wallet_db_clock(), &mut UnwrapErr(Fixed::seed(0)), p, p, &sel, &chg, t, &keys, &addrs, acct, policy);
}
",
        "`shield_transparent_funds` takes its generator from `&mutUnwrapErr(Fixed::seed(0))`",
    );
    refused(
        "\
pub(crate) fn finish(pczt: Pczt) {
    use pczt::roles::tx_extractor::TransactionExtractor;
    let tx = TransactionExtractor::new(pczt).extract(UnwrapErr(Fixed::seed(0)));
}
",
        "`extract` takes its generator from `UnwrapErr(Fixed::seed(0))`",
    );
    refused(
        "\
pub(crate) fn sign(signer: Signer) {
    use std::convert::Infallible;
    signer.sign_orchard(UnwrapErr(Fixed::seed(0)), 0, &ask);
}
",
        "`sign_orchard` takes its generator from `UnwrapErr(Fixed::seed(0))`",
    );
    refused(
        "\
fn sneaky() { use std::io; create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(wdb, params, &crate::rng::wallet_db_clock(), &mut UnwrapErr(SysRng), p, p, &keys, OvkPolicy::Sender, proposal, None); }
",
        "`create_proposed_transactions` takes its generator from `&mutUnwrapErr(SysRng)`",
    );
    // A bare spelling of the seam is refused at every gated position, whatever
    // the file imports and whatever the bare name turns out to be: a zero-field
    // tuple variant of the seam's name glob-imported after the call, a binding
    // behind a string that spells `use crate::rng::`, and the plain bare call
    // under the seam's own import, for the generator and for the clock.
    refused(
        "\
use crate::rng::wallet_rng;
fn create_signed_core() {
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &crate::rng::wallet_db_clock(), &mut wallet_rng(), p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
    #[allow(non_camel_case_types)]
    enum Gen { wallet_rng() }
    use Gen::*;
}
",
        "takes its generator from `&mutwallet_rng()`",
    );
    refused(
        "\
use crate::rng::wallet_rng;
fn create_signed_core() {
    run(\"use crate::rng::\", |wallet_rng: fn() -> Fixed| create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &crate::rng::wallet_db_clock(), &mut wallet_rng(), p, p, &keys, OvkPolicy::Sender, proposal, None,
    ));
}
",
        "takes its generator from `&mutwallet_rng()`",
    );
    refused(
        "\
use crate::rng::wallet_rng;
fn create_signed_core() {
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &crate::rng::wallet_db_clock(), &mut wallet_rng(), p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
}
",
        "takes its generator from `&mutwallet_rng()`",
    );
    refused(
        "\
use crate::rng::wallet_db_clock;
fn create_signed_core() {
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &wallet_db_clock(), &mut crate::rng::wallet_rng(), p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
}
",
        "clock is `&wallet_db_clock()`",
    );
    // A comment that names an entry is not a call.
    let commented = findings(
        "\
// create_proposed_transactions(wdb, params, &SystemClock, &mut UnwrapErr(SysRng))
/// `shield_transparent_funds` is not called here
fn f() {}
",
    );
    assert!(
        commented.violations.is_empty() && commented.sites.is_empty(),
        "a comment is not a call: {:?}",
        commented.violations
    );
}

/// The contract's second mutant, at the tree level: a call planted in a file no
/// list names is found by the walk, because the walk reads every production
/// file of `src/`. A test-only file holding the same call is skipped, so the
/// exclusion is the resolver's and not a list's.
#[test]
fn a_generator_planted_in_a_file_no_list_names_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("src dir");
    let planted = "\
pub(crate) fn sign_elsewhere() {
    create_proposed_transactions::<_, _, Infallible, StandardFeeRule, Infallible, _>(
        wdb, params, &crate::rng::wallet_db_clock(), &mut UnwrapErr(SysRng), p, p, &keys, OvkPolicy::Sender, proposal, None,
    );
}
";
    fs::write(
        src.join("lib.rs"),
        "mod rng;\nmod elsewhere;\n#[cfg(test)]\nmod fixtures;\n",
    )
    .expect("lib.rs");
    fs::write(
        src.join("rng.rs"),
        "pub(crate) fn wallet_rng() -> WalletRng {\n    UnwrapErr(SysRng)\n}\n",
    )
    .expect("rng.rs");
    fs::write(src.join("elsewhere.rs"), planted).expect("elsewhere.rs");
    fs::write(src.join("fixtures.rs"), planted).expect("fixtures.rs");

    let scan = seam_scan(&src);
    assert_eq!(
        scan.files_read, 3,
        "lib.rs, rng.rs and elsewhere.rs are production; fixtures.rs is test-only"
    );
    assert_eq!(
        scan.violations.len(),
        1,
        "exactly the planted production call is refused: {:?}",
        scan.violations
    );
    assert!(
        scan.violations[0].starts_with("elsewhere.rs:2:"),
        "the refusal names the planted file and line: {:?}",
        scan.violations
    );
}

// ── The NU7 wave's step 4: the activation check's source pins ─────────────────
//
// `docs/plan/nu7-wave.md` §3 step 4 and ADR-0579 Decisions 4 and 6, written by the
// step's test author against the contract, blind to the implementation. They sit
// beside this file's resolver for the reason the alias pin does: it is the one
// place that knows which `src/` files are production code.
//
// THE KERNEL IS FOUND BY ITS SHAPE, NOT ITS NAME. The contract names its three
// answers (`Never`, `Through(h)`, `Nowhere`) and leaves every other name to the
// implementer. So "the kernel" here is: the one production `enum` whose variants
// are exactly those three; every `fn` in the file that declares it whose return
// type names it (the outer function over the header bytes and the inner one that
// takes `H`); every `impl` block of it (where `Through(h)` is judged against a
// tip); and every function of that file those parts call, transitively (a helper
// that reads `H` for them is the kernel too: step 4's adjudication found the
// built `H` read in such a helper, where the first cut of this finder never
// looked). A second enum with those variants is a failure of its own: two kernels
// is the defect the source gate exists to refuse.

/// The kernel's answers, as the contract names them (S-N1).
const KERNEL_VARIANTS: [&str; 3] = ["Never", "Nowhere", "Through"];

/// One production file of the core's `src/`, comment-stripped, its test modules
/// taken out by this file's resolver: `lines[n]` is line `n`, empty where line
/// `n` is not production code.
struct ProductionFile {
    relative: String,
    lines: Vec<String>,
}

fn core_production_files() -> Vec<ProductionFile> {
    let src = sdk_root().join("zec-wallet-core/src");
    let test_only = test_only_files(std::slice::from_ref(&src));
    let mut out = Vec::new();
    for file in rust_files(&src) {
        if file
            .canonicalize()
            .is_ok_and(|canonical| test_only.contains(&canonical))
        {
            continue;
        }
        let text =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        let mut lines = vec![String::new(); text.lines().count() + 1];
        for (number, code) in production_lines(&text) {
            lines[number] = code;
        }
        let relative = file
            .strip_prefix(&src)
            .unwrap_or(&file)
            .display()
            .to_string();
        out.push(ProductionFile { relative, lines });
    }
    out
}

/// Files and non-empty production lines read, with the anti-vacuity floor the
/// alias pin uses: a resolver that eats production code fails here first.
fn production_read(files: &[ProductionFile], scan: &str) -> (usize, usize) {
    let lines: usize = files
        .iter()
        .map(|f| f.lines.iter().filter(|l| !l.trim().is_empty()).count())
        .sum();
    assert!(
        files.len() >= 30 && lines >= 20_000,
        "{scan} read {lines} production lines across {} files: the tree was not found, \
         or the resolver is eating production code",
        files.len()
    );
    (files.len(), lines)
}

/// Whether `code` names `ident` as a whole identifier.
fn names_ident(code: &str, ident: &str) -> bool {
    let bytes = code.as_bytes();
    code.match_indices(ident).any(|(at, _)| {
        (at == 0 || !is_ident_byte(bytes[at - 1]))
            && bytes
                .get(at + ident.len())
                .is_none_or(|b| !is_ident_byte(*b))
    })
}

/// `code` with the contents of its string and char literals blanked, so a brace
/// inside `"{…}"` or `'{'` is never counted. A lifetime (`'a`) is left alone.
fn without_literals(code: &str) -> String {
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::with_capacity(code.len());
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                out.push('"');
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
                out.push('"');
                i += 1;
            }
            '\'' if chars.get(i + 2) == Some(&'\'') => {
                out.push_str("' '");
                i += 3;
            }
            '\'' if chars.get(i + 1) == Some(&'\\') => {
                out.push_str("' '");
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// A line's item keyword and name once its visibility and qualifiers are off:
/// `("fn", name)`, `("enum", name)` or `("impl", header)`, else `None`.
fn item_head(line: &str) -> Option<(&'static str, String)> {
    let mut l = line.trim();
    for v in ["pub(crate) ", "pub(super) ", "pub "] {
        if let Some(rest) = l.strip_prefix(v) {
            l = rest;
        }
    }
    while let Some(rest) = ["async ", "const ", "unsafe "]
        .iter()
        .find_map(|q| l.strip_prefix(q))
    {
        l = rest;
    }
    let ident = |rest: &str| -> String {
        rest.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    };
    if let Some(rest) = l.strip_prefix("fn ") {
        return Some(("fn", ident(rest)));
    }
    if let Some(rest) = l.strip_prefix("enum ") {
        return Some(("enum", ident(rest)));
    }
    if l.starts_with("impl ") || l.starts_with("impl<") {
        return Some(("impl", l.to_string()));
    }
    None
}

/// One `fn`, `enum` or `impl` item of a production file: its kind, its name (an
/// `impl`'s whole header), the text from its first line up to the brace that
/// opens its body (the signature, `where` clauses included), and its line span.
struct ItemSpan {
    kind: &'static str,
    name: String,
    header: String,
    start: usize,
    end: usize,
}

/// Every `fn`, `enum` and `impl` item of `lines` with its brace-matched span. A
/// bodiless `fn` (a trait method's declaration) spans its signature only.
fn item_spans(lines: &[String]) -> Vec<ItemSpan> {
    let mut out = Vec::new();
    for start in 0..lines.len() {
        let Some((kind, name)) = item_head(&lines[start]) else {
            continue;
        };
        let mut header = String::new();
        let mut depth = 0usize;
        // Parentheses and brackets before the body opens: a `;` inside them
        // (`[u8; 32]`) is a type, not the end of a bodiless declaration.
        let mut nest = 0usize;
        let mut opened = false;
        let mut end = start;
        'scan: for (j, line) in lines.iter().enumerate().skip(start) {
            let code = without_literals(line);
            for c in code.chars() {
                match c {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' => depth = depth.saturating_sub(1),
                    '(' | '[' if !opened => nest += 1,
                    ')' | ']' if !opened => nest = nest.saturating_sub(1),
                    ';' if !opened && nest == 0 => {
                        end = j;
                        break 'scan;
                    }
                    _ => {}
                }
                if !opened {
                    header.push(c);
                }
            }
            if !opened {
                header.push(' ');
            }
            if opened && depth == 0 {
                end = j;
                break;
            }
            end = j;
        }
        out.push(ItemSpan {
            kind,
            name,
            header,
            start,
            end,
        });
    }
    out
}

/// The variants of the enum whose span is `span`: the identifiers that open a
/// line at the enum's own brace depth.
fn enum_variants(lines: &[String], span: &ItemSpan) -> Vec<String> {
    let mut variants = Vec::new();
    let mut depth = 0usize;
    for line in &lines[span.start..=span.end] {
        let code = without_literals(line);
        let trimmed = code.trim();
        if depth == 1
            && trimmed
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        {
            variants.push(
                trimmed
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect(),
            );
        }
        for c in code.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    variants
}

/// One item of the kernel: its first and last line, and what it is (`enum X`,
/// `impl X`, `fn f`).
type KernelPart = (usize, usize, String);

/// A kernel: its file, its enum's name, and the items that make it up.
type Kernel = (String, String, Vec<KernelPart>);

/// Whether `code` (comment-stripped) calls a function named `name`: the name as a
/// whole identifier, followed by `(` or a turbofish, and reached as a free call or
/// through `Self::` / `self::`. A method call on some value (`.name(`), a path into
/// another type or module (`u32::from(`) and the definition itself (`fn name(`) are
/// not a call of a function of this file.
fn calls_fn(code: &str, name: &str) -> bool {
    let code = without_literals(code);
    let bytes = code.as_bytes();
    code.match_indices(name).any(|(at, _)| {
        let after = &code[at + name.len()..];
        if !(after.starts_with('(') || after.starts_with("::<")) {
            return false;
        }
        if at > 0 && (is_ident_byte(bytes[at - 1]) || bytes[at - 1] == b'.') {
            return false;
        }
        let before = &code[..at];
        if before.trim_end().ends_with("fn") {
            return false;
        }
        match before.strip_suffix("::") {
            None => true,
            Some(path) => {
                let segment = path
                    .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default();
                segment == "Self" || segment == "self"
            }
        }
    })
}

/// The kernel, found by its shape: (file, enum name, the line spans that are the
/// kernel). Empty when no production enum has exactly the contract's variants.
fn find_kernels(files: &[ProductionFile]) -> Vec<Kernel> {
    let mut kernels = Vec::new();
    for file in files {
        let spans = item_spans(&file.lines);
        for span in spans.iter().filter(|s| s.kind == "enum") {
            let mut variants = enum_variants(&file.lines, span);
            variants.sort();
            if variants != KERNEL_VARIANTS {
                continue;
            }
            let name = span.name.clone();
            let mut parts = vec![(span.start, span.end, format!("enum {name}"))];
            for item in &spans {
                let returns_it = item.kind == "fn"
                    && item
                        .header
                        .split("->")
                        .skip(1)
                        .any(|ret| names_ident(ret, &name));
                let impl_of_it = item.kind == "impl" && names_ident(&item.header, &name);
                if returns_it {
                    parts.push((item.start, item.end, format!("fn {}", item.name)));
                } else if impl_of_it {
                    parts.push((item.start, item.end, format!("impl {name}")));
                }
            }
            // The functions of this file the parts call, to a fixed point. A function
            // already inside a part (a method of the kernel's own impl) is not added
            // twice.
            loop {
                let inside = |item: &ItemSpan, parts: &[KernelPart]| {
                    parts
                        .iter()
                        .any(|(s, e, _)| *s <= item.start && item.end <= *e)
                };
                let called: Vec<KernelPart> = spans
                    .iter()
                    .filter(|item| item.kind == "fn" && !inside(item, &parts))
                    .filter(|item| {
                        parts.iter().any(|(s, e, _)| {
                            file.lines[*s..=*e]
                                .iter()
                                .any(|line| calls_fn(line, &item.name))
                        })
                    })
                    .map(|item| (item.start, item.end, format!("fn {}", item.name)))
                    .collect();
                if called.is_empty() {
                    break;
                }
                parts.extend(called);
            }
            kernels.push((file.relative.clone(), name, parts));
        }
    }
    kernels
}

/// The kernel pin's two findings over one kernel's parts in `lines`: whether any
/// part calls `height_bounds`, and the parts that name `KNOWN_UPGRADES`.
fn kernel_h_findings(lines: &[String], parts: &[KernelPart]) -> (bool, Vec<String>) {
    let body = |s: usize, e: usize| lines[s..=e].join("\n");
    let reads_bounds = parts
        .iter()
        .any(|(s, e, _)| names_ident(&body(*s, *e), "height_bounds"));
    let reads_the_list = parts
        .iter()
        .filter(|(s, e, _)| names_ident(&body(*s, *e), "KNOWN_UPGRADES"))
        .map(|(_, _, what)| what.clone())
        .collect();
    (reads_bounds, reads_the_list)
}

/// Every place `code` (one comment-stripped production line) compares an expiry,
/// as the contract's matcher reads it (L3): a free-function call of `past_expiry`,
/// and a `<`, `<=`, `>` or `>=` one of whose operands names an expiry (`expiry`,
/// `expiry_height`, `.expiry`, `expiry_height()`). An operand is the run of
/// non-space text beside the operator, with unbalanced parentheses trimmed:
/// rustfmt spaces every binary operator, and never a generic's angle brackets.
fn expiry_comparisons(code: &str) -> Vec<String> {
    let code = without_literals(code);
    let mut hits = Vec::new();
    for (at, _) in code.match_indices("past_expiry(") {
        let before = &code[..at];
        // A longer identifier, a method on some value, or the definition itself:
        // none of them is a call of the free function.
        if before
            .as_bytes()
            .last()
            .is_some_and(|b| is_ident_byte(*b) || *b == b'.')
            || before.trim_end().ends_with("fn")
        {
            continue;
        }
        hits.push("a call of `past_expiry`".to_string());
    }
    let trim_operand = |raw: &str| -> String {
        let mut s = raw.trim_matches(|c: char| c == ',' || c == ';' || c == '{' || c == '!');
        loop {
            let opens = s.matches('(').count();
            let closes = s.matches(')').count();
            if opens > closes && s.starts_with('(') {
                s = &s[1..];
            } else if closes > opens && s.ends_with(')') {
                s = &s[..s.len() - 1];
            } else {
                break;
            }
        }
        s.to_string()
    };
    for op in [" <= ", " >= ", " < ", " > "] {
        for (at, _) in code.match_indices(op) {
            let left = trim_operand(code[..at].rsplit(' ').next().unwrap_or_default());
            let right = trim_operand(code[at + op.len()..].split(' ').next().unwrap_or_default());
            if left.contains("expiry") || right.contains("expiry") {
                hits.push(format!("`{left}{op}{right}`"));
            }
        }
    }
    hits
}

/// **`the_kernel_reads_h_from_height_bounds_never_known_upgrades`** (ADR-0579
/// Decision 4; `docs/plan/nu7-wave.md` §3 step 4). `H`, the height at which the
/// transaction's own branch stops, comes from the crate's
/// `BranchId::height_bounds(params)`, and never from the hand-kept
/// `KNOWN_UPGRADES`, whose job is the anchor ceiling. A list that lags the crate
/// (as `KNOWN_UPGRADES` did, ending at `Nu6_3` while the crate's newest branch is
/// `Nu7`) would give a send under the last branch it knows no `H`, and the send
/// the upgrade killed would read as still on its way.
///
/// Over the kernel found by its shape (above), the helpers of its file it calls
/// included: exactly one kernel exists, its items call `height_bounds`, and none
/// of them names `KNOWN_UPGRADES`.
///
/// Mutant: name `KNOWN_UPGRADES` in the kernel, or in a helper of its file that
/// it calls. Red.
#[test]
fn the_kernel_reads_h_from_height_bounds_never_known_upgrades() {
    let files = core_production_files();
    let (files_read, lines_read) = production_read(&files, "the kernel pin");
    let kernels = find_kernels(&files);
    println!(
        "the kernel pin read {lines_read} production lines across {files_read} files; \
         kernels found: {:?}",
        kernels
            .iter()
            .map(|(file, name, parts)| format!(
                "{file} {name}: {}",
                parts
                    .iter()
                    .map(|(s, e, what)| format!("{what} ({s}-{e})"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        kernels.len(),
        1,
        "exactly one production enum answers `Never` / `Through(h)` / `Nowhere` (ADR-0579 \
         Decision 4: ONE kernel); found {}",
        kernels.len()
    );
    let (file, name, parts) = &kernels[0];
    let lines = &files
        .iter()
        .find(|f| &f.relative == file)
        .expect("the kernel's file was read")
        .lines;
    assert!(
        parts.iter().any(|(_, _, what)| what.starts_with("fn ")),
        "the kernel `{name}` in {file} has no function returning it: nothing computes the answer"
    );
    let (reads_bounds, reads_the_list) = kernel_h_findings(lines, parts);
    assert!(
        reads_bounds,
        "the kernel `{name}` in {file} never calls `height_bounds`: `H` must be the upper \
         bound of the transaction's own branch, from the crate"
    );
    assert!(
        reads_the_list.is_empty(),
        "the kernel reads the hand-kept `KNOWN_UPGRADES` in {reads_the_list:?} ({file}): `H` \
         comes from `BranchId::height_bounds`, never from a list that can lag the crate"
    );
}

/// **`only_the_kernel_compares_an_expiry_with_a_tip`** (ADR-0579 Decision 4: "a
/// source gate over all of `src/` allows only the kernel to compare an expiry with
/// a tip"; the matcher is the contract's L3, see [`expiry_comparisons`]). Five
/// readers judge whether a signed transaction can still be mined; a sixth with its
/// own comparison is a reader the upgrade check never reaches. It prints the count.
///
/// Mutants, each red: an inline `expiry <= tip` in `delivery.rs`; the same in a new
/// file under no list; `u32::from(h) > expiry` re-added in `enhance.rs`.
#[test]
fn only_the_kernel_compares_an_expiry_with_a_tip() {
    let files = core_production_files();
    let (files_read, lines_read) = production_read(&files, "the expiry gate");
    let kernels = find_kernels(&files);
    let mut inside = 0usize;
    let mut outside = Vec::new();
    for file in &files {
        let kernel_spans: Vec<(usize, usize)> = kernels
            .iter()
            .filter(|(f, _, _)| f == &file.relative)
            .flat_map(|(_, _, parts)| parts.iter().map(|(s, e, _)| (*s, *e)))
            .collect();
        for (number, code) in file.lines.iter().enumerate() {
            for hit in expiry_comparisons(code) {
                if kernel_spans
                    .iter()
                    .any(|(s, e)| (*s..=*e).contains(&number))
                {
                    inside += 1;
                } else {
                    outside.push(format!(
                        "{}:{number}: {hit} in `{}`",
                        file.relative,
                        enclosing_item(&file.lines, number)
                    ));
                }
            }
        }
    }
    println!(
        "the expiry gate read {lines_read} production lines across {files_read} files: \
         {} expiry comparisons, {inside} inside the kernel, {} outside it",
        inside + outside.len(),
        outside.len()
    );
    assert!(
        outside.is_empty(),
        "an expiry is compared outside the kernel; every reader asks the kernel instead \
         (ADR-0579 Decision 4):\n{}",
        outside.join("\n")
    );
    assert_eq!(
        kernels.len(),
        1,
        "the gate allows exactly one kernel (an enum answering `Never` / `Through(h)` / \
         `Nowhere`); found {}: with none, nothing in `src/` can judge an expiry at all",
        kernels.len()
    );
}

/// The keep-list of ADR-0579 Decision 6 and the plan's consumer table, as
/// (file under `src/`, enclosing item): the production items allowed to read
/// `REORG_MAX_BLOCKS`. The constant's own definition; the scan cap
/// (`MAX_SCAN_REORGS_PER_PASS`); the new-wallet birthday lag; the grace clamp
/// (`grace_anchor`); the bind's reorg window (`plan_pool_fetch`); `tip_reference`;
/// and `sync.rs`'s import those two need. Each keeps 100 for a reason that is not
/// a finality decision.
const REORG_KEEPERS: &[(&str, &str)] = &[
    ("constants.rs", "REORG_MAX_BLOCKS"),
    ("constants.rs", "MAX_SCAN_REORGS_PER_PASS"),
    ("constants.rs", "NEW_WALLET_BIRTHDAY_LAG_BLOCKS"),
    ("consensus.rs", "grace_anchor"),
    ("sync.rs", "plan_pool_fetch"),
    ("sync.rs", "tip_reference"),
    ("sync.rs", "use"),
];

/// The one compile-time check the finality pin admits (step 4's adjudication): in
/// `constants.rs`, an anonymous `const` that statically asserts a quantity is at
/// least `REORG_MAX_BLOCKS`. That is the burial depth's ruled floor, "never below 100"
/// (ADR-0579 Decision 6). It runs in the compiler and decides nothing at run time,
/// so it is no finality consumer. It is admitted by its SHAPE, never by its item
/// name `_`, which every anonymous `const` of the file shares (see
/// [`is_a_static_floor_assertion`]).
const FLOOR_ASSERTION_FILE: &str = "constants.rs";

/// Whether `statement` (one item's comment-stripped text, any line breaks) is the
/// floor's static assertion: `const _: () = assert!(<lhs> >= REORG_MAX_BLOCKS);`,
/// optionally with `as u32`, `as u64` or `as usize` after the constant, the one `>=`
/// in it, and the constant named nowhere else. Anything more (a message, a second
/// clause, the constant on the left, an `==` that ties a depth to it) is not
/// admitted.
fn is_a_static_floor_assertion(statement: &str) -> bool {
    let text = statement.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(inner) = text
        .strip_prefix("const _: () = assert!(")
        .and_then(|rest| rest.strip_suffix(");"))
    else {
        return false;
    };
    let mut sides = inner.trim().split(" >= ");
    let (Some(lhs), Some(rhs), None) = (sides.next(), sides.next(), sides.next()) else {
        return false;
    };
    let rhs = rhs.trim();
    !names_ident(lhs, "REORG_MAX_BLOCKS")
        && (rhs == "REORG_MAX_BLOCKS"
            || rhs
                .strip_prefix("REORG_MAX_BLOCKS as ")
                .is_some_and(|ty| ["u32", "u64", "usize"].contains(&ty)))
}

/// The statement of the anonymous `const` whose span holds `line`: from the nearest
/// `const _` line at or above it to the first line ending in `;` at or below it.
fn anonymous_const_statement(lines: &[String], line: usize) -> Option<String> {
    let head = (0..=line)
        .rev()
        .find(|j| lines[*j].trim_start().starts_with("const _"))?;
    let end = (line..lines.len()).find(|j| lines[*j].trim_end().ends_with(';'))?;
    Some(lines[head..=end].join(" "))
}

/// The finality pin's reading of `files`: the production lines that name
/// `REORG_MAX_BLOCKS` outside the keepers and the floor's static assertion
/// (`file:line `item``), the kept items seen, how many lines name the constant, and
/// how many of them the floor's assertion holds.
struct ReorgReaders {
    violations: Vec<String>,
    seen: HashSet<(String, String)>,
    lines: usize,
    floors: usize,
}

fn reorg_readers(files: &[ProductionFile]) -> ReorgReaders {
    let readers = items_naming(files, "REORG_MAX_BLOCKS");
    let mut out = ReorgReaders {
        violations: Vec::new(),
        seen: HashSet::new(),
        lines: readers.len(),
        floors: 0,
    };
    for (file, item, number) in &readers {
        let key = (file.as_str(), item.as_str());
        if REORG_KEEPERS.contains(&key) {
            out.seen.insert((file.clone(), item.clone()));
            continue;
        }
        let lines = &files
            .iter()
            .find(|f| &f.relative == file)
            .expect("the reader's file was read")
            .lines;
        let floor = file == FLOOR_ASSERTION_FILE
            && item == "_"
            && anonymous_const_statement(lines, *number)
                .is_some_and(|statement| is_a_static_floor_assertion(&statement));
        if floor {
            out.floors += 1;
        } else {
            out.violations.push(format!("{file}:{number} `{item}`"));
        }
    }
    out
}

/// Every production item naming `ident`, as (file, item, line).
fn items_naming(files: &[ProductionFile], ident: &str) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    for file in files {
        for (number, code) in file.lines.iter().enumerate() {
            if names_ident(code, ident) {
                out.push((
                    file.relative.clone(),
                    enclosing_item(&file.lines, number),
                    number,
                ));
            }
        }
    }
    out
}

/// **`no_finality_consumer_reads_reorg_max_blocks`** (ADR-0579 Decision 6; the
/// plan's consumer table). Every irreversible step and every finality claim to the
/// host follows `irreversibly_buried` (15,000 s of chain time): delete-on-final,
/// strand, requeue and through it the swap deposit's purge, reap, the
/// stranded-amount weld and the host's finality badge. The badge follows too
/// (ruled), so the pin allows no finality consumer at all: the only production
/// items that may read `REORG_MAX_BLOCKS` are the keepers ([`REORG_KEEPERS`]), plus
/// the burial floor's static assertion in `constants.rs` ([`FLOOR_ASSERTION_FILE`]).
/// The reclaim margin is no keeper: since step 5 it follows the predicate too
/// (`docs/plan/nu7-wave.md` §3 step 5). A new reader of the constant is a finality
/// decision at 100 blocks and fails here by default.
///
/// Mutants, each red: read `REORG_MAX_BLOCKS` in `confirmation_depth_is_final`;
/// read it in the stranded weld (`account.rs`); read it in
/// `EPHEMERAL_RECLAIM_DEADNESS_BLOCKS`, the reclaim margin's depth.
#[test]
fn no_finality_consumer_reads_reorg_max_blocks() {
    let files = core_production_files();
    let (files_read, lines_read) = production_read(&files, "the finality pin");
    let ReorgReaders {
        violations,
        seen,
        lines,
        floors,
    } = reorg_readers(&files);
    println!(
        "the finality pin read {lines_read} production lines across {files_read} files: \
         {lines} lines name REORG_MAX_BLOCKS, {} kept items seen, {floors} in the floor's \
         static assertion",
        seen.len()
    );
    assert!(
        violations.is_empty(),
        "a finality consumer reads REORG_MAX_BLOCKS; it follows `irreversibly_buried` \
         (ADR-0579 Decision 6), and only the keep-list keeps 100:\n{}",
        violations.join("\n")
    );
    let mut stale: Vec<String> = REORG_KEEPERS
        .iter()
        .filter(|(file, item)| !seen.contains(&(file.to_string(), item.to_string())))
        .map(|(file, item)| format!("{file} `{item}`"))
        .collect();
    stale.sort();
    assert!(
        stale.is_empty(),
        "kept items that no longer read REORG_MAX_BLOCKS: remove them from the list: \
         {stale:?}"
    );
}

/// **`the_keepers_do_not_read_the_irreversible_predicate`** (ADR-0579 Decision 6:
/// "`REORG_MAX_BLOCKS` stays 100 for the keep-list"; the plan's D1 keep-list, R-2).
/// The keepers bound an endpoint's over-claim, place an anchor, cap a pass's work
/// or grade a bundle; none is a finality decision, and routing one through the
/// predicate would move it from 100 blocks to 200 or 600 with nobody having asked.
/// The predicate exists in production and is read there (else this pin would pass
/// on a tree that never built it), and no keeper names it. A global edit of the
/// constant reds `extraction_policy`'s `named_constants_at_boundary` instead.
///
/// Mutant: route `grace_anchor` through the new predicate. Red.
#[test]
fn the_keepers_do_not_read_the_irreversible_predicate() {
    const PREDICATE: &str = "irreversibly_buried";
    let files = core_production_files();
    let (files_read, lines_read) = production_read(&files, "the keepers pin");
    let named = items_naming(&files, PREDICATE);
    let defined = files.iter().any(|f| {
        f.lines
            .iter()
            .any(|l| item_head(l).is_some_and(|(kind, name)| kind == "fn" && name == PREDICATE))
    });
    let callers: Vec<&(String, String, usize)> = named
        .iter()
        .filter(|(_, item, _)| item != PREDICATE)
        .collect();
    println!(
        "the keepers pin read {lines_read} production lines across {files_read} files: \
         `{PREDICATE}` defined: {defined}; named by {} production lines outside its definition",
        callers.len()
    );
    assert!(
        defined && !callers.is_empty(),
        "`{PREDICATE}` is not defined in production code, or nothing reads it: the \
         irreversible steps have no predicate to follow (ADR-0579 Decision 6)"
    );
    let keepers_reading: Vec<String> = callers
        .iter()
        .filter(|(file, item, _)| REORG_KEEPERS.contains(&(file.as_str(), item.as_str())))
        .map(|(file, item, number)| format!("{file}:{number} `{item}`"))
        .collect();
    assert!(
        keepers_reading.is_empty(),
        "a keeper reads the irreversible predicate; it keeps REORG_MAX_BLOCKS (100):\n{}",
        keepers_reading.join("\n")
    );
}

/// The scanners above, against planted lines: each of the contract's named
/// mutant shapes is a hit, and the shapes that are not comparisons of an expiry
/// are not. The plants are the mutants' own text, so a matcher that stops seeing
/// them fails here before a mutant watch is ever run.
#[test]
fn the_step_4_scanners_see_the_mutants_shapes_and_nothing_else() {
    // The contract's three gate mutants, as lines.
    assert_eq!(expiry_comparisons("    if expiry <= tip {").len(), 1);
    assert_eq!(
        expiry_comparisons(
            "    if h > evidence.known_tip || h < birthday || (expiry != 0 && u32::from(h) > expiry) {"
        )
        .len(),
        1,
        "the shape `chain_status` has today"
    );
    assert_eq!(
        expiry_comparisons("            && past_expiry(row.expiry_height, tip)").len(),
        1
    );
    assert_eq!(
        expiry_comparisons("        Some(e) if tx.expiry_height() >= target => {}").len(),
        1
    );
    assert_eq!(
        expiry_comparisons("    let ok = self.expiry > tip;").len(),
        1
    );
    // Not comparisons of an expiry: a definition, a method of the kernel, a generic,
    // an arrow, a comparison of other heights, and an expiry inside a string.
    for not_a_hit in [
        "pub(crate) fn past_expiry(expiry: u32, tip: u32) -> bool {",
        "    let dead = answer.past_expiry(tip);",
        "fn read_tx_expiry<DbT>(wdb: &DbT, txid: [u8; 32]) -> Result<Option<u32>, WalletError>",
        "            Some(expiry) => past_expiry_of(expiry, tip),",
        "    if h > evidence.known_tip || h < birthday {",
        "    tracing::warn!(\"expiry <= tip\");",
    ] {
        assert!(
            expiry_comparisons(not_a_hit).is_empty(),
            "not an expiry comparison: {not_a_hit}"
        );
    }

    // The kernel finder: an enum with exactly the three answers, the functions that
    // return it and its impl, and nothing else in the file.
    let planted: Vec<String> = [
        "",
        "pub(crate) enum Mineable {",
        "    Never,",
        "    Through(u32),",
        "    Nowhere,",
        "}",
        "impl Mineable {",
        "    pub(crate) fn dead_at(self, tip: u32) -> bool {",
        "        match self { Mineable::Through(h) => h <= tip, Mineable::Nowhere => true, Mineable::Never => false }",
        "    }",
        "}",
        "pub(crate) fn mineable_through<P: Parameters>(",
        "    raw: &[u8],",
        "    expiry: u32,",
        "    params: &P,",
        ") -> Mineable {",
        "    let h = BranchId::try_from(0).ok().and_then(|b| b.height_bounds(params));",
        "    inner(h, expiry)",
        "}",
        "fn reader(expiry: u32, tip: u32) -> bool {",
        "    expiry <= tip",
        "}",
    ]
    .iter()
    .map(|l| l.to_string())
    .collect();
    let kernels = find_kernels(&[ProductionFile {
        relative: "planted.rs".to_string(),
        lines: planted.clone(),
    }]);
    assert_eq!(kernels.len(), 1, "the planted kernel is found");
    let parts: Vec<&str> = kernels[0].2.iter().map(|(_, _, w)| w.as_str()).collect();
    assert_eq!(
        parts,
        vec!["enum Mineable", "impl Mineable", "fn mineable_through"],
        "the enum, its impl and the function returning it; never the reader"
    );
    let reader_line = planted
        .iter()
        .position(|l| l.trim() == "expiry <= tip")
        .expect("planted");
    assert!(
        kernels[0]
            .2
            .iter()
            .all(|(s, e, _)| !(*s..=*e).contains(&reader_line)),
        "the reader's comparison sits outside every kernel span"
    );
    // A fourth variant is a different enum, never the kernel.
    let mut four = planted.clone();
    four.insert(4, "    Sometimes,".to_string());
    assert!(
        find_kernels(&[ProductionFile {
            relative: "planted.rs".to_string(),
            lines: four,
        }])
        .is_empty()
    );

    // The kernel's helpers (step 4's adjudication): `H` read in a function of the
    // kernel's file that the kernel calls, two calls deep, is the kernel's; a
    // function nothing in the kernel calls is not.
    let helper: Vec<String> = [
        "pub(crate) enum Mineable {",
        "    Never,",
        "    Through(u32),",
        "    Nowhere,",
        "}",
        "pub(crate) fn effective_expiry<P: Parameters>(params: &P, raw: &[u8], expiry: u32) -> Mineable {",
        "    match next_activation(params, raw) {",
        "        Some(h) => Mineable::Through(h.min(expiry)),",
        "        None => Mineable::Never,",
        "    }",
        "}",
        "fn next_activation<P: Parameters>(params: &P, raw: &[u8]) -> Option<u32> {",
        "    let upper = branch_of(raw).and_then(|b| b.height_bounds(params)).and_then(|(_, u)| u);",
        "    upper.map(u32::from)",
        "}",
        "fn branch_of(raw: &[u8]) -> Option<BranchId> {",
        "    BranchId::try_from(u32::from_le_bytes(raw[8..12].try_into().ok()?)).ok()",
        "}",
        "fn reader(expiry: u32, tip: u32) -> bool {",
        "    expiry <= tip",
        "}",
    ]
    .iter()
    .map(|l| l.to_string())
    .collect();
    let kernels = find_kernels(&[ProductionFile {
        relative: "planted.rs".to_string(),
        lines: helper.clone(),
    }]);
    assert_eq!(kernels.len(), 1, "the planted kernel is found");
    let parts: Vec<&str> = kernels[0].2.iter().map(|(_, _, w)| w.as_str()).collect();
    assert_eq!(
        parts,
        vec![
            "enum Mineable",
            "fn effective_expiry",
            "fn next_activation",
            "fn branch_of"
        ],
        "the helpers the kernel calls, transitively, are the kernel; never the reader"
    );
    assert_eq!(
        kernel_h_findings(&helper, &kernels[0].2),
        (true, Vec::new()),
        "`H` from `height_bounds` in a helper is the kernel's `H`: the pin is green"
    );
    // The contract's mutant, planted in the helper: red.
    let mut listed = helper.clone();
    let at = listed
        .iter()
        .position(|l| l.trim() == "upper.map(u32::from)")
        .expect("planted");
    listed.insert(at, "    let _ = KNOWN_UPGRADES.len();".to_string());
    let kernels = find_kernels(&[ProductionFile {
        relative: "planted.rs".to_string(),
        lines: listed.clone(),
    }]);
    assert_eq!(
        kernel_h_findings(&listed, &kernels[0].2).1,
        vec!["fn next_activation".to_string()],
        "`KNOWN_UPGRADES` in a helper the kernel calls is seen"
    );
    // The same name in a function the kernel never calls is not the kernel's.
    let mut elsewhere = helper.clone();
    elsewhere.push("fn anchor_ceiling() -> usize {".to_string());
    elsewhere.push("    KNOWN_UPGRADES.len()".to_string());
    elsewhere.push("}".to_string());
    let kernels = find_kernels(&[ProductionFile {
        relative: "planted.rs".to_string(),
        lines: elsewhere.clone(),
    }]);
    assert_eq!(
        kernel_h_findings(&elsewhere, &kernels[0].2),
        (true, Vec::new()),
        "a function the kernel does not call is not the kernel"
    );
    // What a call is: a free call or one through `Self::`; never a method of some
    // value, a path into another type, or the definition.
    for (code, called) in [
        ("    match next_activation(params, raw) {", true),
        ("    let h = Self::next_activation(params, raw);", true),
        ("    let h = next_activation::<P>(params, raw);", true),
        ("    let h = cache.next_activation(params);", false),
        ("    let h = Other::next_activation(params);", false),
        (
            "    pub(crate) fn next_activation(params: &P) -> u32 {",
            false,
        ),
        ("    let h = my_next_activation(params);", false),
        ("    tracing::debug!(\"next_activation(\");", false),
    ] {
        assert_eq!(calls_fn(code, "next_activation"), called, "{code}");
    }

    // The finality pin admits the burial floor's static assertion in `constants.rs`,
    // by its shape, and nothing else that names the constant.
    let finality = |file: &str, text: &[&str]| -> (Vec<String>, usize) {
        let mut lines = vec![String::new()];
        lines.extend(text.iter().map(|l| l.to_string()));
        let r = reorg_readers(&[ProductionFile {
            relative: file.to_string(),
            lines,
        }]);
        (r.violations, r.floors)
    };
    for floor in [
        &[
            "const _: () =",
            "    assert!(IRREVERSIBLE_BURIAL_SECS / TARGET_BLOCK_SPACING_SECS >= REORG_MAX_BLOCKS as u64);",
        ][..],
        &[
            "const _: () = assert!(IRREVERSIBLE_BURIAL_SECS / TARGET_BLOCK_SPACING_SECS >= REORG_MAX_BLOCKS as u64);",
        ][..],
        &[
            "const _: () = assert!(",
            "    IRREVERSIBLE_BURIAL_SECS / TARGET_BLOCK_SPACING_SECS >= REORG_MAX_BLOCKS as u64",
            ");",
        ][..],
    ] {
        assert_eq!(
            finality("constants.rs", floor),
            (Vec::new(), 1),
            "the floor's static assertion is admitted: {floor:?}"
        );
    }
    for (file, text, why) in [
        (
            "stranded.rs",
            &[
                "const _: () =",
                "    assert!(IRREVERSIBLE_BURIAL_SECS / TARGET_BLOCK_SPACING_SECS >= REORG_MAX_BLOCKS as u64);",
            ][..],
            "the same assertion outside constants.rs",
        ),
        (
            "constants.rs",
            &["pub(crate) const IRREVERSIBLE_DEPTH: u32 = REORG_MAX_BLOCKS * 2;"][..],
            "a depth derived from the constant",
        ),
        (
            "constants.rs",
            &["const _: () = assert!(IRREVERSIBLE_DEPTH == REORG_MAX_BLOCKS as u32 * 2);"][..],
            "an assertion that ties a depth to the constant",
        ),
        (
            "constants.rs",
            &["const _: () = assert!(REORG_MAX_BLOCKS >= 100);"][..],
            "the constant on the left",
        ),
        (
            "constants.rs",
            &[
                "const _: () = assert!(A >= REORG_MAX_BLOCKS as u64 && B >= REORG_MAX_BLOCKS as u64);",
            ][..],
            "a second clause",
        ),
        (
            "constants.rs",
            &["const _: () = assert!(DEPTH >= REORG_MAX_BLOCKS as u64, \"never below 100\");"][..],
            "a message",
        ),
        (
            "constants.rs",
            &[
                "pub(crate) fn is_final(depth: u32) -> bool {",
                "    depth >= REORG_MAX_BLOCKS",
                "}",
            ][..],
            "a run-time comparison",
        ),
    ] {
        let (violations, floors) = finality(file, text);
        assert!(
            violations.len() == 1 && floors == 0,
            "{why} is a finality reader, not the floor: {violations:?}"
        );
    }
}
