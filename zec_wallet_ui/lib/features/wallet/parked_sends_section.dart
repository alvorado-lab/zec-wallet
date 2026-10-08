import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../shared/action_row_layout.dart';
import '../../shared/wallet_dialog.dart';
import '../../shared/wallet_group.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'reclaim.dart';
import 'send/send_state.dart' show parkedAuthorizeErrorPrecedesPersistence;
import 'send_authorization.dart';
import 'sync_status_presentation.dart' show walletCompactTimeFormat;
import 'wallet_activity_controller.dart';
import 'wallet_session.dart' show WalletSession;
import 'wallet_providers.dart';
import 'wallet_sync_controller.dart';
import 'zat_format.dart';

/// The "saved & pending" surface (2e-2b-v-3/v-4b, widened by #331 and again by
/// #400): EVERY committed send that has not completed — a one-time-address (TEX)
/// two-step awaiting its drain, a plainly-queued offline send, or a row the wallet
/// has CLAIMED and is mid-signature on. Surfaced here so a committed spend is never
/// a silent-hide (invisible across a relaunch it was a DOUBLE-PAY window).
///
/// TWO ROW SHAPES, and they differ in every property this doc used to state once
/// (#401 R2c — the previous version described only the first and asserted it of
/// both):
///
///  * A `Queued` row has NO on-chain transaction, so it appears on no other
///    surface; its amount is an EARMARK shown OVER the balance (the funds stay
///    spendable — never added NOR deducted); and it carries BOTH exits, Send now
///    and Cancel.
///  * A `sending` (`Submitting`) row is one the drain or a Send now tap has
///    claimed. It appears NOWHERE only until the engine's `create` commits —
///    after that the transaction is durably in the wallet DB and the SAME payment
///    can also render as a PENDING activity row, while its notes have already left
///    the spendable balance. So the earmark contract does NOT hold for it, and the
///    header says so. It offers NO Cancel (`cancel_parked_send` is `Queued`-guarded
///    and its confirm copy would promise money-safety a claimed row cannot give)
///    and no Send now — EXCEPT while this row's own bracket is what claimed it.
///
/// The row copy is KIND-agnostic ("saved & pending" — true for BOTH
/// [ParkedSendKind]s; audited) but STATUS-split since #315: a `paused` row
/// (the wallet gave up auto-retrying — it never sends on its own) reads "paused"
/// with a hint, because rendering it as a healthy pending send would leave the
/// user waiting forever. Since FR-23-b the affordance on both is "Send now", which
/// signs inside the host's spend bracket (a paused row re-arms first).
///
/// §5.4: amount-only — the recipient never crosses the bridge. Hidden when there
/// are none; an honest error line if the read fails (a parked send is money the
/// user is waiting on, so a failed read is surfaced, never silently hidden).
///
/// KNOWN RESIDUAL, deliberate (#401 R2e): a `sending` row while sync is paused has
/// no IN-APP exit at all — Cancel is refused for the reasons above, Send now would
/// answer `notFound`, and the wallet's own recovery (the §6.3 reconcile that
/// re-queues an unfinished claim) rides a completed sync pass. Restoring a Cancel
/// that can only fail would be an affordance whose confirm dialog opens with a
/// false money statement, so the row's hint NAMES the condition instead and the
/// remedy lives on the sync surface. It is a visibility win over the pre-#400
/// state, where the same row was simply invisible.
/// SECTION-WIDE single-flight for the FR-23-b authorization (#400 R7): the
/// `'<id>-<createdAt>'` key of the row whose spend bracket is open, or `null`.
///
/// Row-scoped state was not enough. Two ordinary taps on two rows opened TWO
/// concurrent brackets, and at HOST custody that is a money problem: a single-slot
/// host stages ONE credential, so B's stage overwrites A's, A's FR-17 binding no
/// longer matches, and A is refused — the user authorized two payments and one
/// silently did not happen.
///
/// ## Its lifetime took two wrong turns before this one
///
/// The thing the latch guards OUTLIVES the widget by construction: the bracket can
/// run for tens of seconds and this handler deliberately captures the container so
/// a landed outcome survives the row's dispose. A latch whose lifetime is shorter
/// than the bracket it guards is not a latch.
///
/// 1. A ROOT `NotifierProvider` — too long. A hung host prompt (the seam contract
///    grants no timeout) would disable "Send now" on every row of EVERY identity
///    for the life of the process, and on a decoy flip the greyed buttons leak
///    that the hidden wallet has a payment in flight.
/// 2. WIDGET STATE — too short, and reachable by scrolling. `ParkedSendsSection`
///    is an unkeyed child of a lazy `ListView`, so it is destroyed both by
///    scrolling past the cache extent AND by an index shift when any banner above
///    it appears. A user who taps "Send now" and scrolls down to watch their
///    activity list comes back to a fresh state with `null`, every button
///    re-enabled, and can open a SECOND bracket on top of the live one. It was
///    also NOT per-identity as its comment claimed: `const _WalletActive()` makes
///    `Element.updateChild` short-circuit on identical widgets, so the state
///    survives an A→B flip verbatim.
///
/// The correct lifetime is CONTAINER-scoped but IDENTITY-KEYED, which is what this
/// is: `ref.watch(walletIdentityProvider)` in `build` rebuilds (and so resets) the
/// notifier the moment the wallet identity changes. That survives every widget
/// rebuild and every scroll — and it bounds a wedged prompt to exactly the escape
/// the seam contract already documents: "until a session change".
final _authorizingRowProvider = NotifierProvider<_AuthorizingRow, String?>(
  _AuthorizingRow.new,
);

class _AuthorizingRow extends Notifier<String?> {
  @override
  String? build() {
    // The identity dependency IS the reset: a flip rebuilds this notifier, which
    // drops any latch the previous wallet's open bracket was holding. Without it
    // wallet B inherits A's wedge — and, on a rowid collision (`id` is a per-DB
    // rowid, so `1` on both is ordinary), B renders A's spinner.
    ref.watch(walletIdentityProvider);
    return null;
  }

  /// Take the section's single slot. `false` ⇒ another row holds it and this tap
  /// must do nothing at all (never a second bracket).
  bool claim(String key) {
    if (state != null) return false;
    state = key;
    return true;
  }

  /// Release the slot — only if `key` still holds it, so a late release from an
  /// abandoned run can never free a bracket a newer one legitimately owns.
  void release(String key) {
    if (state == key) state = null;
  }
}

