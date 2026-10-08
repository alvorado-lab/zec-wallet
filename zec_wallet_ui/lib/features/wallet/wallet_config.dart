import 'dart:typed_data' show Uint8List;

import 'package:zec_wallet/zec_wallet.dart';

/// The example-app wallet host-policy (spec §3.2g iii-B-2-b; ADR-0526). Builds
/// the one [WalletConfig] this example wallet runs under — the endpoint,
/// network, transport, and seed-persistence decisions live HERE, in the host, so
/// the SDK stays policy-free (it validates the config; it does not choose it).
///
/// SCOPE (ADR-0526): these defaults govern ONLY this example app. A production
/// host typically rides its OWN `NetDialer`/Tor infrastructure, so the endpoint
/// and Tor policy there come from the host, NOT this file.
///
/// Each value is a named constant so it is pinned at its boundary by a test
/// (gate 7 — no magic numbers) and a future change is a one-line, reviewable
/// edit rather than a literal buried in a constructor.

/// The mainnet lightwalletd-protocol (Zaino-compatible) gRPC endpoint — the
/// public server the reference app uses (maintainer S56: "the server Zodl uses";
/// zec.rocks is Zodl/Zashi's default and the public endpoint ADR-0005 names).
/// `https://` — TLS to a remote host is mandatory (plaintext would be a leak,
/// which the SDK rejects anyway).
/// The STRING equals the SDK catalog's `zec-rocks` entry byte for byte
/// (P3-13; pinned by the core's `the_dart_reference_endpoints_match_the_catalog`
/// policy row): the picker marks the row in use by URL equality, so the
/// default and the offered entry must spell the same server the same way.
const String referenceMainnetEndpoint = 'https://zec.rocks:443';

/// The TESTNET lightwalletd endpoint — the same operator's testnet rail. Used
/// only under the `ZEC_NETWORK=testnet` build (the v-5c device proof: the
/// `textest1…` TEX HRP is testnet-only).
const String referenceTestnetEndpoint = 'https://testnet.zec.rocks:443';

/// Build-time network switch: `flutter build/run --dart-define=ZEC_NETWORK=
/// testnet` runs the example wallet on testnet; the default (unset or
/// `mainnet`) is mainnet. Any OTHER value fails LOUD at provisioning via
/// [networkForDefine] — a typo'd define silently running MAINNET would point a
/// money proof at real funds.
const String zecNetworkDefine = String.fromEnvironment(
  'ZEC_NETWORK',
  defaultValue: 'mainnet',
);

/// Optional endpoint override for either network:
/// `--dart-define=ZEC_ENDPOINT=https://…` (e.g. a local lightwalletd). Empty ⇒
/// the network's reference endpoint.
const String zecEndpointDefine = String.fromEnvironment(
  'ZEC_ENDPOINT',
  defaultValue: '',
);

/// Map a `ZEC_NETWORK` define value to the SDK network. Pure so every arm is
/// pinned by a unit test (the const define can't vary inside one test binary);
/// the throw is the fail-loud arm — never default a typo to mainnet.
Network networkForDefine(String define) => switch (define) {
  'mainnet' => Network.main,
  'testnet' => Network.test,
  _ => throw StateError(
    'Unknown ZEC_NETWORK "$define" — use "mainnet" or "testnet".',
  ),
};

/// Resolve the endpoint for a network + optional `ZEC_ENDPOINT` override
/// (pure, unit-pinned like [networkForDefine]).
String endpointForNetwork(Network network, {required String override}) {
  if (override.isNotEmpty) return override;
  return network == Network.test
      ? referenceTestnetEndpoint
      : referenceMainnetEndpoint;
}

/// The wallet data-dir LEAF per network: a testnet proof wallet lives BESIDE a
/// mainnet wallet, never over it — switching defines must not hit the SDK's
/// `NetworkMismatch` on an existing mainnet DB, nor invite wiping it. Mainnet
/// keeps the historical `zec_wallet` leaf (zero migration).
String dbDirLeafForNetwork(Network network) =>
    network == Network.test ? 'zec_wallet_testnet' : 'zec_wallet';

/// The network this build runs on (from [zecNetworkDefine]).
Network referenceNetwork() => networkForDefine(zecNetworkDefine);

/// The data-dir leaf this build uses (see [dbDirLeafForNetwork]).
String referenceDbDirLeaf() => dbDirLeafForNetwork(referenceNetwork());

/// Tor policy for the example app: OFF / direct — the example matches Zashi's
/// direct default. NOTE this is an EXAMPLE-app choice; a production host would
/// typically route the wallet through the host's own Tor infrastructure.
const TorPolicy referenceTorPolicy = TorPolicy.off();

/// Broadcast-timing decorrelation window: a uniform random delay in `0..=10s`
/// before each tx broadcast, so a send doesn't timestamp-correlate with other
/// host-app traffic (privacy posture; the SDK's documented default window). No
/// send path ships in this slice, but the policy is wallet-wide so it is set at
/// config time.
const int referenceBroadcastJitterMaxMs = 10000;

/// Zcash Sapling activation (2018-10-22) — the earliest USEFUL wallet birthday:
/// the SDK floors any earlier date to it, so it is the `firstDate` bound on every
/// "how far back to scan" date picker (restore birthday + rescan recovery). One
/// source of truth for the floor (DRY) so the two pickers can never drift; pinned
/// by a boundary test (gate 7 — no magic numbers).
final DateTime kZcashSaplingActivationDate = DateTime(2018, 10, 22);

