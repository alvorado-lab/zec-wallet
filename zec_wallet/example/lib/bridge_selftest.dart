import 'dart:typed_data' show Uint8List;

import 'package:zec_wallet/zec_wallet.dart';

/// The SDK's host-testable bridge self-test: one PASS/FAIL line per check,
/// exercising the full FFI surface in one place — the outbound DTO tree, typed
/// errors caught BY TYPE with stable codes, and inbound conversion + validation.
/// `integration_test/bridge_selftest_test.dart` asserts on this exact result, so
/// "what an adopter can run" is "what CI checks".
class CheckResult {
  CheckResult(this.name, this.passed, this.detail);

  final String name;
  final bool passed;
  final String detail;
}

/// Every check the bridge must pass: outbound DTO tree, typed errors caught
/// BY TYPE with stable codes, inbound conversion + validation.
Future<List<CheckResult>> runBridgeSelftest() async {
  final results = <CheckResult>[];

  void check(String name, bool ok, String detail) {
    results.add(CheckResult(name, ok, detail));
  }

  final version = sdkVersion();
  check('sdk_version', version.isNotEmpty, 'version: $version');

  // outbound: the full DTO tree in one hop
  final bundle = selftestBundle();
  check(
    'wallet_state crossing',
    bundle.walletState.seq == 7 &&
        bundle.walletState.balance.spendableZat == 123450000 &&
        bundle.walletState.syncStatus is SyncStatus_Scanning &&
        // The sample's two adjacent bools DIFFER (spendableReady false,
        // rewound true) so a transposed pair fails here, not just a
        // dropped/defaulted field (#317).
        (bundle.walletState.syncStatus as SyncStatus_Scanning).rewound &&
        !(bundle.walletState.syncStatus as SyncStatus_Scanning)
            .spendableReady &&
        // lastSynced is host-RENDERED since #317 — pin the value AND the
        // seconds unit through the tree's only PlatformInt64 in WalletState.
        bundle.walletState.lastSynced?.height == 2499900 &&
        bundle.walletState.lastSynced?.at == 1780000000 &&
        bundle.walletState.tor is TorState_Active,
    'seq=${bundle.walletState.seq} spendableZat=${bundle.walletState.balance.spendableZat}',
  );
  check(
    // 18 = the 9 non-Stalled arms (Idle/Connecting/Scanning/UpToDate/UpToDateLimited/
    // UpToDateDegraded/EndpointBehind/UpToDateUnverified/Offline) + the 8 Stalled
    // reasons (Endpoint/Tor/StorageFull/ChainReorg/Internal/EndpointMisbehaving/
    // BirthdayInFuture/StorageUnavailable) + Unknown. Bump this when a SyncStatus arm
    // OR a StallReason is added. (Was 10 until the device run caught the missed
    // `StallReason::Internal` bump on the iPhone — 11 until T0-1b, which also added
    // the `UpToDateLimited` sample that had been missing since its arm landed; 14
    // until T0-1c added `EndpointBehind`; 15 until GRACE-1 added
    // `UpToDateUnverified`; 16 until R10 added `StorageUnavailable` — and found the
    // `BirthdayInFuture` sample of 8fd51acf8 had missed its bump, so 16 was stale.)
    'sync_status arms',
    bundle.syncStatuses.length == 18 &&
        bundle.syncStatuses.whereType<SyncStatus_Unknown>().length == 1,
    '${bundle.syncStatuses.length} arms incl. unknown',
  );
  check(
    // BUMP THIS WHEN A SAMPLE IS ADDED — the sibling above carries the same
    // comment because the miss it describes reached an iPhone, and
    // this row then drifted the same way anyway: FR-5 C1 took
    // `sample_tor_states()` from 6 to 8 (the two failing arms gained a
    // `transport` name, so each is sampled named AND unnamed) and left this
    // literal at 7, where only the device walk would have printed it.
    // 9 samples (stage S1 `truth` added `Unanswered`) + the chained
    // `TorState::Unknown` = 10.
    'tor_state arms',
    bundle.torStates.length == 10 &&
        bundle.torStates.whereType<TorState_Unknown>().length == 1 &&
        bundle.torStates.whereType<TorState_Unanswered>().length == 1,
    '${bundle.torStates.length} arms incl. unknown',
  );
  check(
    'tx history rows',
    bundle.txRows.length == 5 &&
        bundle.txRows.first.txidHex.length == 64 &&
        bundle.txRows.first.netAmountZat == -1500000 &&
        // hasMemo/hasTransparentOutput carry OPPOSITE parity (row 0: memo yes,
        // transparent no; row 1 the reverse) so a transposed-field bug shows here.
        bundle.txRows[0].hasMemo &&
        !bundle.txRows[0].hasTransparentOutput &&
        !bundle.txRows[1].hasMemo &&
        bundle.txRows[1].hasTransparentOutput,
    '${bundle.txRows.length} rows, txid ${bundle.txRows.first.txidHex.substring(0, 8)}…',
  );
  check(
    'submit results',
    bundle.submitResults.length == 5 &&
        bundle.submitResults.whereType<TxSubmitResult_Unknown>().length == 1,
    '${bundle.submitResults.length} outcomes incl. unknown',
  );
  check(
    'swap quote + disclosure',
    bundle.quote.zecSideZat == 98000000 &&
        bundle.quote.disclosure.providerSees.length == 5 &&
        bundle.quote.disclosure.deshields,
    'disclosure items: ${bundle.quote.disclosure.providerSees.length}',
  );
  check(
    'swap status arms',
    bundle.swapStatuses.length == 8 &&
        bundle.swapStatuses.whereType<SwapStatus_Unknown>().length == 1,
    '${bundle.swapStatuses.length} arms incl. unknown',
  );
  check(
    'swap record',
    bundle.swapRecord.id == 'selftest-record-1' &&
        bundle.swapRecord.direction == SwapRecordDirection.outOfZec &&
        bundle.swapRecord.depositDeadline == 1780000900 &&
        bundle.swapRecord.expiresAt == 1780172800,
    'id=${bundle.swapRecord.id}',
  );

  // typed errors: caught BY TYPE, stable code, sealed kind
  try {
    selftestThrowWalletError();
    check('wallet error typing', false, 'no exception thrown');
  } on WalletApiError catch (e) {
    check(
      'wallet error typing',
      e.code == 'RW-LIFE-001' && e.kind is WalletErrorKind_WalletAlreadyOpen,
      'code=${e.code} kind=${e.kind.runtimeType}',
    );
  }
  try {
    selftestThrowSwapError();
    check('swap error typing', false, 'no exception thrown');
  } on SwapApiError catch (e) {
    check(
      'swap error typing',
      e.code == 'RW-SWAP-003' && e.kind is SwapErrorKind_QuoteExpired,
      'code=${e.code} kind=${e.kind.runtimeType}',
    );
  }

  // inbound: a Dart-built request through the real conversion layer
  try {
    selftestQuoteRequestRoundtrip(
      request: QuoteRequest(
        direction: SwapDirection.intoZec(
          from: AssetId(chain: 'eth', symbol: 'usdc'),
        ),
        exact: ExactSide.in_(amount: SwapAmount.foreign(amount: '100.0')),
        slippageToleranceBps: 200,
        destination: null,
        refundAddress: '0xSelftestRefund',
      ),
    );
    check('quote request inbound', true, 'valid request accepted');
  } on SwapApiError catch (e) {
    check('quote request inbound', false, 'unexpected reject ${e.code}');
  }
  try {
    selftestQuoteRequestRoundtrip(
      request: QuoteRequest(
        direction: SwapDirection.outOfZec(
          to: AssetId(chain: 'eth', symbol: 'usdc'),
        ),
        exact: ExactSide.in_(amount: SwapAmount.zec(zat: -5)),
        slippageToleranceBps: 200,
        destination: '0xSelftestDestination',
        refundAddress: null,
      ),
    );
    check('quote request bounds', false, 'negative zat accepted');
  } on SwapApiError catch (e) {
    check(
      'quote request bounds',
      e.code == 'RW-SWAP-005',
      'rejected typed: ${e.code}',
    );
  }

  // config validation: the real Rust-side rules, callable pre-open
  try {
    validateWalletConfig(
      config: WalletConfig(
        dbDir: '/tmp/selftest-never-created',
        network: Network.test,
        endpointUrl: 'https://testnet.zec.rocks:443',
        // T0-6 (2026-09-11): `TorPolicy.preferred`/`required_` are now REFUSED
        // at this door, because the only runtime this enum can express
        // (`externalSocks5`) has no dialer behind it — a host that asked for
        // Tor used to validate here and then fail at the first dial. `off` is
        // the only policy a Dart host can currently express, and the row below
        // proves the refusal rather than leaving this one to discover it.
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.none,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.uniform(maxMs: 10000),
        // FR-27: a real read scope, so the good-config row also proves a
        // registered prefix is ACCEPTED (the bad-scope row below is its
        // polarity).
        machineMemoPrefixes: [
          Uint8List.fromList([0x52, 0x4C, 0x4D, 0x01]),
        ],
      ),
    );
    check('config validation (good)', true, 'https endpoint accepted');
  } on WalletApiError catch (e) {
    check('config validation (good)', false, 'unexpected reject ${e.code}');
  }
  // T0-6: the unbuilt Tor runtime is refused HERE, before anything opens, with
  // a reason a developer can act on — not at the first dial as a
  // network-shaped error. The polarity row for the `off` config above.
  try {
    validateWalletConfig(
      config: WalletConfig(
        dbDir: '/tmp/selftest-never-created',
        network: Network.test,
        endpointUrl: 'https://testnet.zec.rocks:443',
        tor: const TorPolicy.preferred(
          runtime: TorRuntimeConfig.externalSocks5(addr: '127.0.0.1:9050'),
        ),
        seedPersistence: SeedPersistence.none,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.uniform(maxMs: 10000),
        machineMemoPrefixes: const [],
      ),
    );
    check('config validation (unbuilt tor runtime)', false, 'accepted');
  } on WalletApiError catch (e) {
    check(
      'config validation (unbuilt tor runtime)',
      e.code == 'RW-CFG-001',
      'rejected ${e.code}',
    );
  }
  try {
    validateWalletConfig(
      config: WalletConfig(
        dbDir: '/tmp/selftest-never-created',
        network: Network.main,
        endpointUrl: 'http://evil.example.com:9067',
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.sealedKeychain,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.none(),
        machineMemoPrefixes: const [],
      ),
    );
    check('config validation (bad)', false, 'remote http accepted');
  } on WalletApiError catch (e) {
    check(
      'config validation (bad)',
      e.code == 'RW-CFG-001' && e.kind is WalletErrorKind_InvalidEndpoint,
      'rejected typed: ${e.code}',
    );
  }
  // #324: a RELATIVE dbDir must be a typed RW-CFG-003 at the config door —
  // unvalidated it would create_dir_all against the process cwd (desktop:
  // the wallet silently lands wherever the app was launched from).
  try {
    validateWalletConfig(
      config: WalletConfig(
        dbDir: 'relative/never-created',
        network: Network.main,
        endpointUrl: 'https://zec.rocks:443',
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.sealedKeychain,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.none(),
        machineMemoPrefixes: const [],
      ),
    );
    check('config validation (bad dbDir)', false, 'relative dbDir accepted');
  } on WalletApiError catch (e) {
    check(
      'config validation (bad dbDir)',
      e.code == 'RW-CFG-003' && e.kind is WalletErrorKind_InvalidDbDir,
      'rejected typed: ${e.code}',
    );
  }
  // FR-27: an EMPTY machine-memo prefix matches every memo ever written, so the
  // scope would filter nothing at all and every unrelated envelope on the chain
  // would reach the host's parser. Refused at the config door, typed, before a
  // wallet exists — the polarity of the good-config row above.
  //
  // (An earlier version of this comment said such a host "would read strangers'
  // envelopes and believe they were addressed to it". The second half is the
  // fallacy this whole design disclaims: with a NON-empty prefix a host also
  // reads strangers' envelopes, because forging a prefix is free. The real and
  // sufficient reason is volume. Caught by the FR-27 security review — it was
  // the only place SDK prose acted as if a match meant something.)
  try {
    validateWalletConfig(
      config: WalletConfig(
        dbDir: '/tmp/selftest-never-created',
        network: Network.main,
        endpointUrl: 'https://zec.rocks:443',
        tor: const TorPolicy.off(),
        seedPersistence: SeedPersistence.sealedKeychain,
        birthdayHeight: null,
        broadcastJitter: const JitterPolicy.none(),
        machineMemoPrefixes: [Uint8List(0)],
      ),
    );
    check(
      'config validation (empty memo prefix)',
      false,
      'empty prefix accepted',
    );
  } on WalletApiError catch (e) {
    check(
      'config validation (empty memo prefix)',
      e.code == 'RW-PAY-010' &&
          e.kind is WalletErrorKind_MachineMemoScopeInvalid,
      'rejected typed: ${e.code}',
    );
  }

  return results;
}
