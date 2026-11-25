# Changelog

## 0.0.7

**A queued send your app cancelled could still be paid. Update.** In 0.0.6
and earlier, the wallet read its queued sends once and then signed them one
after another. If your app cancelled the newest queued send and queued another
in that time, the new send took the cancelled one's id: the wallet signed and
broadcast the cancelled payment and marked the new one sent, so the new one
never went out. A swap deposit signed at execute had the same gap, and a
deposit whose quote had lapsed could delete another send's record.

0.0.7 signs a queued send only while its row is still the exact one the wallet
read: the same id, creation time and request, and still queued. Otherwise it
signs nothing, and a send queued in its place goes out on a later
pass. `authorizeParkedSend` answers `notFound` for a send cancelled or
replaced in the meantime, an answer it already had. Two cases remain, both for
a queued send cancelled and replaced within the second it was created. A
`cancelParkedSend` that names it removes the replacement and returns `true`, so
the replacement has to be queued again. An `authorizeParkedSend` that names it
sends the replacement, a payment your app queued, on that payment's own terms.

**Memos and payment status no longer go missing when the light server has no
answer.** To read memos and to confirm the second step of a payment to a TEX
address, the wallet asks its light server for whole transactions. Four of the
answers were handled wrongly:

- An empty answer deleted the request, so the memo stayed missing until its
  block was scanned again. No server we measured sends one, and a NotFound
  already kept the request, so this is a safeguard. An absent answer of any
  kind now changes nothing, and the wallet asks again on a later sync.
- An "unmined" answer after the transaction's expiry deleted the question that
  confirms a TEX payment's second step. For a transaction with a stored expiry,
  that answer is now refused.
- Every sync asked about the same first 50 requests, so 50 that the server
  never answers held back every request behind them for good. Each sync now
  starts after the last request it asked, so every request gets its turn.
- A reply naming a network branch the wallet does not know stopped the status
  checks for that transaction until the next launch. Now only its memo read
  waits.

The cost: a transaction that will never be mined, such as an expired send that
never went out, keeps its status question for the life of the wallet. The
wallet asks the server about it again in its turn, at most 50 requests per
sync, and each request names the transaction's id to the server.

**Dependencies.** The lock moves `rustix` to 1.1.6 for RUSTSEC-2026-0486 (its
extended-attribute functions could mark unwritten bytes as initialized). The
wallet calls rustix only for a file lock, so it never reached them.
RUSTSEC-2026-0459 (SQLite before 3.54.0 can return uninitialized memory when
it opens a database beside a crafted `-wal` file) has no fixed SQLCipher build
yet, so the lock keeps SQLite 3.46.1 inside SQLCipher. The attack needs a
crafted file placed beside the wallet's database on the device.

### Upgrading

From 0.0.5 or 0.0.6, run `flutter pub upgrade zec_wallet zec_wallet_tor`.
Your current lines already accept the new versions, and so does the
`zec_wallet: ^0.0.5` inside `zec_wallet_ui` 0.0.4: pub reads `^0.0.5` as at
least 0.0.5 and below 0.1.0. The API and the database do not change.

From 0.0.4 or earlier, follow "Upgrading from 0.0.4" under 0.0.5 below.

For a new app, the working set is:

```yaml
dependencies:
  zec_wallet: ^0.0.7
  zec_wallet_ui: ^0.0.4
  zec_wallet_ui_platform: ^0.0.3
  zec_wallet_tor: ^0.0.6
```

`zec_wallet_ui` and `zec_wallet_ui_platform` did not change in this release.
`zec_wallet_tor` 0.0.6 moves its lock to `url` 2.5.9.

## 0.0.6

**Android builds work with Gradle 9.** Apps created with current Flutter could
not build 0.0.5 or earlier for Android. Flutter 3.44 sets new apps up with
Gradle 9.1, and Gradle 9 removed a call the package's
`cargokit/gradle/plugin.gradle` made, so the build stopped with "Could not find
method exec()". 0.0.6 builds on Gradle 8 and 9. The API, the database and the
wallet's behaviour do not change.

### Upgrading

From 0.0.5, run `flutter pub upgrade zec_wallet zec_wallet_ui zec_wallet_tor`.
Your current lines already accept the new versions, because pub reads `^0.0.5`
as at least 0.0.5 and below 0.1.0.

