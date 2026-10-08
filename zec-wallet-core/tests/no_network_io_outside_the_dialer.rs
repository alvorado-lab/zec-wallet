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
