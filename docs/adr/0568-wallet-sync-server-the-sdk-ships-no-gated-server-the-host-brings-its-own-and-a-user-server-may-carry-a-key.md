# 0568 — Sync servers: the SDK ships no gated server; the host brings its own, and a user's server may carry a key

- **Status:** Accepted. Ruled by the maintainer S314 and confirmed directly S315 ("Yes,
  build that", 2026-10-01). Reviewed on the design (security, arch, crypto, a confirmation
  pass) and on the built diff twice (the same three angles, then code reviewer); the folds are
  `docs/plan/srv-key-the-sdk-ships-no-gated-server.md` §8–§9.
- **Date:** 2026-10-01
- **Links:** `docs/specs/sync-server-picker.md` (D1, D5, D6 — amended by this ADR) ·
  `docs/plan/srv-key-the-sdk-ships-no-gated-server.md` · `docs/specs/wallet-sdk.md` §2.3, §5.4 ·
  supersedes the 2026-09-18 "built-in key" amendment to `sync-server-picker.md` D6 (recorded
  in that spec, not in an ADR)

## Context

On 2026-09-18 the maintainer ruled "just insert builtin key". Since then the reference UI
package (`zec_wallet_ui`) has carried a built-in access key for a gated lightwalletd, and the
SDK's reference catalog has named that server, so every host and every build offered it.

`zec_wallet` and `zec_wallet_ui` are about to be published. A key in a published package can be
read by anyone who downloads it, and a gated server in a public catalog is one operator's
server offered to every app built on the SDK. The maintainer's rulings (S314 and S315, the
second given directly):

1. The public SDK ships no gated server and no key.
2. A host app (Relim) starts the SDK with an extra list of servers, each with its own key. Relim
   keeps offering its gated server that way, with its key supplied at build time.
3. A user can add their own server along with an authentication key.

## Decision

- **The reference catalog names public servers only.** The gated entry, the catalog's
  `auth_header` field, and the `gatedKey` parameter of `referenceSyncServers` are removed. The
  UI package's `kReferenceGatedKeyBuiltIn`, `zecWalletRpcKeyDefine` and `referenceGatedKey()`
  are removed.
- **A host's extra servers use the mechanism that already exists**: `WalletConfig.syncServers`,
  with a per-entry `authHeader` + `authValue`. A host passes
  `[...referenceSyncServers(network: n), ...itsOwn]`. There is no second list field: one list,
  one validating door, one place the offered set is decided.
- **A custom server may carry a key.** `SyncServerChoice.custom` gains an optional header +
  value pair, validated at the same `EndpointAuth` door as a host's key. The probe and every
  later request to that server send it.
- **The SDK persists the user's key** in the choice row of the SQLCipher-sealed aux database.
  It is erased when the user picks anything else: the aux connection runs `secure_delete`, and
  the switch truncates the WAL. It goes with the wallet at wipe. A host's
  key is still never persisted: the host supplies it at every open. The difference is
  ownership. The host owns its key and can re-supply it; only the user knows theirs, and a
  key the user had to retype at every launch would be a picker that forgets.
- **The key never leaves Rust on the way out.** A status or a list returned to the host names
  the header (so the UI can say "Key saved"), and its value is always the empty string. It is
  never logged.

## Alternatives considered

- **A separate `extraSyncServers` field beside `syncServers`.** Rejected. It would add two lists
  and a merge rule (which wins on an id clash?) for a concatenation the host can write in one
  line.
- **Keep the gated entry in the catalog and let the host pass the key** (the D6 design from
  before 2026-09-18). Rejected because the server is the maintainer's, not the public's. The
  catalog would still name it to every app built on the SDK, and with no key the entry is left
  out anyway. Moving the entry into the host loses nothing.
- **Keep the user's key in Dart preferences or the platform keystore.** Rejected because Rust
  owns all state (architecture invariant 1). The aux database is already encrypted, already
  holds the choice, and already dies at wipe.
- **Re-ask for the key at every open.** Rejected. A picker that forgets is broken (D2's own
  argument).
- **A fixed header name for user keys.** Rejected. There is no standard; a gated lightwalletd
  is gated by whatever proxy sits in front of it, so the user names the header their server
  expects.

## Consequences

- **The Dart API changes.** `referenceSyncServers` loses `gatedKey`, `SyncServerChoice.custom`
  gains an optional `key: SyncServerKey`, and a key or header the door refuses now has its own
  error kind, `invalidEndpointAuth`. Nothing has been published (the push gate holds), so the
  break reaches only Relim. It is recorded in the CHANGELOGs, and Relim builds its gated entry
  itself.
- **The user's key can identify them.** `EndpointAuth`'s rule ("the same value for every
  install, or it is an identifier") cannot hold for a key the user brings. A personal key lets
  that server link every request, including broadcasts sent on a fresh circuit, to one account.
  The trust notice says so when a key is entered. This server is one the user chose and keys
  they hold, so the linkage is theirs to accept.
- **Changing only the key is a switch.** A choice that resolves to the server already in use but
  with a different key now rebuilds the session, because the old clients would otherwise keep
  sending the old key. The no-rebuild shortcut (`sync-server-picker.md` §4, "a switch onto
  itself must not forgive" the rewinding streak) still holds for an identical choice. This
  makes forgiving cheaper: on a server that ignores the header, typing any key resets the
  streak in one switch, where before it took a switch away and back, which needs a second
  reachable server. That is accepted. The streak is a cue; the guards still judge every pass,
  and no fund path depends on it. Carrying the streak across a key change was weighed and
  left out (sized to the change, founder S266).
- **The stored key is bound to its URL.** A third column records the URL the key was saved
  for. A row whose URL no longer matches (an older build rewrote it) is unreadable, never
  lent to another server.
- **The old built-in key is in this repository's private history and in Relim beta builds
  already shipped.** It never reached a public package (the push gate holds). It is not
  rotated: the maintainer ruled (S315) that it is not a secret, only a handle that keeps dumb
  bots off the service. Relim's next build carries it from a build define.