class ParkedSendsSection extends ConsumerWidget {
  const ParkedSendsSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final parked = ref.watch(walletParkedSendsProvider);
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    return parked.when(
      // `skipLoadingOnReload` keeps the populated surface visible while an
      // invalidate (post-cancel / sync-edge / resume) re-pulls — without it the
      // section would BLINK invisible for a frame on every refresh. Only the COLD
      // first load shows the loading arm, which renders nothing (no header
      // flicker) — there is simply nothing pending to earmark yet.
      skipLoadingOnReload: true,
      loading: () => const SizedBox.shrink(),
      // A read failure is surfaced honestly (a parked send is money the user is
      // waiting on) — NEVER a silent-hide — AND recoverable in place. The home
      // has no pull-to-refresh, so without an inline retry a failed read stays a
      // dead "couldn't load" until a resume/sync edge happens to re-pull it — on
      // a stalled-sync flaky link that can be a long wait for money the user is
      // tracking (UX+reliability MED). liveRegion so a screen reader
      // ANNOUNCES the loading→error transition in place, matching the receive /
      // store-busy honest-degradation elements (a11y discipline).
      error: (_, _) {
        // While the tapped re-pull is in flight, `skipLoadingOnReload` keeps
        // THIS arm rendered and the AsyncValue reports isLoading — reflect it
        // (spinner + disabled) so a tap on a persistently-failing store is
        // never a MUTE no-op (review, converged UX+reliability MED): the
        // sighted user sees work happen, and the label swap below re-fires the
        // live region when the identical error re-lands (#407 R5 — the flag had
        // to move INSIDE for that second claim to be true).
        final refreshing = parked.isLoading;
        return Padding(
          padding: const EdgeInsets.only(bottom: 28),
          child: Semantics(
            container: true,
            liveRegion: true,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l10n.walletParkedError,
                  style: textTheme.bodyMedium?.copyWith(
                    color: colors.textMuted,
                  ),
                ),
                // Start-aligned by the Column's crossAxisAlignment (RTL-correct).
                TextButton(
                  key: const Key('parked-error-retry'),
                  // Re-pull the identity-scoped reader (the same provider the
                  // post-cancel / resume paths invalidate) → the section re-runs
                  // loading→data, recovering without a full app resume.
                  onPressed: refreshing
                      ? null
                      : () => ref.invalidate(walletParkedSendsReadProvider),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (refreshing) ...[
                        const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator.adaptive(
                            strokeWidth: 2,
                          ),
                        ),
                        const SizedBox(width: 8),
                      ],
                      // FLEXIBLE for the same reason the Send now label is
                      // (#401 R8a): a Row child gets unbounded width, so a long
                      // label at 3× scale on a 320dp phone overflows instead of
                      // wrapping. "Try again" is short in en and not in
                      // es/uk/de/nl/pl/pt — and this arm renders exactly when the
                      // user is already looking at an error.
                      //
                      // THE LIVE REGION LIVES HERE, NOT ON THE OUTER CONTAINER
                      // (#407 R5 — the #403 R1 shape, fourth site). The outer
                      // `Semantics(container: true, liveRegion: true)` announces
                      // the arm when it first appears, and that half works; what
                      // it CANNOT do is re-announce a retry, because its own
                      // SemanticsData is byte-identical across the refreshing
                      // flip — measured, while the comment above claimed the
                      // opposite in so many words. A flag on a node whose data
                      // never changes fires once, ever. Flagging the label that
                      // SWAPS is what makes the retry audible.
                      Flexible(
                        child: Semantics(
                          liveRegion: refreshing,
                          child: Text(
                            refreshing
                                ? l10n.walletParkedErrorRetryInProgress
                                : l10n.walletParkedErrorRetry,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        );
      },
      data: (sends) {
        if (sends.isEmpty) return const SizedBox.shrink();
        // The S12 look (S13 Build B): the Activity section's header, its lines
        // at the header's inset, and ONE group of rows with hairlines inset to
        // the rows' text.
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsetsDirectional.only(start: 4),
              child: Text(l10n.walletParkedTitle, style: textTheme.titleLarge),
            ),
            const SizedBox(height: 4),
            // THE EARMARK CLAIM IS NOT UNIVERSAL (#401 R2a). "Their amounts are
            // still part of your balance" is true of a `Queued` row and FALSE of a
            // claimed one: past the engine's create those notes are already
            // locally spent, so the amount has left the spendable balance. #400
            // recorded that conflict in the preparing hint's own description and
            // left the header asserting it anyway — three inconsistent statements
            // about one amount on one screen. Carve out only the preparing rows,
            // with the same hedge the row hint uses (the exact instant of the
            // create is not observable from here).
            Padding(
              padding: const EdgeInsetsDirectional.only(start: 4),
              child: Text(
                sends.any((s) => s.sending)
                    ? l10n.walletParkedSubtitlePreparing
                    : l10n.walletParkedSubtitle,
                style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
              ),
            ),
            // (the R1 honesty family): parked sends drain ONLY on sync
            // passes, so with none "haven't been sent YET" would quietly imply a
            // progress that cannot happen — say the pause plainly. Orange (a money
            // expectation is on hold), same muted size as the subtitle it
            // qualifies.
            //
            // Its OWN note, not the shared walletSyncOffMoneyNote (#401 R2d): this
            // is the one paused surface with a per-row escape, and the shared note
            // sits directly above a Send now that #400 R9 deliberately left working
            // at every custody tier. A user who reads "paused" and stops looking
            // has abandoned funds one tap would release.
            //
            // Keyed on the DRIVE (#401 R5), so a FAILED sync start — where passes
            // also never happen, while the host policy still reads on — gets the
            // same honest pause, phrased without naming a settings remedy that
            // would be wrong for it.
            //
            // …BUT ONLY WHILE THAT ESCAPE IS ON SCREEN (#403 R2). The whole
            // argument above is "name the per-row escape" — and Send now is
            // suppressed on every `sending` row, so a section whose rows are ALL
            // mid-signature showed a note naming a control that is not there, one
            // line above the row this file's own KNOWN RESIDUAL says has no
            // in-app exit. The shared note it replaced named no affordance, so
            // that case is a REGRESSION of the fix, not a pre-existing gap. Fall
            // back to the shared, affordance-free note: it is cause-agnostic and
            // already translated in all 16 locales, so this costs no new string.
            if (!ref.watch(walletSyncPassesRunProvider)) ...[
              const SizedBox(height: 4),
              Padding(
                padding: const EdgeInsetsDirectional.only(start: 4),
                child: Text(
                  sends.any((s) => !s.sending)
                      ? l10n.walletParkedSyncPausedNote
                      : l10n.walletSyncPausedMoneyNote,
                  key: const ValueKey('wallet-parked-sync-off'),
                  style: textTheme.bodySmall?.copyWith(color: colors.orange),
                ),
              ),
            ],
            const SizedBox(height: 12),
            WalletGroup(
              key: const ValueKey('wallet-parked-group'),
              dividerIndent: _ParkedRow.textInset,
              children: [
                for (final send in sends)
                  _ParkedRow(
                    key: ValueKey('${send.id}-${send.createdAt}'),
                    send: send,
                  ),
              ],
            ),
            // #315 slice 2: when ANY send is PAUSED, the one-time-address window
            // is likely bricked by leaked reservations — offer the account-level
            // "reopen sending" reclaim (the honest discovery point; the SDK
            // cannot tell a bricked window from a healthy one, so it is gated on
            // a real paused send, never shown speculatively).
            //
            // `&& !s.sending` (#400 R2 review): the CTA's own success copy tells
            // the user to "send the paused payment" afterwards, and a row that is
            // mid-signature renders no Send now button — so a `sending && paused`
            // row alone would offer a flow whose next step is not on screen.
            if (sends.any((s) => s.paused && !s.sending))
              const _ReopenSendingCta(),
            const SizedBox(height: 28),
          ],
        );
      },
    );
  }
}

