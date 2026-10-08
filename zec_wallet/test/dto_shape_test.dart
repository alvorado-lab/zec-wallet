// Host-VM shape tests: construct the generated Dart surface WITHOUT loading
// the native library (constructors and sealed-class pattern matching are
// pure Dart). Freezes the public DTO shape — a codegen regression that
// renames/retypes a field fails here, on any machine, with no device.
import 'dart:typed_data' show Uint8List;

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';

void main() {
  test('config surface constructs and pattern-matches', () {
    final config = WalletConfig(
      dbDir: '/tmp/x',
      network: Network.main,
      endpointUrl: 'https://zec.rocks:443',
      tor: const TorPolicy.required_(
        runtime: TorRuntimeConfig.externalSocks5(addr: '127.0.0.1:9050'),
      ),
      seedPersistence: SeedPersistence.sealedKeychain,
      birthdayHeight: 2400000,
      broadcastJitter: const JitterPolicy.uniform(maxMs: 10000),
      // FR-27 — the machine-memo read scope crosses as `List<Uint8List>`.
      // Pinned with a REAL prefix (not an empty list) so the generated type is
      // frozen too: a codegen change to `List<List<int>>` or `Uint8List` would
      // compile past an empty literal and fail here.
      machineMemoPrefixes: [
        Uint8List.fromList([0x52, 0x4C, 0x4D, 0x01]),
      ],
    );
    // the G2 contract: ALWAYS keep a default arm — the SDK may add variants
    final mode = switch (config.tor) {
      TorPolicy_Off() => 'off',
      TorPolicy_Required() => 'required',
      _ => 'other',
    };
    expect(mode, 'required');
    expect(config.machineMemoPrefixes.single, isA<Uint8List>());
    expect(config.machineMemoPrefixes.single, [0x52, 0x4C, 0x4D, 0x01]);
  });

  test('status enums expose the forward-compatibility arm', () {
    // hosts can rely on the unknown arm EXISTING (render it neutrally)
    expect(const SyncStatus.unknown(), isA<SyncStatus>());
    expect(const TorState.unknown(), isA<TorState>());
    expect(const TxStatus.unknown(), isA<TxStatus>());
    expect(const SwapStatus.unknown(), isA<SwapStatus>());
    expect(const SwapDirection.unknown(), isA<SwapDirection>());
    expect(StallReason.unknown, isA<StallReason>());
    // A sealed class since FR-29 (the host-dialer arm carries data), so the
    // arm is a const factory like the four above, not an enum value.
    expect(const TorRuntimeKind.unknown(), isA<TorRuntimeKind>());
    expect(SwapFailureCode.unknown, isA<SwapFailureCode>());
    expect(DisclosureItem.unknown, isA<DisclosureItem>());
  });

  test('state DTOs carry value equality (freezed)', () {
    const a = SyncStatus.scanning(
      from: 1,
      to: 100,
      percent: 42.5,
      spendableReady: true,
      rewound: false,
    );
    const b = SyncStatus.scanning(
      from: 1,
      to: 100,
      percent: 42.5,
      spendableReady: true,
      rewound: false,
    );
    expect(a, b);
    final state = WalletState(
      balance: const BalanceSnapshot(
        spendableZat: 1,
        pendingIncomingZat: 0,
        pendingChangeZat: 0,
        transparentZat: 0,
        totalZat: 1,
      ),
      syncStatus: a,
      tor: const TorState.off(),
      tip: 100,
      lastSynced: null,
      everSynced: true,
      rescanRebuilding: false,
      seq: 1,
    );
    expect(state.balance.spendableZat, 1);
  });

  test(
    'typed errors are catchable exception types with code + sealed kind',
    () {
      const err = WalletApiError(
        code: 'RW-LIFE-001',
        message: 'wallet already open',
        kind: WalletErrorKind.walletAlreadyOpen(),
      );
      expect(err, isA<Exception>());
      final rendered = switch (err.kind) {
        WalletErrorKind_WalletAlreadyOpen() => 'already-open',
        _ => 'other',
      };
      expect(rendered, 'already-open');
    },
  );

  test('swap-disabled error surfaces typed with a forward-compat arm', () {
    // the regulatory kill switch reaching the SDK (RW-SWAP-008): hosts match
    // swapDisabled to hide the swap surface, ALWAYS keeping a default arm.
    const err = SwapApiError(
      code: 'RW-SWAP-008',
      message: 'swap is disabled',
      kind: SwapErrorKind.swapDisabled(),
    );
    expect(err, isA<Exception>());
    final rendered = switch (err.kind) {
      SwapErrorKind_SwapDisabled() => 'swap-off',
      _ => 'other',
    };
    expect(rendered, 'swap-off');
    // the forward-compat arm must EXIST (G2): an older host can't panic on a
    // future swap-error kind.
    expect(const SwapErrorKind.unknown(), isA<SwapErrorKind>());
  });
}
