import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_store.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/shared_prefs_onboarding_store.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// The shared_preferences-backed onboarding gate flag (spec §3.2g iii-B-2-b).
/// The money-safety contract: fail-safe `false` on an unknown key, durable
/// round-trip, and a namespaced key that can't collide with the appearance
/// prefs. Driven over `SharedPreferences.setMockInitialValues` (no platform
/// channel) — the same in-memory backing the production plugin uses under test.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() => SharedPreferences.setMockInitialValues({}));

  test(
    'an unset flag reads false (fail-safe — the gate stays CLOSED)',
    () async {
      final store = SharedPrefsOnboardingStore();
      expect(await store.isBackupConfirmed(), isFalse);
    },
  );

  test('a confirmed flag round-trips to true', () async {
    final store = SharedPrefsOnboardingStore();
    await store.setBackupConfirmed(confirmed: true);
    expect(await store.isBackupConfirmed(), isTrue);
  });

  test(
    'resetting to false round-trips back to false (the create-start reset)',
    () async {
      final store = SharedPrefsOnboardingStore();
      await store.setBackupConfirmed(confirmed: true);
      await store.setBackupConfirmed(confirmed: false);
      expect(await store.isBackupConfirmed(), isFalse);
    },
  );

  test('the flag is DURABLE across instances (survives a relaunch)', () async {
    await SharedPrefsOnboardingStore().setBackupConfirmed(confirmed: true);
    // A fresh instance (a new launch) reads the persisted value — the gate
    // decision must survive process restart.
    expect(await SharedPrefsOnboardingStore().isBackupConfirmed(), isTrue);
  });

  test(
    'persists under the namespaced key (no collision with other prefs)',
    () async {
      await SharedPrefsOnboardingStore().setBackupConfirmed(confirmed: true);
      final prefs = await SharedPreferences.getInstance();
      expect(
        prefs.getBool(SharedPrefsOnboardingStore.backupConfirmedKey),
        isTrue,
      );
      expect(
        SharedPrefsOnboardingStore.backupConfirmedKey,
        'wallet.onboarding.backupConfirmed',
      );
    },
  );

  test(
    'TRIPWIRE (#356-F6): the store must ride the LEGACY SharedPreferences API '
    '— the backup gate\'s crash ordering depends on its commit() durability',
    () async {
      // This suite mocks ONLY the legacy platform store (setMockInitialValues
      // in setUp). A migration to SharedPreferencesAsync/WithCache would route
      // the write through SharedPreferencesAsyncPlatform instead — UNMOCKED
      // here, so the write below would throw: that failure IS the tripwire.
      // Before "fixing" a red run of this test, read the store's MIGRATION
      // TRIPWIRE doc: the async API's Android backend (Jetpack DataStore) does
      // not carry the synchronous-durable commit() contract that the
      // write-false-BEFORE-create ordering — proven by the on-device
      // force-stop test — relies on. Moving the flag means moving it
      // into the Rust DB, not onto the async prefs API.
      final store = SharedPrefsOnboardingStore();
      await store.setBackupConfirmed(confirmed: true);
      // And the value must land in the LEGACY keyspace, readable back through
      // the legacy facade (the same store the production plugin commits).
      final prefs = await SharedPreferences.getInstance();
      expect(
        prefs.getBool(SharedPrefsOnboardingStore.backupConfirmedKey),
        isTrue,
      );
    },
  );

  // ── #390 C2: the post-restore note lifecycle (a separate, non-money key) ────
  group('deep-scan post-restore note lifecycle', () {
    test('unset reads notApplicable (fail-safe — no note)', () async {
      expect(
        await SharedPrefsOnboardingStore().deepScanRestoreNoteState(),
        DeepScanRestoreNoteState.notApplicable,
      );
    });

    test('pending and done round-trip durably across instances', () async {
      await SharedPrefsOnboardingStore().setDeepScanRestoreNoteState(
        DeepScanRestoreNoteState.pending,
      );
      expect(
        await SharedPrefsOnboardingStore().deepScanRestoreNoteState(),
        DeepScanRestoreNoteState.pending,
      );
      await SharedPrefsOnboardingStore().setDeepScanRestoreNoteState(
        DeepScanRestoreNoteState.done,
      );
      expect(
        await SharedPrefsOnboardingStore().deepScanRestoreNoteState(),
        DeepScanRestoreNoteState.done,
      );
    });

    test(
      'notApplicable REMOVES the key (a created wallet leaves no residue)',
      () async {
        final store = SharedPrefsOnboardingStore();
        await store.setDeepScanRestoreNoteState(
          DeepScanRestoreNoteState.pending,
        );
        await store.setDeepScanRestoreNoteState(
          DeepScanRestoreNoteState.notApplicable,
        );
        final prefs = await SharedPreferences.getInstance();
        expect(
          prefs.containsKey(SharedPrefsOnboardingStore.deepScanRestoreNoteKey),
          isFalse,
        );
        expect(
          await store.deepScanRestoreNoteState(),
          DeepScanRestoreNoteState.notApplicable,
        );
      },
    );

    test(
      'uses its OWN namespaced key (no collision with the backup gate)',
      () async {
        expect(
          SharedPrefsOnboardingStore.deepScanRestoreNoteKey,
          'wallet.onboarding.deepScanRestoreNote',
        );
        // Setting the note must not touch the backup gate, and vice-versa.
        final store = SharedPrefsOnboardingStore();
        await store.setBackupConfirmed(confirmed: true);
        await store.setDeepScanRestoreNoteState(
          DeepScanRestoreNoteState.pending,
        );
        expect(await store.isBackupConfirmed(), isTrue);
        expect(
          await store.deepScanRestoreNoteState(),
          DeepScanRestoreNoteState.pending,
        );
      },
    );
  });
}
