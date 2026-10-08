//! §4.3a context-handoff shim, Rust side — the ONE JNI export in this
//! cdylib (it must live here: this is the library `System.loadLibrary`
//! loads). SAFE Rust throughout — no `JNI_OnLoad`, no raw pointers: the
//! jni 0.22 `EnvUnowned` entry shape + `Env::get_java_vm()` inside
//! `zec_wallet_core::init_android_vault` cover everything
//! (`jni_custody_exports_one_java_symbol` pins the export set).
//!
//! `#[jni_mangle]` derives the `Java_com_flutter_1rust_1bridge_zec_1wallet_
//! ZecWalletPlugin_nativeInit` export name from the courier's class path —
//! the Java side is a context COURIER ONLY, see
//! `android/src/main/java/.../ZecWalletPlugin.java` and the
//! `kotlin_shim_is_courier_only` policy test.
#![deny(unsafe_code)]

use jni::objects::{JClass, JObject};
use jni::{EnvUnowned, Outcome, jni_mangle};

/// Called once from `ZecWalletPlugin.onAttachedToEngine` with the
/// application context. Failure is NOT fatal here: the vault context simply
/// stays uninitialized and every later custody call fails CLOSED with the
/// typed `VaultAbsent` (§4.3a — never a silent in-memory fallback).
#[jni_mangle("com.flutter_rust_bridge.zec_wallet.ZecWalletPlugin")]
pub extern "system" fn native_init<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    context: JObject<'local>,
) {
    let outcome: Outcome<(), zec_wallet_core::WalletError> = unowned
        .with_env(|env| zec_wallet_core::init_android_vault(env, &context))
        .into_outcome();
    // Ok: initialized (logged inside). Err/Panic: fail-closed posture —
    // custody calls will return `VaultAbsent`; nothing to throw back at the
    // courier (and unwinding into the JVM would abort the process).
    let _ = outcome;
}
