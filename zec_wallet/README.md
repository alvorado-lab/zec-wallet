# zec_wallet

Add private Zcash (ZEC) payments to any Flutter app. `zec_wallet` creates or
restores a shielded wallet, keeps it in sync with the network, and sends and
receives ZEC, while the seed and spending keys stay in a Rust core your Dart
code never touches.

Take as much as you need:

- **`zec_wallet`** (this package): the SDK. Build your own screens on its API.
- **[`zec_wallet_ui`](https://pub.dev/packages/zec_wallet_ui)** (optional): a
  complete drop-in wallet UI, from onboarding and backup to send, receive,
  history, shielding and swaps, styled by your theme, in 16 languages.
- **[`zec_wallet_ui_platform`](https://pub.dev/packages/zec_wallet_ui_platform)**
  (optional): native screen protection for that UI (Android screenshot
  blocking, the iOS app-switcher cover and backup exclusion).
- **[`zec_wallet_tor`](https://pub.dev/packages/zec_wallet_tor)** (optional):
  routes the wallet's traffic through Tor. An app that doesn't add it carries
  no Tor code.

Built on [librustzcash] components and [flutter_rust_bridge]. Pre-release:
read [Known limitations](#known-limitations) before holding real value.

## Keys stay in Rust

**Spending keys never enter Dart.** Seeds and spending keys live and die in
Rust. Dart sees addresses, balances, statuses, and typed errors. A
`Uint8List` cannot be zeroized, so key material never becomes one. Two
crossings are sanctioned, each enforced by a CI policy test rather than
convention:

- the recovery words, for backup and restore (`restore` inbound,
  `revealMnemonic` outbound, word-list form), and the optional BIP39
  passphrase `restore` takes inbound (the secret half of the seed);
- the unified full viewing key, for watch-only (`exportUfvk` outbound,
  `createWatchOnly` inbound). It cannot spend, but it shows every past and
  future payment to whoever holds it: gate it like the phrase reveal
  (re-auth, screenshot protection, a warning), and never log or persist it
  beyond the user's explicit share.

**Platforms.** A wallet runs on Android, iOS and macOS. The package also
builds on Linux and Windows, but cannot hold a wallet there yet: see
[Building](#building).

## Status: pre-release

What works today:

- **The `WalletHandle`**: `createGenerated` / `restore` (from a recovery phrase) / `open` / `createWatchOnly`, cold `snapshot` (with the persisted `lastSynced` stamp) + `currentAddress`, deterministic `close`.
- **Background sync** over the lightwalletd/Zaino gRPC protocol: `startSync` / `stopSync` + the live `watchSyncStatus` stream; `syncServers` / `switchSyncServer` / `probeSyncServer`; `rescanFrom`.
- **Send**: ZIP-321 payment URIs, ZIP-302 memos, `propose` → `send`; **offline-first queueing** (`queueSend`, `listParkedSends`, `retryParkedSend`, `authorizeParkedSend`, `cancelParkedSend`); `listInFlightSends`.
- **Shielding** (`proposeShield`), transparent-funds handling, ephemeral-address sweep and reclaim.
- **Transaction history** (`transactions`, `machineMemos`), the incoming-funds stream (`watchIncomingFunds`), diversified addresses (`mintDiversifiedAddress`).
- **Swaps** behind a pluggable provider port: `swapListTokens` / `swapQuote` / `swapExecute`, `watchSwapStatus`, `listInFlightSwaps`. A swap out of ZEC takes the ZEC amount you pay (`ExactSide.in_`). `ExactSide.out` with a foreign amount is refused with `RequestInvalid`, since nothing the user typed would bound the deposit, and a quote whose ZEC deposit differs from the amount you asked to pay is refused with `QuoteOutOfBounds`.
- **Recovery-phrase backup + restore**: `revealMnemonic` (outbound) / `restore` (inbound), word-list form; viewing-key export (`exportUfvk`) and watch-only wallets.
- **Host transport**: register your own dialer through the C contract (Tor, a proxy, a tunnel); honest transport state in the UI. Tor without a transport of your own is the optional `zec_wallet_tor` package.
- The full public TYPE surface, `validateWalletConfig`, `estimateBirthday`, typed errors with stable `RW-*` codes, catchable by type; the bridge self-test.

The Rust core also implements seed sealing (XChaCha20-Poly1305,
OS-keychain-held key), BIP39 derivation, address validation, and the
user-anchored swap quote funnel. Mainnet sends have been tested on real
devices. The pre-release label means the API may still change before 0.1.0.

## Integration

The integration surface is one opaque handle plus a config struct; all method
names below are the generated Dart (camelCase) form.

### The wallet handle: lifecycle

```text
                 walletExists(config) → bool   (boot fork: false → createGenerated, true → open)
RustLib.init() →
                 createGenerated(config)   ┌─ snapshot()        → WalletState (cold balance/sync/Tor/tip)
                 │   or                  → │─ currentAddress()  → String (Unified Address for receiving)
                 open(config)              │─ revealMnemonic()  → List<String> (back up ONCE, secure screen)
                                           │─ startSync()/stopSync()
                                           │─ watchSyncStatus() → Stream<SyncStatus>
                                           └─ close()           (releases the single-writer lock + storage)
```

- **`WalletHandle.walletExists(config)`**: the cheap, LOCK-FREE boot probe: is a
  COMPLETED wallet provisioned at `config.dbDir`? `false` → offer create, `true`
  → `open` it. Reads only the on-disk provisioning marker (no SQLCipher open, no
  single-writer lock), so it never blocks the `open` it precedes; validates
  `dbDir` only (not the endpoint/Tor). An interrupted-create remnant reads
  `false` (→ `createGenerated` resumes its repair); a corrupt store throws
  `storeCorrupt` (recover from the phrase, never silently "no wallet").
- **`WalletHandle.createGenerated(config)`**: generates a fresh 24-word seed
  in Rust (`OsRng`), seals it per `config.seedPersistence`, opens the wallet.
  Throws `walletAlreadyExists` rather than ever clobbering. Local-only and
  bounded (no network); first sync resolves a `null` birthday to the chain tip.
- **`WalletHandle.open(config)`**: opens an existing wallet at `config.dbDir`
  (typed `notFound` / `networkMismatch` / `walletAlreadyOpen` /
  `provisioningIncomplete`).
- **`snapshot()` / `currentAddress()`**: cheap cold reads; safe on resume
  before re-subscribing the live stream. `snapshot().lastSynced` is the
  persisted time of the last completed sync pass (it survives relaunches;
  `null` until the first pass completes).
- **`revealMnemonic()`**: the recovery words, in index order (the one outbound
  key crossing). Show **once** on a secure, screenshot-blocked screen; never
  log/persist/screenshot. Throws `noMnemonic` for a raw-seed wallet (a host
  whose recovery is its own master phrase; see the integration note).
- **`startSync()` / `watchSyncStatus()`**: a freshly opened wallet does NOT
  auto-sync; call `startSync` to advance. The stream emits the current status
  on subscribe, never completes on a transient fault (a stall is a
  `SyncStatus.stalled` **event**), and must be paused on
  `AppLifecycleState.paused` / reconnected on stream error (unstable-network
  normal). `close()` is the deterministic teardown.
- **`syncFor(budgetMs:)`**: for a host that holds the process only briefly (a
  background wake, a pull-to-refresh): ONE pass for at most your budget, then
  `BoundedSync { scannedTo, tip, finished, resubmitted }` read from the
  wallet's database. When the budget runs out the pass stops the way
  `stopSync` stops the loop (progress kept), and `finished` is `false`, never
  an error. It returns within the budget plus one request timeout (30 s).
  Queued sends are resubmitted only when two minutes of budget remain after
  the pass (`resubmitted` says whether they were). The budget is always yours;
  the SDK schedules nothing. Throws `syncRunning` while the `startSync` loop
  runs: a wallet runs the loop or one bounded pass, never both.
- **A send's unknown outcome: `TxSummary.expiryHeight`.** For a transaction
  this wallet created, the height past which it can no longer be mined
  (`null` for anything received). Read it against the wallet's OWN scan, as
  `syncFor` reports it: while `expiryHeight <= tip` the chain has already
  decided and a sync up to `expiryHeight` tells you which way; while
  `expiryHeight > tip` it is still open on chain, so wait for block
  `expiryHeight`. Beside `delivery == retryPending` this is the whole answer
  to "did it go, and when will I know".
- **The BIP39 passphrase is normalized (NFKD) before it derives the seed.**
  A passphrase typed with a composed "é" and one typed with "e" plus a
  combining accent restore the same wallet, as BIP39 requires and as other
  BIP39 wallets do. A host that supplies its OWN seed bytes
  (`createWithHostSeed`, the seed port) is untouched, since the SDK derives
  nothing from them. But if you derive those bytes from a BIP39 phrase WITH
  a passphrase, NFKD the passphrase yourself first (or hand the phrase to
  `restore` instead), or the same words restore a different wallet in any
  other BIP39 wallet.
- **`WalletHandle.severCustody(config:, deadlineMs:)`: the duress
  force-sever (FR-53).** For a panic wipe under a time budget, where `wipe`
  would refuse with `walletOpen` because something still holds the wallet (a
  handle never closed, a straggler past `close`, a rescan or server switch
  mid-rebuild, an open in flight, another process). It answers
  `SeverReport { severed, holder, files }` by `deadlineMs +
  severAnswerGraceMs()` (see below), and never throws for a custody
  outcome; it throws only for a bad `dbDir` (pass the
  SAME absolute `dbDir` you opened with) or an I/O fault. `deadlineMs` is
  clamped to `[0, 4000]` ms (the 4 s key-store budget of a wipe; the floor is
  0, so any small deadline is kept as given), and it is a HARD bound on the
  WHOLE call, not only the key store: the answer comes by
  `deadlineMs + severAnswerGraceMs()` (250 ms in this build) whatever stalls.
  An overrun (ADR-0566) answers the DISTINCT `notSevered(stillRunning)`,
  `files: stillInUse`, `holder: unknown`. The SDK's own sever cannot be
  cancelled: it still runs in the background and may still be deleting in
  `dbDir`. **On `stillRunning` / `stillInUse`, do NOT purge or re-create at
  that `dbDir`.** Call `severCustody` again, with a backoff, until the cause is
  not `stillRunning`, or exit the process (as Relim does, which is safe). A
  call made while that sever runs answers `stillRunning` at once and never
  reaches the store, so it cannot double-purge. Once the sever ends, a call
  answers the true outcome. Each overrun parks one uncancellable blocking
  task, so do not call it in a tight loop. While that sever runs, every other
  door at that `dbDir` refuses before any work: `open`, the creates and
  `restore` answer `walletAlreadyOpen`, and `wipe` / `wipeForce` answer
  `walletOpen`. "That `dbDir`" means the directory's real path, so a symlinked
  or `..` spelling of it counts; a directory created between two calls under
  two different spellings may not (ADR-0566). The guard fails CLOSED: during
  a filesystem stall while a sever is in flight, these doors may refuse at
  any directory, and they never hang. It is a check at the door, not a hold:
  a sever that STARTS after a door has passed it runs alongside that door's
  work, under the SDK's own store lock and tombstone (ADR-0566). A deadline
  of `0` makes no key-store call and answers `notSevered(pastDeadline)`. It
  is STATIC, and it needs no open handle: it reads only `config.dbDir`
  (every other field may be anything). Calling it again after it ended, or
  calling it on a `dbDir` that never held a wallet, answers `alreadyGone`
  with no error. The report names no path, key id or free text.
  `NotSeveredCause` is closed: `nothingSevered` · `timeout` · `busy` · `pastDeadline` · `vaultAbsent` · `keystoreUnavailable` · `stillRunning` · `unknown` (a cause a newer core added; treat it as custody possibly live).
  If nothing holds the wallet, it is
  the ordinary `wipe` (`files: removed`, `holder: none`). If something holds it,
  the key store is severed anyway, NO file is deleted
  (`files: leftForHost`), and every instance of the wallet in this process
  answers `invalidState` (`wiped`) from then on; a broadcast in flight does
  not go out. Like `wipe`, it sets the device log `off` and closes the
  `watchDeviceLog` stream first. **Your four obligations:**
  1. **Read `severed`, not the absence of an exception.** `severed` is
     proven. `severedUnproven(countUnreadable)`: the phone was locked; the
     delete was issued but cannot be confirmed, and a `wipe` after the next
     unlock confirms it. `notSevered(timeout | busy)`: the key store was
     occupied, by a custody write already in flight OR by ANY other key-store
     call in this process (another open wallet's, or `selftestSeedCustody`'s),
     since all of them share one worker. Call `severCustody` again: no new
     custody write can land meanwhile, and the retry succeeds once that other
     call returns, so a host that keeps several wallets open, or runs the
     selftest, should not expect `severed` within the first deadline.
     `notSevered(stillRunning)`: the SDK stopped waiting, but its own sever
     is still running; call again (with a backoff) until it is not. Any other
     `notSevered`, and `unknown`: the custody may be live.
  2. **Purge ONLY on `files: leftForHost`: you delete the directory
     yourself**, after the sever has answered. **On `stillInUse`, never**:
     the SDK is still working in it. On `unknown`, call again.
  3. **Severed means the key that unlocks the files on disk is deleted from
     the key store.** `custodyDisclosure`'s `eraseAssurance` says how strong
     that is, and no value of it proves an earlier copy of the key unusable.
     It does not mean the live instance's memory is clean: its seed and
     database key stay in RAM until the holder drops. A host that must end
     that exits the process.
  4. **Before creating a wallet at the same `dbDir` again in this process**,
     run a plain `wipe` of it. Until then `create`/`open` there answer
     `invalidState` (`wiped`). A process restart also clears that; after a
     restart, an `open` at a severed directory you did NOT delete answers the
     store's custody error, not `wiped`; the plain `wipe` is still the
     remedy.

  One side effect to know: if a two-transaction send had its first
  transaction accepted before the sever, the send still returns its results,
  with the second transaction as `GrpcFailure`. That row ordinarily promises
  a resubmission; on a severed wallet none happens.

### Configuration: `WalletConfig`

Validate up front with `validateWalletConfig(config: config)` (the same
Rust-side rules `create`/`open` enforce) before any wallet exists. Fields
marked *optional* may be omitted; every other field is required.

| Field | Type | What it is |
|---|---|---|
| `dbDir` | `String` | Host app data dir. The host owns backup-exclusion (iOS `isExcludedFromBackup`, Android `no_backup/`). |
| `network` | `Network` | `main` / `test`; one binary serves both. |
| `endpointUrl` | `String` | lightwalletd/Zaino gRPC endpoint. `https` everywhere; `http` only to loopback; userinfo (`user:pass@`) rejected. **The SDK never picks a server**: the host states it. |
| `endpointAuthHeader`, `endpointAuthValue` | `String?` | *Optional.* One request header sent to the endpoint (e.g. an API key); both or neither. **Use one key for every install:** a per-user key re-links each broadcast to the wallet that synced, defeating the fresh circuit per send, so it becomes a tracking identifier. A key shipped in an app can be extracted from the binary: it is abuse control, not access control. |
| `syncServers` | `List<SyncServer>?` | *Optional.* The servers a user may switch between; `endpointUrl` stays the default. `referenceSyncServers` gives the public ones; append your own: `[...referenceSyncServers(network: n), SyncServer(id: 'mine', label: 'My server', url: 'https://…', authHeader: 'x-api-key', authValue: key)]`. The package ships no key. A gated entry's `authValue` follows the `endpointAuthValue` rule: one key for every install, from a `--dart-define`, never a committed file. A user can add their own server with a key in the picker (`SyncServerChoice.custom(url:, key:)`). |
| `tor` | `TorPolicy` | `off` / `preferred { runtime }` / `required { runtime }` (fail-closed). The one Dart-expressible `runtime` is `TorRuntimeConfig.hostDialer()`, the dialer your native code registered (see **Host transport** below). `externalSocks5 { addr }` is refused at the config door (the SDK opens no loopback proxy). Nothing Tor-related is compiled into this package; an app without its own transport adds the optional `zec_wallet_tor` package. |
| `seedPersistence` | `SeedPersistence` | `sealedKeychain` (seal seed + mnemonic under a keychain-held key; enables `revealMnemonic`) or `none` (nothing persisted; the host re-supplies the seed each launch). |
| `birthdayHeight` | `int?` | *Optional.* Restore-from height; `null` on create (= current tip). Above-tip rejected; below-activation clamped. `estimateBirthday(network: …, approxUnixSecs: …)` derives a conservative floor. |
| `broadcastJitter` | `JitterPolicy` | `none` or `uniform { maxMs }`: a random pre-broadcast delay so sends don't timestamp-correlate (documented default window 0 to 10 s). |
| `machineMemoPrefixes` | `List<Uint8List>` | The byte prefixes an on-chain `0xFF` memo must carry for `machineMemos` to return it. Pass `const []` if you do not read machine memos: empty keeps that verb closed. At most 8, each 1 to 32 bytes. |

**Reference defaults** (what the in-repo example app ships; the SDK itself
requires the host to choose): `endpointUrl: https://zec.rocks` (mainnet, the
public endpoint Zashi/Zodl uses), `tor: off` (direct), `seedPersistence:
sealedKeychain`, `birthdayHeight: null` on create.

> **Integration note (host owns the network).** In a host that runs its own
> connection stack (e.g. a privacy app that dials lightwalletd over its own
> Tor/NetDialer infrastructure), the `endpointUrl`/`tor` here are the
> *host's* policy, not the SDK's. Such a host typically derives the wallet
> from its **one master seed** (raw-bytes persistence), so `revealMnemonic`
> returns `noMnemonic` and recovery is the host's master phrase, not a
> wallet-local one. The generated-seed + `revealMnemonic` backup flow above is
> for a standalone wallet that owns its own seed.

### Host transport: bring your own dialer

A host that runs its own network transport (Tor, Shadowsocks, VLESS, a
tunnel, a plain connection behind a proxy) routes the wallet's traffic
through it by REGISTERING a dialer with the wallet's native library. The SDK
opens no listener and connects to no loopback proxy; your transport is used
exactly as you use it, and the wallet renders what you declared about it.

**The contract is the C header** `rust/include/zec_wallet_net_dialer.h`
(`ZW_NET_DIALER_ABI_VERSION` 4; the header's own history block says what
each version changed). It wins over every prose sentence, including this
one. In short:

1. **Register once, at trusted init, BEFORE validating the wallet config.**
   Resolve the three verbs by name from the already-loaded wallet image
   (never `RTLD_DEFAULT`) and call `zec_wallet_register_net_dialer(
   ZW_NET_DIALER_ABI_VERSION, &vtable, &descriptor, auth_out)`. First-wins; the 32-byte token written
   to `auth_out` gates every later replace, clear and push.
2. **The vtable** is `dial / read / write / close` with completion
   callbacks: the SDK hands you a buffer and an op id, you complete on ANY
   of your threads, exactly once, possibly before the verb returns. Buffer
   ownership and the lifetime of a superseded backing are spelled out in
   the header; read them before writing `close`.
3. **The descriptor is yours to name.** `{ name[32], name_len, readiness,
   isolation, exposure, health }`: `name` is your own display name for the transport
   ("Tor", "Shadowsocks", a profile's label: the SDK keeps no list of names),
   1..=32 bytes of UTF-8, not blank, no control or bidi/zero-width format
   characters, or the verb returns `-6`; `readiness` 0..=100 (below 100 the
   SDK does not dial and shows "starting up"); `isolation` whether your path
   keeps the wallet's per-purpose connections apart; `exposure` whether it
   hides the device's address from the server (`HIDDEN` for Tor, a proxy, a
   tunnel; `EXPOSED` for a plain connection; `UNKNOWN` if you cannot say);
   `health` `STARTING` / `READY` / `FAILED` (say `FAILED` when you have
   given up, not while retrying).
   Push a fresh descriptor through `zec_wallet_net_dialer_notify` whenever
   any of it changes; `retire = 1` tears the current backing down honestly.
4. **Errors are a frozen five-code table** (`ZW_DIAL_*`). `NOT_READY`,
   `REFUSED` and `RETIRED` are never reachability failures. Under
   `TorPolicy.preferred`, two things can move the wallet to clearnet, both
   visibly: a dial answered `UNREACHABLE` or `TIMEOUT`, and a descriptor
   declaring `health = FAILED` for a full patience minute (measured from the
   declaration). A transport that is only not ready never reaches clearnet,
   however long it lasts. `TorPolicy.required` never falls back at all. A
   dial you cannot carry now is refused synchronously, never parked.
5. **Then configure** `TorPolicy.required_(runtime:
   TorRuntimeConfig.hostDialer())`. Validating that config while nothing is
   registered is refused (`RW-CFG-001`), so register first. The state you
   read back is `TorState.active(runtime: TorRuntimeKind.hostDialer(name,
   isolation, exposure))`: your name verbatim, "connections can be linked
   by the proxy" from `isolation`, "not private" from `exposure`.

A complete reference implementation of the HOST side, in Rust, is the
SDK's own test fake: `rust/tests/host_dialer_fake/mod.rs` declares the
vtable and descriptor independently of the SDK, runs its own runtime and
completes on its own threads.

### Tor without your own transport: the optional `zec_wallet_tor` package

If your app has no transport of its own, add
[`zec_wallet_tor`](https://pub.dev/packages/zec_wallet_tor): a second,
optional Flutter package that bundles a Tor client (the arti-based
[`dialer-tor`](https://crates.io/crates/dialer-tor) crate) in its own native
library and, at its init, registers itself with the wallet through exactly
the contract above: a descriptor named "Tor", readiness that follows the Tor
bootstrap, isolation supported, exposure hidden. Your app then configures
`TorPolicy.required_(runtime: TorRuntimeConfig.hostDialer())` as for any
registered transport, and the wallet's UI renders the same honest state. Its
own README is its API.

- **`zec_wallet` stays Tor-free.** Adding `zec_wallet_tor` is the only
  way Tor code enters your build; an app that does not add it pays nothing.
- **One transport per app.** The registry slot is first-wins: if your app
  registers its own dialer first, `ZecWalletTor.init` throws
  `TorPluginError(kind: slotOccupied)` and starts no Tor client. Which one
  speaks is decided by your init order.
- **Honest on mobile.** iOS and Android suspend an app's sockets in the
  background; the package reports readiness down on suspend and back up
  after a rebuild on resume (arti's persisted directory cache makes a warm
  bootstrap seconds rather than minutes), and under `required` there is no
  silent fallback to a plain connection.
- **It makes your app larger:** its native library measured 9.8 to 15.3 MB per
  Android ABI in the example's release build.

### Device log: the switch, and the lines as a stream

The SDK installs its device log OFF. Nothing is written until you call
`setDeviceLog(level: DeviceLogLevel.errors)` or `.detailed`, and the level you set is
the one your user chose. `setDeviceLog` and `deviceLogLevel()` answer the
EFFECTIVE level. It is `off` when nothing can write: a process whose `tracing`
default another library took first installs nothing. Only the SDK's own
events are logged, at INFO and above. Every field is checked against a fixed
allowlist of fields that carry no address, amount, key or id, and a refused
one is counted as `withheld=<n>`.

Where the lines go: logcat on Android (tags `zec_wallet_core` and
`zec_wallet`), and the unified log on iOS and macOS (subsystem `zec_wallet`,
the same two as categories). On Linux and Windows they go to the process's
stderr, one write per line, as `<LEVEL> <tag> <line>`. **A stderr write can
block.** If your app pipes stderr to a reader that stops draining it, an SDK
thread that logs waits until the reader drains. A stalled logd on Android has
the same effect. It happens only while you have the log on, and a wipe never
waits on it. Keep a piped stderr drained, or leave the log `off`.

`watchDeviceLog()` gives you the same lines as a `Stream<DeviceLogLine>`, for
your own in-app log or a bug report the user sends:

- **Call it after `RustLib.init()`, beside `setDeviceLog`, and keep
  draining it.** The SDK keeps no queue of its own. It sends at most 60 lines
  a minute to your stream; a line over that still reaches the platform log.
- **One subscriber per process.** A second `watchDeviceLog()` replaces the
  first, whose stream closes. That is how a hot restart or a re-subscribe
  after unlock takes over. A host with two consumers fans out on its own side.
- **Any `wipe`, `wipeForce` or `severCustody` closes the stream and sets the
  log `off`**, before anything else and whether or not it then succeeds. No
  line is sent after that. Nothing re-opens it on its own: re-arm with `setDeviceLog`
  and `watchDeviceLog`. The example re-arms after every Delete, which is
  housekeeping for an app with no duress trigger. **A host with a duress
  path must NOT re-arm on that path.**
- **`seq` numbers every line the log produced while it was on**, delivered
  or not. Within one subscription a gap is a line your stream did not get:
  the 60-a-minute budget, or no stream registered at that moment. It restarts
  at 0 when a wipe begins. It cannot show lines posted to an isolate that
  died before reading them.
- **`text` and `tag` are control-safe, not delimiter-safe.** They are
  printable ASCII with no newline, so they cannot forge a second line in a
  line-oriented log. They can hold `"`, `\`, `,`, `|` and `=`, so escape them
  when you put them into JSON, CSV or any structured format.
- **A silent stream is not an error.** The stream never errors. If
  `deviceLogLevel()` answers `off` after you asked for more, nothing is
  installed and the stream will stay silent.
- **Apple, static linking:** if your app statically links this library beside
  another Rust library that also installs a global `tracing` subscriber, the
  first to install wins. Ours then answers `off` and never panics. Initialise
  the wallet's runtime first if its log matters to you.

### Sessions: what belongs to the process, what belongs to a wallet

A process can open wallets one after another (account switching: close A,
open B). Some state is per PROCESS and outlives every wallet; the rest is
per WALLET and goes with it. Open ONE wallet at a time: nothing below is
built for two open at once.

| State | Scope | What closing a wallet does to it |
|---|---|---|
| Host dialer registration and its generation (`zec_wallet_register_net_dialer`) | process: ONE active registrant | nothing; the next wallet dials through the same registration. Register once, not per wallet. |
| Device log level (`setDeviceLog`) | process | nothing; the level stays what the host last set. |
| Device log stream (`watchDeviceLog`) | process: ONE subscriber; a second call replaces the first | nothing on `close`. A `wipe`, `wipeForce` or `severCustody` closes it and sets the level `off` until the host re-arms both. |
| Seed port (`zec_wallet_register_seed_port_bound` / `_update_seed_port`) | process: ONE active supplier; the slot carries no wallet, no `dbDir` | nothing. A host whose supplier's `ctx` is per wallet must switch it itself (`zec_wallet_update_seed_port` with its token) before opening the next wallet; otherwise wallet B's seed pulls reach wallet A's supplier. |
| Everything under `dbDir` (the wallet and data databases, the seals) | wallet | closed with it; the single-writer lock is released. |
| The keychain namespace and the custody index | wallet | untouched by `close`; `wipe` destroys them. |
| The table of live wallet paths, with each severed `dbDir`'s tombstone (FR-53) | process | nothing on `close`. A `severCustody` tombstones the `dbDir`: every live instance there answers `wiped`, and so do `create`/`open` there until a plain `wipe` of it completes or the process exits. |
| The key-store worker (FR-47: the one thread every key-store call runs on) | process: ONE worker, however many wallets | nothing. A key-store call stuck in one wallet's operation makes every wallet's key-store calls answer `keystoreUnavailable` (`RW-KEY-008`) until it returns, since the device's key store is process-wide anyway. |
| Host dialer operations (dials and reads/writes in flight) | the table is process-wide; its records carry no wallet | `close` marks every operation still waiting on the host as closed and hands back to your `close` any connection you deliver to one afterwards. Nothing you still hold is freed: an operation's buffer is yours from the verb to your completion, and its record leaves the table only at that completion. |
| The operation id counter | process: never reset | nothing. Ids are unique across every wallet the process opens, and say nothing about which wallet minted them. |

Because the operation table is process-wide, closing one wallet while a
second is open also closes the second's in-flight dials; they fail typed and
the next dial proceeds. A per-wallet close is owed (FR-39) and will arrive
with the first host that needs two wallets open at once.

## Design rules you can rely on

- **Sealed classes grow.** Status enums gain variants in minor releases, so
  always keep a default arm when switching, and render the `unknown` arm
  (it exists on every status enum by design) neutrally.
- **Errors are typed.** Catch `WalletApiError` / `SwapApiError` by type,
  branch on the sealed `kind`, log the stable `code` (`RW-STORE-005`-style,
  append-only). Never match on message text.
- **Amounts are `int` zatoshis** (fields suffixed `Zat`; max supply
  ≈ 2.1e15 < 2^53, exact in every Dart runtime). Foreign-asset amounts in
  swaps are decimal **strings**. There is no float money anywhere.
- **Money-safety signals on a send proposal.** `SendProposal` carries
  user-error-hardening signals on top of the already-validated send (address
  parse, amount bounds, the §5.1 de-shield `hasTransparentRecipient`):
  `largeSend` (non-null ⇒ show ONE deliberate large-amount confirm before
  signing; it is set when the debit is ≥ 90% of the available balance OR
  ≥ 1 ZEC, and its `LargeSendReason` keys the copy) and `selfSend` (a best-effort "sending to
  your own address" note). Render `largeSend` as the *only* friction step and
  everything else as a passive cue, so there is no alert fatigue. Surface the
  confirm only once the spendable balance is settled (the denominator excludes in-flight
  change, so a mid-scan send can transiently over-signal).
- **Honest networking.** The endpoint is required configuration (the SDK
  never picks a server for you); `https` everywhere, `http` only to
  loopback; Tor policy is an explicit choice (`off` / `preferred` /
  `required`, fail-closed) with degradation always visible as state.
- **A key-store call answers in bounded time.** The phone's key store
  (Android Keystore, Apple Keychain / Secure Enclave) can stop answering, and
  a native call cannot be cancelled. The SDK stops WAITING instead: a `wipe`
  answers within 4 s for all its key-store calls, and `create` / `open` within
  8 s per call. Past that you get kind `keystoreUnavailable` with `code`
  `RW-KEY-008`; a locked keychain keeps `RW-KEY-001`. While a stuck call is
  still inside the key store, every further key-store call answers
  `RW-KEY-008` at once. Only the stuck call returning, often a restart, clears
  it, so a retry loop gains nothing. A purge that lands after its wipe timed
  out is converged by the next plain `wipe` in the same process. After a
  restart that wipe answers `keystoreInconsistent` and `wipe_force` finishes
  it, as it always has.

## Known limitations

- **A light server can lower recorded subtree heights after a rewind.** After
  a chain reorganisation the wallet rewinds a fixed distance and takes new
  completion heights from the server. It refuses an upward change, but
  accepts a downward one larger than any reorg of that depth could cause,
  because it cannot yet find the actual fork point. A server that does this
  can make the wallet misjudge which funds are spendable: a note can be
  offered before it is safe to spend, or funds can look stuck. The server
  cannot take funds. Until this is fixed, 0.0.1 is not ready to hold large
  amounts; use it with small amounts and a light server you trust.
  [Issue #1](https://github.com/alvorado-lab/zec-wallet/issues/1) tracks it.

## Building

The Rust core compiles from source; there are **no prebuilt binaries**.
You need a Rust toolchain (rustup) on the build machine; [Cargokit] wires
the build into `flutter build`/`flutter run` automatically for Android,
iOS, macOS, Linux, and Windows.

**A wallet runs on Android, iOS and macOS.** Android and iOS are tested on
phones and macOS on the development machine.

**Linux and Windows build, but cannot hold a wallet yet.** The wallet keeps
its database key in the platform's key store, and there is no desktop key
store adapter yet. On Linux and Windows every `WalletHandle` constructor
(`createGenerated`, `restore`, `createWatchOnly`, `open` and the host-seed
variants) fails with `vaultAbsent`. No wallet is created and no key is
stored, though `dbDir` and its lock file may be left behind. Linux arm64 is
compiled and its device-log tests run; no Windows build of this release has
been compiled or run. A desktop key store is planned for a later
release.

The package carries its whole Rust workspace under `rust/` (the core crates
in `rust/crates/`, the pins and release profile in `rust/Cargo.toml`, the
exact dependency versions in `rust/Cargo.lock`), so it builds with nothing
but a Rust toolchain beside it. The first build compiles the Zcash stack
and takes several minutes.

**iOS/macOS hosts load the wallet from its OWN image.** The podspec
force-loads the Rust core into the pod's product, which under `use_frameworks!`
(Flutter's default Podfile, and this package's example) is
`zec_wallet.framework`, so open that framework:

```dart
await RustLib.init(
  externalLibrary: (Platform.isIOS || Platform.isMacOS)
      ? ExternalLibrary.open('zec_wallet.framework/zec_wallet')
      : null,
);
```

**Do not pass `ExternalLibrary.process(...)` in an app that carries a second
flutter_rust_bridge library.** FRB's Dart runtime looks up seven UNPREFIXED
`frb_*` symbols through the loader it is given (its content hash, its Dart-API
init and its two function-id dispatchers among them), and the process loader
answers each from whichever image the dynamic loader finds first, which can be
the other library. FRB's content-hash check then fails `RustLib.init` with a
`StateError` (seen on a real iPhone beside a second FRB library, September
2026); without that check the wallet's calls would be dispatched into the other
library. The process loader is right only for a host that links the wallet
STATICALLY (`:linkage => :static`, or no `use_frameworks!`: no framework exists
to open, and FRB's default loader would fail) AND carries no other FRB library.

Call `RustLib.init()` exactly once (guard with `RustLib.instance` checks if
your app structure could race two inits). Android floors: `minSdk 23`
(Android Keystore, the sealed-seed persistence hard floor).

**`RustLib.init()` refuses a mismatched pair.** Before the first bridge call it
compares the bridge version this Dart package was generated against with the
one the native library reports, and throws `BridgeAbiMismatch` if they differ
(or if the library is too old to report one). The two halves must come from the
same `zec_wallet` release; a mismatch would otherwise decode values wrongly.
Catch it by type: it is an `Exception`, never an `ArgumentError`, so it is
distinguishable from "the native library is not in this build", which still
fails with `dart:ffi`'s `ArgumentError`, as it always did, on every platform
(from the loader itself when it opens a named library or framework; under the
process loader, which opens nothing, at the first symbol lookup). Log it; do not render its text to
an end user: it is advice for the developer who shipped the mismatched pair.

**Web/WASM is permanently out of scope** (keys-in-Rust is not enforceable
in WASM); the generated `frb_generated.web.dart` is an inert codegen
artifact, never activated.

```sh
cd example && flutter run   # the example app runs the bridge self-test
```

Regenerating bindings after changing `rust/src/api/*.rs`:

```sh
flutter_rust_bridge_codegen generate && dart run build_runner build
```

## Independence and trademarks

`zec_wallet` is an independent, open-source project. It is not affiliated
with, endorsed by, or sponsored by the Zcash Foundation. "Zcash" is a
trademark of the Zcash Foundation; it is used here only to describe the
cryptocurrency this library works with. No Zcash logo is used. This project
is unrelated to ZecWallet, the discontinued Zcash wallet application.

## License

MIT; see `LICENSE`.

[librustzcash]: https://github.com/zcash/librustzcash
[flutter_rust_bridge]: https://pub.dev/packages/flutter_rust_bridge
[Cargokit]: https://github.com/irondash/cargokit
