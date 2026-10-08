fn main() {
    // Android 16 KB page-size compliance (Android 15+ / Google Play requirement),
    // the same argument the wallet's own bridge passes (`sdk/zec_wallet/rust/build.rs`).
    //
    // NDK r27 and earlier do NOT default to 16 KB-aligned ELF LOAD segments, and
    // the example pins r27, so without this the plugin's cdylib ships 4 KB-aligned:
    // the device walk showed Android flag `libzec_wallet_tor.so` alone as
    // "LOAD segment not aligned", and Play rejects such an upload. Force the
    // linker's max page size to 16 KB (`p_align = 0x4000`).
    //
    // A link-arg, not RUSTFLAGS: cargokit sets `CARGO_ENCODED_RUSTFLAGS`, which
    // cargo treats as the only source of rustflags, so a `.cargo/config.toml`
    // `rustflags` would be silently ignored; `cargo:rustc-link-arg` survives it.
    //
    // Gated to Android: Apple `ld` and MSVC `link.exe` reject `-z max-page-size`.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }
}