From 0.0.4 or earlier, follow "Upgrading from 0.0.4" under 0.0.5 below
instead: 0.0.5 changed the API and migrates the wallet's database one way.
The `zec_wallet: ^0.0.5` line those steps give then resolves to 0.0.6.

For a new app, the working set is:

```yaml
dependencies:
  zec_wallet: ^0.0.6
  zec_wallet_ui: ^0.0.4
  zec_wallet_ui_platform: ^0.0.3
  zec_wallet_tor: ^0.0.5
```

`zec_wallet_ui` and `zec_wallet_ui_platform` did not change in this release.
`zec_wallet_tor` 0.0.5 carries the same Gradle fix.

**A correction to the 0.0.5 notes.** They said `flutter pub upgrade` stays on
0.0.4 because `^0.0.4` accepts only 0.0.4. That was wrong: `^0.0.4` also
accepts 0.0.5 and 0.0.6, so `flutter pub upgrade` can take an app on 0.0.4
past the 0.0.5 changes. If that app still has `zec_wallet_ui` 0.0.3, its build
stops with "The type 'TxStatus' is not exhaustively matched", because
`zec_wallet_ui` 0.0.3 does not build with `zec_wallet` 0.0.5 or later. The
0.0.5 steps move both packages together.

## 0.0.5

**Upgrade before the NU7 network upgrade activates on mainnet, at block
3,543,405 (about 2026-11-06 02:00 UTC).** After that block, 0.0.4 and earlier
cannot send. Against a server that has upgraded they refuse to sign. Against a
server still on the old rules, or one that does not report which rules it
follows, they sign payments the network rejects.

### Upgrading from 0.0.4

Change these lines in your `pubspec.yaml`, for the packages you use. Together
they are the working set for this release:

```yaml
dependencies:
  zec_wallet: ^0.0.5
  zec_wallet_ui: ^0.0.4
  zec_wallet_ui_platform: ^0.0.3
  zec_wallet_tor: ^0.0.4
```

`zec_wallet_ui` 0.0.4 is the release that accepts `zec_wallet` 0.0.5.
`zec_wallet_ui_platform` stays at 0.0.3 and `zec_wallet_tor` at 0.0.4: neither
changed in this release, so their own pub.dev pages still show the older
`zec_wallet_ui: ^0.0.3` and `zec_wallet: ^0.0.4` lines. Use the lines above,
then run `flutter pub get` and build your app again.

- The minimum Rust version is still 1.96.
- The first time 0.0.5 opens a wallet, it migrates the wallet's database. The
  migration is one-way.
- Downgrading to 0.0.4 is not supported: in our test 0.0.4 still opened a
  wallet that 0.0.5 had migrated, with the same balance and history, but the
  migration is one-way and 0.0.4 cannot send after the NU7 activation.
- If your code switches over `TxStatus`, `WalletErrorKind` or `SwapErrorKind`,
  or calls `txConfirmationIsFinal`, make the changes below.

### Breaking API changes

`TxStatus` has a new case, `cancelledByUpgrade(stoppedAt, releasedAt, phase)`:
a payment this wallet signed under the old rules that was still unmined when
the upgrade activated. No block from the activation on can carry it.
`stoppedAt` is the activation height. `releasedAt` is the height its amount is
released at, the same value as `TxSummary.expiryHeight` (`null` for a payment
built with no expiry). `phase` is a `CancelPhase` the wallet computes:
`notFinal`, `finalNotReleased`, `released` or `unknown`. Say the amount is back
only at `released`, and while it is `notFinal` do not offer to send the payment
again. An exhaustive `switch` over `TxStatus` needs an arm for it:

```dart
// 0.0.4
String label(TxStatus s) => switch (s) {
      TxStatus_Queued() => 'Queued',
      TxStatus_Pending() => 'Pending',
      TxStatus_Confirmed() => 'Confirmed',
      TxStatus_Expired() => 'Expired',
      TxStatus_Failed() => 'Failed',
      TxStatus_Unknown() => 'Unknown',
    };

// 0.0.5
String label(TxStatus s) => switch (s) {
      TxStatus_Queued() => 'Queued',
      TxStatus_Pending() => 'Pending',
      TxStatus_Confirmed() => 'Confirmed',
      TxStatus_Expired() => 'Expired',
      TxStatus_Failed() => 'Failed',
      TxStatus_CancelledByUpgrade(:final phase) => switch (phase) {
          CancelPhase.released => 'Cancelled by upgrade, amount returned',
          _ => 'Cancelled by upgrade',
        },
      TxStatus_Unknown() => 'Unknown',
    };
```

