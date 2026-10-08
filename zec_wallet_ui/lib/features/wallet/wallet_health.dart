import 'package:zec_wallet/zec_wallet.dart';

/// Traffic-light health level for the persistent wallet status badge
/// (ADR-0533, spec §3.3). The single product signal a glance needs: can I
/// trust the balance and transact right now?
///
///  - [ok] (GREEN): operations available and the balance is fresh — fully
///    synced, OR usable via spend-before-sync while the historical backfill
///    runs in the background (the backfill only makes the balance grow as old
///    notes are found, never stale).
///  - [caution] (YELLOW): usable with a caveat — sync still catching up to the
///    user's funds (operations limited), connecting/starting, offline (queued
///    sends are normal), the network/server unreachable (#399 — the same
///    normal-offline family; see below), a self-healing reorg, or a stale
///    balance after a failed cold refresh.
///  - [error] (RED): cannot sync — a hard stall that needs the user (Tor
///    required-but-down, storage full, a local store fault).
enum WalletBadgeLevel { ok, caution, error }

/// Derive the badge level from the live [SyncStatus] and whether the last cold
/// snapshot refresh failed ([stale]). PURE and total — the gate-8 truth table
/// pins every arm; never throws. A forward-compat / unknown status is
/// [WalletBadgeLevel.caution] (the honesty rule: never GREEN or RED for a state
/// this build cannot interpret, mirroring the Tor-unknown treatment).
///
/// Deliberately does NOT take the spendable AMOUNT: the SDK's `spendableReady`
/// hint on the `Scanning` arm is the authoritative spend-before-sync signal
/// (both derive from the same ZIP-315 policy, `account.rs::spendable_policy`),
/// and a fully-synced empty wallet is still healthy (GREEN) with a zero
/// balance. The Send gate reads the amount separately (`wallet_screen`).
///
/// [startFailed] = the sync-drive START command failed (#356-F8): the status
/// stream retains the LAST GOOD value across a pause, so a resume-time start
/// fault can sit under a retained `UpToDate` — a GREEN badge there would claim
/// "fresh and syncing" while nothing is checking the chain, right beside the
/// orange retry notice. A failed drive caps an otherwise-GREEN level at
/// caution; it never *lowers* a caution/error (those are already honest).
/// [syncDisabled] = the HOST's sync policy is off (#383 R1,
/// `WalletSyncDrive.disabledByHost`): sync-off OWNS the level — always
/// caution, in BOTH directions. A retained `UpToDate` must not
/// read GREEN ("fresh") while nothing checks the chain; and a retained
/// `Stalled` must not read RED: the stall is not live (nothing is retrying
/// under a deliberate off — it re-surfaces when sync resumes), and an error
/// tint under the calm "Sync off" headline would contradict the copy a
/// screen reader hears (the two channels must agree; the presentation's
/// syncDisabled arm owns the words the same way). Caution is the one honest
/// tone: usable, deliberately not fresh, nothing to fix here but a setting.
WalletBadgeLevel walletBadgeLevel({
  required SyncStatus status,
  required bool stale,
  bool startFailed = false,
  bool syncDisabled = false,
}) {
  if (syncDisabled) return WalletBadgeLevel.caution;
  final level = _statusLevel(status: status, stale: stale);
  if (startFailed && level == WalletBadgeLevel.ok) {
    return WalletBadgeLevel.caution;
  }
  return level;
}

