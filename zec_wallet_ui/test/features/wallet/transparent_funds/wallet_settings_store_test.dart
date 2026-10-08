import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/wallet_settings_store.dart';

/// The transparent-funds policy store (§3.2i-3) and its identity-namespacing
/// (board A4 / security MINOR-5). The contracts pinned here:
///  * fail-safe defaults per flag (expert unset ⇒ false, auto-shield unset ⇒ true);
///  * the DEVICE-GLOBAL (`namespace: null`) form keeps the exact legacy keys, so
///    an existing install reads its saved flags with no migration;
///  * a namespaced store is ISOLATED — the duress/decoy headline: identity A's
///    posture never leaks into identity B's, and neither can read or flip the
///    device-global (or the other identity's) flags.
/// Driven over `SharedPreferences.setMockInitialValues` — the same in-memory
/// backing the production plugin uses under test, so exact key names are
/// assertable.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() => SharedPreferences.setMockInitialValues({}));

  group('device-global store (namespace: null — the default)', () {
    test(
      'an unset expert gate reads false (fail-safe: gate stays closed)',
      () async {
        expect(
          await SharedPrefsWalletSettingsStore().isExpertTransparentFunds(),
          isFalse,
        );
      },
    );

    test(
      'an unset auto-shield reads true (default posture: everything shielded)',
      () async {
        expect(
          await SharedPrefsWalletSettingsStore().isAutoShieldEnabled(),
          isTrue,
        );
      },
    );

    test(
      'both flags round-trip durably across instances (survives a relaunch)',
      () async {
        final w = SharedPrefsWalletSettingsStore();
        await w.setExpertTransparentFunds(enabled: true);
        await w.setAutoShieldEnabled(enabled: false);

        final reopened = SharedPrefsWalletSettingsStore();
        expect(await reopened.isExpertTransparentFunds(), isTrue);
        expect(await reopened.isAutoShieldEnabled(), isFalse);
      },
    );

    test('reads the EXACT legacy keys — an install upgraded in place keeps its '
        'saved flags with no migration', () async {
      // Values a prior release persisted under the bare device-global keys.
      SharedPreferences.setMockInitialValues({
        SharedPrefsWalletSettingsStore.expertKey: true,
        SharedPrefsWalletSettingsStore.autoShieldKey: false,
      });
      final store = SharedPrefsWalletSettingsStore();
      expect(await store.isExpertTransparentFunds(), isTrue);
      expect(await store.isAutoShieldEnabled(), isFalse);
    });

    test('the legacy key strings are frozen (drift guard — changing them '
        'silently orphans every existing install)', () {
      expect(
        SharedPrefsWalletSettingsStore.expertKey,
        'wallet.transparentFunds.expert',
      );
      expect(
        SharedPrefsWalletSettingsStore.autoShieldKey,
        'wallet.transparentFunds.autoShield',
      );
    });
  });

  group('identity-namespaced store (board A4)', () {
    test('a namespaced store does NOT read the device-global flags — a decoy '
        'identity sees the SAFE defaults, not the owner\'s posture', () async {
      // The owner (device-global) has the sophisticated posture.
      SharedPreferences.setMockInitialValues({
        SharedPrefsWalletSettingsStore.expertKey: true,
        SharedPrefsWalletSettingsStore.autoShieldKey: false,
      });
      final decoy = SharedPrefsWalletSettingsStore(namespace: 'decoy');
      // The headline: the decoy reads its OWN unset keyspace ⇒ fail-safe
      // defaults, never the owner's expert-on / auto-shield-off fingerprint.
      expect(await decoy.isExpertTransparentFunds(), isFalse);
      expect(await decoy.isAutoShieldEnabled(), isTrue);
    });

    test(
      'two identities are ISOLATED — A\'s flags never surface under B',
      () async {
        final alice = SharedPrefsWalletSettingsStore(namespace: 'alice');
        final bob = SharedPrefsWalletSettingsStore(namespace: 'bob');

        await alice.setExpertTransparentFunds(enabled: true);
        await alice.setAutoShieldEnabled(enabled: false);

        // Bob still reads his own untouched keyspace (the fail-safe defaults).
        expect(await bob.isExpertTransparentFunds(), isFalse);
        expect(await bob.isAutoShieldEnabled(), isTrue);
        // Alice keeps hers.
        expect(await alice.isExpertTransparentFunds(), isTrue);
        expect(await alice.isAutoShieldEnabled(), isFalse);
      },
    );

    test('a namespaced write does NOT flip the device-global flags — one '
        'identity can\'t change another\'s policy', () async {
      final ns = SharedPrefsWalletSettingsStore(namespace: 'alice');
      await ns.setAutoShieldEnabled(enabled: false);

      // Isolation, proven both ways in the SAME body so it can't pass on a lost
      // write: alice KEEPS her false AND the device-global store is untouched
      // (still the shipped default). (Asserting only the device-global default
      // would pass whether the write was isolated OR silently lost.)
      expect(await ns.isAutoShieldEnabled(), isFalse);
      expect(
        await SharedPrefsWalletSettingsStore().isAutoShieldEnabled(),
        isTrue,
      );
    });

    test('the namespaced key FORMAT is frozen (drift guard — a refactor that '
        'moved the segment would orphan every identity\'s saved flags)', () async {
      // Seed the concrete expected key strings and prove the store reads them —
      // pins wallet.<ns>.transparentFunds.{expert,autoShield}. A layout refactor
      // keeps every behavioural test green (isolation holds either way) yet
      // silently orphans saved flags; this catches it.
      SharedPreferences.setMockInitialValues({
        'wallet.alice.transparentFunds.expert': true,
        'wallet.alice.transparentFunds.autoShield': false,
      });
      final alice = SharedPrefsWalletSettingsStore(namespace: 'alice');
      expect(await alice.isExpertTransparentFunds(), isTrue);
      expect(await alice.isAutoShieldEnabled(), isFalse);
    });

    test('namespaced flags round-trip durably per identity', () async {
      await SharedPrefsWalletSettingsStore(
        namespace: 'alice',
      ).setExpertTransparentFunds(enabled: true);
      final reopened = SharedPrefsWalletSettingsStore(namespace: 'alice');
      expect(await reopened.isExpertTransparentFunds(), isTrue);
    });

    test('an EXPLICIT empty namespace is ISOLATED, not device-global — a '
        'switch-window empty id reads safe defaults, never the owner\'s posture '
        '(S172 security review, the safe fail direction)', () async {
      // The owner's real posture lives in the device-global keys.
      await SharedPrefsWalletSettingsStore().setExpertTransparentFunds(
        enabled: true,
      );
      // A host that momentarily passes '' (an id that hasn't resolved during an
      // identity switch) must NOT read the owner's device-global flags — it gets
      // its own isolated keyspace, so the fail-safe default (gate closed).
      final empty = SharedPrefsWalletSettingsStore(namespace: '');
      expect(await empty.isExpertTransparentFunds(), isFalse);
    });
  });

  group('teardown / migration helpers', () {
    test(
      'clearDeviceGlobalFlags removes the legacy owner posture from disk',
      () async {
        await SharedPrefsWalletSettingsStore().setExpertTransparentFunds(
          enabled: true,
        );
        await SharedPrefsWalletSettingsStore().setAutoShieldEnabled(
          enabled: false,
        );

        await SharedPrefsWalletSettingsStore.clearDeviceGlobalFlags();

        // Both device-global keys are gone → reads fall back to the fail-safe
        // defaults (nothing left on disk for a forensic read).
        final prefs = await SharedPreferences.getInstance();
        expect(
          prefs.containsKey(SharedPrefsWalletSettingsStore.expertKey),
          isFalse,
        );
        expect(
          prefs.containsKey(SharedPrefsWalletSettingsStore.autoShieldKey),
          isFalse,
        );
        expect(
          await SharedPrefsWalletSettingsStore().isExpertTransparentFunds(),
          isFalse,
        );
      },
    );

    test(
      'clearNamespace removes only that identity\'s flags, leaving others',
      () async {
        await SharedPrefsWalletSettingsStore(
          namespace: 'alice',
        ).setExpertTransparentFunds(enabled: true);
        await SharedPrefsWalletSettingsStore(
          namespace: 'bob',
        ).setExpertTransparentFunds(enabled: true);

        await SharedPrefsWalletSettingsStore.clearNamespace('alice');

        // Alice is wiped (back to defaults); Bob is untouched.
        expect(
          await SharedPrefsWalletSettingsStore(
            namespace: 'alice',
          ).isExpertTransparentFunds(),
          isFalse,
        );
        expect(
          await SharedPrefsWalletSettingsStore(
            namespace: 'bob',
          ).isExpertTransparentFunds(),
          isTrue,
        );
      },
    );

    test(
      'clearDeviceGlobalFlags does NOT touch namespaced identities',
      () async {
        await SharedPrefsWalletSettingsStore(
          namespace: 'alice',
        ).setExpertTransparentFunds(enabled: true);
        await SharedPrefsWalletSettingsStore.clearDeviceGlobalFlags();
        expect(
          await SharedPrefsWalletSettingsStore(
            namespace: 'alice',
          ).isExpertTransparentFunds(),
          isTrue,
        );
      },
    );
  });
}
