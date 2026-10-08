/// ZEC wallet SDK for Flutter — Rust-core wallet with a keys-never-in-Dart
/// boundary. [WalletHandle] is the whole integration surface: create, restore
/// or open a wallet, sync it, send, shield, swap, read its history, back it
/// up and wipe it. Call `RustLib.init()` first; the README is the guide, and
/// its "Building" section names the loader iOS and macOS need.
library;

export 'src/rust/api/config.dart';
export 'src/rust/api/error.dart';
export 'src/rust/api/meta.dart';
// The ZIP-321 payment surface + the send DTOs (parse/encode, SendProposal,
// OutputPool, ProposalStep/Recipient, PaymentDraft/ParsedPayment). Without this
// the send pipeline crossing (WalletHandle.propose/send/queueSend) is unreachable
// from a host — the returned SendProposal/OutputPool types could not even be named.
export 'src/rust/api/payments.dart';
export 'src/rust/api/restore.dart';
export 'src/rust/api/selftest.dart';
export 'src/rust/api/state.dart';
export 'src/rust/api/swap.dart';
export 'src/rust/api/wallet.dart';
// `RustLib` is the package's own entry point (FR-33): the generated one is
// `ZecWalletRustLib`, wrapped so `RustLib.init` checks the bridge version
// before the first bridge call. `BridgeAbiMismatch` is what that check throws —
// catch it by type.
export 'src/rust_lib.dart' show RustLib;
export 'src/bridge_abi.dart' show BridgeAbiMismatch, kBridgeAbiVersion;
// FR-48: the total-supply bound, so a host bounds an amount by the SDK's value
// instead of keeping its own copy.
export 'src/money.dart' show maxMoneyZat;
// hosts need this to init correctly on iOS/macOS (the framework loader — README
// § Building); re-exported so no direct flutter_rust_bridge dep is needed
// ignore: invalid_use_of_internal_member -- the only entrypoint exporting
// ExternalLibrary (platform-conditional io/web), same import the generated
// bindings use
export 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