/// The account-level "reopen one-time-address sending" affordance (#315 slice 2),
/// shown under the parked list when a send is PAUSED. Explains the stuck window
/// and runs the authorized, honest-cost-disclosed reclaim; disabled + spinnered
/// while a reclaim is in flight (a re-tap would burn a redundant fee).
class _ReopenSendingCta extends ConsumerWidget {
  const _ReopenSendingCta();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    final inFlight = ref.watch(reclaimInFlightProvider);
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      // #401 R3b, the same family: the reclaim's in-flight state is a spinner and
      // a label swap, both purely visual. It burns a real fee, so a re-tap is not
      // free — announce that it started.
      // ON THE LABEL, NOT THE COLUMN (#403 R1). Wrapping the Column made ONE node
      // that absorbed the explainer Text, so the node's label was the CONSTANT
      // explainer and never changed on the inFlight flip — a live region whose
      // content is static announces nothing. Inside the button, the flag and the
      // swapping label share the button's own node.
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.walletReclaimExplainer,
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
          const SizedBox(height: 6),
          ConstrainedBox(
            // Bound the CTA width so a long label at large text scale WRAPS inside
            // the button (the .icon label is Flexible) instead of overflowing the
            // section (review D, H9). Left-aligned by the Column's
            // crossAxisAlignment.start; compact when the label is short.
            constraints: const BoxConstraints(maxWidth: 400),
            child: FilledButton.tonalIcon(
              onPressed: inFlight
                  ? null
                  : () => confirmAndReclaim(context, ref),
              icon: inFlight
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                    )
                  : const WalletIcon(WalletGlyph.unlock, size: 18),
              label: Semantics(
                liveRegion: inFlight,
                child: Text(
                  inFlight
                      ? l10n.walletReclaimInProgress
                      : l10n.walletReclaimButton,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// One parked-send row: the committed amount + a Cancel button — and, on a PAUSED
/// row (#315: the wallet gave up auto-retrying a one-time-address send after its
/// capped attempts), a distinct "paused" label, an explanatory hint, and a Retry
/// button. The honesty split is money-relevant: a paused payment NEVER sends on
/// its own, so it must not read like a healthy "saved & pending" one (the user
/// would wait forever). Retry resumes the SAME intent (never a re-send — a fresh
/// send would double-pay and evade the retry cap), so it needs no confirm dialog;
/// Cancel is IRREVERSIBLE and keeps its confirm.
class _ParkedRow extends ConsumerWidget {
  const _ParkedRow({super.key, required this.send});

  final ParkedSend send;

  /// Where a row's text starts: the 16 padding, the 18 glyph, the 10 gap —
  /// the group's hairlines are inset to it.
  static const double textInset = 44;

  /// This row's identity in the section single-flight. The same `(id, createdAt)`
  /// pair the SDK verbs pin on, so the latch key cannot drift from the row it
  /// guards.
  String get _key => '${send.id}-${send.createdAt}';

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    // The section single-flight (#400 R7), watched so BOTH the owning row (which
    // shows progress) and every other row (which merely disables) rebuild on a
    // claim. Container-scoped + identity-keyed — see `_authorizingRowProvider`.
    final authorizingKey = ref.watch(_authorizingRowProvider);
    final authorizingThis = authorizingKey == _key;
    final authorizingAny = authorizingKey != null;
    // Whether a background sync PASS will run decides which "preparing" hint is
    // TRUE for this row (#400 R2 review): the wallet's own recovery for a
    // mid-signature row rides a COMPLETED sync pass, so with no passes "it returns
    // to the list on its own" is a promise the wallet cannot keep. Keyed on the
    // DRIVE, not on the host policy (#401 R5) — a FAILED sync start strands the row
    // exactly as a sync-off policy does, and that user was being told to wait for a
    // self-heal that could not come.
    final syncPassesRun = ref.watch(walletSyncPassesRunProvider);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    final amount = l10n.walletAmount(formatZec(send.amountZat));
    // Two parked rows with the SAME amount were indistinguishable — visually
    // AND as two identical "Cancel" semantics nodes (review; likelier
    // post-#331, where any two plain queued sends collide). The save time is
    // the honest discriminator; the activity rows' locale-aware absolute time
    // via the shared factory (#317 — relative time needs localized plural
    // strings, a follow-up).
    final time = walletCompactTimeFormat(l10n.localeName).format(
      DateTime.fromMillisecondsSinceEpoch(send.createdAt * 1000).toLocal(),
    );
    // At LARGE text scales the buttons starve the shared row to ~120px and
    // the save time — the whole point of the timed copy — never renders
    // (wrap UX MAJOR-2, measured with the real font at ru/2.0×/320dp).
    // Above 1.4× the buttons move UNDER the text so the amount AND time keep
    // the full width; at ordinary scales the compact single-row shape stays.
    // EVERY row stacks its buttons under the text now (#361): two actions never
    // fit beside the label at any scale on a narrow phone, and since FR-23-b the
    // healthy row carries two as well (Send now + Cancel), not one. Previously
    // only a paused row — or a large text scale — stacked.
    // One coherent screen-reader node for the icon + amount line (not an
    // icon fragment then a disconnected sentence a swipe apart).
    // THREE row states, and `sending` wins (#400 R2). A mid-signature row is being
    // worked on RIGHT NOW — or was, until the app was killed mid-proof — so
    // "preparing" is the true present tense; a `paused` bit alongside it only
    // governs the NEXT attempt and would read as a contradiction here.
    final label = MergeSemantics(
      child: Row(
        children: [
          WalletIcon(
            send.sending
                ? WalletGlyph.sending
                : (send.paused ? WalletGlyph.paused : WalletGlyph.scheduled),
            size: 18,
            color: colors.orange,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              send.sending
                  ? l10n.walletParkedRowPreparingTimed(amount, time)
                  : (send.paused
                        ? l10n.walletParkedRowPausedTimed(amount, time)
                        : l10n.walletParkedRowTimed(amount, time)),
              style: textTheme.bodyMedium,
              // Bounded lines before ellipsizing (UX review M2): the
              // figures that disambiguate two same-amount rows must survive
              // for sighted large-text users, not only in the semantics.
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
    final cancel = TextButton(
      // DISABLED while THIS row's bracket is open (#400 R4). The confirm dialog
      // behind it asserts "It hasn't been sent, so nothing leaves your wallet" —
      // which stops being true the moment the sign claims the row, and the sign is
      // running. Only this row: cancelling a DIFFERENT parked send meanwhile is
      // both safe and legitimate.
      onPressed: authorizingThis ? null : () => _confirmAndCancel(context, ref),
      // The visible label stays the short "Cancel"; the SEMANTIC label
      // binds the action to its amount AND save time, so same-amount rows
      // never read as identical bare "Cancel" buttons.
      child: Text(
        l10n.walletParkedCancel,
        semanticsLabel: l10n.walletParkedCancelSemanticTimed(amount, time),
      ),
    );
    // FR-23-b (#361): ONE user-paced "do it now" affordance on EVERY row, paused
    // or not — it signs the committed send inside the host's authorize-spend
    // bracket. At held custody (pass-through authorizer) it is a no-prompt "don't
    // wait for the next pass"; at HOST custody it is the only way the offline
    // queue drains at all (a background pass has no credential to sign with).
    //
    // ONE LABEL FOR BOTH ROW STATES (arch review M4). The first cut kept
    // "Retry" on a paused row — but that button now does strictly MORE than the
    // healthy row's (re-arm AND sign, inside a spend bracket) while carrying the
    // weaker verb, and its screen-reader label never said a payment would be
    // signed. At host custody it would raise a spend prompt the label never
    // promised. "Send now" is what the button does in both states.
    //
    // NOT OFFERED ON A MID-SIGNATURE ROW (#400 R2): every sign verb requires a
    // still-`Queued` row, so on a `sending` row this button could only ever answer
    // "not found" — the exact "affordance that can only fail" the package refuses
    // elsewhere. NOT gated on `walletOfflineQueueSupportedProvider` either (#400 R9,
    // deliberate): that flag governs whether the queue is OFFERED at the send form,
    // not whether an ALREADY-COMMITTED row may drain. A host can flip it to `false`
    // with rows already saved, and a committed send must keep both of its exits —
    // drain and cancel — or the user's money has nowhere to go.
    //
    // PROGRESS IS LOAD-BEARING, not decoration (#400 R4). At HELD custody (the
    // default pass-through authorizer) there is no host prompt to look at, and the
    // call runs an unbounded proving step — tens of seconds on a fragmented wallet.
    // A bare disabled button for that long reads as a dead app, and a user who
    // believes nothing happened re-enters the payment.
    // THE IN-FLIGHT CUE IS ANNOUNCED, not only drawn (#401 R3b). The whole
    // argument for the spinner is written above — a bare disabled button for tens
    // of seconds reads as a dead app, and a user who believes nothing happened
    // re-enters the payment. That double-pay path was closed only for SIGHTED
    // users: nothing in this arm was a live region, so a screen reader announced
    // the button's disable and then went silent for the entire proving step.
    // `liveRegion` only while THIS row is authorizing, so the section is not a
    // permanent announcer.
    // THE FLAG GOES ON THE LABEL, INSIDE THE BUTTON (#403 R1, MEASURED). #401 R3b
    // wrapped the BUTTON, and that announced nothing: `ButtonStyleButton` already
    // builds its own `Semantics(container: true)`, so an outer annotation cannot
    // merge into it — it forms a SEPARATE, permanently EMPTY parent node. The
    // engine announces updates to the flagged node, and the only thing that
    // changes is the label on the child, so the fold shipped the claim without the
    // behaviour: silent for the entire unbounded prove, i.e. the double-pay path
    // was still open for exactly the users it was reopened for. Annotated INSIDE,
    // the config merges into the button's own node and the flag and the changing
    // label are one node (probe: `live=true btn=true label="…"`).
    final sendNow = TextButton(
      onPressed: authorizingAny ? null : () => _sendNow(context, ref),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (authorizingThis) ...[
            const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator.adaptive(strokeWidth: 2),
            ),
            const SizedBox(width: 8),
          ],
          // FLEXIBLE, not a bare Text: the label now lives inside a Row (for the
          // spinner), and a Row child has unbounded width — at 3× scale on a
          // narrow phone that overflows the Wrap run instead of wrapping, which
          // is exactly the -H9 shape the reopen CTA already guards against.
          Flexible(
            child: Semantics(
              liveRegion: authorizingThis,
              child: Text(
                authorizingThis
                    ? l10n.walletParkedSendNowInProgress
                    : l10n.walletParkedSendNow,
                // THE BINDING SURVIVES THE IN-FLIGHT STATE (#401 R3c). A `null`
                // here falls back to the visible label — a bare "Sending…" — and
                // two rows with the same amount become two identical nodes again,
                // which is the exact collision the timed labels were added for
                // The row a user authorized must stay identifiable while
                // it is the one thing on screen that is happening.
                semanticsLabel: authorizingThis
                    ? l10n.walletParkedSendNowInProgressSemanticTimed(
                        amount,
                        time,
                      )
                    : l10n.walletParkedSendNowSemanticTimed(amount, time),
              ),
            ),
          ),
        ],
      ),
    );
    // The action strip. `Wrap` lets a run BREAK instead of overflowing when the
    // labels grow (review D, H9); `Align` is what actually right-anchors it
    // — a bare `WrapAlignment.end` inside a Column is a NO-OP, because a Column
    // passes LOOSE cross-axis constraints so the Wrap sizes to its content and
    // has nothing to distribute (arch review M2, measured: the buttons
    // rendered flush LEFT). Cancel is deliberately LAST: the destructive action
    // sits furthest from the thumb's resting edge, and it is the one with a
    // confirm dialog behind it.
    final actions = Align(
      alignment: AlignmentDirectional.centerEnd,
      child: Wrap(
        alignment: WrapAlignment.end,
        // BOTH axes (UX H6). 4dp put two OPPOSITE money actions a mis-tap
        // apart — and the first fix only widened the horizontal gap. At 320dp ×
        // 3× the labels cannot share a run, so the Wrap breaks and "Send now"
        // lands directly ABOVE "Cancel" at the old 4dp runSpacing. The risk is
        // asymmetric: aiming for Cancel and hitting Send now signs a payment
        // with NO confirm dialog; the reverse is protected by one.
        spacing: 8,
        runSpacing: 8,
        children: [
          // A mid-signature row offers no authorize affordance — EXCEPT while this
          // row's own bracket is what put it there (#400 R2 review). The SDK marks
          // the row `sending` at the atomic claim, which is BEFORE the proving
          // step, so any parked-list refresh during the tens of seconds this call
          // runs (an app resume, a sync edge) flips `send.sending` true underneath
          // an in-flight authorization. Dropping the button then would delete the
          // spinner and the "Sending…" label mid-flight and leave a dead-looking
          // row — precisely the state the cue exists to prevent.
          // "Send now" is NOT offered while a signing block stands (the
          // network upgrade, or a server that will not say which network it
          // is on): it would open the host's authorize-spend bracket (a
          // biometric prompt, at host custody) only to return the typed
          // refusal. Cancel stays — the intent is still the user's to discard.
          if ((!send.sending || authorizingThis) && send.signingBlock == null)
            sendNow,
          // CANCEL IS NOT OFFERED ON A MID-SIGNATURE ROW (#400 R2 review, converged
          // HIGH). `cancel_parked_send` is `Queued`-guarded, so on this row it can
          // only return `false` — and its confirm dialog first promises "It hasn't
          // been sent, so nothing leaves your wallet", which is exactly what a
          // claimed row cannot guarantee: past the engine's create the notes are
          // already locally spent. Offering it produces two contradictory
          // statements about the same money, the first of them possibly untrue.
          if (!send.sending) cancel,
        ],
      ),
    );
    return Padding(
      // A group row: 16 to the text's side, 8 to the actions'.
      padding: const EdgeInsetsDirectional.fromSTEB(16, 8, 8, 8),
      // STACK vs INLINE by MEASURED width, not by platform assumption (arch
      // review M3). Two actions never fit beside the figures on a narrow phone —
      // and since FR-23-b every row carries two — but this SDK ships desktop as a
      // first-class target, where an unconditional stack makes every row three
      // lines tall for content that fits on one. The money-safety property being
      // defended is that the amount AND save time keep the full row width when
      // they need it (wrap UX MAJOR-2), which the width/scale test captures
      // exactly.
      child: LayoutBuilder(
        builder: (context, constraints) {
          final stacked = walletRowStacksAction(context, constraints.maxWidth);
          // `sending` wins over `paused` here too — same reason as the label. The
          // sync-paused variant is not decoration (#400 R2 review): the wallet clears
          // a mid-signature row by re-queueing it on a COMPLETED sync pass, so
          // "it returns to the list on its own" is true only where passes happen.
          // Without them the row sits there indefinitely, and a user told to wait
          // for a self-heal that cannot come re-enters the payment — the double pay
          // this whole section exists to prevent.
          // Ironwood/NU6.3 (`ironwood-nu63-support.md` §3.2) + GRACE-1 (§4p
          // item 2): the drain skips EVERY queued row while signing is
          // refused, so the row must say why — and the two causes have
          // different next steps, so each has its own copy: "waiting for an
          // app update" ONLY for the network upgrade; "waiting for a server
          // that reports the network version — switch servers" for the
          // silent server (plus "check the device's date and time" when the
          // grace ran out on the clock). Ranked ABOVE `paused`: both mean
          // "this will not send on its own", but a signing block is the one
          // where the per-row Retry is useless, so its copy must be the one
          // shown. Below `sending`, which is the true present tense of a row
          // already being worked on.
          final hintText = send.sending
              ? (syncPassesRun
                    ? l10n.walletParkedPreparingHint
                    : l10n.walletParkedPreparingHintSyncPaused)
              : switch (send.signingBlock) {
                  SigningBlock_NetworkUpgrade() =>
                    l10n.walletParkedBlockedByNetworkUpgrade,
                  SigningBlock_GraceExpired(:final by) =>
                    by == GraceExpiry.clock
                        ? l10n.walletParkedBlockedByServerSilentClock
                        : l10n.walletParkedBlockedByServerSilent,
                  // Forward-compat: a block this UI does not know still says
                  // the row will not go on its own — never a healthy row.
                  SigningBlock_Unknown() =>
                    l10n.walletParkedBlockedByServerSilent,
                  null => send.paused ? l10n.walletParkedPausedHint : null,
                };
          final hint = hintText == null
              ? null
              : Padding(
                  // DIRECTIONAL (#400 R9): the 28dp indent aligns the hint under the
                  // label's text, past the leading icon — in an RTL locale (the
                  // package ships ar + he) a hard `left` puts it on the wrong side
                  // and breaks exactly that alignment.
                  padding: const EdgeInsetsDirectional.only(start: 28, top: 2),
                  child: Text(
                    hintText,
                    style: textTheme.bodySmall?.copyWith(
                      color: colors.textMuted,
                    ),
                  ),
                );
          if (stacked) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [label, ?hint, actions],
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(child: label),
                  actions,
                ],
              ),
              ?hint,
            ],
          );
        },
      ),
    );
  }

