# 0563 — Wallet custody: a key-store call answers inside a bound, and a late one converges on evidence

- **Status:** Accepted (stage S9 `bound`, S298, 2026-09-23;
  `docs/plan/stage-9-a-wedged-keystore-answers-in-time.md` §3.1, revision 3).
  **Extends [ADR-0559](0559-wallet-custody-a-wallets-identity-is-a-minted-id-in-the-wrap-artifact-not-its-path.md)
  and [ADR-0560](0560-wallet-custody-the-index-outlives-every-purge-and-a-wipe-that-cannot-read-it-fails-closed.md);
  supersedes nothing in either.** It closes the residual ADR-0559 recorded at
  IT-17: "a native call that WEDGES is still unbounded".
- **Date:** 2026-09-23
- **Links:** FR-47 (`docs/handoff/host-feature-requests.md`) · the S9 design
  pass (security review ∥ crypto audit on revision 1; the plan's §5 review
  table) · the adjudication (CONTRACT_WRONG, folded as revision 3) ·
  `docs/REVIEW.md` §3

## Context

Every `KeychainPort` call (the Android Keystore through JNI, the Apple Security
framework) ran on the caller's thread with no deadline. A key store that stops
answering hung the host past any budget of its own. Relim's panic wipe gives
our wipe 10 s for two attempts, and its Dart `.timeout` cannot cancel a native
call. A JNI or Security.framework call cannot be cancelled by the SDK either,
so the only bound available is to stop WAITING for it. Whatever the abandoned
call does after that must be something the next wipe converges from.

## Decision

1. **One process-wide key-store worker.** `platform_vault` wraps every native
   vault in `BoundedVault`. Each port call runs on one lazily spawned thread
   (`zec-keychain`), and the caller waits at most 8 s
   (`KEYCHAIN_CALL_BOUND`), or whatever is left of the wipe's 4 s budget for
   all its calls (`KEYCHAIN_WIPE_BUDGET`). It then answers
   `WalletError::KeychainTimeout { cause }`, code `RW-KEY-008`.
2. **Never two native calls, never one that starts after its caller left.**
   A queued job whose caller gave up is skipped. While an abandoned call is
   in flight, every new call answers `busy` at once and is never queued. A
   wedge holds one call and one thread, however many callers retry.
3. **No key outlives its caller's interest.** `SealKey` is heap-resident
   (`Box<Zeroizing<[u8; 32]>>`) and moves to the worker by pointer. An
   abandoned job's inputs are dropped at abandonment. A late result is
   dropped before the busy flag clears.
4. **A timeout is never read as an absent item.** Four fail-open arms (open's
   index read, the wipe's index read, the header proof, settle's index read)
   pass a timeout through, whether or not `force` is set. Every other error
   behaves as before.
5. **A late purge converges on EVIDENCE.** When an abandoned purge later
   returns `Ok(n > 0)`, the worker records the namespace. The wipe's
   zero-sever guard accepts that record the way it accepts the `.wipe-committed`
   breadcrumb. The record is in-process only; after a restart the host uses
   `wipe_force`, as before.
6. **Folded, not new, at the bridge.** The variant crosses as the existing
   kind `keystoreUnavailable`, and `WalletApiError.code` carries `RW-KEY-008`.
   ABI 5 is unchanged. The lockstep gate gains `FOLDED_INTO_KIND`, a REMAP
   exemption: each entry must have its convert arm and its `code()` entry.

## Alternatives considered

- **Converge a late purge when the index agrees with the header** (revision
  1). Rejected by both design reviewers independently. On iOS the index read
  and the purge are separate calls; a lock between them (the panic gesture)
  filters the Secure Enclave key query, so the purge severs 0 while the key is
  live, and the files would be deleted. On macOS the index and the SE key do
  not even share a protection class. A zero count proves nothing.
- **Move an inline `[u8; 32]` key into the worker** (revision 1). A move
  copies the bytes into the closure, the boxed job and the channel slot, none
  of which are wiped. Heap residency makes the move a pointer copy.
- **A new error kind** (`keychainTimeout`, ABI 6). Rejected at sync point 1.
  Relim's panic wipe never reads the kind, and the SDK's own Dart layer
  already maps a keychain timeout to `keystoreUnavailable`. The distinct code
  gives a host the difference without a regen.
- **A host-set bound.** Rejected: a host cannot know the key store's latency
  better than the SDK, and could set it below what StrongBox key generation
  needs.
- **A worker thread per call.** Rejected: a key store that stays wedged would
  accumulate one abandoned thread per retry.

## Consequences

- A wedged key store answers typed within 4 s in a wipe and 8 s elsewhere,
  instead of never. Relim's two attempts take about 4 s plus about 0 s.
- **Residual, stated:** a wrap key that lands after its create timed out is
  an orphan under a freshly minted id namespace that NO wipe reaches. This is
  the same class as a kill between custody and the index write under S2. No
  sealed blob of that attempt reaches disk, so the orphan decrypts nothing on
  disk. On the raw-Apple dev fallback it is a live `SealKey` item.
- **Residual, stated:** a late purge converges without `wipe_force` only in
  the process that saw it land. Relim's panic wipe deletes the directory
  first and never reaches that guard.
- Key-store calls from two wallets in one process are serialised, and a wedge
  in one makes every wallet answer `keystoreUnavailable` until the stuck call
  returns. That is true of the device already (the keystore daemon and the
  Security framework are process-wide).
- `keychain::selftest` (the device self-test) builds its own selftest-scoped
  vaults, and it too runs through `BoundedVault` under the 8 s per-call bound.
  It is host-callable (`selftest_seed_custody` over the FFI), so a
  "diagnostic, not a host path" exemption, as first built, left one key-store
  path that could still hang forever. The S9 diff review's security angle
  found it.
- The 4 s and 8 s values are priced from principle, and from the one host
  requirement on record (Relim's 10 s for two attempts). The first
  `wallet.vault_call` duration from a phone is the first real number. A
  second host needing a different budget revisits the "host-set bound"
  alternative above; it does not fork the constants.
- **Owed, from the diff review:** a randomized-interleaving stress test of the
  worker's state machine (today's tests each drive one hand-sequenced
  schedule; four review angles traced the locking and found no race). A late
  sever recorded after a `wipe_force` already finished leaves a stale entry
  in the evidence set. That is harmless, because custody namespaces are
  freshly minted ids and never reused.
