//! §4.3a custody-surface policy tests (wallet-sdk §8 register).
//!
//! These are SOURCE-shape gates: honest small guarantees over the files
//! where the §4.3a rules are easiest to silently violate. The behavioral
//! halves run elsewhere — the wrap/binding/rotation semantics in
//! zec-wallet-core's keychain tests (real AEAD), and the JNI plumbing on the
//! device E2E (`wrap_key_roundtrip_via_keystore`, manual gate). The
//! BINARY-symbol-table half of the export check rides the ndk lane (nm on
//! the built .so) — a source gate cannot see linker output.

use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("policy test must read {}: {e}", path.display()))
}

/// The gates police CODE, not prose: strip `//`-style and javadoc/block
/// comment lines so documentation may NAME the hazards it forbids.
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
            // Single-line /* ... */ lines are skipped too (the else-less
            // fallthrough would otherwise include their text in the scan).
            if !trimmed.contains("*/") {
                in_block = true;
            }
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("*") {
            continue;
        }
        // Drop trailing line comments too.
        let code = line.split("//").next().unwrap_or(line);
        out.push_str(code);
        out.push('\n');
    }
    out
}

/// §4.3a: the Java courier is a context courier and NOTHING else — no key
/// material, no crypto imports, no method channels, no extra native
/// methods, application-context-only. (Named for the spec row; the courier
/// is Java, not Kotlin — an as-implemented note, same shape.)
#[test]
fn kotlin_shim_is_courier_only() {
    let courier = manifest_dir()
        .join("../android/src/main/java/com/flutter_rust_bridge/zec_wallet/ZecWalletPlugin.java");
    let src = strip_comments(&read(&courier));

    // No crypto surface, no channels, no storage, no logging of anything.
    for forbidden in [
        "javax.crypto",
        "java.security",
        "KeyStore",
        "Cipher",
        "MethodChannel",
        "EventChannel",
        "BasicMessageChannel",
        "SharedPreferences",
        "byte[]",
        "Log.",
    ] {
        assert!(
            !src.contains(forbidden),
            "courier must not reference `{forbidden}` — it is a context \
             courier only (§4.3a)"
        );
    }

    // Exactly one native method, and it carries a Context — nothing else.
    let native_count = src.matches("static native").count();
    assert_eq!(native_count, 1, "exactly one native method (nativeInit)");
    assert!(
        src.contains("private static native void nativeInit(android.content.Context context)"),
        "the one native method is nativeInit(Context)"
    );

    // The stripper's blind spot is string literals — so the courier simply
    // may not HAVE any (other than the loadLibrary name): hostile tokens
    // cannot hide where no strings exist (security fold).
    assert_eq!(
        src.matches('"').count(),
        2,
        "courier carries exactly one string literal: the loadLibrary name"
    );
    assert!(src.contains("System.loadLibrary(\"zec_wallet\")"));

    // The context handed over is the APPLICATION context, never an Activity.
    assert!(
        src.contains("getApplicationContext()"),
        "courier must pass binding.getApplicationContext()"
    );
    assert!(
        !src.contains("Activity"),
        "courier must never touch an Activity (leak + lifecycle hazard)"
    );
}

/// §4.3a: the cdylib exports exactly ONE Java_* entry point, the custody
/// JNI code contains zero `unsafe`, and both honest-transit zero-fills are
/// present next to their `doFinal` calls. Source-level tripwire — drift
/// (a second export, a removed zero-fill) fails here before review.
#[test]
fn jni_custody_zeroizes_and_exports_one_symbol() {
    // Half 1: the export set. jni_mangle is the only sanctioned way to
    // export, and it appears exactly once across the bridge's handwritten
    // sources (frb_generated.rs is FRB's own surface, not JNI exports).
    let bridge_android = strip_comments(&read(&manifest_dir().join("src/android.rs")));
    assert_eq!(
        bridge_android.matches("#[jni_mangle(").count(),
        1,
        "exactly one #[jni_mangle] export in the bridge"
    );
    for src_name in ["src/lib.rs", "src/convert.rs", "src/api/mod.rs"] {
        let src = strip_comments(&read(&manifest_dir().join(src_name)));
        assert!(
            !src.contains("jni_mangle") && !src.contains("export_name"),
            "{src_name} must not export JNI symbols"
        );
    }
    assert!(
        !bridge_android.contains("JNI_OnLoad"),
        "no JNI_OnLoad — the as-implemented design needs none (§4.3a)"
    );

    // Half 2: zero unsafe in the custody path (both crates' android code).
    let core_android = strip_comments(&read(
        &manifest_dir().join("../../zec-wallet-core/src/keychain/android.rs"),
    ));
    for (name, src) in [
        ("bridge android.rs", &bridge_android),
        ("core android.rs", &core_android),
    ] {
        assert!(
            !src.contains("unsafe "),
            "{name} must contain zero unsafe — the safe jni API suffices"
        );
    }
    assert!(
        bridge_android.contains("#![deny(unsafe_code)]"),
        "bridge android module carries the handwritten-module deny"
    );

    // Half 3: the honest-transit zero-fills. Both doFinal directions
    // (wrap's plaintext-in, unwrap's plaintext-out) zero the Java array.
    assert_eq!(
        core_android
            .matches("set_region(env, 0, &[0i8; 32])")
            .count(),
        2,
        "both Cipher.doFinal paths zero-fill the Java-side key bytes (§4.3a)"
    );

    // Half 4: key bytes never flow INTO a Java method other than the
    // Cipher path: the only byte_array_from_slice of key material is the
    // doFinal plaintext (everything else is aad/iv/ct — non-secret).
    assert_eq!(
        core_android
            .matches("byte_array_from_slice(seal_key.as_bytes())")
            .count(),
        1,
        "exactly one key-bytes JNI crossing (the wrap doFinal input)"
    );
    assert!(
        !core_android.contains("as_bytes()).unwrap"), /* no unchecked escapes */
        "key-byte crossings stay on the checked path"
    );
}