  /// FR-23-b (#361) — SEND THIS PARKED SEND NOW: sign the already-committed
  /// intent inside the host's authorize-spend bracket, instead of waiting for a
  /// background pass to sign it (which at HOST custody never can — no live user,
  /// no credential). Reversible-by-nature and explicitly requested, so no confirm
  /// dialog: the payment was confirmed when the user queued it, and this decides
  /// nothing new — it only moves WHEN the signature happens.
  ///
  /// A PAUSED row re-arms its retry budget first — INSIDE the bracket (the
  /// crypto audit fold; this doc said "outside" until #400 corrected it, which was
  /// wrong the moment the code moved). The re-arm moves no money and signs
  /// nothing, but without it a budget-capped row would authorize straight into
  /// "still waiting" (the drain's cap gate refuses before it ever reaches the
  /// seed) — and running it before the prompt meant a DENIED authorization still
  /// re-enabled the send for the unattended drain. Its `false` (row gone /
  /// changed) is deliberately NOT short-circuited on: the authorization re-reads
  /// the row and reports the truth through ONE outcome path.
  Future<void> _sendNow(BuildContext context, WidgetRef ref) async {
    final l10n = WalletLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    // Read the authorizer at TAP time, before any await — a post-await
    // `ref.read` sits in the disposed-ref danger zone, and a throw there would be
    // swallowed by the catch below and mislabel a SIGNED send as failed.
    final authorizer = ref.read(walletSendAuthorizerProvider);
    // Whether background sync PASSES will run — also at tap time and for the same
    // reason (#400 R1): it only picks between two success phrasings, so it must
    // never be the thing that turns a landed signature into "couldn't send it".
    // The DRIVE, not the host policy (#401 R5): a failed sync start leaves the
    // signed-but-unbroadcast residual with no `after_synced` to correct it, exactly
    // as a sync-off policy does.
    final syncPassesRun = ref.read(walletSyncPassesRunProvider);
    // The CONTAINER, captured at tap time (arch review H1). The bridge call
    // can run for tens of seconds (an unbounded ZK prove, plus a host prompt with
    // no timeout by contract), and the user may well navigate away meanwhile —
    // this row is then disposed and `ref` is dead. The container outlives it, so
    // a LANDED money outcome still refreshes the surfaces that must show it.
    final container = ProviderScope.containerOf(context, listen: false);
    // SECTION-level single-flight (#400 R7) — one open bracket for the whole list,
    // not one per row. `false` ⇒ another row owns it; do nothing at all.
    if (!container.read(_authorizingRowProvider.notifier).claim(_key)) return;
    // Whether THIS tap re-armed a paused row (#400 R5) — set inside the closure, so
    // a denial before the closure runs leaves it false.
    var rearmed = false;
    try {
      await _authorize(
        l10n: l10n,
        messenger: messenger,
        session: session,
        authorizer: authorizer,
        container: container,
        syncPassesRun: syncPassesRun,
        onRearm: () => rearmed = true,
        wasRearmed: () => rearmed,
      );
    } finally {
      // ALWAYS release, on every exit including the early returns and a throw — a
      // stuck latch disables "Send now" on every row until the identity changes.
      // Guarded: a whole-scope teardown makes the read throw, and in that case the
      // latch died with the container anyway.
      try {
        container.read(_authorizingRowProvider.notifier).release(_key);
      } catch (_) {}
    }
  }

