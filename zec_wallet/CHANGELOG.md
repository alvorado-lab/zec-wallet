# Changelog

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
