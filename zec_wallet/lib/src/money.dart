/// FR-48 — the total ZEC supply bound, exported from the package every host
/// imports, so a host checking an amount cites the SDK instead of keeping its
/// own copy.
library;

/// The total ZEC supply in zatoshis: 21,000,000 ZEC × 10⁸ zatoshis per ZEC.
///
/// This is the consensus supply cap (`MAX_MONEY` in Zcash), not a limit this
/// wallet sets. No real amount, balance or payment can exceed it, and the SDK
/// rejects anything above it as corrupt or hostile input. It is below 2⁵³, so
/// it is exact in a Dart `int` on every platform the SDK ships.
///
/// It MUST equal `MAX_MONEY_ZAT` in `zec-wallet-core/src/constants.rs`; the
/// Rust test `convert::tests::the_dart_max_money_constant_equals_the_rust_one`
/// parses this line, so keep it a single `const int` declaration with an
/// integer literal.
const int maxMoneyZat = 2100000000000000;