  /// The body of [_sendNow], split out so the single-flight release above is a plain
  /// `finally` over ONE call instead of being repeated at five exits.
  Future<void> _authorize({
    required WalletLocalizations l10n,
    required ScaffoldMessengerState messenger,
    required WalletSession session,
    required WalletSendAuthorizer authorizer,
    required ProviderContainer container,
    required bool syncPassesRun,
    required void Function() onRearm,
    required bool Function() wasRearmed,
  }) async {
    String message;
    var leftTheSurface = false;
    // Which bridge calls the closure reached (R13 §4.4), each set IMMEDIATELY
    // before its call: the optional re-arm moves no money, and an error before
    // `authorizeParkedSend` was called cannot be over a saved transaction.
    var rearmCalled = false;
    var authorizeCalled = false;
    Object? authorizeThrew;
    // Whether the parked list was re-read to decide the message — that fetch
    // already primes the view, so the refresh below does not fetch it again.
    var reread = false;
    try {
      final outcome = await authorizer.authorizeSpend(
        WalletSpendIntent(
          kind: WalletSpendKind.queuedSend,
          // The typed PRINCIPAL (the amount the user entered) — the fee is
          // computed at signing time and is on top, exactly as the queuedSend
          // contract on WalletSpendIntent.amountZat states.
          amountZat: send.amountZat,
          // FR-17: THIS row's binding, so a bound host stages for the row it is
          // prompting about — a stage recorded for any other row is refused and
          // the send stays parked (funds safe), never wrongly signed.
          bindingToken: send.binding,
        ),
        () async {
          // The identity-switch fence, the same one every other spend
          // bracket in the package carries: a host wallet flip while the prompt
          // was open must NEVER let the approval land on the DEAD identity's
          // money — every visible surface already shows the new one, so the user
          // could not see what they just approved. This bracket has the widest
          // window of any (a prompt may sit open indefinitely, by contract).
          //
          // IDENTITY ONLY — no widget-lifetime leg (security HIGH-1). The
          // first cut also refused on `!mounted`, i.e. `State.mounted`, false as
          // soon as THIS ROW leaves the tree. The eight sibling sites read
          // `ref.mounted` on a Notifier's Ref, which is CONTAINER lifetime;
          // widget dispose does not trip it, so the two are NOT equivalent and
          // this site was strictly more trigger-happy than the surface it
          // claimed to mirror. The difference is not academic: a transient
          // parked-read failure swaps the section to its `error:` arm, which
          // builds no `_ParkedRow` at all — so a flaky store during an open
          // prompt disposed every row and turned the user's approval into a
          // SILENT refusal (`WalletSpendSessionChanged` shows nothing, by
          // contract) on a perfectly live identity. It also contradicted this
          // same handler, which captures `container` precisely so a LANDED
          // outcome survives dispose.
          //
          // `identical(session)` is the whole fence on its own — it is the exact
          // condition the seam contract states.
          //
          // A whole-scope teardown makes this `container.read` throw riverpod's
          // internal `StateError`. That is mapped to the DOCUMENTED type here
          // rather than left to propagate (#400 R2 review): the throw happens
          // inside the host's `authorizeSpend` action, and the seam contract says
          // "any OTHER throw is treated as a real failure and classified like a
          // signing error" — so leaking it would hand the host an untyped error
          // for a case where nothing was even attempted. This is the same reason
          // the eight sibling brackets carry a `!ref.mounted` leg; this site has
          // no container-lifetime `ref` to test, so it catches instead.
          final WalletSession? current;
          try {
            current = container.read(walletSessionProvider);
          } catch (_) {
            throw const WalletSpendSessionChanged();
          }
          if (!identical(current, session)) {
            throw const WalletSpendSessionChanged();
          }
          // The paused-row re-arm lives INSIDE the bracket (crypto audit
          // H3 / security MEDIUM-2). It moves no money itself — but it clears
          // the cap gate that was the ONLY thing keeping the row off the
          // unattended background drain, so running it BEFORE the prompt meant a
          // DENIED authorization still re-enabled the send at held custody. Now
          // a denial reaches this closure never, and the re-arm never happens.
          if (send.paused) {
            // Only a TRUE return re-armed anything (#400 R2 review). `false` means
            // the row was already un-paused or gone — and the outcome copy this
            // flag selects claims "it will be tried again", which would be a
            // statement about a re-arm that did not happen.
            rearmCalled = true;
            if (await session.retryParkedSend(
              id: send.id,
              createdAt: send.createdAt,
            )) {
              onRearm();
            }
          }
          authorizeCalled = true;
          // Recorded as it throws, so only the SDK's OWN error is graded by
          // [parkedAuthorizeErrorPrecedesPersistence] — never one the host
          // substituted after the spend ran.
          return session
              .authorizeParkedSend(id: send.id, createdAt: send.createdAt)
              .then(
                (outcome) => outcome,
                onError: (Object error, StackTrace stack) {
                  authorizeThrew = error;
                  Error.throwWithStackTrace(error, stack);
                },
              );
        },
      );
      leftTheSurface =
          outcome == ParkedAuthorization.signed ||
          outcome == ParkedAuthorization.notFound ||
          outcome == ParkedAuthorization.expired;
      message = switch (outcome) {
        // SIGNED, but the broadcast is a DETACHED best-effort kick with a bounded
        // retry (#400 R1). If every attempt misses, the durable re-send lives in
        // the drain — which runs only after a completed sync pass. Where no pass
        // will run there is no correction at all: say so. Everything else about the
        // outcome is identical, so the two strings differ only in that tail — and
        // the tail names the CONDITION, not a remedy, because the two ways passes
        // stop (the host's policy, a failed start) have different ones and the
        // screen already carries whichever applies (#401 R5).
        ParkedAuthorization.signed =>
          syncPassesRun
              ? l10n.walletParkedAuthorizeSent
              : l10n.walletParkedAuthorizeSentSyncPaused,
        // NOT a failure (the SDK contract): nothing was signed, nothing lost,
        // the row is untouched and still cancellable. Copy that says "failed"
        // here invites the user to re-enter the payment — a DOUBLE-PAY. The
        // `unknown` forward-compat arm folds here deliberately: it is the
        // money-SILENT direction (never claim a send, never invite a re-send).
        //
        // "saved and UNCHANGED" is false once the re-arm ran (#400 R5): the row
        // un-paused under the user's eyes — icon, label and hint all change, and
        // the reopen-sending prompt can disappear with it — so a paused row that
        // was re-armed gets the sibling string that says it will be tried again.
        ParkedAuthorization.stillQueued || ParkedAuthorization.unknown =>
          wasRearmed()
              ? l10n.walletParkedAuthorizeRearmed
              : l10n.walletParkedAuthorizeStillWaiting,
        // TWO OUTCOMES, TWO TRUTHS (#401 R2b). `expired` deleted the row, so
        // "isn't waiting anymore" is exactly right. `notFound` only means the row
        // is no longer QUEUED — and the commonest way that happens is the
        // background drain winning the claim, after which the SAME payment
        // re-renders as PREPARING two lines above. Telling the user a visible row
        // does not exist is how they conclude the payment was lost and re-enter
        // it. Both point at the surfaces and neither invites a re-send.
        ParkedAuthorization.expired => l10n.walletParkedRetryStale,
        ParkedAuthorization.notFound => l10n.walletParkedAlreadyInProgress,
      };
    } on WalletSpendAuthorizationDenied {
      // The host's own prompt IS the user-facing channel for a denial (the seam
      // contract) — show nothing, change nothing, when the decline came before
      // `authorizeParkedSend` was called: no signing call ran (the re-arm, if
      // any, moves no money). A host that ran the sign and THEN declined
      // (R13 §4.4) may be declining over a saved transaction — re-read the row.
      if (!authorizeCalled) return;
      final (text, left) = await _rereadAfterLostAnswer(
        l10n,
        container,
        rearmed: rearmCalled && wasRearmed(),
      );
      message = text;
      leftTheSurface = left;
      reread = true;
    } on WalletSpendSessionChanged {
      // The identity flipped mid-prompt and the fence refused: NOTHING was
      // signed, and the whole screen context already belongs to another wallet.
      // Show nothing (the type's own contract) — "payment failed" would be
      // untrue, and this row is not even the current identity's. Reported by
      // a host AFTER the sign ran (R13 §4.4), it is no longer "nothing" — the
      // row is re-read (and the delivery fence below still drops it if the
      // identity really flipped).
      if (!authorizeCalled) return;
      final (text, left) = await _rereadAfterLostAnswer(
        l10n,
        container,
        rearmed: rearmCalled && wasRearmed(),
      );
      message = text;
      leftTheSurface = left;
      reread = true;
    } catch (error) {
      // Before `authorizeParkedSend` (the fence, a throwing re-arm), or the
      // SDK's OWN typed refusal of a kind raised before signing
      // ([parkedAuthorizeErrorPrecedesPersistence]: busy, a closed handle, an
      // unstaged or wrong-row credential): honest "try again", the payment is
      // untouched — or, once a re-arm succeeded, the wording that says so
      // (#400 R5). Anything else may have arrived over a saved transaction
      // (R13 §4.4): re-read the row rather than claim "unchanged" on faith.
      final beforeSigning =
          !authorizeCalled ||
          (identical(error, authorizeThrew) &&
              parkedAuthorizeErrorPrecedesPersistence(error));
      if (beforeSigning) {
        message = rearmCalled && wasRearmed()
            ? l10n.walletParkedAuthorizeRearmed
            : l10n.walletParkedAuthorizeFailed;
      } else {
        final (text, left) = await _rereadAfterLostAnswer(
          l10n,
          container,
          rearmed: rearmCalled && wasRearmed(),
        );
        message = text;
        leftTheSurface = left;
        reread = true;
      }
    }
    // THE IDENTITY FENCE AGAIN, AT DELIVERY (#400 R6). The pre-action fence proves
    // the identity was live when the user approved; it says nothing about the tens
    // of seconds of proving that followed. If the host flipped wallets in that
    // window, this outcome belongs to a wallet that is no longer on screen: showing
    // "Sending your payment now." over wallet B is a claim about B that is false,
    // and on a duress/decoy flip it DISCLOSES that the hidden wallet had a payment
    // in flight. Deliver nothing — the same rule `send_authorization.dart` states
    // for its own bracket ("its result screen is deliberately discarded"). The old
    // identity's surfaces refresh when the user returns to it.
    //
    // Guarded because a whole-scope teardown makes the read throw; that is the same
    // "nothing to deliver to" case.
    try {
      if (!identical(container.read(walletSessionProvider), session)) return;
    } catch (_) {
      return;
    }
    // DELIVERED EVEN IF THIS ROW IS GONE (arch review H1). The messenger and
    // container were captured at tap time and outlive the widget, so a sign that
    // LANDED while the user was on another screen still tells them and still
    // refreshes the surfaces — the alternative was a signed, broadcasting payment
    // with no confirmation and no activity row until the next sync edge (the #309
    // H1 window).
    //
    // Which surfaces: the parked read always changes shape. On a SIGNED outcome
    // the send became a real transaction, so the in-flight cue (a two-step) and
    // the activity list (a single-step, already persisted by the sign) must
    // refresh in the same instant. `notFound`/`expired` get activity too — the
    // copy for them says "check your pending payments and activity", and the row
    // left this surface for exactly that one (the cancel path's `drainWonRace`
    // rule, same shape). `stillQueued` does NOT: an invalidate blanks activity to
    // its cold loading arm and resets pagination, which an unchanged list must
    // not pay for (#318).
    // After an R13 re-read the parked list was just fetched — that fetch
    // decided the message and primes the view, so it is not fetched again.
    if (!reread) container.invalidate(walletParkedSendsReadProvider);
    container.invalidate(walletInFlightSendsReadProvider);
    if (leftTheSurface) container.invalidate(walletActivityProvider);
    // `mounted` guard on the MESSENGER too (#400 R6): an invalidate on a live
    // container is safe whatever happened to the widget tree, but showing a snackbar
    // through a ScaffoldMessengerState whose element is gone throws — and it would
    // throw AFTER the money already moved, out of any handler that could report it.
    if (messenger.mounted) {
      messenger.showSnackBar(SnackBar(content: Text(message)));
    }
  }