`txConfirmationIsFinal` now takes the wallet's network and the row's mined
height beside the depth, because the number of blocks that makes a payment
final changes at the activation:

```dart
// 0.0.4
bool isFinal(TxSummary tx, int depth) =>
    txConfirmationIsFinal(confirmationDepth: depth);

// 0.0.5
bool isFinal(TxSummary tx, int depth) => txConfirmationIsFinal(
      network: Network.main, // the network in your WalletConfig
      minedHeight: tx.minedHeight!, // set on every confirmed row
      confirmationDepth: depth,
    );
```

`WalletErrorKind` and `SwapErrorKind` each have a new case,
`upgradeImminent(activationHeight, blocksLeft)`, for the pause described
below. A `switch` over either one with no default arm needs a case for it.

The bridge ABI is now 12. The Dart code and the native library ship together
in this package, so this needs nothing from you; `RustLib.init` refuses a
mismatched pair with `BridgeAbiMismatch`.

### Behaviour changes

- **A payment is final after 15,000 seconds of chain time, about 4.2 hours.**
  That is 200 blocks before the activation and 600 after it, since NU7 blocks
  come every 25 seconds; it was 100 blocks. `txConfirmationIsFinal` follows
  it, and so does the wallet: a payment whose attempt expired stays
  `retryPending` until that expiry is final, and a TEX payment that was only
  partly mined is marked stranded later.
- **Reclaiming a stuck ephemeral address waits longer:** 241 blocks before the
  activation and 721 after it (it was 141).
- **A new wallet's birthday is later.** The bundled checkpoints above the
  Ironwood activation are now used, so a new wallet starts near the newest
  one and its first scan is shorter. The birthday is still never above the
  height you asked for.
- **A proposal made before the activation and sent after it** is refused with
  `WalletErrorKind.proposalStale` (RW-SEND-004). Call `propose` again. From
  the activation a proposal also goes stale sooner: about 9 minutes after
  `propose` instead of 10, because 22 blocks of 25 seconds pass before the 10
  minutes do.
- **Sending pauses before an activation.** In the last 20 blocks before a
  network upgrade, about 25 minutes, a new payment or shielding is refused
  with `WalletErrorKind.upgradeImminent` (RW-SEND-009), so it cannot be
  signed under rules that are about to end. A queued payment stays queued and
  goes out after the window the usual way: by itself in a wallet that signs
  unattended, otherwise when your app calls `authorizeParkedSend`. A
  `swapExecute` out of ZEC is refused with
  `SwapErrorKind.upgradeImminent` (RW-SWAP-017) before its quote is used, so
  an unexpired quote can still be executed after the window. A swap into ZEC
  is not refused. `authorizeParkedSend` throws the same error and leaves the
  payment queued; `reclaimEphemeralSlots` throws it too. `sweepEphemeralFunds`
  started inside the window is refused; one already running when the window
  opens stops and returns what it swept so far. `zec_wallet_ui` 0.0.4 tells
  the user "The Zcash network upgrades in about N minutes. Sending is paused
  until then so your payment can't get stuck."
- **A payment too large for one block is refused.** From the NU7 activation a
  block holds at most 330 Orchard actions, 330 Ironwood actions, 300 Sapling
  spends and outputs, and 330 of all three together (ZIP 218). A payment that
  would need more is refused at `propose` with `WalletErrorKind.proposeFailed`
  (RW-SEND-005), instead of being built as a transaction no block can include.
  In practice this hits a wallet whose balance sits in several hundred small
  notes. A smaller amount goes through, so such a payment can be sent in
  parts. A queued payment over the limits stays queued.
- **One new log line,** at DEBUG with the target `wallet.history`, when a
  payment is read as cancelled by an upgrade: "a payment signed before a
  network upgrade can no longer be mined", with the two consensus branch ids,
  the two heights and the phase. It names no transaction, amount or address.
  The device log keeps INFO and above, so `watchDeviceLog` never shows it.
  The existing WARN line `wallet.propose_refused` has a new `code` value,
  `zip218_block_limit`, for the refusal above.
- `zec_wallet_ui` 0.0.4 adds the strings for the cancelled payment and the
  pause, in all 16 languages.

### Builds this package refuses

