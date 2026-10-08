//! Crate-level policy tests — the manifest-shape half of the §8 rows
//! `tor_required_covers_swap_api_calls` (behavioral half: provider tests)
//! and the extraction policy (spec §1.2): this crate must be INCAPABLE of
//! independent network access, and every dep must be publicly resolvable
//! except the extraction-unit-internal `zec-wallet-core` path dep.

use std::fs;
use std::path::PathBuf;

fn manifest() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("crate manifest readable")
}

/// Strip comments; return only the `[dependencies]` section lines.
fn dependency_lines(manifest: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut out = Vec::new();
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim().to_owned();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if in_deps && !line.is_empty() {
            out.push(line);
        }
    }
    out
}

/// Every `.rs` file this crate compiles, as `(path, source)`.
///
/// Its own sources ONLY — not a dependency's, not a sibling's. The question
/// this file asks is what THIS crate does, and the answer has to be read from
/// the code that ships in it.
fn crate_sources() -> Vec<(PathBuf, String)> {
    fn walk(dir: &PathBuf, out: &mut Vec<(PathBuf, String)>) {
        for entry in fs::read_dir(dir).expect("readable source dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let src = fs::read_to_string(&path).expect("readable source file");
                out.push((path, src));
            }
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &mut out);
    assert!(
        out.len() >= 5,
        "the walk found {} source files — a walk that finds nothing asserts nothing",
        out.len()
    );
    out
}

/// Source with line comments removed — ONE implementation, so the guard below and
/// the meta-guard that proves it can fail cannot drift apart (review LOW).
///
/// Not a lexer, and the limit is stated rather than hidden: a `//` inside a
/// string literal truncates the rest of THAT line, so a socket API written after
/// such a literal on the same line is invisible here. rustfmt keeps one
/// statement per line, and the threat this guards against is our own future
/// code rather than an adversary editing around a scanner, so the residual is
/// accepted — but it is a residual, not a property.
fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// m5: this crate opens no socket of its own — the injected transport is the
/// ONLY way out.
///
/// **T0-5 rewrote this row, because the version before it proved nothing.** It
/// read the crate's own `[dependencies]` and asserted that `tokio` was declared
/// without `"net"` and `hyper` without `"server"`. Cargo UNIFIES features across
/// a workspace: there is ONE `tokio` in the graph, and `zec-wallet-core` needs
/// `net` for gRPC, so the tokio this crate links against HAS `net` (and `mio`,
/// and `socket2`) whatever this manifest says. MEASURED, not argued:
/// `cargo tree -e features -i tokio` from `sdk/` lists `tokio feature "net"`,
/// `"mio"` and `"socket2"` as enabled, reached through
/// `zcash_client_backend`'s `lightwalletd-tonic-transport`. The old assertions
/// were true of the manifest and false of the build, which is the shape of a
/// guard that passes forever.
///
/// So the row now asserts BOTH halves of what is actually enforceable:
///
/// 1. **The direct-dependency half (kept, and it IS real).** Rust cannot `use`
///    a crate that is not a direct dependency, so a `reqwest`/`socket2`/`ureq`
///    line appearing here is the only way this crate could name one. This is a
///    necessary condition, and unification cannot weaken it.
/// 2. **The source half (new, and it is the load-bearing one).** Because
///    `tokio/net` is unified ON, this crate CAN write `tokio::net::TcpStream`
///    today and it will compile. Nothing in any manifest can prevent that; only
///    reading the crate's own code can. So the code is read, and must name no
///    socket API.
///
/// What neither half can prove is that the *transport* this crate is handed
/// actually goes where it says. That is the honest trust boundary the module
/// doc states, and the `tor_required_covers_swap_api_calls` behavioural rows are
/// where it is exercised.
#[test]
fn adapter_cannot_open_its_own_sockets() {
    // 1. The direct-dependency half.
    let deps = dependency_lines(&manifest());
    for banned in [
        "reqwest",
        "native-tls",
        "openssl",
        "curl",
        "ureq",
        "socket2",
        "mio",
        "tokio-tungstenite",
    ] {
        assert!(
            !deps.iter().any(|l| l.starts_with(banned)),
            "socket-capable/forbidden dep `{banned}` in [dependencies] — this is the only way \
             this crate could NAME one"
        );
    }
    // The manifest must still not REQUEST these itself. It is no longer a
    // guarantee (unification decides what is compiled in), but a manifest that
    // asks for sockets is a declaration of intent, and it should have to be
    // written down here.
    let tokio_line = deps
        .iter()
        .find(|l| l.starts_with("tokio ") || l.starts_with("tokio="))
        .expect("tokio dependency present");
    assert!(
        !tokio_line.contains("\"net\""),
        "this crate must not REQUEST tokio `net` (it gets it anyway through unification — see \
         this row's doc — but asking for it is a different statement): {tokio_line}"
    );
    let hyper_line = deps
        .iter()
        .find(|l| l.starts_with("hyper "))
        .expect("hyper present");
    assert!(
        !hyper_line.contains("\"server\""),
        "hyper `server` feature has no business here: {hyper_line}"
    );

    // 2. The source half — the one that survives feature unification.
    const SOCKET_APIS: [&str; 9] = [
        "tokio::net",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "UnixStream",
        "UnixListener",
        "socket2",
        "hyper::server",
        "std::net::",
    ];
    for (path, src) in crate_sources() {
        // Strip line comments so this row's OWN doc (which names every one of
        // these) cannot trip it — the lesson: a falsifier that fires on its
        // own artifact is re-aimed, never struck.
        let code = strip_line_comments(&src);
        for api in SOCKET_APIS {
            assert!(
                !code.contains(api),
                "{}: names `{api}`. `tokio/net` IS compiled in for this crate (workspace \
                 feature unification — see this row's doc), so the manifest cannot stop a \
                 socket here and this assertion is what does. The injected transport is the \
                 only sanctioned way out",
                path.display()
            );
        }
    }
}

