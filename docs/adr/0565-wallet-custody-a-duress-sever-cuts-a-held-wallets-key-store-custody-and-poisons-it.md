# 0565 — Wallet custody: a duress sever cuts a held wallet's key-store custody and poisons every instance of it

- **Status:** Accepted (stage S16 `sever` + `poison`, S308, 2026-09-28). The core is
  built and reviewed on the branch `wf/s16/core/adj`. The bridge (`severCustody`, ABI 7,
  since S5 takes 6) is owed. · 2026-09-29: its residuals list, and the bridge it named as
  owed, are amended by ADR-0566 (the bridge is built, ABI 7).
- **Date:** 2026-09-28
- **Links:** FR-53 (`docs/handoff/host-feature-requests.md`) · the contract
  `docs/plan/stage-16-the-duress-force-sever.md` (§3.1, §3.2; §8 carries the review
  tables) · ADR-0563 (the bounded key-store worker this reuses) · `wallet-sdk.md`, the
  wipe API's S306 correction and §4.3a · Relim `censorship.md` §1 D14 (the 10 s duress
  budget)

## Context

`Wallet::wipe` takes the store lock first, so any live holder turns it into a
`WalletOpen` refusal and nothing is severed. `close()` frees the lock except in one
bounded residual: a holder past `QUIESCE_MAX` (30 s). Relim's duress wipe has 10 s. Under
the SDK's documented order, a straggler therefore meant the keychain wrap key survived a
duress wipe. Relim's workaround deleted `db_dir` first, so the wipe took its lock on a fresh
inode beside the live connection. The SDK spec calls that fallback WRONG. The founder put
FR-53 on the path to 0.0.1 at S307, and at S308 ruled that the poison also stops an
in-flight broadcast ("Yes, stop it").

## Decision

`Wallet::sever_custody(db_dir, deadline) -> Result<SeverReport, WalletError>`:

1. **Try the lock, never wait.** If it is free, the verb is the ordinary wipe (key store
   first, then the files) and reports `files: Removed`, `holder: None`. If it is held, the
   verb severs the key-store custody anyway, deletes NO file, and reports
   `files: LeftForHost`.
2. **One process-wide path table** (`PATHS`) records each path's live instances, its
   openers and a tombstone. Every CREATING key-store call checks the tombstone INSIDE its
   job on the one FIFO key-store worker. Deletes and reads are exempt. So no custody write
   that runs after the sever began can land. Provisioning writes its index before its key.
3. **Poison:** every live instance moves to the existing terminal phase `Wiped`. A birth
   gate refuses any new instance at a tombstoned path (open, create, rescan, switch), and an
   open also refuses early, before the store is touched. The rescan maps any error at a
   tombstoned path to `Wiped`. `broadcast_one` checks the phase at its top and again before
   `send_transaction`.
4. **The report never overclaims.**
   - `Severed { count }` requires a non-zero count.
   - A locked key store gets a BLIND delete scoped to this wallet's namespaces, reported as
     `SeveredUnproven { CountUnreadable }`. That path tombstones the path itself on both
     branches, and removes no file.
   - Every other custody outcome is `NotSevered { cause }`, returned inside the deadline.
     The deadline bounds the key-store budget as the earlier of it and
     `KEYCHAIN_WIPE_BUDGET`.
5. **Once the sever begins, it logs nothing.** One field-free `wallet.sever` line is emitted
   before the lock try. The verb's blocking body runs with no subscriber on its thread. A
   refused broadcast and a send on a wiped wallet log nothing.

## Alternatives considered

- **Wait for the holder, or poll the lock.** The duress budget cannot wait out
  `QUIESCE_MAX`, and a wedged holder never lets go.
- **Delete the directory under the live connection** (Relim's workaround). It buys secrecy
  with integrity, and a fresh inode lets a new open take a "free" lock beside the old
  connection.
- **A per-path gate `RwLock`, a `WriteTicket`, a `BroadcastMiss` type** (design revision
  3). Withdrawn: the one FIFO worker already orders every custody write against the sever's
  own calls, and a check inside the job is the whole gate.
- **Per-namespace worker queues or a priority lane**, so another wallet's wedge cannot
  delay a sever. Rejected: it undoes ADR-0563's bound of one native call in flight on one
  thread, and a wedged native call cannot be cancelled.
- **Keep a post-drain re-check** in the rescan and switch. Removed at the fold: no test
  could isolate it, because the birth gate and the rescan's error mapping each cover its
  window.

## Consequences — the residuals, stated so none is mistaken for closed

1. **At rest only.** A severed wallet's live holder still holds the seed and the DB key in
   RAM until it drops. A host that must end that exits the process, as Relim does.
2. **The blind delete trusts an unproven header.** On a locked device the header's
   namespace cannot be proven (the proof needs the key store). Someone with write access to
   this app's sandbox could point it at another wallet of the SAME app, and a locked-phone
   sever would delete that wallet's key too. No other app can do this.
3. **The shared worker.** Every wallet in the process and the custody selftest share one
   key-store worker. A call from any of them that is in flight or abandoned makes the sever
   answer `notSevered(busy|timeout)` inside its deadline. A retry succeeds only once THAT
   call returns. Relim runs one wallet.
4. **Another process's racing write** runs on its own worker and is not checked. The report
   says `otherProcess`. Relim has no second process.
5. **The tombstone is in-process only.** After a restart, an open at a severed directory the
   host has not purged answers the store's custody error, not `wiped`. A plain `wipe` is the
   remedy, as it is before any re-create in the same process.
6. **One OS-log line in flight.** The pre-sever `wallet.sever` line may still sit in the
   platform log buffer (logcat or os_log) after the host's own ring is wiped.
7. **The owed log exceptions, a BLOCKING dependency before Relim's duress build ships.**
   Two kinds of line are outside the silenced thread:
   - lines on the key-store WORKER thread: the `wallet.vault_call` `late` line when a call
     the sever abandoned at its deadline finishes afterwards (the likely case under a tight
     duress budget), and a panicking job's line;
   - a poisoned instance's own later lines on other threads (a rescan's or switch's `wiped`
     outcome, a close).

   S5's `device_log::quiesce_for_sever()` closes them. Whichever of S5 and S16 lands second
   wires it at the sever's entry.
8. **A rescan poisoned mid-drain still rebuilds `wallet.db` first.** The rebuilt file is
   SQLCipher ciphertext under the same DB key, dead once custody is purged. No file is
   deleted on the held branch.
9. **Owed:** the `locked-delete` device measurement (does `SecItemDelete` on class-A items
   succeed on a locked iPhone?). If it cannot, the spec carries "a phone that locks during
   the panic gesture may keep the wallet key until next unlock". Also owed: the bridge
   (ABI 7), the README lifecycle row and its four host obligations, and a founder question:
   does a non-Relim host need a policy switch (fail closed on a locked key store; keep
   post-sever diagnostics)?