WalletBadgeLevel _statusLevel({
  required SyncStatus status,
  required bool stale,
}) {
  switch (status) {
    case SyncStatus_UpToDate():
      // Fully synced. A failed refresh on top of it means the figures may be
      // out of date → caution (maintainer: "yellow if the balance is stale").
      return stale ? WalletBadgeLevel.caution : WalletBadgeLevel.ok;
    case SyncStatus_UpToDateLimited():
      // Scanned to the tip, but this version could not read every block it
      // passed (`ironwood-nu63-support.md` §6.4). NEVER ok, stale or not: the
      // balance beside this badge is a FLOOR, not a total, and sending is
      // separately refused. Green here would be the badge telling the user
      // everything is fine while the wallet cannot transact — the §0 silence
      // wearing the health surface.
      return WalletBadgeLevel.caution;
    case SyncStatus_UpToDateDegraded():
      // T0-1b: scanned to the tip, but THIS SERVER refused, withheld or lied
      // about a pool's subtree roots. The same argument as the arm above: the
      // balance beside this badge is a FLOOR for that pool and funds received
      // there cannot be spent through this server, so never ok — and not error
      // either, since the wallet is otherwise usable and the fix is a server
      // switch, not a repair.
      return WalletBadgeLevel.caution;
    case SyncStatus_EndpointBehind():
      // T0-1c: scanned to THIS SERVER's tip, and the server is behind the
      // network. The balance beside this badge is current only as of a block
      // the network passed before this build shipped, and a send built against
      // that tip may not go through — so never ok. Not error either: the wallet
      // is usable at that height, the pass completed over a live link, and the
      // fix is a server switch, not a repair.
      return WalletBadgeLevel.caution;
    case SyncStatus_UpToDateUnverified():
      // GRACE-1 (§4p): scanned to the tip, but THIS SERVER will not say which
      // network it is on. Never ok: while the grace runs, sending is on a
      // countdown the badge must not paint green over; once it has ended,
      // sending is refused. Not error either — the pass completed over a live
      // link and the fix is a server switch (or the device clock), not a
      // repair. Under a reported rewinding streak the claim carries
      // `streakReported` and the sheet drops "your balance is current"
      // (P3-12); the LEVEL stays caution by P2-6's ranking (the grace wins),
      // although the streak it outranks maps to error below — whether the
      // composite should read error is the maintainer's call (security
      // review, LOW), not this arm's.
      return WalletBadgeLevel.caution;
    case SyncStatus_Scanning(:final spendableReady):
      if (stale) return WalletBadgeLevel.caution;
      // Spend-before-sync: once near-tip notes are spendable the wallet is
      // usable and fresh → GREEN even while history backfills. Until then
      // operations are limited (can't send yet) → YELLOW.
      return spendableReady ? WalletBadgeLevel.ok : WalletBadgeLevel.caution;
    case SyncStatus_Connecting():
    case SyncStatus_Idle():
    case SyncStatus_Offline():
      // Transient or offline-first: usable with a caveat, never "broken".
      // Offline keeps the last-known balance and queued sends are a normal
      // state (invariant 4), not an error.
      return WalletBadgeLevel.caution;
    case SyncStatus_Stalled(:final reason):
      // A chain reorg is transient and self-heals with no user action → YELLOW.
      //
      // #399: an unreachable endpoint is YELLOW too —
      // the normal-offline family, not an error. The core can only prove "the
      // dial failed"; it cannot tell airplane mode (a deliberate, routine act)
      // from a down server (a REFUSED dial even proves the device's own path
      // works — the FR-21 diagnostic rank), so the spec's §3.2a row calls this
      // arm "(normal offline)" and it is the deliberate lowest-claim fallback
      // for unmapped transient faults. RED here rendered every airplane-mode
      // flight as a hard error beside honest "queued sends are normal" copy —
      // and FR-21's sub-second verdicts made the false alarm near-instant.
      // Caution is the lowest-claim tone (the Unknown-arm honesty rule):
      // usable, last-known balance, retries automatically.
      //
      // Every other stall needs the user / cannot sync → RED (honest
      // degradation, invariant 6 — the badge carries a next step). That
      // DELIBERATELY includes torUnavailable: fail-closed privacy down is
      // never calm — so a Tor-REQUIRED wallet in airplane mode reads RED
      // (the SDK cannot verify the privacy path, and must not soothe there).
      //
      // R10: storageUnavailable is YELLOW — a busy or briefly unreadable store
      // is transient, the store is intact, the SDK retries, and the loop only
      // publishes it at the second local fault before a pass completes.
      // `internal` (a corrupt store, "restore") stays RED.
      return reason == StallReason.chainReorg ||
              reason == StallReason.endpointUnreachable ||
              reason == StallReason.storageUnavailable
          ? WalletBadgeLevel.caution
          : WalletBadgeLevel.error;
    case SyncStatus_Unknown():
      return WalletBadgeLevel.caution;
  }
}