The Rust core, and with it the `zec_wallet` plugin's build, stops with a
`compile_error!` that names the setting when any of these reaches the core's
own compile, from `RUSTFLAGS` or a cargo config:

- `--cfg zcash_unstable="nutachyon"` or `--cfg zcash_unstable="zip233"`;
- `--cfg getrandom_backend=` with `custom`, `extern_impl`, `rdrand`, `rndr`,
  `efi_rng` or `unsupported`, each of which replaces the operating system's
  random number generator;
- `--cfg secp256k1_fuzz` or `--cfg hashes_fuzz`.

0.0.4 refused none of them. `zec_wallet_tor` builds without the core and
refuses none.

### Limits

- If your build sets `panic = "abort"` and the operating system cannot supply
  random numbers while a payment is built, your app exits instead of getting
  an error. Nothing is stored or sent. Under the default `panic = "unwind"` the
  wallet stops signing and the app keeps running.
- On a server that does not report its network, sending is allowed for a grace
  of 1,152 blocks. Before NU7 those take about a day; from NU7 they take about
  8 hours, but `UnknownBranchGrace.running`'s `secsLeft` still counts them at
  75 seconds each, so it can show up to three times the real time. From NU7,
  show the smaller of `secsLeft` and `blocksLeft` × 25 seconds instead. The
  refusal itself comes on time; nothing is signed after it.

### Dependencies

- The NU7 releases of the Zcash crates: `zcash_protocol` 0.11.0, which carries
  the mainnet activation height, with `zcash_primitives` 0.31.0,
  `zcash_client_backend` 0.25.0, `zcash_client_sqlite` 0.23.0 and
  `zcash_keys` 0.17.0.
- `tokio` 1.53.3 (RUSTSEC-2026-0386). 0.0.4 ships 1.52.3, but nothing in the
  wallet's graph builds an `AsyncFd`, so 0.0.4 is not exposed.
- Released together with `zec_wallet_ui` 0.0.4.

## 0.0.4

A security release. Update: every wallet reaches its light server through
the TLS stack fixed here.

- `webpki-roots` 1.0.10 and `rustls-webpki` 0.103.16 (RUSTSEC-2026-0381 and
  -0382). The private keys of the emSign intermediates are public,
  and the old roots accepted chains through them, so an attacker on the
  network path could present a certificate the wallet trusted. Two
  connections verify against these roots: the light server, and the swap
  API, whose quote carries the deposit address.
- `rustls` 0.23.46 (RUSTSEC-2026-0346 to -0349). Only -0349 is reached, and
  it has no effect here: the `ring` provider offers no post-quantum group to
  downgrade from. The update clears the four advisories from your audit.
- The swap client no longer resumes TLS sessions. A session ticket from one
  swap's connection could be offered on the next swap's, so the swap provider
  could link two swaps meant to be unlinkable.
- The example app's README reads more plainly, and the README's
  known-limitation note names no version. Version 0.0.3 was prepared with
  these changes and never published.
- Released together with `zec_wallet_ui` 0.0.3, `zec_wallet_ui_platform`
  0.0.3, `zec_wallet_tor` 0.0.3 and `dialer-tor` 0.1.3. Each `zec_wallet_ui`
  release is built against one `zec_wallet` release, so use the versions from
  one release together.

## 0.0.2

Documentation only; the code is the same as 0.0.1.

- The README now opens with what the SDK gives an app, and introduces the
  optional companion packages: `zec_wallet_ui` (a drop-in wallet UI),
  `zec_wallet_ui_platform` (native screen protection for it) and
  `zec_wallet_tor` (Tor).
- A shorter package description to match.

## 0.0.1

Initial release. A Zcash wallet SDK for Flutter with a Rust core: seeds and
spending keys never enter Dart.

- **The `WalletHandle`**: `createGenerated`, `restore` (from a BIP39 recovery
  phrase, passphrase NFKD-normalized), `open`, `createWatchOnly`,
  `createWithHostSeed`; the lock-free `walletExists` boot probe; cold
  `snapshot` (balance, sync, transport, tip, the persisted `lastSynced`) and
  `currentAddress`; deterministic `close`.