  /// "Send now" lost its answer after `authorizeParkedSend` was called (R13
  /// §4.4): the error may have arrived over a saved transaction (`drain_multi`
  /// commits the create before `mark_sent_multi` and `read_raw_tx`), so the
  /// row itself is asked. ONE fetch through the parked reader — invalidated,
  /// then its `.future` read — decides the message and primes the view.
  ///
  /// The row matched by `(id, createdAt)`, never the id alone (rowids are
  /// reused), present and not `sending` → it IS unchanged (or re-armed, when
  /// this tap's re-arm succeeded). Absent, `sending`, or the read failing →
  /// [WalletLocalizations.walletParkedAlreadyInProgress], which never invites
  /// a re-send, with the activity refresh (the second value).
  Future<(String, bool)> _rereadAfterLostAnswer(
    WalletLocalizations l10n,
    ProviderContainer container, {
    required bool rearmed,
  }) async {
    var stillWaiting = false;
    try {
      container.invalidate(walletParkedSendsReadProvider);
      final reader = walletParkedSendsReadProvider(
        container.read(walletIdentityProvider),
      );
      // Held open across the await so the auto-dispose reader is not torn
      // down mid-fetch when no view is watching it.
      final subscription = container.listen(reader.future, (_, _) {});
      try {
        final rows = await subscription.read();
        stillWaiting = rows.any(
          (row) =>
              row.id == send.id &&
              row.createdAt == send.createdAt &&
              !row.sending,
        );
      } finally {
        subscription.close();
      }
    } catch (_) {
      stillWaiting = false;
    }
    if (stillWaiting) {
      return (
        rearmed
            ? l10n.walletParkedAuthorizeRearmed
            : l10n.walletParkedAuthorizeFailed,
        false,
      );
    }
    return (l10n.walletParkedAlreadyInProgress, true);
  }

