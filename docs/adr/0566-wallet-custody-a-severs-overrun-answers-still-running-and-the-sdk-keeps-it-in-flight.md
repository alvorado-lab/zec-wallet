# 0566 — Wallet custody: a sever's overrun answers "still running", and the SDK keeps it in flight

- **Status:** Accepted (stage S16 `bridge`, the bridge diff review's fold, S308, 2026-09-28;
  the coordinator's ruling plus Relim's `stillInUse` addition). **Amends ADR-0565's residual
  list** with the residuals below; ADR-0565 itself is unchanged.
- **Date:** 2026-09-28
- **Links:** ADR-0565 (the duress sever and its residuals) · FR-53
  (`docs/handoff/host-feature-requests.md`) · `docs/plan/stage-16-the-duress-force-sever.md`
  §3.3 (Relim's four asks; the overrun) · `sdk/zec_wallet/rust/src/convert.rs` `answer_by`

## Context

Relim asked at sync point 1 for a HARD wall-clock bound on the whole `severCustody` call. The
core bounds each key-store call by the deadline, but not a stall outside it (a slow
filesystem, a saturated blocking pool). So the bridge takes the answer by a fixed time after
the deadline. The core call it stops waiting for runs on the blocking pool and cannot be
cancelled: it runs to its own end, and it may still be sweeping files in `dbDir`. The first
overrun answer was `notSevered(timeout)` with `files: leftForHost`. It read like a finished
sever, and `leftForHost` is the value every host purges on. A host that obeyed it would purge
or re-create at a directory the SDK was still deleting in. The bridge diff review found this
three times over, on one seam.

## Decision

1. **The bound:** the bridge answers `severCustody` by `deadlineMs + SEVER_ANSWER_GRACE_MS`
   (250 ms), whatever stalls. The grace crosses to Dart as `severAnswerGraceMs()`.
2. **The overrun answer is its own:** `notSevered(stillRunning)` with `files: stillInUse` and
   `holder: unknown`.
   - `StillRunning` and `StillInUse` are bridge-only variants that only the overrun produces.
     `extraction_policy::sever_bridge_only_variants_are_declared` pins them as declared.
   - The core's `timeout` keeps its one meaning (a key store that did not answer).
   - `leftForHost` stays the ONE value a host purges on. The host never purges on
     `stillInUse`.
   - `HolderSeen::Unknown` carries two documented meanings: forward compatibility, or the
     `stillRunning` answer, which carries no holder (the running sever reads it later,
     unreported).
3. **A process-wide in-flight set:** the core call is spawned as its own task, and its `dbDir`
   stays in the set until that call ends, answered or not. A call made meanwhile at that
   `dbDir` answers `stillRunning` AT ONCE and never reaches the store. Once the call ends, the
   next call reads the true state.

   Letting a retry through would bring the race back. During the orphan's file sweep the key
   store is free and the orphan holds the lock, so the core would answer
   `holder: otherProcess` and `files: leftForHost` while the sweep still runs. That is why a
   retry answers `stillRunning`, not `busy` as first proposed.
4. **The key is the directory's REAL path** (`convert::sever_key`, one helper beside
   `validate_db_dir`). The deepest ancestor that exists is `canonicalize`d (symlinks, `..`,
   the iOS `/var` → `/private/var` alias), and the rest is re-attached lexically, so a
   directory the orphan's sweep has already removed still keys like the one it replaced.
   Only the KEY is canonical: the core always gets the host's own `dbDir`, from which it
   derives the custody namespace.