/// The source half above is only a guard if it can FAIL — and a substring scan
/// over a tree that happens to contain no matches looks identical to one whose
/// scanner is broken (`a_clean_allowlisted_field_set_has_no_false_positive`'s
/// sibling problem).
///
/// So: run the same scan over a synthetic file that DOES open a socket, and
/// assert it trips — including through the comment strip, which is the part
/// most likely to be wrong.
#[test]
fn the_socket_scan_actually_trips_on_a_socket() {
    let planted = "\
        use tokio::net::TcpStream;\n\
        // a comment mentioning TcpListener must NOT be what trips it\n\
        async fn sneak() { let _ = TcpStream::connect(\"1.2.3.4:80\").await; }\n";
    let code = strip_line_comments(planted);
    assert!(
        code.contains("tokio::net"),
        "the scan must see a real socket import through the comment strip"
    );
    assert!(
        !code.contains("TcpListener"),
        "…and the comment strip must remove a mention that is only prose — otherwise the guard \
         fires on its own documentation instead of on code"
    );

    let commented_only = "// TcpStream is discussed here and nowhere used\n";
    let stripped = strip_line_comments(commented_only);
    assert!(
        !stripped.contains("TcpStream"),
        "a file that only TALKS about sockets must not trip the guard"
    );
}

/// Extraction policy (spec §1.2, same shape as the core's
/// `core_and_bridge_have_no_relim_deps`): the ONE sanctioned path dep is
/// `zec-wallet-core` (the unit moves to a new repo together); everything else
/// resolves publicly.
#[test]
fn adapter_has_no_relim_deps_beyond_the_unit() {
    for line in dependency_lines(&manifest()) {
        let name = line.split(['=', ' ']).next().unwrap_or("").trim();
        if name == "zec-wallet-core" {
            continue; // the extraction-unit-internal exception
        }
        assert!(
            !name.starts_with("relim-"),
            "non-unit relim-* dependency: {line}"
        );
        assert!(
            !line.contains("path =") && !line.contains("git ="),
            "non-public dependency source: {line}"
        );
    }
}
