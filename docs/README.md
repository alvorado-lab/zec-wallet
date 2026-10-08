# The design record

The SDK was designed and built inside a larger project, Relim, a messenger that embeds it. This
directory carries the part of that project's design record that belongs to the SDK, at the
paths the code cites, so a reference such as `docs/specs/wallet-sdk.md §6.3` or `ADR-0552`
resolves here.

| path | what it is |
|---|---|
| `specs/wallet-sdk.md` | the SDK's spec: the Rust core, the bridge, the UI package |
| `specs/tor-plugin.md` | the optional Tor plugin (`zec_wallet_tor`, `dialer-tor`) |
| `specs/host-transport-crossing.md` | how a host carries the wallet's traffic (the dialer contract) |
| `specs/sync-server-picker.md` | choosing and checking the light server |
| `specs/ironwood-nu63-support.md` | the NU6.3 (Ironwood) upgrade: the crate wave and the sealed Orchard pool |
| `specs/app-frame.md` | the example app's frame |
| `adr/05NN-*.md` | the SDK's architecture decisions, one per file, append-only |
| `adr/0005`, `0007`, `0013`, `0014` | four earlier decisions the SDK rests on: the engine, the licence, the SDK's shape, the swap port |
| `feature-requests.md` | an index of the `FR-N` ids: requests an integrating application made of the SDK |

## References that do not resolve here

- **Other ADR ids below 0500** (for example `ADR-0026`, `ADR-0031`) and
  `docs/specs/design-refresh.md` are the embedding project's own record. The code cites them
  where it meets that project's integration; they are not published. Some comments say so
  (`Relim ADR-0031`).
- **Paths under `docs/plan/`, `docs/handoff/`, `docs/adjudication/`, `docs/reviews/`,
  `docs/research/`, and `docs/ROADMAP.md`, `docs/REVIEW.md`, `docs/arch/`** are the working
  record: build plans, review rounds, probes and hand-offs. They are not published. What a
  plan decided lands in a spec section or an ADR here.
- **Session ids** (`S290`) and stage ids (`S7`) number the working sessions and release stages
  a decision was made in. **"The founder"** is the project's owner, who made the product
  calls the records attribute to him.

## Status

The specs are living documents and describe the code at the commit this directory was
exported with. An ADR is never edited after acceptance; a later ADR supersedes it and says
so in its header.
