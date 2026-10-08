package com.flutter_rust_bridge.zec_wallet;

import io.flutter.embedding.engine.plugins.FlutterPlugin;

/**
 * The §4.3a context-handoff courier (wallet-sdk spec) — and NOTHING else.
 *
 * <p>This class exists because Android Keystore is a Java-only API and the
 * Rust core needs a {@code JavaVM} + application {@code Context} to reach it
 * over JNI; FRB loads the native library via Dart-FFI {@code dlopen}, which
 * never involves the JVM, so the handoff must come from here.
 *
 * <p>RULES (enforced by the {@code kotlin_shim_is_courier_only} policy test):
 * NO key material, NO crypto imports, NO method channels, NO state. One
 * native call, passing the APPLICATION context (never an Activity). The
 * Rust side re-derives {@code getApplicationContext()} defensively and
 * treats the context as custody-inert — see wallet-sdk spec §4.3a.
 *
 * <p>Written in Java (not Kotlin) so the plugin needs no Kotlin toolchain —
 * an as-implemented note against §4.3a's "ZecWalletPlugin.kt".
 */
public class ZecWalletPlugin implements FlutterPlugin {
  @Override
  public void onAttachedToEngine(FlutterPluginBinding binding) {
    // Same library FRB dlopen()s; loading via the JVM is what makes the
    // native method below resolvable. Idempotent per process.
    System.loadLibrary("zec_wallet");
    nativeInit(binding.getApplicationContext());
  }

  @Override
  public void onDetachedFromEngine(FlutterPluginBinding binding) {
    // Nothing to tear down: the Rust side holds the application context
    // (process-global) — engine lifecycle does not affect it.
  }

  private static native void nativeInit(android.content.Context context);
}