/// Build the reference-app [WalletConfig] for the wallet rooted at [dbDir].
///
/// The network comes from the `ZEC_NETWORK` build define (mainnet by default;
/// `testnet` for the v-5c device-proof builds) with the matching reference
/// endpoint (or the `ZEC_ENDPOINT` override); Tor [referenceTorPolicy]; the
/// seed sealed under the device keychain ([SeedPersistence.sealedKeychain] —
/// required for `revealMnemonic` to work after a restart, the Dart-path
/// default); and a null `birthdayHeight` (a freshly created wallet starts at
/// the current tip). The RESTORE path overrides the birthday via
/// [WalletConfigBirthday.withBirthdayHeight] (from the user's optional
/// creation date); the base config stays null for create.
///
/// [dbDir] MUST be a resolved, plain platform path (path_provider), NOT a value
/// recomputed inside a provider `build()` — see `wallet_composition.dart` for
/// the resolve-before-override discipline (the build()-rerun double-create
/// footgun).
///
/// [tor] defaults to [referenceTorPolicy] (off). A host that registered a
/// transport before opening the wallet — the example's optional Tor plugin —
/// passes `TorPolicy.required_(runtime: TorRuntimeConfig.hostDialer())`.
WalletConfig buildWalletConfig({
  required String dbDir,
  List<Uint8List> machineMemoPrefixes = const [],
  List<SyncServer>? syncServers,
  TorPolicy tor = referenceTorPolicy,
}) {
  final network = referenceNetwork();
  return WalletConfig(
    dbDir: dbDir,
    network: network,
    endpointUrl: endpointForNetwork(network, override: zecEndpointDefine),
    // P3-13: the servers the picker OFFERS. The reference app passes
    // `referenceOfferedServers()` (the SDK's public catalog; a host appends
    // its own, ADR-0568) from `main()` AFTER the FFI is up; this builder
    // stays pure so it is unit-pinned on the host VM. `null` = nothing
    // offered beyond the default.
    syncServers: syncServers,
    tor: tor,
    seedPersistence: SeedPersistence.sealedKeychain,
    birthdayHeight: null,
    broadcastJitter: const JitterPolicy.uniform(
      maxMs: referenceBroadcastJitterMaxMs,
    ),
    // FR-27: EMPTY by default — the reference policy defines no machine-memo
    // envelope of its own, so `machineMemos` stays closed unless a consumer
    // asks for it. The example app passes a demo prefix to exercise the seam;
    // a real host passes its own. "Not my feature" is the default, not "off".
    machineMemoPrefixes: machineMemoPrefixes,
  );
}

/// Copy this config with a different restore [birthdayHeight] (the SDK's
/// `WalletConfig` is FRB-generated and carries no `copyWith`). This is the ONE
/// place a config field is overridden after [buildWalletConfig], so the field
/// list lives in a single file (DRY). Every other field is carried: the config
/// test copies a config with EVERY field set and compares it with the generated
/// `==`, so a field added to `WalletConfig` and missed here fails that test.
///
/// `birthdayHeight` is the wallet's creation height — the ONE source of truth the
/// SDK reads for where the restore scan starts (the §3.3 sketch's separate
/// `birthdayHeight` restore param is superseded by this config field;
/// manager-flagged, reversible). `null` floors the scan to Sapling activation: a
/// full, slower, but money-SAFE scan that never silently skips older funds.
extension WalletConfigBirthday on WalletConfig {
  WalletConfig withBirthdayHeight(int? birthdayHeight) => WalletConfig(
    dbDir: dbDir,
    network: network,
    endpointUrl: endpointUrl,
    // Carried: a restore against a gated endpoint that lost its key would
    // provision, then never sync.
    endpointAuthHeader: endpointAuthHeader,
    endpointAuthValue: endpointAuthValue,
    tor: tor,
    seedPersistence: seedPersistence,
    birthdayHeight: birthdayHeight,
    broadcastJitter: broadcastJitter,
    // Carried, not defaulted. This copy runs on the RESTORE path, and a host
    // whose read scope silently vanished on restore would read "no machine
    // memo" over every payment ever made to it — the same silent-drop shape
    // FR-28 fixed at re-compose, one config away.
    machineMemoPrefixes: machineMemoPrefixes,
    // Carried for the same reason (P3-13): a restore that silently offered no
    // servers would leave the picker with the default and a custom entry only.
    syncServers: syncServers,
  );
}

/// The servers the reference app OFFERS: the SDK's public catalog for the
/// build's network. The SDK ships no gated server and no key (ADR-0568). A
/// host that offers servers of its own appends them, each with its own key,
/// and passes the list to [buildWalletConfig]'s `syncServers`:
///
/// ```dart
/// const myKey = String.fromEnvironment('MY_SERVER_KEY'); // never a file
/// final offered = [
///   ...referenceOfferedServers(),
///   if (myKey.isNotEmpty)
///     SyncServer(id: 'mine', label: 'My server', url: 'https://…:443',
///         authHeader: 'x-api-key', authValue: myKey),
/// ];
/// ```
///
/// Calls the bridge — run it AFTER the FFI is initialised (`main()`), never
/// from a pure builder or a host-VM test.
List<SyncServer> referenceOfferedServers() =>
    referenceSyncServers(network: referenceNetwork());
