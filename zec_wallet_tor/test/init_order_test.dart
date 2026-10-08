// P22 (docs/specs/tor-plugin.md §8): `init` before the wallet's
// `RustLib.init()` throws `TorPluginError(kind: walletNotLoaded)`, caught by
// TYPE; a second `init` returns the same status. Plus the plan's D-11: the
// native contract version is read BY NAME before any other call, and a
// mismatch is refused typed.
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_tor/src/bindings_generated.dart';
import 'package:zec_wallet_tor/zec_wallet_tor.dart';

import 'fake_native_api.dart';

const _torDir = '/data/user/0/app/files/tor';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late FakeTorNativeApi native;

  setUp(() {
    native = FakeTorNativeApi();
    ZecWalletTor.debugReset(nativeApi: native);
  });

  tearDown(() => ZecWalletTor.debugReset());

  test('init before the wallet library is loaded throws walletNotLoaded, '
      'caught by type', () async {
    native.walletLoaded = false;
    TorPluginErrorKind? caught;
    try {
      await ZecWalletTor.init(torDir: _torDir);
    } on TorPluginError catch (e) {
      caught = e.kind;
    }
    expect(caught, TorPluginErrorKind.walletNotLoaded);

    // Once the wallet is loaded, the same call succeeds: nothing latched.
    native.walletLoaded = true;
    final status = await ZecWalletTor.init(torDir: _torDir);
    expect(status.phase, TorPluginPhase.bootstrapping);
  });

  test('a second init returns the same status', () async {
    final first = await ZecWalletTor.init(torDir: _torDir);
    final second = await ZecWalletTor.init(torDir: _torDir);
    expect(second, first);
  });

  test('the contract version is read by name before any other call', () async {
    await ZecWalletTor.init(torDir: _torDir);
    expect(native.calls.first, 'abi_version');
    // Once per process, not once per call.
    await ZecWalletTor.init(torDir: _torDir);
    expect(native.calls.where((c) => c == 'abi_version'), hasLength(1));
  });

  test('a native library from another release is refused before any other '
      'call', () async {
    native.abi = ZWT_ABI_VERSION + 1;
    await expectLater(
      ZecWalletTor.init(torDir: _torDir),
      throwsA(
        isA<TorPluginError>()
            .having((e) => e.kind, 'kind', TorPluginErrorKind.pluginAbiMismatch)
            .having((e) => e.nativeAbiVersion, 'native', ZWT_ABI_VERSION + 1),
      ),
    );
    expect(native.calls, ['abi_version']);
    // Every other verb is refused the same way — none reaches the library.
    expect(
      () => ZecWalletTor.status(),
      throwsA(
        isA<TorPluginError>().having(
          (e) => e.kind,
          'kind',
          TorPluginErrorKind.pluginAbiMismatch,
        ),
      ),
    );
    expect(native.calls.where((c) => c != 'abi_version'), isEmpty);
  });

  test('a library without the version export is libraryUnavailable', () async {
    native.abiThrows = const TorPluginError(
      TorPluginErrorKind.libraryUnavailable,
    );
    await expectLater(
      ZecWalletTor.init(torDir: _torDir),
      throwsA(
        isA<TorPluginError>().having(
          (e) => e.kind,
          'kind',
          TorPluginErrorKind.libraryUnavailable,
        ),
      ),
    );
  });

  test('status before init throws notInitialized', () {
    expect(
      () => ZecWalletTor.status(),
      throwsA(
        isA<TorPluginError>().having(
          (e) => e.kind,
          'kind',
          TorPluginErrorKind.notInitialized,
        ),
      ),
    );
  });

  test('the torDir crosses as UTF-8 bytes', () async {
    await ZecWalletTor.init(torDir: '/tmp/tör');
    // 'ö' is U+00F6: two bytes in UTF-8, one code unit in UTF-16.
    expect(native.lastTorDirSeen, [...'/tmp/t'.codeUnits, 0xc3, 0xb6, 0x72]);
  });

  test(
    'the bridge bytes reach the plugin and are zeroed after the call',
    () async {
      const paste = 'obfs4 192.0.2.1:443 AAAA cert=x iat-mode=0';
      await ZecWalletTor.init(torDir: _torDir, bridges: paste);
      expect(String.fromCharCodes(native.lastBridgesSeen!), paste);
      expect(native.lastBridgesArg!.every((b) => b == 0), isTrue);

      await ZecWalletTor.setBridges(paste);
      expect(String.fromCharCodes(native.lastBridgesSeen!), paste);
      expect(native.lastBridgesArg!.every((b) => b == 0), isTrue);
    },
  );

  test('no bridges cross as null', () async {
    await ZecWalletTor.init(torDir: _torDir);
    expect(native.lastBridgesArg, isNull);
  });

  test('a refused paste carries the class the plugin recorded', () async {
    native
      ..initRc = ZWT_RC_BRIDGES_REFUSED
      ..current = const TorPluginStatus(
        phase: TorPluginPhase.notRegistered,
        readiness: 0,
        failureClass: TorFailureClass.bridgeInvalidAddress,
      );
    await expectLater(
      ZecWalletTor.init(torDir: _torDir, bridges: 'nonsense'),
      throwsA(
        isA<TorPluginError>()
            .having((e) => e.kind, 'kind', TorPluginErrorKind.bridgesRefused)
            .having(
              (e) => e.failureClass,
              'class',
              TorFailureClass.bridgeInvalidAddress,
            )
            .having((e) => e.code, 'code', ZWT_RC_BRIDGES_REFUSED),
      ),
    );
  });

  test(
    'clearState is refused while Tor runs and succeeds after dispose',
    () async {
      await ZecWalletTor.init(torDir: _torDir);
      await expectLater(
        ZecWalletTor.clearState(_torDir),
        throwsA(
          isA<TorPluginError>().having(
            (e) => e.kind,
            'kind',
            TorPluginErrorKind.notInitialized,
          ),
        ),
      );
      await ZecWalletTor.dispose();
      await ZecWalletTor.clearState(_torDir);
      expect(String.fromCharCodes(native.lastClearStateDirSeen!), _torDir);
    },
  );

  test('clearState waits out a client still stopping after dispose (2026-10-07 '
      'review, finding 1)', () async {
    await ZecWalletTor.init(torDir: _torDir);
    await ZecWalletTor.dispose();
    native.clearStateStoppingAnswers = 3;
    await expectLater(ZecWalletTor.clearState(_torDir), completes);
    expect(
      native.calls.where((c) => c == 'clear_state'),
      hasLength(4),
      reason: 'three STOPPING answers, then the removal',
    );
  });

  test('init waits out a client still stopping after dispose too', () async {
    await ZecWalletTor.init(torDir: _torDir);
    await ZecWalletTor.dispose();
    native.initStoppingAnswers = 2;
    await expectLater(ZecWalletTor.init(torDir: _torDir), completes);
    expect(
      native.calls.where((c) => c == 'init'),
      hasLength(4),
      reason: 'the first init, two STOPPING answers, then the registration',
    );
  });

  test('a dispose during the init wait wins: the pending init never registers '
      '(2026-10-07 review, follow-up finding 2)', () async {
    await ZecWalletTor.init(torDir: _torDir);
    await ZecWalletTor.dispose();
    native.initStoppingAnswers = 1000; // the old client keeps stopping
    final pending = ZecWalletTor.init(torDir: _torDir);
    await Future<void>.delayed(const Duration(milliseconds: 150));
    native.initStoppingAnswers = 0; // it would now let a registration through
    await ZecWalletTor.dispose();
    final callsAtDispose = native.calls.length;
    await expectLater(
      pending,
      throwsA(
        isA<TorPluginError>().having(
          (e) => e.kind,
          'kind',
          TorPluginErrorKind.disposed,
        ),
      ),
    );
    expect(
      native.calls.skip(callsAtDispose),
      isNot(contains('init')),
      reason: 'no native init after the dispose',
    );
  });

  test(
    'clearState throws any other refusal at once, without retrying',
    () async {
      await ZecWalletTor.init(torDir: _torDir);
      await ZecWalletTor.dispose();
      native.clearStateRc = ZWT_RC_INVALID_DATA_DIR;
      await expectLater(
        ZecWalletTor.clearState(_torDir),
        throwsA(
          isA<TorPluginError>().having(
            (e) => e.kind,
            'kind',
            TorPluginErrorKind.invalidDataDir,
          ),
        ),
      );
      expect(native.calls.where((c) => c == 'clear_state'), hasLength(1));
    },
  );

  test('restartRequired is thrown at once by clearState: a stuck client never '
      'clears by waiting (plan §5)', () async {
    await ZecWalletTor.init(torDir: _torDir);
    await ZecWalletTor.dispose();
    native.clearStateRc = ZWT_RC_RESTART_REQUIRED;
    await expectLater(
      ZecWalletTor.clearState(_torDir),
      throwsA(
        isA<TorPluginError>().having(
          (e) => e.kind,
          'kind',
          TorPluginErrorKind.restartRequired,
        ),
      ),
    );
    expect(native.calls.where((c) => c == 'clear_state'), hasLength(1));
  });

  test('dispose with nothing running is a no-op, not an error', () async {
    await ZecWalletTor.dispose();
    expect(native.calls, ['abi_version', 'dispose']);
  });
}
