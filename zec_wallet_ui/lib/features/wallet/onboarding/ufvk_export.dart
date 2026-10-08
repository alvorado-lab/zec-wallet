import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../reveal_grant.dart';
import 'onboarding_providers.dart';

/// The UFVK export provider — the ONE sanctioned viewing-key egress into the UI
/// (#397 §3.7 D1 / ADR-0538), surfaced as an `autoDispose` future so the string
/// is dropped the instant the export screen leaves the tree. Deliberately in
/// its own file, mirroring [mnemonicRevealProvider]'s discipline.
///
/// A UFVK cannot spend, but it shows the wallet's whole history, so it is
/// sensitive: the user may share it on purpose, and nothing may log it by
/// accident. It reaches the UI in a [RedactedViewingKey], like the seed's
/// `RedactedMnemonic`, so a host's value-logging `ProviderObserver` records the
/// box, never the key. Only the export screen unwraps it, to show, copy and
/// encode it as a QR code. There is no §10 zeroize concern (no spend
/// authority). The autoDispose lifetime is still the right idiom (smallest
/// scope, no long-lived parking in the controller), and `retry: null` is still
/// correct: a failed export must not silently re-read the store on a transient
/// fault; the export screen owns the explicit user retry (which invalidates
/// this provider).
/// SOURCE ORDER (G1, #361 companion — the port asymmetry the host audit
/// found): the provisioner when the package owns provisioning, ELSE the session
/// port. Before G1 a session-only host could render the whole watch-only chrome
/// (`isWatchOnly` is on the session port) but could not export the key that
/// chrome points at — the screen dead-ended in "managed by host" and the host had
/// to reach around the package to the raw bridge handle, re-implementing the §3.7
/// D9 gate itself. Both sources hit the SAME Rust `export_ufvk`; the reveal gate,
/// warning, and screen-security envelope are the screen's, identical either way.
///
/// KEYED BY THE [RevealGrant] (S2 §3.4, R06): the read happens only under a
/// grant that is still current — checked here, immediately before the export —
/// and the provider READS its seams rather than watching them, so a wallet
/// identity change under the mounted screen can never re-run the export
/// against the next wallet (the pre-S2 shape: the session changed, this
/// re-ran, and wallet B's key was exported under wallet A's authorization).
/// A void grant throws [RevealGrantVoid] and exports nothing.
final ufvkExportProvider = FutureProvider.autoDispose
    .family<RedactedViewingKey, RevealGrant>((ref, grant) async {
      if (!revealGrantIsCurrentIn(ref, grant)) throw const RevealGrantVoid();
      final provisioner = ref.read(walletProvisionerProvider);
      if (provisioner != null) {
        return RedactedViewingKey(await provisioner.exportUfvk());
      }
      final session = grant.session;
      if (session == null) {
        // Neither seam wired: nothing to export. Fail loud rather than show an
        // empty artifact (the screen's own gate renders the honest terminal
        // state before this provider is ever read).
        throw StateError(
          'ufvkExportProvider read with no provisioner and no session',
        );
      }
      return RedactedViewingKey(await session.exportUfvk());
    }, retry: (retryCount, error) => null);

/// The exported viewing key in a box whose [toString] hides it, so a
/// value-logging observer, an error report or a debug print records
/// `RedactedViewingKey(redacted)`. Read [key] only where the user sees or
/// copies it.
final class RedactedViewingKey {
  const RedactedViewingKey(this.key);

  /// The unified full viewing key, as the wallet exported it.
  final String key;

  @override
  String toString() => 'RedactedViewingKey(redacted)';
}
