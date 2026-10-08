# Fuzzing the 1Click response funnel

The `fuzz_swap_status_decode` target (wallet-sdk spec §8) drives hostile
bytes through the adapter's whole response funnel (serde decode → §4.6
bounds → port mapping) for all three response families: status, quote and
tokens. Seed the corpus from the recorded and derived fixtures:

```sh
cargo +nightly fuzz run fuzz_swap_status_decode -- -max_len=262144
```

This is its own cargo workspace (see `Cargo.toml`), so the stable-pinned
parent never builds it. `corpus/`, `artifacts/` and `target/` are
git-ignored; `Cargo.lock` is committed for reproducible runs.

| target | funnel |
|---|---|
| `fuzz_swap_status_decode` | `fuzzing::decode_and_map_all` |
