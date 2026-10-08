// FR-33 — the bridge-version check `RustLib.init` runs before the first
// bridge call. Host-VM tests of the pure comparison (the native half is a
// closure, so no library is loaded) and of the real facade over the process
// loader. The Rust side binds the two constants
// (`bridge_abi.rs::the_dart_bridge_abi_constant_equals_the_rust_one`).
import 'dart:io' show Platform;

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/src/bridge_abi.dart';
import 'package:zec_wallet/zec_wallet.dart' show ExternalLibrary, RustLib;

bool _neverAsked() => fail('the presence probe must not run on this path');

void main() {
  test('the same bridge version on both halves passes', () {
    expect(
      () => verifyBridgeAbi(
        () => kBridgeAbiVersion,
        walletLibraryPresent: _neverAsked,
      ),
      returnsNormally,
    );
  });

  test(
    'a different native bridge version throws BridgeAbiMismatch naming both',
    () {
      for (final native in [kBridgeAbiVersion - 1, kBridgeAbiVersion + 1]) {
        expect(
          () =>
              verifyBridgeAbi(() => native, walletLibraryPresent: _neverAsked),
          throwsA(
            isA<BridgeAbiMismatch>()
                .having((e) => e.dartVersion, 'dartVersion', kBridgeAbiVersion)
                .having((e) => e.nativeVersion, 'nativeVersion', native),
          ),
        );
      }
    },
  );

  // A host reads `ArgumentError` from init as "this build has no wallet
  // library" (a PERMANENT verdict). A native library too old to export the
  // version IS present — so its failed lookup must reach the host as the
  // mismatch, never as the lookup's own `ArgumentError`.
  test(
    'a native library with no version export is a mismatch and never an ArgumentError',
    () {
      Object? thrown;
      try {
        verifyBridgeAbi(
          () => throw ArgumentError('Failed to lookup symbol'),
          walletLibraryPresent: () => true,
        );
      } catch (e) {
        thrown = e;
      }
      expect(thrown, isA<BridgeAbiMismatch>());
      expect(thrown, isNot(isA<ArgumentError>()));
      expect(thrown, isA<Exception>());
      final mismatch = thrown! as BridgeAbiMismatch;
      expect(mismatch.dartVersion, kBridgeAbiVersion);
      expect(mismatch.nativeVersion, isNull);
      expect(mismatch.toString(), contains('reports no bridge version'));
    },
  );

  // The FR-33 security review's HIGH: on iOS/macOS the process loader opens
  // nothing, so a build WITHOUT the wallet library is first noticed at the
  // version lookup. That must stay the loader's own "absent" — the SAME
  // ArgumentError, rethrown — never a mismatch.
  test(
    'with no wallet library at all the lookup ArgumentError is rethrown unchanged',
    () {
      final lookupFailure = ArgumentError('Failed to lookup symbol');
      Object? thrown;
      try {
        verifyBridgeAbi(
          () => throw lookupFailure,
          walletLibraryPresent: () => false,
        );
      } catch (e) {
        thrown = e;
      }
      expect(identical(thrown, lookupFailure), isTrue);
    },
  );

  // The same rule through the REAL facade and the real dart:ffi lookups: this
  // test process has no wallet library, and the process loader (what the
  // README tells iOS/macOS hosts to pass) opens nothing — so init must fail
  // with ArgumentError, exactly as it did before the check, and leave the
  // runtime uninitialised. Relim's `a_real_host_init_failure_is_graded_
  // libraryAbsent` depends on this.
  test(
    'init over the process loader without the wallet library fails as absent',
    () async {
      await expectLater(
        RustLib.init(
          externalLibrary: ExternalLibrary.process(iKnowHowToUseIt: true),
        ),
        throwsA(isA<ArgumentError>()),
      );
      expect(RustLib.instance.initialized, isFalse);
    },
    skip: Platform.isWindows
        ? 'the process loader is an Apple/Linux shape'
        : false,
  );
}
