import 'package:zec_wallet/zec_wallet.dart';

/// The aggregate of the wallet's recoverable one-time-address funds, reduced for
/// the balance card's subset note (2e-2b-iv).
///
/// [totalZat] is the summed [RecoverableEphemeralFunds.recoverableZat]; [allFinal]
/// is true only when EVERY entry is reorg-final.
typedef RecoverableSummary = ({int totalZat, bool allFinal});

/// Reduce the per-address [RecoverableEphemeralFunds] list to the balance row's
/// aggregate. PURE + total — unit-tested without a device.
///
/// CONTRACT — NEVER ADDITIVE. Each [RecoverableEphemeralFunds.recoverableZat] is a
/// SUBSET of `BalanceSnapshot.transparentZat`/`.totalZat` (the engine already folds
/// these strand/return outputs into the displayed balance). So [totalZat] is
/// rendered as "X OF your balance is on a one-time address", NEVER as "+X
/// recoverable" — adding it to the balance would over-count holdings ~2×.
/// Cause-agnostic by design (an entry is an exchange RETURN as much as an expired
/// transfer), so the copy stays locational, never "stranded"/"bounced".
///
/// [allFinal] folds the per-entry `isFinal` weld: any still-confirming portion
/// makes it false, so the row renders "still confirming" rather than implying the
/// whole amount is settled/ready (conservative — matches the SDK's weld, which is
/// relative to the last-synced tip). For an empty list [totalZat] is 0 (the row is
/// hidden) and [allFinal] is vacuously true.
RecoverableSummary summarizeRecoverable(List<RecoverableEphemeralFunds> funds) {
  var totalZat = 0;
  var allFinal = true;
  for (final f in funds) {
    totalZat += f.recoverableZat;
    if (!f.isFinal) allFinal = false;
  }
  return (totalZat: totalZat, allFinal: allFinal);
}

/// The recoverable amount CLAMPED to the transparent balance it is a subset of.
/// The two figures come from independent providers that can momentarily skew
/// (the list re-pulled at a newer tip, or an action that dropped
/// `transparentZat` first), so every reader clamps: the subset invariant then
/// holds by construction. The balance card's note and the Move review's
/// shield-floor figure (S14) both read it here — one rule, one copy.
int clampRecoverableToTransparent(int recoverableZat, int transparentZat) =>
    recoverableZat < transparentZat ? recoverableZat : transparentZat;
