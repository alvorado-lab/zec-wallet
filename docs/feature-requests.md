# Feature requests — index

The `FR-N` ids the code and the specs cite: requests an integrating application made
of the SDK. The full request text is the requesting project's working record and is
not published; where a request was accepted, the spec section or ADR that answers it
is the published record.

| id | request |
|---|---|
| FR-1 | Receive eventing: incoming-payment stream + tx history + memo CONTENT read |
| FR-3 | Master-seed (convergence) construction over a host seam |
| FR-4 | `RelimNetDialer` adapter + composition wiring |
| FR-5 | Tor for a standalone host: the optional plugin `zec_wallet_tor` over the shared crate `dialer-tor` (was: built-in arti runtime, `tor-builtin`) |
| FR-6 | Stable Rust embedding surface for the host |
| FR-8 | Diversified-address minting API |
| FR-10 | Unlock-state SSOT: atomic consumed-txid ledger that survives seed-restore |
| FR-11 | Mobile/NSE + cross-channel-timing constraints |
| FR-12 | Spend-time seed-supply seam (`WalletSeedPort`) for `SeedPersistence::None` |
| FR-13 | Per-wallet keychain custody namespacing (multi-identity coexistence) |
| FR-14 | Public crypto-shred (`Wallet::wipe`/`destroy_custody`) for panic-wipe |
| FR-15 | C-ABI seed-port (cross-dylib seed supply, keys-in-Rust across two native libs) |
| FR-17 | Proposal-bound seed supply (amends FR-12 / FR-15) |
| FR-18 | Authenticated / set-once seed-port registration (hardens FR-15) |
| FR-19 | Linux build: bind the wallet `.so`'s bundled SQLCipher to itself (`-Bsymbolic`) |
| FR-20 | Keep the committed `zec_wallet` FRB Dart bindings in sync with the Rust (regen + a CI guard) |
| FR-21 | `DirectTcpDialer` happy-eyeballs / IPv4 fallback (the `TorPolicy::Off` clearnet dial) |
| FR-22 | IntoZec swap quote needs a provisioned account: confirm the intended contract |
| FR-23 | Swap at every custody tier: bracketed deposit signing (sign-at-execute + parked-authorizer drain) |
| FR-24 | Offline receive for a fresh wallet: eager account import at create over the bundled anchor |
| FR-25 | Prefilled-send entry seam on `zec_wallet_ui` (ZIP-321-first) |
| FR-26 | The FR-25 prefilled entry must REPORT the outcome of the flow it opened |
| FR-27 | Machine-memo BYTES on the read side for a Dart-driven host (`transaction_memos`' binding is still deferred, and its documented shape is length-only) |
| FR-28 | Machine-memo BYTES on the COMPOSE side: the FR-25 prefill path re-composes from text-only fields, so a host envelope cannot survive to the send |
| FR-29 | A host that runs its own transport cannot route wallet traffic through it unless it embeds the SDK in its own cargo workspace, and the two shipping consumption models cannot |
| FR-30 | The wallet's failing arms say "Tor" whatever the host named its transport, and a host readiness of 0 reads as *starting* for a path the host has declared failed |
| FR-31 | Opaque wire-level isolation keys: the SEMANTIC label prefix tells a registrant which dial is a broadcast |
| FR-32 | Two arms still claim more than the SDK can attest: "Tor active" for ANY injected dialer, and "is required" on a path a `Preferred` wallet chose |
| FR-33 | The FRB pairing guard does not notice a DTO shape change, so a version-skewed consumer decodes one tag off and renders a privacy claim that is false |
| FR-34 | Raise a user notification when the wallet tells you it has switched off the private path |
| FR-35 | The SDK's device log is switched by the HOST, reflecting a choice its user can see; a host that says nothing gets OFF |
| FR-36 | The private path's state must say whether it is CARRYING, and FALL when it is not |
| FR-37 | A host can SHOW which route served the wallet: dials (and bytes) by arm, readable, since start |
| FR-38 | A wipe a native host can call INSIDE its own ordered sequence |
| FR-39 | Two wallets in one process: closing one leaves nothing of it observable to the next |
| FR-40 | A bounded sync for a background wake: "sync what you can in N seconds, then stop cleanly" |
| FR-41 | What a SEND reports when the transport is retired under it |
| FR-42 | A device-log sink for Windows and Linux |
| FR-43 | A rescan a HOST-SEED wallet can run, the seed supplied through the registered seed port |
| FR-44 | `TorState_Unanswered`'s presentation must branch on `exposure`, as `Active` already does |
| FR-46 | A tagged send's report names the amount the RECIPIENT received |
| FR-47 | A keychain call that wedges returns, typed, inside a bound the host can plan around |
| FR-48 | The total-supply bound is exported from `zec_wallet`, one home |
| FR-49 | The wallet UI follows the refreshed design, and a host can supply type, icons and surfaces |
| FR-51 | Sync whenever the app is in the foreground (`walletSyncDriveProvider`) |
| FR-53 | A duress sever that works while a straggler still holds the wallet (the hung-wallet force-sever) |
| FR-54 | `dialer-tor` recovers from a bad exit instead of reusing its circuit for ten minutes |
