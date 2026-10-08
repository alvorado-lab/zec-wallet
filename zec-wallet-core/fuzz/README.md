# zec-wallet-core fuzzing (hostile-input parsers)

Hostile-input fuzz targets for the wallet SDK's attacker-controlled parsers,
per `docs/specs/wallet-sdk.md` §4.6 and operating-principle 7 (every
attacker-controlled parser gets a fuzz target). This is a **separate
workspace**, so the stable-pinned parent never builds it. It needs a nightly
toolchain and `cargo-fuzz` (`cargo install cargo-fuzz`).

- **`fuzz_payment_uri`** — `zec_wallet_core::parse_payment_uri`, the QR-code/link
  door (size-cap → audited `zip321` parser → per-leg re-validation). Asserts
  no panic on any string under either network, AND the encode∘parse identity
  on every input the funnel accepts (a divergence is a validation gap).
- **`fuzz_subtree_root`** — the note-commitment subtree-root leaf-hash parsers
  (`__fuzz_parse_subtree_root` → `parse_{sapling,orchard}_root`, wallet-sdk spec
  §3.2g). A `SubtreeRoot.root_hash` + completing height arrive from
  an untrusted lightwalletd endpoint; asserts no panic on any length / bytes /
  height (the decode delegates to the audited `Node::from_bytes` canonical-field
  check after a length + u32-range guard).

```sh
cargo +nightly fuzz run fuzz_payment_uri
cargo +nightly fuzz run fuzz_subtree_root
```

## Seeds and corpus

`target/`, `corpus/` and `artifacts/` are git-ignored. `seeds/<target>/` is
tracked: three small inputs for four of the six targets. `fuzz_compact_block`
and `fuzz_enhance_parse` have none yet and start cold; a structured v4/v5 tx
skeleton seed for them is still to be written. Every run copies the seeds into
`corpus/<target>/` with `cp -n`, so nothing the corpus already holds is
overwritten. A failing leg promotes its newest crash artifact into
`seeds/<target>/` and prints it as base64 into the log.

## How it runs

Run one target from this directory with
`cargo +nightly fuzz run <target> -- -max_total_time=60` (the same for
`zec-wallet-swap-near/fuzz`). The maintainers run every target of both
workspaces for 60 seconds each night, each as its own graded run, and the
corpus accumulates across nights. As a check, a planted `panic!` on every
`zcash:` input in `parse_payment_uri` was found by `fuzz_payment_uri` within
15 seconds, with the other six legs still green.

The no-panic and round-trip properties are also covered host-side by the
`parse_payment_uri_never_panics*` and
`payment_uri_roundtrip_preserves_validated_requests` proptests.
