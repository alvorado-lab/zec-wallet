/// Human-readable chain names for the provider's short chain codes (`eth`, `btc`,
/// …). The 1Click token list speaks codes; a non-crypto user reads "Ethereum",
/// not "ETH", and "BTC on Bitcoin" not the nonsensical "BTC on BTC". This matches
/// the convention the curated OutOfZec list already uses ("USDC on Ethereum").
///
/// Unknown chains fall back to the upper-cased code — honest (we never guess a
/// pretty name we don't know), and the swap still works (the code is the wire id).
library;

const Map<String, String> _chainNames = {
  'btc': 'Bitcoin',
  'eth': 'Ethereum',
  'near': 'NEAR',
  'sol': 'Solana',
  'arb': 'Arbitrum',
  'base': 'Base',
  'pol': 'Polygon',
  'matic': 'Polygon',
  'avax': 'Avalanche',
  'bsc': 'BNB Chain',
  'bnb': 'BNB Chain',
  'op': 'Optimism',
  'ton': 'TON',
  'xrp': 'XRP Ledger',
  'doge': 'Dogecoin',
  'ltc': 'Litecoin',
  'bch': 'Bitcoin Cash',
  'tron': 'Tron',
  'trx': 'Tron',
  'zec': 'Zcash',
  'gnosis': 'Gnosis',
  'cardano': 'Cardano',
  'ada': 'Cardano',
  'stellar': 'Stellar',
  'xlm': 'Stellar',
  'sui': 'Sui',
  'apt': 'Aptos',
  'aptos': 'Aptos',
};

/// The display name for a provider chain code (case-insensitive). Falls back to
/// the upper-cased code for any chain we don't have a friendly name for.
String chainDisplayName(String chain) =>
    _chainNames[chain.toLowerCase()] ?? chain.toUpperCase();
