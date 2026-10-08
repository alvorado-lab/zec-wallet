/// Restore-input normalization — the ONE place the host honours the load-bearing
/// BIP39 host-UI contract (spec §3.6; wallet-sdk §3.3). The SDK keeps the audited
/// `bip39` parser WHOLE and does NOT case-fold: `Mnemonic::parse_in_normalized`
/// rejects an autocapitalized or space-padded word as `InvalidMnemonic` carrying
/// its INDEX — so a CORRECT backup typed with a soft-keyboard's autocapitalization
/// (or pasted with stray whitespace) would fail to restore unless the host
/// normalizes first. That normalization is a money-reliability requirement, not a
/// nicety; it lives here, once (DRY), and BOTH the restore screen (live word
/// count) and the controller (the authoritative pre-restore pass — the chokepoint
/// that guarantees the contract regardless of caller) call it.
///
/// Deliberately NARROW: lowercase + trim each word and split on whitespace. We do
/// NOT strip accents, fold Unicode, or "correct" words — that is the audited
/// parser's job (it validates against the wordlist + checksum). Over-normalizing
/// here could mask a genuinely wrong word or, worse, silently turn one valid word
/// into another and derive a DIFFERENT wallet. Lowercase + whitespace is exactly
/// the gap between a phone keyboard's output and the wordlist's lowercase ASCII.
///
/// SAFE-WORDLIST DEPENDENCY: the "lowercase can never fold one valid word into
/// another" guarantee holds because the BIP39 ENGLISH wordlist is pure lowercase
/// ASCII `[a-z]` and Dart's locale-independent `toLowerCase()` has no
/// English-letter collisions. If a non-English wordlist is ever supported, re-audit
/// `toLowerCase()` for case-fold collisions (and the Turkish-İ class of traps)
/// before reusing this normalizer for it.
library;

/// The maximum raw restore-input length the normalizer/field will process — the
/// Dart-side half of the §4.6 size-cap-before-alloc discipline (the SDK caps the
/// at-rest phrase at `SEAL_MNEMONIC_MAX_BYTES`, but that engages only at restore;
/// it cannot see the per-keystroke live-count recompute). 1024 chars is an order
/// of magnitude above any valid phrase (24 of the 8-char-max words + whitespace is
/// ~250 chars) yet hard-bounds a pathological multi-megabyte clipboard paste on a
/// low-memory device. Pinned by a test (gate 7).
const int kMaxMnemonicInputChars = 1024;

/// The maximum number of pills the restore field will hold. Deliberately ABOVE
/// the longest valid phrase (24) so an over-paste shows an INVALID length (Submit
/// stays disabled) rather than being silently truncated to a plausible-looking
/// 24 words (which could derive the WRONG wallet). It also hard-bounds the widget
/// count so a pathological repeated paste can't grow the pill `Wrap` without
/// limit (a low-memory-device jank/OOM guard). Pinned by a test (gate 7).
const int kMaxMnemonicWords = 48;

/// Whitespace splitters — the ONE definition of "what separates recovery words"
/// (DRY): a run of any whitespace splits words; a trailing whitespace commits the
/// in-progress word. Used by both the normalizer and the pill field.
final RegExp mnemonicWhitespaceRun = RegExp(r'\s+');
final RegExp mnemonicAnyWhitespace = RegExp(r'\s');
final RegExp mnemonicTrailingWhitespace = RegExp(r'\s$');

/// Split raw restore input into candidate BIP39 words: split on any run of
/// whitespace (spaces / newlines / tabs — paste-friendly), [normalizeMnemonicWord]
/// each, and drop the empties a leading/trailing/double space would otherwise
/// produce. Pure + total; never throws.
///
/// The raw input is bounded to [kMaxMnemonicInputChars] BEFORE the split/map so
/// neither the per-keystroke recompute nor the chokepoint pass allocates without a
/// cap (the restore field also caps storage, but this protects any other caller).
/// A truncated over-long paste can only fail BIP39 validation downstream — never a
/// silent wrong-wallet — so the cap is safe.
List<String> normalizeMnemonicInput(String raw) =>
    (raw.length > kMaxMnemonicInputChars
            ? raw.substring(0, kMaxMnemonicInputChars)
            : raw)
        .split(mnemonicWhitespaceRun)
        .map(normalizeMnemonicWord)
        .where((word) => word.isNotEmpty)
        .toList(growable: false);

/// Normalize a single candidate word to the wordlist's lowercase ASCII form:
/// trim surrounding whitespace, then lowercase. Idempotent — applying it to
/// already-normalized input is a no-op, which is why the controller can re-apply
/// it defensively over a screen that already normalized.
String normalizeMnemonicWord(String word) => word.trim().toLowerCase();
