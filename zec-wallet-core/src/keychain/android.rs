//! §4.3a Android adapter — `AndroidKeyStore` over JNI, all SAFE `jni`-crate
//! API (this crate keeps `#![forbid(unsafe_code)]`; there is no hand-rolled
//! FFI here — the one JNI *export* lives in the loaded cdylib, see the
//! `zec_wallet` bridge crate's `android` module).
//!
//! As-implemented simplification vs the §4.3a draft (fold at review): no
//! `JNI_OnLoad` is needed at all. `nativeInit` receives the env, and
//! `Env::get_java_vm()` is safe — the courier's single call initializes
//! everything. One exported `Java_*` symbol, zero unsafe.
//!
//! Trust argument (§4.3a, verbatim design): the Context is INERT for
//! custody — `KeyStore.getInstance("AndroidKeyStore")` is a JCA *name*
//! lookup that never goes through the Context. The Context is used for
//! exactly one thing here: `PackageManager.hasSystemFeature(STRONGBOX)`.
//! `init_android_vault` is defensive: it stores
//! `context.getApplicationContext()`, never the raw caller object.
//!
//! Honest-transit discipline (§4.3a): the 32-byte wrap key transits the
//! Java heap as a `byte[]` for the duration of `Cipher.doFinal` — both
//! directions zero-fill the array immediately after (`set_byte_array_region`
//! of zeros), best-effort by JVM nature, stated, not hidden. The SEED never
//! crosses JNI in any form.
//!
//! CANNOT run on this dev host: compile-gated by
//! `cargo check --target aarch64-linux-android`; behavior-proven by the
//! on-device `wrap_key_roundtrip_via_keystore` manual E2E gate (§8).

use std::sync::OnceLock;

use jni::objects::{Global, JByteArray, JObject, JString, JValue};
use jni::{Env, JavaVM, jni_sig, jni_str};
use zeroize::Zeroizing;

use super::envelope::{self, WRAP_IV_LEN};
use super::{KeychainNamespace, KeychainPort, VaultTier, WrapArtifact, seal_key_from_vault};
use crate::error::WalletError;
use crate::seal::SealKey;

/// Keystore alias BASE; the production alias is `<ALIAS_BASE>.<namespace>.v<gen>`
/// (FR-13 per-wallet namespacing — see [`AndroidKeystoreVault::new`]). The
/// generation suffix makes one alias = one encryption (§4.3a rotation rule).
/// Versioned, per-device, NOT recovery-critical (the phrase + chain are the backup).
///
/// **BACKWARD-COMPAT (FR-13, pre-prod break):** a wallet provisioned BEFORE FR-13
/// holds aliases at the bare `zec_wallet.wrap.v<gen>`; post-FR-13 scans only the
/// namespaced `zec_wallet.wrap.<namespace>.v` prefix, and `'v'` is not a hex digit
/// so the old aliases are INVISIBLE to every new scan — the wallet surfaces a typed
/// `KeystoreInconsistent` (fail-closed, never a wrong open), recovery is
/// restore-from-mnemonic / re-provision. No automatic migration (no shipped
/// consumers); the orphaned old AES key lingers in the Keystore (FR-14 territory).
const ALIAS_BASE: &str = "zec_wallet.wrap";

/// S2 custody — the INDEX item's alias base. AndroidKeyStore holds KEYS, not
/// data, so the path-keyed index item is PRESENCE-ENCODED: one generated
/// (never used) key under `zec_wallet.custody-index.<namespace>.<hex(entry)>`
/// per wallet path; the item's identity is the alias, its "value" the encoded
/// suffix. The base diverges from `ALIAS_BASE` at `custody-index` vs `wrap`,
/// so the wrap generation scans (`.<ns>.v<gen>`, canonical-u32 gated) and the
/// index scan (`.<ns>.<hex>`) can never see each other's aliases.
const INDEX_ALIAS_BASE: &str = "zec_wallet.custody-index";

/// §4.3a device-E2E selftest aliases — DISJOINT from every production prefix.
/// Production prefixes are the family `zec_wallet.wrap.<32-hex>.v`; this one
/// diverges at `selftest.` vs `wrap.` (and `selftest`'s `s`/`t` are non-hex), so
/// neither is a prefix of the other and generation scans never cross (FR-13).
const SELFTEST_ALIAS_PREFIX: &str = "zec_wallet.selftest.wrap.v";

/// `PackageManager.FEATURE_STRONGBOX_KEYSTORE` (API-frozen string).
const FEATURE_STRONGBOX: &str = "android.hardware.strongbox_keystore";

// API-frozen Android constants (documented values, stable since their
// introduction — reading them reflectively would be motion without safety):
/// `KeyProperties.PURPOSE_ENCRYPT | PURPOSE_DECRYPT`.
const PURPOSES_ENCRYPT_DECRYPT: i32 = 1 | 2;
/// `Cipher.ENCRYPT_MODE` / `Cipher.DECRYPT_MODE`.
const ENCRYPT_MODE: i32 = 1;
const DECRYPT_MODE: i32 = 2;
/// GCM tag length in BITS (the §4.3a envelope's 16-byte tag).
const GCM_TAG_BITS: i32 = 128;
/// `KeyProperties.SECURITY_LEVEL_*` (API 31).
const SECURITY_LEVEL_TRUSTED_ENVIRONMENT: i32 = 1;
const SECURITY_LEVEL_STRONGBOX: i32 = 2;
/// `KeyProperties.SECURITY_LEVEL_UNKNOWN_SECURE` — secure but unattested.
const SECURITY_LEVEL_UNKNOWN_SECURE: i32 = -1;
// `SECURITY_LEVEL_SOFTWARE` (0) has no constant: `measured_tier`'s catch-all maps
// it, with any unrecognised level, to the degraded floor.
/// First API level with `setIsStrongBoxBacked`/`setUnlockedDeviceRequired`.
const API_28: i32 = 28;
/// First API level with `KeyInfo.getSecurityLevel`.
const API_31: i32 = 31;