5. **The rule is FAIL-CLOSED** (`convert::SeverRegistry`). The real-path key is filesystem
   I/O, and a stall can hit registration and a door at different times, so the two sides may
   not compute the same key. The rule therefore settles every doubt by refusing:
   - **Registration comes FIRST.** At its start, before any I/O, a sever inserts its
     LEXICAL key (`.` and `..` collapsed) and marks itself UNRESOLVED, under one lock. Then it
     computes its REAL-PATH key on the blocking pool, inside its bound. If the key resolves,
     the sever adds it and clears its own mark, under the lock again. If it does not, the
     sever stays UNRESOLVED until its core call ends. There is no window between a sever's
     start and its registration.
   - **A door with nothing in flight** does no key work at all: no blocking-pool hop, no
     `canonicalize`. That is the common case, and it costs a door nothing.
   - **Otherwise:**
     - while ANY sever is unresolved, EVERY door refuses, whatever its spelling;
     - a door refuses on its lexical key;
     - it then computes its real-path key under `DOOR_KEY_BOUND` (250 ms). After that
       await it takes ONE snapshot, and refuses if any sever is now unresolved or its real
       key is in flight;
     - a door whose own real-path key does not resolve in time REFUSES too, and never hangs.

   **The exact guarantee:** a door refuses when, at either of its two snapshots (on entry,
   or after its key computation), any sever is unresolved or registered at the door's
   lexical or real-path key. The guard is a CHECK, not a hold: a door that has passed it
   holds no entry. So a sever that STARTS after a door's check can run alongside that door's
   work. The core then governs that case, as it does for any sever beside a live holder: the
   store lock, the tombstone and the birth gate (ADR-0565). Beyond that, the one miss is
   residual 11.

   **The cost:** while a sever is in flight during a filesystem stall, opens, creates and
   wipes at ANY directory can be refused. That happens only mid-duress, during a stall.
6. **ONE guard for every other door** (`convert::refuse_while_severing`). While a sever of
   the directory is in flight, open, create, restore, watch-only and the host-seed pair
   refuse with `walletAlreadyOpen`, and wipe and wipeForce with `walletOpen`. Both are
   existing typed "something holds this wallet" answers (no new variant; ABI 7 unchanged by
   this), given before any work.
7. **The entry's removal** is a guard held by the SPAWNED task. It runs when the core call
   returns, when it panics (the task's future unwinds; pinned by a row), and when the runtime
   shuts down and cancels the task. That last case removes the entry while the blocking-pool
   work may run on. The FRB runtime lives as long as the process, so this happens only at
   exit.
8. **The host's rule:** on `stillRunning`, do not purge or re-create at that `dbDir`. Call
   again with a backoff until the cause is not `stillRunning`, or exit the process. Relim
   exits, which is safe.

## Alternatives considered

- **Keep `timeout` + `leftForHost`, and document the case.** Rejected. Every host's rule is
  "purge on `leftForHost`", so the doc would have to fight the value it sits beside.
- **Await the abandoned call.** Rejected: it breaks the hard bound Relim asked for.
- **Cancel the core call.** Impossible: a blocking-pool section cannot be cancelled.
- **Let a retry reach the core and answer `busy`.** Rejected: the key store is free during the
  file sweep, so the retry would answer `leftForHost` mid-sweep (point 3).
- **Make the file sweep deadline-aware.** It narrows the window but does not close it, since
  one `remove_dir_all` cannot be interrupted. It is not needed once the set exists.

## Consequences — the residuals (amend ADR-0565)

10. **Each overrun parks one uncancellable blocking task** until its core call ends. A host
    that calls in a tight loop would stack them. The docs ask for a backoff, and the in-flight
    set answers any call made at the same `dbDir` at once, so one path parks at most one
    task. Different paths still each park one.
11. **The only remaining miss.** A stall can no longer let a spelling through (point 5).
    What remains is a directory created between two calls under two different spellings,
    when BOTH real-path keys resolved but to different paths. For example, a symlink
    component appears between the calls, so the first call keyed through the not-yet-existing
    tail and the second through the new link. Where the platform's `canonicalize` does not
    return the on-disk case, a case variant is one more instance of the same miss. A host
    passes the same absolute `dbDir` it opened with, as the verb's doc asks.
12. **Fail-closed refusals during a stall.** While a sever is in flight and a real-path key
    stalls, doors at unrelated directories may be refused (point 5). That is the chosen
    cost: a false `walletOpen` / `walletAlreadyOpen` mid-duress, never a door that hangs.
    While one sever is still resolving its key, a second sever of any directory also
    answers `stillRunning` (registration refuses while any sever is unresolved).
