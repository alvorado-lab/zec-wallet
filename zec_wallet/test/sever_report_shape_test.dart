// Stage S16 `bridge` (§3.3) — the Dart shape of the duress sever's answer,
// built on the host VM without loading the native library. Written against
// the names the contract says the fold's regen ADDS (`WalletHandle
// .severCustody`, `SeverReport`, and its five enums, each with the
// forward-compatibility arm the README's "sealed classes grow" rule asks
// for); it cannot analyze until that regen lands.
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// What a host under duress does with the report: obligation 1 reads
/// `severed`, never the absence of an exception, and ALWAYS keeps a default
/// arm, because the SDK may add an outcome.
String hostReads(SeverReport report) => switch (report.severed) {
  SeverOutcome_Severed() => 'severed',
  SeverOutcome_SeveredUnproven() => 'confirm after unlock',
  SeverOutcome_AlreadyGone() => 'nothing to sever',
  SeverOutcome_NotSevered(cause: NotSeveredCause.stillRunning) =>
    'still running: do not purge, call again',
  SeverOutcome_NotSevered(cause: NotSeveredCause.timeout) ||
  SeverOutcome_NotSevered(cause: NotSeveredCause.busy) => 'retry',
  SeverOutcome_NotSevered() => 'custody may be live',
  _ => 'custody may be live',
};

/// EXHAUSTIVE, no default arm: a cause added to the bridge does not analyze
/// until a host's reading of it is written here.
String causeReads(NotSeveredCause cause) => switch (cause) {
  NotSeveredCause.nothingSevered => 'custody may be live',
  NotSeveredCause.timeout => 'retry',
  NotSeveredCause.busy => 'retry',
  NotSeveredCause.pastDeadline => 'custody may be live',
  NotSeveredCause.vaultAbsent => 'custody may be live',
  NotSeveredCause.keystoreUnavailable => 'custody may be live',
  NotSeveredCause.stillRunning => 'still running: do not purge, call again',
  NotSeveredCause.unknown => 'custody may be live',
};

/// EXHAUSTIVE: the ONE value a host purges on is `leftForHost`.
bool hostPurges(FilesOutcome files) => switch (files) {
  FilesOutcome.removed => false,
  FilesOutcome.leftForHost => true,
  FilesOutcome.stillInUse => false,
  FilesOutcome.unknown => false,
};

void main() {
  test('the overrun is its own answer: still running, never purged', () {
    expect(
      causeReads(NotSeveredCause.stillRunning),
      'still running: do not purge, call again',
    );
    expect(hostPurges(FilesOutcome.stillInUse), isFalse);
    expect(hostPurges(FilesOutcome.leftForHost), isTrue);
    expect(
      hostReads(
        const SeverReport(
          severed: SeverOutcome.notSevered(cause: NotSeveredCause.stillRunning),
          holder: HolderSeen.unknown,
          files: FilesOutcome.stillInUse,
        ),
      ),
      'still running: do not purge, call again',
    );
  });

  test('the duress verb is on the handle, beside wipe', () {
    expect(WalletHandle.severCustody, isA<Function>());
  });

  test('the sever report enums expose the forward-compatibility arm', () {
    expect(const SeverOutcome.unknown(), isA<SeverOutcome>());
    expect(UnprovenReason.unknown, isA<UnprovenReason>());
    expect(NotSeveredCause.unknown, isA<NotSeveredCause>());
    expect(HolderSeen.unknown, isA<HolderSeen>());
    expect(FilesOutcome.unknown, isA<FilesOutcome>());
  });

  test('a host tells the outcomes apart, and a retry from a live custody', () {
    SeverReport report(SeverOutcome severed) => SeverReport(
      severed: severed,
      holder: HolderSeen.thisProcess,
      files: FilesOutcome.leftForHost,
    );
    expect(
      hostReads(
        report(
          const SeverOutcome.severedUnproven(
            reason: UnprovenReason.countUnreadable,
          ),
        ),
      ),
      'confirm after unlock',
    );
    expect(
      hostReads(report(const SeverOutcome.alreadyGone())),
      'nothing to sever',
    );
    expect(
      hostReads(
        report(const SeverOutcome.notSevered(cause: NotSeveredCause.busy)),
      ),
      'retry',
    );
    expect(
      hostReads(
        report(const SeverOutcome.notSevered(cause: NotSeveredCause.timeout)),
      ),
      'retry',
    );
    expect(
      hostReads(
        report(
          const SeverOutcome.notSevered(cause: NotSeveredCause.nothingSevered),
        ),
      ),
      'custody may be live',
    );
    expect(
      hostReads(report(const SeverOutcome.unknown())),
      'custody may be live',
    );
    final r = report(const SeverOutcome.alreadyGone());
    expect(r.holder, HolderSeen.thisProcess);
    expect(r.files, FilesOutcome.leftForHost);
    expect(
      HolderSeen.values,
      containsAll([HolderSeen.none, HolderSeen.otherProcess]),
    );
    expect(FilesOutcome.values, contains(FilesOutcome.removed));
  });
}
