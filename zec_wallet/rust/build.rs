fn main() {
    // Android 16 KB page-size compliance (Android 15+ / Google Play requirement).
    //
    // NDK r27 and earlier do NOT default to 16 KB-aligned ELF LOAD segments; the
    // app pins NDK r27 (flutter_zxing's zxing-cpp build fails on r28+, upstream
    // #225), so without this our cdylib ships 4 KB-aligned (`p_align = 0x1000`)
    // and a device flags "ELF alignment check failed" / Play rejects the upload.
    // Force the linker's max page size to 16 KB (`p_align = 0x4000`).
    //
    // WHY a link-arg and not RUSTFLAGS: cargokit's Android build sets
    // `CARGO_ENCODED_RUSTFLAGS`, which cargo treats as the single exclusive
    // source of rustflags — a `.cargo/config.toml` `rustflags` (or `--config`)
    // would be silently ignored. `cargo:rustc-link-arg` is passed to rustc as
    // `-Clink-arg` independently of RUSTFLAGS, so it survives that env.
    //
    // Gated to Android: Apple `ld` and MSVC `link.exe` reject `-z max-page-size`
    // (desktop targets are first-class). Harmless on r28+ (already aligned) and
    // on debug builds. Single colon `cargo:` keeps this MSRV-safe.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }

    // FR-19 (#395): on Linux, bind this cdylib's OWN bundled SQLCipher to itself.
    //
    // We link `bundled-sqlcipher-vendored-openssl`, so the wallet `.so` carries
    // its own `sqlite3_*` (SQLCipher, encrypted-DB critical). A host that ALSO
    // links SQLCipher and loads both libs hits Linux's default FLAT namespace +
    // `RTLD_GLOBAL` plugin loading: the FIRST-loaded `sqlite3_*` wins the global
    // lookup, so the wallet's internal calls can resolve to the *host's* copy —
    // a plain-SQLite or version/cipher-mismatched SQLCipher — and the encrypted
    // wallet DB silently fails to open. `-Bsymbolic` binds every reference to a
    // symbol DEFINED in this `.so` to that local definition at link time, so the
    // wallet's `sqlite3_*` (and its vendored OpenSSL) calls resolve to ITS OWN
    // copy regardless of load order. The `.dynsym` EXPORT table is untouched, so
    // the `zec_wallet_*` FFI surface and the Android `Java_*` shim stay callable
    // via `dlsym`; and UNDEFINED symbols (our libc `malloc` etc.) are unaffected,
    // so an `LD_PRELOAD` heap/observability shim still interposes them.
    //
    // DEFENSE-IN-DEPTH, likely redundant (be honest): rustc's cdylib codegen
    // localizes non-exported symbols, so the bundled `sqlite3_*` may never reach
    // `.dynsym` at all — in which case they cannot be preempted and the wallet's
    // internal calls already bind locally, with or without this flag. VERIFIED on
    // the arm64-android build: it exports its 35 FFI globals (incl `zec_wallet_*`)
    // but ZERO `sqlite3_*`; macOS keeps them LOCAL too. The same rustc mechanism
    // applies to `x86_64-unknown-linux-gnu`, so the Linux `.so` very likely also
    // keeps `sqlite3_*` local and the classic RTLD bleed is already prevented.
    // We keep `-Bsymbolic` regardless: it is a byte-cheap safety net if a future
    // rustc/link-flag change ever re-exported the vendored symbols, and the CI
    // witness (`just fr19-symbolic-witness`) REPORTS whether `.dynsym` actually
    // exports any global `sqlite3_*` — turning "is this flag load-bearing?" into an
    // empirical line on the real Linux artifact rather than an assumption.
    //
    // Plain `-Bsymbolic` (functions AND data), not `-Bsymbolic-functions`: a
    // self-contained bundled-crypto `.so` WANTS its OpenSSL/SQLCipher data bound
    // locally too. The only hazard of binding DATA symbols locally is a copy
    // relocation (`R_*_COPY`) desync — but those are emitted by the linker of an
    // EXECUTABLE against a data symbol in a `.so` it names at STATIC-LINK time.
    // This artifact is consumed only via `dlopen`+`dlsym` / `System.loadLibrary`
    // (never an `-lzec_wallet` `NEEDED`), so no executable can emit a copy
    // relocation against its data symbols — that desync class is unreachable, and
    // the superset is both safe and a more complete isolation (the FR-19 targets,
    // `sqlite3_*`, are all functions, so `-Bsymbolic-functions` would fix the DB
    // open just the same; the extra data binding hardens the vendored crypto).
    //
    // Gated to Linux: this is an ELF/GNU-ld concept. macOS/iOS already use a
    // two-level namespace and Apple `ld` rejects `-Bsymbolic`; Windows binds
    // per-DLL — none have the bleed, so none need (or accept) the flag.
    //
    // `rustc-cdylib-link-arg` (NOT the broader `rustc-link-arg` the Android line
    // uses): scope the flag to the SHARED OBJECT only. `-Bsymbolic` is about a
    // `.so`'s namespace behavior — applying it to the test/bench executables that
    // also link the staticlib would be meaningless and could mask an interposition
    // test. Same `CARGO_ENCODED_RUSTFLAGS`-survival reason as the Android arg above
    // (a `.cargo/config.toml` rustflag would be silently dropped under cargokit).
    // Single colon `cargo:` keeps this MSRV-safe.
    //
    // Coverage: every shipped Linux consumer loads the cdylib via `dlopen`
    // (cargokit emits `staticlib` only for Apple `-force_load`; the Linux/Windows
    // CMake path bundles the `.so`), so cdylib scope covers 100% of them. A
    // hypothetical future Linux host that STATIC-links `libzec_wallet.a` into its
    // own executable would not inherit this flag — `-Bsymbolic` is a producer
    // LINK-TIME arg, uncapturable in an archive — so such a consumer must pass
    // `-Wl,-Bsymbolic` on its OWN link. No shipped consumer does this today.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-Bsymbolic");
    }

    // SCAN-1 (§4o S5) — the device-timing door, door (b) of the contract.
    //
    // The measurement build (`just wallet-device-timing-apk`) needs a RELEASE
    // `.so` that still writes the per-batch `wallet.sync` line to logcat; the
    // logcat layer in `api/meta.rs` is the only sink, it is debug-only, and the
    // maintainer's rule forbids quoting a sync speed from a debug build. So
    // `init_app` installs that same layer under `cfg(zec_wallet_device_timing)`,
    // and THIS is where the cfg comes from: emitted only when
    // `ZEC_WALLET_DEVICE_TIMING=1` is in the build's environment. OFF in every
    // build nobody set it in — a default `flutter build`, a pub.dev consumer's,
    // CI's — and nothing committed arms it (door (a), a `cargokit.yaml`
    // `extra_flags` feature, ships in the package and would arm every consumer's
    // profile build). `rerun-if-env-changed` makes flipping the variable rebuild
    // the crate; `rustc-check-cfg` declares the name so `unexpected_cfgs` stays
    // quiet on every target. Why an env var and not RUSTFLAGS: cargokit sets
    // `CARGO_ENCODED_RUSTFLAGS` (above), under which plain `RUSTFLAGS` never
    // reaches rustc; a build-script env read survives it. The variable reaches
    // this script through Gradle → cargokit (`includeParentEnvironment = true`);
    // a long-lived Gradle daemon born without it is the one hop that can drop
    // it, which is why the recipe stops the daemon first and why the logcat
    // `loop_start` line — not the build log — is the proof the door was open.
    // `tests/extraction_policy.rs::the_device_timing_door_is_closed_in_every_default_build`
    // pins this shape: the emission sits inside the env-gated branch, once.
    println!("cargo:rustc-check-cfg=cfg(zec_wallet_device_timing)");
    println!("cargo:rerun-if-env-changed=ZEC_WALLET_DEVICE_TIMING");
    println!("cargo:rerun-if-env-changed=CARGOKIT_CONFIGURATION");
    if std::env::var("ZEC_WALLET_DEVICE_TIMING").as_deref() == Ok("1") {
        // THE PROFILE GUARD (review repair, §4o S5): the variable alone is
        // not the door. cargokit forwards the Flutter configuration as
        // `CARGOKIT_CONFIGURATION` (plugin.gradle → build_tool, the parent env
        // inherited), so a shell that exported the variable and then ran a
        // Flutter RELEASE build is refused HERE, loudly, at build time — never
        // a release artifact that logs. Outside cargokit there is no
        // configuration to read: a bare `cargo build --release` of this crate is
        // a distributable artifact too, so it is refused the same way; a bare
        // debug build is allowed (its `debug_assertions` door is open anyway).
        // Only Flutter `profile` (cargo release under cargokit) and `debug` pass,
        // and the artifact that passes marks itself: `sdk_version()` ends in
        // `+devtiming`.
        let configuration = std::env::var("CARGOKIT_CONFIGURATION").ok();
        let profile = std::env::var("PROFILE").unwrap_or_default();
        match configuration.as_deref() {
            Some("release") => panic!(
                "ZEC_WALLET_DEVICE_TIMING=1 is set but this is a Flutter RELEASE build: \
                 the device-timing door never arms a release artifact. Unset the \
                 variable, or build with --profile (just wallet-device-timing-apk)."
            ),
            None if profile == "release" => panic!(
                "ZEC_WALLET_DEVICE_TIMING=1 is set on a release build outside cargokit \
                 (no CARGOKIT_CONFIGURATION): the door is for Flutter profile builds \
                 only. Unset the variable."
            ),
            _ => {}
        }
        println!("cargo:rustc-cfg=zec_wallet_device_timing");
    }

    bridge_abi_gate();
}