struct AndroidVaultCtx {
    vm: JavaVM,
    /// The APPLICATION context (defensively re-derived), held as a global
    /// ref — process-global, leaks nothing (§4.3a).
    context: Global<JObject<'static>>,
}

static VAULT_CTX: OnceLock<AndroidVaultCtx> = OnceLock::new();

/// Attach-level / stray JNI errors collapse to the retryable class; the
/// precise taxonomy (invalidated key, bad tag, …) is mapped where the
/// pending exception can be inspected (`rethrow`).
impl From<jni::errors::Error> for WalletError {
    fn from(_: jni::errors::Error) -> Self {
        WalletError::KeystoreUnavailable
    }
}

/// The §4.3a context-handoff entry point. Called once per process by
/// whichever host loads this library: the `zec_wallet` plugin courier
/// (`ZecWalletPlugin.nativeInit`) or a host app's own Android FFI wiring.
/// Idempotent — later calls are no-ops (first-caller-wins is safe
/// BECAUSE the context is custody-inert; see module docs).
pub fn init_android_vault(env: &mut Env<'_>, context: &JObject<'_>) -> Result<(), WalletError> {
    if VAULT_CTX.get().is_some() {
        return Ok(());
    }
    // Defensive: never trust the caller's object to BE the application
    // context — derive it.
    let app_ctx = {
        let r = env
            .call_method(
                context,
                jni_str!("getApplicationContext"),
                jni_sig!("()Landroid/content/Context;"),
                &[],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let jr = env.new_global_ref(&app_ctx);
    let global = rethrow(env, jr)?;
    let jr = env.get_java_vm();
    let vm = rethrow(env, jr)?;
    let _ = VAULT_CTX.set(AndroidVaultCtx {
        vm,
        context: global,
    });
    tracing::info!(target: "zec_wallet_core::keychain", "android vault context initialized");
    Ok(())
}

/// Map a pending Java exception to the §4.3a typed-error taxonomy. Always
/// clears the exception (a pending throwable across JNI calls is UB-bait).
fn map_exception(env: &mut Env<'_>) -> WalletError {
    let Some(throwable) = env.exception_occurred() else {
        // Defensive: unreachable in practice (called only after
        // Err(JavaException)); exception_clear() is a no-op when nothing
        // is pending.
        env.exception_clear();
        return WalletError::KeystoreUnavailable;
    };
    env.exception_clear();
    let mut is =
        |cls: &'static jni::strings::JNIStr| env.is_instance_of(&throwable, cls).unwrap_or(false);
    if is(jni_str!(
        "android/security/keystore/KeyPermanentlyInvalidatedException"
    )) {
        // Lock-screen reset killed the alias key: unrecoverable locally,
        // recovery path = restore from phrase (§4.3a failure posture).
        WalletError::KeystoreInconsistent {
            permanently_invalidated: true,
        }
    } else if is(jni_str!("java/security/UnrecoverableKeyException")) {
        WalletError::KeystoreInconsistent {
            permanently_invalidated: false,
        }
    } else if is(jni_str!(
        "android/security/keystore/UserNotAuthenticatedException"
    )) {
        // A setUnlockedDeviceRequired(true) key used while the device is
        // locked — the same retryable class as any other unavailability
        // (unlock first). Explicit arm so the taxonomy is audit-complete;
        // the catch-all would map it identically.
        WalletError::KeystoreUnavailable
    } else if is(jni_str!("javax/crypto/AEADBadTagException")) {
        // The AAD binding (or ciphertext integrity) failed — substitution
        // or corruption, loud and layer-attributed (§4.3a).
        WalletError::WrapArtifactInvalid
    } else {
        WalletError::KeystoreUnavailable
    }
}

/// `jni::errors::Error` → typed error, inspecting any pending exception.
fn rethrow<T>(env: &mut Env<'_>, r: jni::errors::Result<T>) -> Result<T, WalletError> {
    match r {
        Ok(v) => Ok(v),
        Err(jni::errors::Error::JavaException) => Err(map_exception(env)),
        Err(_) => Err(WalletError::KeystoreUnavailable),
    }
}

fn sdk_int(env: &mut Env<'_>) -> Result<i32, WalletError> {
    let r = env
        .get_static_field(
            jni_str!("android/os/Build$VERSION"),
            jni_str!("SDK_INT"),
            jni_sig!("I"),
        )
        .and_then(|v| v.i());
    rethrow(env, r)
}

fn has_strongbox(env: &mut Env<'_>, context: &JObject<'_>) -> Result<bool, WalletError> {
    let pm = {
        let r = env
            .call_method(
                context,
                jni_str!("getPackageManager"),
                jni_sig!("()Landroid/content/pm/PackageManager;"),
                &[],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let jr = env.new_string(FEATURE_STRONGBOX);
    let feature = rethrow(env, jr)?;
    let r = env
        .call_method(
            &pm,
            jni_str!("hasSystemFeature"),
            jni_sig!("(Ljava/lang/String;)Z"),
            &[JValue::Object(&feature)],
        )
        .and_then(|v| v.z());
    rethrow(env, r)
}

/// `KeyStore.getInstance("AndroidKeyStore")` + `load(null)` — the JCA NAME
/// lookup the trust argument rests on (never via the Context).
fn keystore<'l>(env: &mut Env<'l>) -> Result<JObject<'l>, WalletError> {
    let jr = env.new_string("AndroidKeyStore");
    let name = rethrow(env, jr)?;
    let ks = {
        let r = env
            .call_static_method(
                jni_str!("java/security/KeyStore"),
                jni_str!("getInstance"),
                jni_sig!("(Ljava/lang/String;)Ljava/security/KeyStore;"),
                &[JValue::Object(&name)],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let r = env.call_method(
        &ks,
        jni_str!("load"),
        jni_sig!("(Ljava/security/KeyStore$LoadStoreParameter;)V"),
        &[JValue::Object(&JObject::null())],
    );
    rethrow(env, r)?;
    Ok(ks)
}

fn alias_name(prefix: &str, generation: u32) -> String {
    format!("{prefix}{generation}")
}

/// Highest existing wrap-alias generation (0 = none). Scans
/// `KeyStore.aliases()` so generations survive process restarts without any
/// extra persisted state.
fn max_generation(env: &mut Env<'_>, ks: &JObject<'_>, prefix: &str) -> Result<u32, WalletError> {
    let aliases = {
        let r = env
            .call_method(
                ks,
                jni_str!("aliases"),
                jni_sig!("()Ljava/util/Enumeration;"),
                &[],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let mut max = 0u32;
    loop {
        let has_more = {
            let r = env
                .call_method(&aliases, jni_str!("hasMoreElements"), jni_sig!("()Z"), &[])
                .and_then(|v| v.z());
            rethrow(env, r)?
        };
        if !has_more {
            break;
        }
        let element = {
            let r = env
                .call_method(
                    &aliases,
                    jni_str!("nextElement"),
                    jni_sig!("()Ljava/lang/Object;"),
                    &[],
                )
                .and_then(|v| v.l());
            rethrow(env, r)?
        };
        let Ok(jstr) = env.cast_local::<JString>(element) else {
            env.exception_clear();
            continue;
        };
        let Ok(chars) = jstr.mutf8_chars(env) else {
            env.exception_clear();
            continue;
        };
        let name: String = chars.to_str().into_owned();
        if let Some(suffix) = name.strip_prefix(prefix)
            && let Ok(generation) = suffix.parse::<u32>()
            // Canonical-form check (security fold): one alias = one
            // generation must be INVERTIBLE — "v007" parsing to 7 would
            // alias-collide with "v7"; reject non-canonical suffixes.
            && suffix == generation.to_string()
        {
            max = max.max(generation);
        }
    }
    Ok(max)
}

/// Every alias under `prefix` whose suffix is a canonical generation (the live
/// generation PLUS any failed-provision / incomplete-rotation orphan gens). The
/// FR-14 purge scan — mirrors [`max_generation`]'s enumeration but COLLECTS the
/// matching names instead of maxing. FR-13's equal-length-hex namespace keeps
/// this wallet-local: no namespace's prefix is a prefix of another's.
///
/// DRY NOTE: this shares [`max_generation`]'s exact `aliases()` enumeration +
/// canonical-suffix gate (collect-vs-max is the only difference). A shared
/// `fold_canonical_aliases` would unify them, but both are `#[cfg(android)]` (no
/// host coverage — only `cargo ndk check`), so the merge is deferred to keep the
/// rotation path's `max_generation` byte-stable; the gate MUST stay identical in
/// both — change one, change the other.
fn aliases_under_prefix(
    env: &mut Env<'_>,
    ks: &JObject<'_>,
    prefix: &str,
) -> Result<Vec<String>, WalletError> {
    let aliases = {
        let r = env
            .call_method(
                ks,
                jni_str!("aliases"),
                jni_sig!("()Ljava/util/Enumeration;"),
                &[],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let mut matched = Vec::new();
    loop {
        let has_more = {
            let r = env
                .call_method(&aliases, jni_str!("hasMoreElements"), jni_sig!("()Z"), &[])
                .and_then(|v| v.z());
            rethrow(env, r)?
        };
        if !has_more {
            break;
        }
        let element = {
            let r = env
                .call_method(
                    &aliases,
                    jni_str!("nextElement"),
                    jni_sig!("()Ljava/lang/Object;"),
                    &[],
                )
                .and_then(|v| v.l());
            rethrow(env, r)?
        };
        let Ok(jstr) = env.cast_local::<JString>(element) else {
            env.exception_clear();
            continue;
        };
        let Ok(chars) = jstr.mutf8_chars(env) else {
            env.exception_clear();
            continue;
        };
        let name: String = chars.to_str().into_owned();
        // Same canonical-generation gate as `max_generation`: the suffix must
        // parse back to itself (reject "v007"-style non-canonical names), so a
        // malformed alias never gets swept under a confused identity.
        if let Some(suffix) = name.strip_prefix(prefix)
            && let Ok(generation) = suffix.parse::<u32>()
            && suffix == generation.to_string()
        {
            matched.push(name);
        }
    }
    Ok(matched)
}

/// Every alias whose name starts with `prefix` — NO suffix gate (unlike
/// [`aliases_under_prefix`]'s canonical-generation filter). The S2 index
/// aliases carry a hex-encoded entry as their suffix, so the raw scan is the
/// one that reaches them. Mirrors the enumeration loop exactly.
fn raw_aliases_with_prefix(
    env: &mut Env<'_>,
    ks: &JObject<'_>,
    prefix: &str,
) -> Result<Vec<String>, WalletError> {
    let aliases = {
        let r = env
            .call_method(
                ks,
                jni_str!("aliases"),
                jni_sig!("()Ljava/util/Enumeration;"),
                &[],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let mut matched = Vec::new();
    loop {
        let has_more = {
            let r = env
                .call_method(&aliases, jni_str!("hasMoreElements"), jni_sig!("()Z"), &[])
                .and_then(|v| v.z());
            rethrow(env, r)?
        };
        if !has_more {
            break;
        }
        let element = {
            let r = env
                .call_method(
                    &aliases,
                    jni_str!("nextElement"),
                    jni_sig!("()Ljava/lang/Object;"),
                    &[],
                )
                .and_then(|v| v.l());
            rethrow(env, r)?
        };
        let Ok(jstr) = env.cast_local::<JString>(element) else {
            env.exception_clear();
            continue;
        };
        let Ok(chars) = jstr.mutf8_chars(env) else {
            env.exception_clear();
            continue;
        };
        let name: String = chars.to_str().into_owned();
        if name.starts_with(prefix) {
            matched.push(name);
        }
    }
    Ok(matched)
}

/// deleteEntry an alias BY NAME (containsAlias-guarded ⇒ idempotent). The caller
/// already holds the JNI env + keystore (the FR-14 purge enumerated the name, so
/// it exists — but the guard keeps it a no-op against a concurrent sever).
fn delete_alias_in(env: &mut Env<'_>, ks: &JObject<'_>, alias: &str) -> Result<(), WalletError> {
    let jr = env.new_string(alias);
    let alias_obj = rethrow(env, jr)?;
    let contains = {
        let r = env
            .call_method(
                ks,
                jni_str!("containsAlias"),
                jni_sig!("(Ljava/lang/String;)Z"),
                &[JValue::Object(&alias_obj)],
            )
            .and_then(|v| v.z());
        rethrow(env, r)?
    };
    if !contains {
        return Ok(());
    }
    let r = env
        .call_method(
            ks,
            jni_str!("deleteEntry"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[JValue::Object(&alias_obj)],
        )
        .map(|_| ());
    rethrow(env, r)
}

/// Generate the alias key INSIDE AndroidKeyStore: StrongBox attempted first
/// (API ≥ 28 + feature), TEE fallback on `StrongBoxUnavailableException` —
/// never silently: the MEASURED tier is what `tier()` later reports.
fn generate_key<'l>(
    env: &mut Env<'l>,
    context: &JObject<'_>,
    alias: &str,
) -> Result<JObject<'l>, WalletError> {
    let sdk = sdk_int(env)?;
    let try_strongbox = sdk >= API_28 && has_strongbox(env, context)?;
    match generate_key_inner(env, alias, sdk, try_strongbox) {
        Ok(key) => Ok(key),
        Err(_first) if try_strongbox => {
            // StrongBoxUnavailable (or any generation failure under the
            // StrongBox request): retry once on the TEE path. A genuine
            // daemon flake fails the retry too — typed, surfaced.
            generate_key_inner(env, alias, sdk, false)
        }
        Err(e) => Err(e),
    }
}

fn generate_key_inner<'l>(
    env: &mut Env<'l>,
    alias: &str,
    sdk: i32,
    strongbox: bool,
) -> Result<JObject<'l>, WalletError> {
    let jr = env.new_string(alias);
    let alias_str = rethrow(env, jr)?;
    let builder = {
        let r = env.new_object(
            jni_str!("android/security/keystore/KeyGenParameterSpec$Builder"),
            jni_sig!("(Ljava/lang/String;I)V"),
            &[
                JValue::Object(&alias_str),
                JValue::Int(PURPOSES_ENCRYPT_DECRYPT),
            ],
        );
        rethrow(env, r)?
    };

    let jr = env.new_string("GCM");
    let gcm = rethrow(env, jr)?;
    let modes = {
        let r = env.new_object_array(1, jni_str!("java/lang/String"), &gcm);
        rethrow(env, r)?
    };
    let r = env.call_method(
        &builder,
        jni_str!("setBlockModes"),
        jni_sig!("([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
        &[JValue::Object(&modes)],
    );
    rethrow(env, r)?;

    let jr = env.new_string("NoPadding");
    let nopad = rethrow(env, jr)?;
    let paddings = {
        let r = env.new_object_array(1, jni_str!("java/lang/String"), &nopad);
        rethrow(env, r)?
    };
    let r = env.call_method(
        &builder,
        jni_str!("setEncryptionPaddings"),
        jni_sig!("([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
        &[JValue::Object(&paddings)],
    );
    rethrow(env, r)?;

    let r = env.call_method(
        &builder,
        jni_str!("setKeySize"),
        jni_sig!("(I)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
        &[JValue::Int(256)],
    );
    rethrow(env, r)?;

    if sdk >= API_28 {
        // §4.3a v1 auth policy: vault ACL + device lock, no per-use auth.
        // setUserAuthenticationRequired(false) is the Android DEFAULT — the
        // spec line is satisfied by omission, not dropped.
        // `setUnlockedDeviceRequired(true)` where the API exists (minSdk 23
        // gates it).
        let r = env.call_method(
            &builder,
            jni_str!("setUnlockedDeviceRequired"),
            jni_sig!("(Z)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
            &[JValue::Bool(true)],
        );
        rethrow(env, r)?;
        if strongbox {
            let r = env.call_method(
                &builder,
                jni_str!("setIsStrongBoxBacked"),
                jni_sig!("(Z)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                &[JValue::Bool(true)],
            );
            rethrow(env, r)?;
        }
    }

    let spec = {
        let r = env
            .call_method(
                &builder,
                jni_str!("build"),
                jni_sig!("()Landroid/security/keystore/KeyGenParameterSpec;"),
                &[],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };

    let jr = env.new_string("AES");
    let aes = rethrow(env, jr)?;
    let jr = env.new_string("AndroidKeyStore");
    let provider = rethrow(env, jr)?;
    let generator = {
        let r = env
            .call_static_method(
                jni_str!("javax/crypto/KeyGenerator"),
                jni_str!("getInstance"),
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/KeyGenerator;"),
                &[JValue::Object(&aes), JValue::Object(&provider)],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let r = env.call_method(
        &generator,
        jni_str!("init"),
        jni_sig!("(Ljava/security/spec/AlgorithmParameterSpec;)V"),
        &[JValue::Object(&spec)],
    );
    rethrow(env, r)?;
    let r = env
        .call_method(
            &generator,
            jni_str!("generateKey"),
            jni_sig!("()Ljavax/crypto/SecretKey;"),
            &[],
        )
        .and_then(|v| v.l());
    rethrow(env, r)
}

/// `ks.getKey(alias, null)` — null ⇒ the keysMissing class, surfaced.
fn get_key<'l>(
    env: &mut Env<'l>,
    ks: &JObject<'_>,
    alias: &str,
) -> Result<JObject<'l>, WalletError> {
    let jr = env.new_string(alias);
    let alias_str = rethrow(env, jr)?;
    let key = {
        let r = env
            .call_method(
                ks,
                jni_str!("getKey"),
                jni_sig!("(Ljava/lang/String;[C)Ljava/security/Key;"),
                &[JValue::Object(&alias_str), JValue::Object(&JObject::null())],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    if key.is_null() {
        return Err(WalletError::KeystoreInconsistent {
            permanently_invalidated: false,
        });
    }
    Ok(key)
}

fn new_cipher<'l>(env: &mut Env<'l>) -> Result<JObject<'l>, WalletError> {
    let jr = env.new_string("AES/GCM/NoPadding");
    let transform = rethrow(env, jr)?;
    let r = env
        .call_static_method(
            jni_str!("javax/crypto/Cipher"),
            jni_str!("getInstance"),
            jni_sig!("(Ljava/lang/String;)Ljavax/crypto/Cipher;"),
            &[JValue::Object(&transform)],
        )
        .and_then(|v| v.l());
    rethrow(env, r)
}

fn update_aad(
    env: &mut Env<'_>,
    cipher: &JObject<'_>,
    sealed_blob: &[u8],
) -> Result<(), WalletError> {
    let aad = envelope::wrap_aad(sealed_blob);
    let jr = env.byte_array_from_slice(&aad);
    let aad_arr = rethrow(env, jr)?;
    let r = env
        .call_method(
            cipher,
            jni_str!("updateAAD"),
            jni_sig!("([B)V"),
            &[JValue::Object(&aad_arr)],
        )
        .map(|_| ());
    rethrow(env, r)
}

/// Wrap: Keystore-GCM encrypt of the SealKey under the alias key, AAD-bound
/// to the sealed blob. The Keystore supplies the IV (randomized encryption
/// REQUIRED — never `setRandomizedEncryptionRequired(false)`); the plaintext
/// `byte[]` is zero-filled immediately after `doFinal` on every path.
fn wrap_under_alias(
    env: &mut Env<'_>,
    key: &JObject<'_>,
    seal_key: &SealKey,
    sealed_blob: &[u8],
    generation: u32,
) -> Result<WrapArtifact, WalletError> {
    let cipher = new_cipher(env)?;
    let r = env.call_method(
        &cipher,
        jni_str!("init"),
        jni_sig!("(ILjava/security/Key;)V"),
        &[JValue::Int(ENCRYPT_MODE), JValue::Object(key)],
    );
    rethrow(env, r)?;
    update_aad(env, &cipher, sealed_blob)?;

    // The honest-transit moment: the wrap key on the Java heap.
    let jr = env.byte_array_from_slice(seal_key.as_bytes());
    let pt_arr = rethrow(env, jr)?;
    let ct_result = env
        .call_method(
            &cipher,
            jni_str!("doFinal"),
            jni_sig!("([B)[B"),
            &[JValue::Object(&pt_arr)],
        )
        .and_then(|v| v.l());
    // Zero-fill BEFORE inspecting the result — error paths included.
    let _ = pt_arr.set_region(env, 0, &[0i8; 32]);
    let ct_obj = rethrow(env, ct_result)?;
    let jr = env.cast_local::<JByteArray>(ct_obj);
    let ct_arr = rethrow(env, jr)?;
    let jr = env.convert_byte_array(&ct_arr);
    let ct = rethrow(env, jr)?;

    let iv_obj = {
        let r = env
            .call_method(&cipher, jni_str!("getIV"), jni_sig!("()[B"), &[])
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let jr = env.cast_local::<JByteArray>(iv_obj);
    let iv_arr = rethrow(env, jr)?;
    let jr = env.convert_byte_array(&iv_arr);
    let iv_vec = rethrow(env, jr)?;
    // Validate, never trust: a non-12-byte IV from the platform would
    // corrupt the envelope shape.
    let iv: [u8; WRAP_IV_LEN] = iv_vec
        .try_into()
        .map_err(|_| WalletError::KeystoreUnavailable)?;
    if ct.len() != envelope::WRAP_CT_LEN {
        return Err(WalletError::KeystoreUnavailable);
    }
    Ok(WrapArtifact::from_freshly_wrapped(
        envelope::encode_artifact(generation, &iv, &ct),
    ))
}

fn unwrap_with_alias(
    env: &mut Env<'_>,
    key: &JObject<'_>,
    parsed: &envelope::WrapArtifactV1,
    sealed_blob: &[u8],
) -> Result<SealKey, WalletError> {
    let cipher = new_cipher(env)?;
    let jr = env.byte_array_from_slice(&parsed.iv);
    let iv_arr = rethrow(env, jr)?;
    let spec = {
        let r = env.new_object(
            jni_str!("javax/crypto/spec/GCMParameterSpec"),
            jni_sig!("(I[B)V"),
            &[JValue::Int(GCM_TAG_BITS), JValue::Object(&iv_arr)],
        );
        rethrow(env, r)?
    };
    let r = env.call_method(
        &cipher,
        jni_str!("init"),
        jni_sig!("(ILjava/security/Key;Ljava/security/spec/AlgorithmParameterSpec;)V"),
        &[
            JValue::Int(DECRYPT_MODE),
            JValue::Object(key),
            JValue::Object(&spec),
        ],
    );
    rethrow(env, r)?;
    update_aad(env, &cipher, sealed_blob)?;

    let jr = env.byte_array_from_slice(&parsed.ct);
    let ct_arr = rethrow(env, jr)?;
    let pt_obj = {
        // AEADBadTagException here = the binding spoke (substitution or
        // corruption) → WrapArtifactInvalid via map_exception.
        let r = env
            .call_method(
                &cipher,
                jni_str!("doFinal"),
                jni_sig!("([B)[B"),
                &[JValue::Object(&ct_arr)],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    // Zero-fill ordering (crypto fold MINOR-2): the cast cannot fail in
    // practice (doFinal([B)[B returns a byte[]), but the zero-fill must not
    // sit behind ANY fallible `?` — convert, zero IMMEDIATELY, then
    // propagate errors. No `.to_vec()` second copy (MINOR-3): the Zeroizing
    // buffer flows to seal_key_from_vault as-is.
    let pt = match env.cast_local::<JByteArray>(pt_obj) {
        Ok(pt_arr) => {
            let converted = env.convert_byte_array(&pt_arr).map(Zeroizing::new);
            // Zero the Java-side plaintext on EVERY path before returning.
            let _ = pt_arr.set_region(env, 0, &[0i8; 32]);
            rethrow(env, converted)
        }
        Err(e) => rethrow(env, Err(e)),
    };
    seal_key_from_vault(pt?)
}

/// Measured tier of the CURRENT (highest-generation) alias key — `KeyInfo`
/// introspection, floor-honest: pre-API-31 cannot distinguish StrongBox from
/// TEE, so it reports `Tee` (never overclaims StrongBox); outside secure
/// hardware is `SoftwareKeystore` (surfaced as degraded upstream).
fn measured_tier(
    env: &mut Env<'_>,
    ks: &JObject<'_>,
    prefix: &str,
) -> Result<VaultTier, WalletError> {
    let generation = max_generation(env, ks, prefix)?;
    if generation == 0 {
        return Err(WalletError::KeystoreInconsistent {
            permanently_invalidated: false,
        });
    }
    let key = get_key(env, ks, &alias_name(prefix, generation))?;

    let jr = env.new_string("AES");
    let aes = rethrow(env, jr)?;
    let jr = env.new_string("AndroidKeyStore");
    let provider = rethrow(env, jr)?;
    let factory = {
        let r = env
            .call_static_method(
                jni_str!("javax/crypto/SecretKeyFactory"),
                jni_str!("getInstance"),
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/SecretKeyFactory;"),
                &[JValue::Object(&aes), JValue::Object(&provider)],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };
    let jr = env.find_class(jni_str!("android/security/keystore/KeyInfo"));
    let key_info_class = rethrow(env, jr)?;
    let key_info = {
        let r = env
            .call_method(
                &factory,
                jni_str!("getKeySpec"),
                jni_sig!("(Ljavax/crypto/SecretKey;Ljava/lang/Class;)Ljava/security/spec/KeySpec;"),
                &[JValue::Object(&key), JValue::Object(&key_info_class)],
            )
            .and_then(|v| v.l());
        rethrow(env, r)?
    };

    if sdk_int(env)? >= API_31 {
        let level = {
            let r = env
                .call_method(
                    &key_info,
                    jni_str!("getSecurityLevel"),
                    jni_sig!("()I"),
                    &[],
                )
                .and_then(|v| v.i());
            rethrow(env, r)?
        };
        return Ok(match level {
            SECURITY_LEVEL_STRONGBOX => VaultTier::StrongBox,
            SECURITY_LEVEL_TRUSTED_ENVIRONMENT | SECURITY_LEVEL_UNKNOWN_SECURE => VaultTier::Tee,
            // SECURITY_LEVEL_SOFTWARE (0) and any UNRECOGNIZED future level
            // land here — degraded is the honest floor, never an overclaim.
            _ => VaultTier::SoftwareKeystore,
        });
    }
    let inside = {
        let r = env
            .call_method(
                &key_info,
                jni_str!("isInsideSecureHardware"),
                jni_sig!("()Z"),
                &[],
            )
            .and_then(|v| v.z());
        rethrow(env, r)?
    };
    Ok(if inside {
        VaultTier::Tee
    } else {
        VaultTier::SoftwareKeystore
    })
}

/// One transient retry on `KeystoreUnavailable` (operating principles: retry
/// transient infra once) — never on the typed terminal states.
fn with_retry<T>(mut op: impl FnMut() -> Result<T, WalletError>) -> Result<T, WalletError> {
    match op() {
        Err(WalletError::KeystoreUnavailable) => op(),
        other => other,
    }
}

pub(crate) struct AndroidKeystoreVault {
    /// Owned (not `&'static`) since FR-13: the production prefix carries the
    /// per-wallet namespace. The generation suffix is appended on top
    /// (`<prefix><generation>`), so the namespace sits BEFORE the version+gen.
    alias_prefix: String,
    /// S2 custody: this namespace's index-alias prefix
    /// (`zec_wallet.custody-index.<namespace>.<hex-entry>`).
    index_alias_prefix: String,
}

impl AndroidKeystoreVault {
    /// Production custody, NAMESPACED per wallet (FR-13): the alias prefix
    /// embeds `namespace` (`zec_wallet.wrap.<namespace>.v<gen>`) so each wallet's
    /// wrap key is a distinct Keystore alias — a second wallet on the same device
    /// never overwrites the first.
    ///
    /// LOAD-BEARING INVARIANT: `namespace` is a validated [`KeychainNamespace`]
    /// (a FIXED-LENGTH lowercase-hex token). The cross-wallet isolation of
    /// `max_generation`'s prefix scan depends on it: equal-length hex (and the `.v`
    /// delimiter before the generation) guarantees no namespace's prefix is a prefix
    /// of another's. The TYPE enforces this in EVERY build (a variable-length or
    /// `.`-bearing namespace is unconstructable), so no `debug_assert` is owed.
    pub(crate) fn new(namespace: &KeychainNamespace) -> Self {
        Self {
            alias_prefix: format!("{ALIAS_BASE}.{}.v", namespace.as_str()),
            index_alias_prefix: format!("{INDEX_ALIAS_BASE}.{}.", namespace.as_str()),
        }
    }

    /// Device-E2E vault: same code path, DISJOINT alias namespace — the
    /// selftest can create/wipe freely without ever touching production
    /// custody (§4.3a device gate).
    pub(crate) fn selftest() -> Self {
        Self {
            alias_prefix: SELFTEST_ALIAS_PREFIX.to_owned(),
            index_alias_prefix: format!("{INDEX_ALIAS_BASE}.selftest."),
        }
    }

    fn ctx() -> Result<&'static AndroidVaultCtx, WalletError> {
        // No context handoff ⇒ no path to the vault ⇒ FAIL-CLOSED
        // (`VaultAbsent`), exactly the §4.3a no-vault posture — the
        // in-memory fallback is structurally unreachable.
        VAULT_CTX.get().ok_or(WalletError::VaultAbsent)
    }

    fn delete_generation(&self, generation: u32) -> Result<(), WalletError> {
        let ctx = Self::ctx()?;
        ctx.vm.attach_current_thread(|env| {
            let ks = keystore(env)?;
            let jr = env.new_string(alias_name(&self.alias_prefix, generation));
            let alias = rethrow(env, jr)?;
            let contains = {
                let r = env
                    .call_method(
                        &ks,
                        jni_str!("containsAlias"),
                        jni_sig!("(Ljava/lang/String;)Z"),
                        &[JValue::Object(&alias)],
                    )
                    .and_then(|v| v.z());
                rethrow(env, r)?
            };
            if !contains {
                // Idempotent: already severed is the terminal no-op.
                return Ok(());
            }
            let r = env
                .call_method(
                    &ks,
                    jni_str!("deleteEntry"),
                    jni_sig!("(Ljava/lang/String;)V"),
                    &[JValue::Object(&alias)],
                )
                .map(|_| ());
            rethrow(env, r)
        })
    }
}

impl KeychainPort for AndroidKeystoreVault {
    fn probe(&self) -> Result<(), WalletError> {
        // Fast-path gate, NOT a keystore-liveness guarantee: a usable tier
        // is only proven by store/tier (measured on a live key). A
        // no-keystore device still fails closed at generate_key.
        Self::ctx().map(|_| ())
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        let ctx = Self::ctx()?;
        ctx.vm.attach_current_thread(|env| {
            let ks = keystore(env)?;
            measured_tier(env, &ks, &self.alias_prefix)
        })
    }

    fn store_wrap_key(
        &self,
        key: SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        let ctx = Self::ctx()?;
        with_retry(|| {
            ctx.vm.attach_current_thread(|env| {
                let ks = keystore(env)?;
                // One alias = one encryption, ever: every store takes a
                // FRESH generation (max existing + 1).
                let generation = max_generation(env, &ks, &self.alias_prefix)?
                    .checked_add(1)
                    .ok_or(
                        // 2^32 rotations is not a real device state — corruption is.
                        WalletError::KeystoreInconsistent {
                            permanently_invalidated: false,
                        },
                    )?;
                let alias_key = generate_key(
                    env,
                    ctx.context.as_obj(),
                    &alias_name(&self.alias_prefix, generation),
                )?;
                wrap_under_alias(env, &alias_key, &key, sealed_blob, generation)
            })
        })
    }

    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError> {
        let ctx = Self::ctx()?;
        let parsed = envelope::parse_artifact(artifact.as_bytes())?;
        with_retry(|| {
            ctx.vm.attach_current_thread(|env| {
                let ks = keystore(env)?;
                let key = get_key(env, &ks, &alias_name(&self.alias_prefix, parsed.alias_gen))?;
                unwrap_with_alias(env, &key, &parsed, sealed_blob)
            })
        })
    }

    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        // Recover under the OLD generation, re-custody under a NEW one —
        // store_wrap_key takes max+1, so the IV-reuse-free invariant holds.
        let key = self.load_wrap_key(artifact, sealed_blob)?;
        self.store_wrap_key(key, sealed_blob)
    }

    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        let old_gen = envelope::parse_artifact(old.as_bytes())?.alias_gen;
        let new_gen = envelope::parse_artifact(new.as_bytes())?.alias_gen;
        if old_gen == new_gen {
            // Deleting the live alias would be a lockout — no-op.
            return Ok(());
        }
        self.delete_generation(old_gen)
    }

    fn delete_wrap_key(&self, artifact: &WrapArtifact) -> Result<(), WalletError> {
        let generation = envelope::parse_artifact(artifact.as_bytes())?.alias_gen;
        self.delete_generation(generation)
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        // FR-14 crypto-shred: deleteEntry EVERY alias under this wallet's
        // namespaced prefix — the live generation AND any orphan generations a
        // killed provision / incomplete rotation left behind (an artifact-keyed
        // `delete_wrap_key` reaches only ONE generation; this reaches them all).
        // Returns the count severed (the verify-real-sever signal). `with_retry`
        // covers the transient `KeystoreUnavailable` class; a non-transient
        // failure propagates so `store::destroy` does NOT delete files until the
        // sever actually completes. (The pre-FR-13 bare-name `zec_wallet.wrap.v<gen>`
        // legacy item is NOT swept here — pre-prod, no shipped consumers; it rides
        // the on-device gate, mirroring the FR-13 backward-compat note.)
        let ctx = Self::ctx()?;
        with_retry(|| {
            ctx.vm.attach_current_thread(|env| {
                let ks = keystore(env)?;
                let names = aliases_under_prefix(env, &ks, &self.alias_prefix)?;
                for name in &names {
                    delete_alias_in(env, &ks, name)?;
                }
                // The S2 index alias is NOT swept: a purge is wrap material
                // only. The index is the breadcrumb a wipe needs AFTER this
                // purge (the deferred legacy purge runs on the index's own
                // namespace), so its one deleter is the wipe, LAST
                // (`delete_index`; ADR-0560).
                Ok(names.len())
            })
        })
    }

    fn store_index(&self, entry: &crate::custody::CustodyIndexEntry) -> Result<(), WalletError> {
        // PRESENCE-ENCODED (see INDEX_ALIAS_BASE): replace-don't-duplicate —
        // delete the prior index alias(es), then generate the fresh one. A
        // crash between leaves ZERO (reads as absent, the writer re-runs); a
        // crash after leaves exactly ONE.
        let alias = format!("{}{}", self.index_alias_prefix, hex::encode(entry.encode()));
        let ctx = Self::ctx()?;
        with_retry(|| {
            ctx.vm.attach_current_thread(|env| {
                let ks = keystore(env)?;
                for name in raw_aliases_with_prefix(env, &ks, &self.index_alias_prefix)? {
                    delete_alias_in(env, &ks, &name)?;
                }
                // The generated key is never used for crypto — it IS the item.
                generate_key(env, ctx.context.as_obj(), &alias)?;
                Ok(())
            })
        })
    }

    fn load_index(&self) -> Result<Option<crate::custody::CustodyIndexEntry>, WalletError> {
        let ctx = Self::ctx()?;
        with_retry(|| {
            ctx.vm.attach_current_thread(|env| {
                let ks = keystore(env)?;
                let names = raw_aliases_with_prefix(env, &ks, &self.index_alias_prefix)?;
                let mut suffixes = names
                    .iter()
                    .map(|n| n.strip_prefix(&self.index_alias_prefix))
                    .collect::<Option<Vec<_>>>()
                    .ok_or(WalletError::KeystoreInconsistent {
                        permanently_invalidated: false,
                    })?;
                suffixes.sort_unstable();
                match suffixes.as_slice() {
                    [] => Ok(None),
                    [only] => {
                        let bytes =
                            hex::decode(only).map_err(|_| WalletError::KeystoreInconsistent {
                                permanently_invalidated: false,
                            })?;
                        crate::custody::CustodyIndexEntry::decode(&bytes).map(Some)
                    }
                    // More than one index alias under one namespace cannot be
                    // produced by the store path (replace-don't-duplicate) —
                    // never silently pick one.
                    _ => Err(WalletError::KeystoreInconsistent {
                        permanently_invalidated: false,
                    }),
                }
            })
        })
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        let ctx = Self::ctx()?;
        with_retry(|| {
            ctx.vm.attach_current_thread(|env| {
                let ks = keystore(env)?;
                for name in raw_aliases_with_prefix(env, &ks, &self.index_alias_prefix)? {
                    delete_alias_in(env, &ks, &name)?;
                }
                Ok(())
            })
        })
    }
}
