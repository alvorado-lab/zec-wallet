# Contributing to zec-wallet

Bug reports, fixes and integration feedback are all welcome. What helps most
right now is hearing from people who embed the packages in an app: what
broke, what was missing, what was hard to understand.

**A security problem is not an issue.** Email **security@relim.io**, as
[SECURITY.md](SECURITY.md) explains.

## Issues

Search the open issues first. A good report names the package and its
version, the platform and device, what you did, what you expected and what
happened, with the error kind and code the SDK returned (they are stable, so
quote them as they are). Leave out recovery phrases, viewing keys, addresses,
transaction ids and Tor bridge lines: they are almost never needed to
reproduce a bug, and an issue is public. Read a device log before you paste
it, and remove anything of that kind.

For a feature, describe what your app needs to do and why the current API
gets in the way, before proposing an API. Open an issue before starting a
large change, so we can agree on the shape first.

## How a change gets in

This repository is published from the maintainers' tree: each release is one
commit on top of the last, cut from that tree. So a pull request is not
merged here directly. When one is accepted, we apply it in our tree, it
appears in the next release commit, and we close the pull request with a
link to that commit. Your name goes in the package CHANGELOG entry for the
change, unless you ask us to leave it out.

Open pull requests against `main`. Keep each one to one change; split a
large change into a series, each step reviewable on its own.

## Before you open a pull request

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) lists every check a
change has to pass. Locally, at least:

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd dialer-tor && cargo test)
(cd zec_wallet_tor/rust && cargo test)
(cd zec_wallet && flutter analyze && flutter test)   # and in each package you touched
```

A change to the bridge crate's API (`zec_wallet/rust/src/api/`) regenerates
the Dart bindings in the same change; the command is in the `zec_wallet`
README under "Building".

## Style

We follow the [`librustzcash` style guides]. In short:

- **Commits.** Each commit is one discrete change, with a short title and a
  body that says why. No "work in progress" commits. A bug fix starts with a
  commit that adds a failing test, then the fix.
- **Changelog.** A change to a public API, a bug fix, or a change in what a
  caller can expect updates that package's `CHANGELOG.md` in the same commit,
  written for the people who use the package, not about the implementation.
- **Rust.** Make invalid states unrepresentable: private fields behind
  checked constructors, newtypes instead of bare integers and strings in a
  public API, enums instead of boolean flags. Errors are `Result` with a
  typed error enum. Public items have complete rustdoc.
- **Serialization.** Anything stored is versioned at the top level, and a
  stored format is never changed in place.

And the rules specific to this SDK, which a review will hold you to:

- **Keys stay in Rust.** No seed, spending key or database key crosses into
  Dart. The only crossings are the two the `zec_wallet` README names (the
  recovery words on backup and restore, the viewing key for watch-only), and
  policy tests enforce that.
- **No new cryptography.** Consensus, proving and key derivation come from
  librustzcash, `orchard` and `sapling-crypto`, used as they are; sealing
  uses audited RustCrypto crates. A change that implements a primitive or a
  protocol step itself will not be accepted.
- **Every byte from the network is untrusted.** Check sizes before
  allocating and validate at the boundary. Assume a light server can lie,
  and handle it when it does.
- **Errors cross the bridge as types,** with a stable kind and code, never
  as text for the caller to parse.
- **A change to how money moves** (sending, shielding, swaps, spendability)
  comes with a test that fails without it.
- **No `unsafe`** outside the few FFI modules that already need it, and a
  new one needs a stated reason.

## Licence

By contributing you agree that your contribution is licensed under the MIT
licence of this repository ([LICENSE](LICENSE)).

[`librustzcash` style guides]: https://github.com/zcash/librustzcash/blob/main/CONTRIBUTING.md#styleguides
