/// Exact narrowing for the bridge-carried u64 diversifier index (FR-8).
///
/// Extracted from `FrbWalletSession` so the precision-loss class
/// dart2js makes SILENT is pinned by a unit test — the adapter itself is
/// untestable in `flutter test` (opaque FRB handle). The SDK's frozen
/// public-receive region tops out below 2^41, well inside every platform's
/// exact-int range (web: 2^53); the guard is one conservative bit tighter
/// (2^52) and fails LOUDLY if the frozen bound ever moves. Payload-free by
/// §5.4 — the index value never appears in the error.
int narrowDiversifierIndex(BigInt index) {
  if (index.bitLength > 52) {
    throw StateError('diversifier index outside the frozen region');
  }
  return index.toInt();
}