/// FR-33 — the bridge's wire-contract version (`src/bridge_abi.rs`) is forced to
/// move with the generated codec, HERE, at build time.
///
/// Why a build script and not only a test (the FR-33 security review's
/// MEDIUM): no test runs at commit time on this track, and the pre-push hook is
/// inert while `.push-gate` stands — so a regen committed without a test run
/// would ship a changed codec under the old version, which is FR-33's exact
/// failure. Every `cargo build`, `check` and `clippy` runs this, so the
/// pre-commit `sdk-clippy` leg refuses such a commit, and so does any device
/// build.
///
/// Three checks, each failing with the exact fix:
/// * the history `BRIDGE_ABI_HISTORY` is contiguous from 1 and its last version
///   is `BRIDGE_ABI_VERSION`;
/// * its last fingerprint is the committed `src/frb_generated.rs` now;
/// * the Dart package's `kBridgeAbiVersion` (`../lib/src/bridge_abi.dart`) is
///   the same number.
///
/// The fingerprint is FNV-1a 64 over the NON-WHITESPACE bytes: a change
/// detector for a file this repo generates and commits, not cryptography —
/// nothing adversarial chooses its input. Whitespace is dropped so a rustfmt
/// difference between machines is not a contract change; a REORDERING rustfmt
/// makes still is — conservative, and harmless: a spurious bump only forces the
/// two halves to be rebuilt together. The file fingerprinted is the canonical
/// one `wallet-bridge-verify` pins (regenerated, then `cargo fmt -p zec_wallet`).
///
/// Append-only is a CONVENTION, as for the frozen derivation labels: an
/// in-place edit of an existing row is visible in review, not here. Two rows
/// MAY name the same fingerprint — a reverted codec change gets the next
/// version, never an edited row (the review's LOW).
fn bridge_abi_gate() {
    use std::path::{Path, PathBuf};

    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let generated = root.join("src/frb_generated.rs");
    let module = root.join("src/bridge_abi.rs");
    let dart = root.join("../lib/src/bridge_abi.dart");
    for path in [&generated, &module, &dart] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let read = |path: &Path| {
        std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!(
                "FR-33 bridge ABI gate: {} is unreadable: {e}",
                path.display()
            )
        })
    };

    let fingerprint = read(&generated)
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });

    let module_src = read(&module);
    let version: u32 = value_after(&module_src, "pub const BRIDGE_ABI_VERSION: u32 = ")
        .parse()
        .expect("BRIDGE_ABI_VERSION is an integer literal");
    let start = module_src
        .find("const BRIDGE_ABI_HISTORY: &[(u32, u64)] = &[")
        .expect("src/bridge_abi.rs declares BRIDGE_ABI_HISTORY");
    let block = &module_src[start..];
    let block = &block[..block.find("];").expect("BRIDGE_ABI_HISTORY closes")];
    let history: Vec<(u32, u64)> = block
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let inner = line
                .trim_start_matches('(')
                .trim_end_matches(',')
                .trim_end_matches(')');
            let (v, fp) = inner
                .split_once(',')
                .expect("a history row is `(version, 0x…)`");
            let fp = fp.trim().trim_start_matches("0x").replace('_', "");
            (
                v.trim().parse().expect("a history version is an integer"),
                u64::from_str_radix(&fp, 16).expect("a history fingerprint is hex"),
            )
        })
        .collect();
    assert!(
        !history.is_empty(),
        "FR-33 bridge ABI gate: BRIDGE_ABI_HISTORY has no row"
    );
    for (i, (v, _)) in history.iter().enumerate() {
        assert!(
            *v as usize == i + 1,
            "FR-33 bridge ABI gate: BRIDGE_ABI_HISTORY must be contiguous from 1 (row {i} says {v})"
        );
    }
    let (last_version, last_fingerprint) = history[history.len() - 1];
    assert!(
        last_version == version,
        "FR-33 bridge ABI gate: BRIDGE_ABI_VERSION is {version} but the history's last row is \
         version {last_version}"
    );
    if last_fingerprint != fingerprint {
        let next = version + 1;
        panic!(
            "FR-33 bridge ABI gate: src/frb_generated.rs's wire code changed since bridge ABI \
             version {version} was recorded. A Dart package generated before this change would \
             decode this library's messages wrongly. Append `({next}, {fingerprint:#018x})` to \
             BRIDGE_ABI_HISTORY in src/bridge_abi.rs, set BRIDGE_ABI_VERSION = {next}, and set \
             kBridgeAbiVersion = {next} in lib/src/bridge_abi.dart (never edit an existing row)."
        );
    }

    let dart_src = read(&dart);
    let declared: Vec<&str> = dart_src
        .lines()
        .filter_map(|line| line.trim().strip_prefix("const int kBridgeAbiVersion = "))
        .collect();
    assert!(
        declared.len() == 1,
        "FR-33 bridge ABI gate: lib/src/bridge_abi.dart must declare `const int kBridgeAbiVersion \
         = N;` exactly once (found {declared:?})"
    );
    let dart_version: u32 = declared[0]
        .trim_end_matches(';')
        .trim()
        .parse()
        .expect("kBridgeAbiVersion is an integer literal");
    assert!(
        dart_version == version,
        "FR-33 bridge ABI gate: the Dart package's kBridgeAbiVersion ({dart_version}) differs from \
         BRIDGE_ABI_VERSION ({version}) — a package built from this tree would refuse its own \
         library. Move them together."
    );

    /// The text after `prefix` up to the line's `;`.
    fn value_after<'a>(src: &'a str, prefix: &str) -> &'a str {
        let at = src
            .find(prefix)
            .unwrap_or_else(|| panic!("`{prefix}` is not in the source"));
        let rest = &src[at + prefix.len()..];
        rest[..rest.find(';').expect("the declaration ends with `;`")].trim()
    }
}
