//! Two checks that go RED if this crate stops being safe to ship inside a
//! third-party product. Both read files; neither needs cargo or the network.
//! The equix/hashx bans live in THIS crate's own `deny.toml` `bans` section
//! (`sdk/dialer-tor/deny.toml`; `just sdk-audit` runs cargo-deny there — the
//! SDK workspace's `sdk/deny.toml` never sees arti's graph); this test is the
//! net that runs on every `cargo test -p dialer-tor`, against THIS
//! workspace's lock.
//!
//! Copied from the first host's Tor transport crate (ADR-0550): there the lock sat
//! two levels up (the monorepo root); here the crate is ITS OWN workspace root
//! at `sdk/dialer-tor` (its manifest says why it cannot join `sdk/`'s), so the
//! lock is the crate directory's own `Cargo.lock`. Any other level would read
//! a different workspace's lock with no arti in it — which fails the
//! anti-vacuity floor below rather than passing silently, but the honest path
//! is the right one.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("this crate's own directory is its workspace root")
}

fn locked_package_names() -> HashSet<String> {
    let lock = workspace_root().join("Cargo.lock");
    let text = std::fs::read_to_string(&lock)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", lock.display()));
    text.lines()
        .filter_map(|l| l.strip_prefix("name = \""))
        .filter_map(|l| l.strip_suffix('"'))
        .map(str::to_owned)
        .collect()
}

/// `equix` and `hashx` are LGPL-3.0-only. They are reachable from arti only
/// through `tor-hscrypto/hs-pow-full` — and `tor-hscrypto` itself IS in the
/// graph (via `tor-keymgr`), so crate-absence is not the invariant and reading
/// the feature list is not evidence. The lockfile is.
#[test]
fn licence_carve_out_holds() {
    let names = locked_package_names();

    // Anti-vacuity: prove the reader found the real lockfile before believing
    // anything it does NOT contain. A typo'd path or a changed lock format
    // would otherwise report a clean carve-out.
    assert!(
        names.len() > 100,
        "only {} packages parsed out of Cargo.lock; the reader is broken, not the graph",
        names.len()
    );
    for expected in ["arti-client", "tor-hscrypto", "tor-proto", "dialer-tor"] {
        assert!(
            names.contains(expected),
            "{expected} is not in Cargo.lock, so this test cannot see arti's graph at all"
        );
    }

    for forbidden in ["equix", "hashx"] {
        assert!(
            !names.contains(forbidden),
            "{forbidden} (LGPL-3.0-only) entered the dependency graph. An arti \
             onion-service or hs-pow feature was turned on; this crate can ship \
             inside a third-party product and must not carry LGPL code."
        );
    }

    // The maintainer-gated TLS provider, banned in deny.toml for the same class of
    // reason. arti's rustls backend installs it as a TEST fallback, so a stray
    // `testing` feature would drag it in.
    for forbidden in ["aws-lc-rs", "aws-lc-sys"] {
        assert!(
            !names.contains(forbidden),
            "{forbidden} entered the graph; rustls must stay on the ring provider"
        );
    }

    // `pt-client` is OFF, and this is the only thing that grades it.
    //
    // A DIFFERENT class from the two above: not a licence and not a provider,
    // but a delivery shape — a pluggable transport needs
    // an external executable arti launches at a configured path, which this
    // crate's first consumer refused and iOS cannot have. `bridge-client` adds
    // no package at all, so a green run over the rows above proves NOTHING
    // about this one; `tor-ptmgr` is the name that appears the moment somebody
    // turns the other feature on, and the copy would then be offering a
    // capability the build cannot deliver.
    assert!(
        !names.contains("tor-ptmgr"),
        "tor-ptmgr entered the graph: `pt-client` is on. D13 clause 3 keeps it \
         off at v1.0b and the surface states that limit instead — a build that \
         carries it needs the refusal copy re-derived, not this test relaxed"
    );
}

/// A lexical guard, and only a lexical guard: it cannot prove the crate never
/// opens a socket, only that the obvious spellings are absent. The structural
/// half is `TorStream`'s private field plus its `pub(crate)` constructor.
#[test]
fn no_source_file_reaches_for_a_socket() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut read = 0usize;
    let mut saw_datastream = false;

    let entries = std::fs::read_dir(&src).expect("the crate has a src/ directory");
    for entry in entries {
        let path = entry.expect("readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable source file");
        read += 1;
        saw_datastream |= text.contains("DataStream");
        for needle in ["TcpStream", "UdpSocket", "TcpListener", "SocketAddr::"] {
            assert!(
                !text.contains(needle),
                "{} mentions {needle}: this crate has no path to the network except arti",
                path.display()
            );
        }
    }

    // Anti-vacuity again: a reader that found zero files, or files with none of
    // the crate's own vocabulary, proves nothing by finding no sockets.
    assert!(read >= 4, "only {read} source files were read");
    assert!(
        saw_datastream,
        "no source file mentions DataStream; the reader is not reading this crate"
    );
}