  Future<void> _confirmAndCancel(BuildContext context, WidgetRef ref) async {
    final l10n = WalletLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    // Destructive (stage S11 C3): "Discard" is red text, never a filled
    // button, so the safe "Keep" is not out-shouted by the discard.
    final confirmed = await showWalletConfirm(
      context,
      title: l10n.walletParkedCancelConfirmTitle,
      body: l10n.walletParkedCancelConfirmBody,
      cancelLabel: l10n.walletParkedCancelConfirmKeep,
      confirmLabel: l10n.walletParkedCancelConfirmDiscard,
      kind: WalletConfirmKind.destructive,
    );
    if (!confirmed) return;
    final session = ref.read(walletSessionProvider);
    if (session == null) return;

    String message;
    var drainWonRace = false;
    try {
      final removed = await session.cancelParkedSend(
        id: send.id,
        createdAt: send.createdAt,
      );
      // `false` is NOT "cancelled": the send may already be on its way (once the
      // multi-step path is enabled). NEVER imply a safe re-send — point the user
      // at their activity list (the DOUBLE-PAY contract).
      drainWonRace = !removed;
      message = removed
          ? l10n.walletParkedCancelDone
          : l10n.walletParkedCancelAlreadySending;
    } catch (_) {
      // A typed WalletApiError (e.g. busy/closed handle) — honest "try again",
      // the queued send is untouched.
      message = l10n.walletParkedCancelFailed;
    }
    // The widget may have been disposed during the await (the session closed, the
    // screen was replaced). Guard before touching `ref`/messenger: the durable
    // state is already correct, so there's nothing to show — and a post-dispose
    // `ref` throw must NEVER be caught above and mislabel a success as a failure.
    if (!context.mounted) return;
    // Refresh the surface either way: the row may now be gone, or it began
    // sending and left the parked set. On a `false` (the drain WON the race) the
    // send just moved parked → IN-FLIGHT — so the in-flight cue must refresh in
    // the same instant, or the send the user just tried to discard shows on
    // ZERO surfaces until the next sync edge (#309 holistic H1). The ACTIVITY
    // list refreshes too (review F3): a drained SINGLE-STEP send is not
    // in-flight-listed (that cue is two-step-only), so for it the activity row
    // — already persisted by the drain — is the surface the "check your
    // activity" copy points at. Activity ONLY on the lost race (wrap
    // review): an invalidate blanks the list to its cold loading arm and
    // resets pagination, so an ORDINARY cancel — which changes no activity
    // row — must not pay that; the in-place first-page refresh is owed (#318).
    ref.invalidate(walletParkedSendsReadProvider);
    ref.invalidate(walletInFlightSendsReadProvider);
    if (drainWonRace) ref.invalidate(walletActivityProvider);
    messenger.showSnackBar(SnackBar(content: Text(message)));
  }
}