- **Sync** over the lightwalletd/Zaino gRPC protocol: `startSync` /
  `stopSync` with the live `watchSyncStatus` stream, the bounded
  `syncFor(budgetMs:)` for a host that holds the process briefly, a
  user-switchable server list (`syncServers`, `switchSyncServer`,
  `probeSyncServer`), `rescanFrom`. `referenceSyncServers` names public
  servers only, and the package ships no server key. A host appends its own
  servers, each with its own `authHeader`/`authValue`. A user may add their
  own server with a key (`SyncServerChoice.custom(url:, key:)`). The wallet
  stores that key encrypted, erases it when another server is chosen, and
  never returns or logs it. A refused key or header is its own kind,
  `invalidEndpointAuth`.
- **Send**: ZIP-321 payment URIs, ZIP-302 memos, `propose` → `send`;
  offline-first queueing (`queueSend` and the parked-send verbs);
  `listInFlightSends`; `TxSummary.expiryHeight` and
  `SendProposal.singleRecipientZat`. A queued payment whose attempt expired
  unmined is sent again by the wallet once that expiry is past any reorg; until
  then the expired attempt reads `delivery == retryPending`. **A host must not
  call a transaction cancelled unless `delivery == null`.**
- **Shielding** (`proposeShield`), transparent-funds handling, ephemeral-address
  sweep and reclaim.
- **History** (`transactions`, `machineMemos`), `watchIncomingFunds`,
  diversified addresses (`mintDiversifiedAddress`).
- **Swaps** behind a pluggable provider port (`swapListTokens`, `swapQuote`,
  `swapExecute`, `watchSwapStatus`, `listInFlightSwaps`), with a NEAR Intents
  1Click adapter. A swap out of ZEC takes the ZEC amount paid: a request that
  fixes the foreign amount received instead is refused (`RequestInvalid`), and
  a quote whose ZEC deposit differs from the amount requested is refused
  (`QuoteOutOfBounds`). The adapter checks the provider's echo of the quote
  request against what it sent and refuses a quote that differs:
  `refundAddressMismatch` for the refund address, `requestEchoMismatch` for
  any other term (the recipient, the assets, the amount, the slippage).
- **Recovery**: `revealMnemonic` (word-list form), viewing-key export
  (`exportUfvk`), watch-only wallets; `wipe`, `wipeForce`, and
  `severCustody(config:, deadlineMs:)`, the deadline-bounded duress sever
  (the README carries the host's four obligations). `custodyDisclosure`
  reports the key store tier and `eraseAssurance` (`hardwareKeyDeleted`,
  `bestEffort`, `unknown`). It says what deleting the wallet does, never that
  the deleted key cannot come back.
- **Broadcast timing**: `broadcastJitter` waits a random 0 to 10 s before
  each broadcast by default. A window above 30 s is refused at create and
  open (`broadcastJitterTooLong`). A swap deposit never waits more than 10 s,
  and is checked against its deadline again right before each send.
- **Host transport**: a C contract (`rust/include/zec_wallet_net_dialer.h`,
  ABI 4) through which a host registers its own dialer (Tor, a proxy, a
  tunnel) and names it; the SDK opens no listener and no loopback proxy.
  Tor without a transport of your own is the optional `zec_wallet_tor`
  package.
- **Host seed custody**: a C seed port for a host that keeps the seed
  itself, bound to the proposal it signs.
- **The device log**: installed off; `setDeviceLog` / `deviceLogLevel` and
  the `watchDeviceLog` stream, under a fixed field allowlist.
- Typed errors with stable `RW-*` codes, catchable by type;
  `validateWalletConfig`, `estimateBirthday`, `maxMoneyZat`; the bridge
  self-test.
- **`StallReason.storageUnavailable`**: this device's storage was busy or
  briefly unreadable; sync retries by itself, and a host must not offer a
  restore for it (`internal` is kept for a corrupt store). Every store error
  is classified before it reaches you: a busy or locked database is
  `StoreBusy`, a full disk `DiskFull`, an I/O fault `Io`, and only damaged
  data (including a stored transaction that no longer decodes) is
  `StoreCorrupt`. A full disk ends an ephemeral sweep or reclaim with
  `DiskFull` instead of failing each item. The background sync loop shows a local stall only at the second local fault
  before a pass completes; a single `syncFor` pass reports its own at once.
- **Bridge ABI 11.** `RustLib.init` refuses a Dart half and a native library
  from different releases (`BridgeAbiMismatch`).
- A wallet runs on Android, iOS and macOS. Linux builds, but has no key
  store adapter yet, so creating or opening a wallet there fails with
  `vaultAbsent`. Windows is not yet built or run.
