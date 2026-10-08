import '../recoverable_ephemeral.dart' show clampRecoverableToTransparent;
import '../transparent_funds/transparent_funds_providers.dart'
    show walletAutoShieldFloorZat;

/// The public balance the wallet's shield could take after a Move to public
/// (stage S14, `docs/plan/stage-14-move-below-the-shield-floor.md` §3).
///
/// Move lands on external index 0, the shield's own source, so what can be
/// shielded back afterwards is what is already public there plus [movedZat].
/// [recoverableZat] (funds on one-time addresses) is a SUBSET of
/// [transparentZat] that the shield never takes, so it comes off first,
/// clamped as the balance card clamps it. A still-loading snapshot
/// ([transparentZat] null) counts as nothing public, which errs toward the
/// warning. A recoverable list not yet read, or failed with no earlier read
/// ([recoverableZat] null), counts as NO one-time funds: that is what the
/// auto-shield loop assumes too (it gates on the raw public balance), so the
/// review and the loop agree on whether a sweep comes. Counting it all
/// off-source instead would hide a TRUE "shielded back automatically" (fold
/// review). A warm failure keeps the last good list (Riverpod's
/// previous value) until a read succeeds again. PURE, unit-tested without a
/// device.
///
/// It is the figure once what is in flight has confirmed: [movedZat] is itself
/// unconfirmed, and so is any public deposit still short of its confirmations
/// (`transparentZat` is the engine's unshielded TOTAL, pending included). Known
/// gaps (plan §3), each stated with what the user loses:
/// - funds on a swap-refund address (index ≥ 1) count in [transparentZat] but
///   are no shield source: a wallet already holding a failed swap's refund can
///   miss a warning it should have seen;
/// - a pending public deposit that never confirms (evicted, reorged out) was
///   counted: the same missed warning, on that rarer path;
/// - one-time funds while their list is unread or failed with no earlier read
///   are counted: the same missed warning on a wallet that holds such funds,
///   and the review can promise a sweep the core then declines.
int movePublicAfterZat({
  required int? transparentZat,
  required int? recoverableZat,
  required int movedZat,
}) {
  final transparent = transparentZat ?? 0;
  final offSource = clampRecoverableToTransparent(
    recoverableZat ?? 0,
    transparent,
  );
  return transparent - offSource + movedZat;
}

/// The public balance after the move as the auto-shield LOOP reads it: the
/// snapshot's raw `transparentZat` (unknown counts as nothing) plus the
/// amount, with no one-time-address subset taken off
/// (`auto_shield_controller.dart`, its threshold gate).
int moveRawPublicAfterZat({
  required int? transparentZat,
  required int movedZat,
}) => (transparentZat ?? 0) + movedZat;

/// Whether [publicAfterZat] is under the core's shield floor
/// (`SHIELDING_THRESHOLD_ZAT`, mirrored as [walletAutoShieldFloorZat]): no
/// Shield, manual or automatic, can take it until more arrives.
bool moveLeavesPublicBelowFloor(int publicAfterZat) =>
    publicAfterZat < walletAutoShieldFloorZat;

/// Whether the Move review may promise "shielded back automatically": the loop
/// is effectively on ([autoShieldEffective]); it fires, which it decides on the
/// RAW public balance after the move ([rawPublicAfterZat] = the snapshot's
/// `transparentZat` + the amount, `auto_shield_controller.dart`) against the
/// host's threshold ([hostThresholdZat]); and the shield it proposes can take
/// the funds, which needs [publicAfterZat] at or over the core floor. Power-save
/// only DEFERS the loop, so the promise stays true there.
bool moveReviewPromisesAutoShield({
  required int publicAfterZat,
  required int rawPublicAfterZat,
  required bool autoShieldEffective,
  required int hostThresholdZat,
}) =>
    autoShieldEffective &&
    !moveLeavesPublicBelowFloor(publicAfterZat) &&
    rawPublicAfterZat >= hostThresholdZat;
