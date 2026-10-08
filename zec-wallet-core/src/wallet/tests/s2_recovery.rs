//! Stage S2 `recovery` — the test author's rows (contract
//! `docs/plan/stage-2-the-hosts-lifecycle.md` §3.3, revision 3: rows 1–4, 8,
//! 9, 10, 11; the legacy rows 5–7 and 12 are dropped), written blind (IT-2a)
//! against `2067f489`. A child of `wallet::tests` for its fixtures (`cfg`,
//! `test_vault`, `create_none_then_reopen_with_port`, `put_live_proposal`), in
//! its own file so the parent's cited lines stay where their watches printed
//! them (the `host_seed_rescan` precedent).
//!
//! The fix: the BIP39 passphrase is NFKD-normalized (the audited crate's own
//! `Mnemonic::normalize_utf8_cow`, into a `Zeroizing` buffer) before PBKDF2.
//! The R03 probe in `derivation.rs` is the orchestrator's; these rows are the
//! rest of the contract.
//!
//! RED AT BASE, by design: `a_compatibility_character_passphrase_recovers_the_same_wallet`
//! (the behaviour) and `the_normalized_passphrase_copy_is_zeroized` (the
//! site). Every other row is a PIN, green at base ON PURPOSE and saying so at
//! its head: the vectors, the already-NFKD passphrase, and the three
//! host-seed rows — the fix must not reach a seed the SDK did not derive.
//! Rows 9 and 10's expected values were RECORDED by running the SDK at the
//! base commit, never computed by hand.

use super::*;
use crate::derivation::resolve_seed;
use bip39::{Language, Mnemonic};
use rusqlite::OptionalExtension;

// ── Fixtures ────────────────────────────────────────────────────────────────

/// Relim's frozen wallet sub-seed — leg 1 of their `wallet_ua_kat_from_canonical_seed`
/// (the first host's identity KAT, `GOLDEN_WALLET_SUB_SEED`):
/// `HKDF(canonical BIP39 seed, "relim/wallet/zip32-seed/v1")`, the exact 32
/// bytes that host hands this SDK as raw bytes. COPIED as hex, never imported
/// — the SDK takes no `relim-*` dependency (A1).
const RELIM_GOLDEN_WALLET_SUB_SEED_HEX: &str =
    "fc049182562048693ab56bbf61ae4caf36cf60404cb43254dc7e306e46b4b55a";

/// The mainnet default UA the SDK derives from that sub-seed — leg 2, RECORDED
/// at the base commit `2067f489` through `resolve_seed(RawBytes)` →
/// `derive_default_address`, then frozen.
const RELIM_GOLDEN_MAINNET_UA: &str = "u1vhmjrzt6yahdxx2yawnv92vg5v0yzw93t62d5ns280wjsydjwp504fh6zwwnkhsumamtq47jjhp3s2hqmluwru72wya7jmpmhp8sweuft2mkwagdetvqctdjmyg2w05aa0c2evpft4ljnh3ag9hwlk3gy06fglwnpled7vd6ygn5j0xv";
/// The same, testnet — what the host-seed create path below derives
/// (recorded from that path's `current_address` at the base commit).
const RELIM_GOLDEN_TESTNET_UA: &str = "utest1pw8lp4h6uhyc6ft6rfenukjsxceh2p7rx6mt4fgva5kdn4nzhuevf3v9nj44tlc75rsk4n8738ydvjrrdzw0ek4aknfanvf035vvs6anlwd55v8sll27gkmlnyqcfnx3h5fqcqcxj5h6rjsvw5ape9lvecckm4mz2xpac7k9xgcqea5t";
/// The #357 fingerprint row a `SeedPersistence::None` create writes for that
/// sub-seed — RECORDED raw from the aux DB at the base commit, then frozen.
const RELIM_GOLDEN_FINGERPRINT_ROW_HEX: &str =
    "4e11372eea50e17a0b51ef99d0942a7693ded33524bdcf1d0abd589af88ea2ea";

/// The public BIP39 test mnemonic (12 words, entropy 0x00×16). Never a user
/// wallet.
const ABANDON_ABOUT: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// The 24-word Trezor #1 phrase — `derivation.rs`'s own vector.
const TREZOR_24: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
                         abandon abandon abandon abandon abandon abandon abandon abandon \
                         abandon abandon abandon abandon abandon abandon abandon art";

fn sub_seed() -> Vec<u8> {
    hex::decode(RELIM_GOLDEN_WALLET_SUB_SEED_HEX).expect("the pinned sub-seed is hex")
}

/// The seed the SDK's phrase path derives for `phrase` + `passphrase`.
fn phrase_seed(phrase: &str, passphrase: &str) -> Vec<u8> {
    resolve_seed(SeedSource::mnemonic(
        phrase.to_owned(),
        Some(passphrase.to_owned()),
    ))
    .expect("a valid phrase resolves")
    .seed()
    .to_vec()
}

/// BIP39's own answer, from the audited crate's NORMALIZING `to_seed` — the
/// reference the SDK must agree with. Independent of `resolve_seed`.
fn bip39_reference(phrase: &str, passphrase: &str) -> Vec<u8> {
    Mnemonic::parse_in_normalized(Language::English, phrase)
        .expect("a valid phrase parses")
        .to_seed(passphrase)
        .to_vec()
}

/// The #357 fingerprint row, raw from the aux DB.
fn fingerprint_row(w: &Wallet) -> Option<Vec<u8>> {
    let aux = w.inner.aux_db.lock().expect("aux db mutex poisoned");
    aux.query_row(
        "SELECT fp FROM wallet_seed_fingerprint WHERE singleton = 1",
        [],
        |r| r.get(0),
    )
    .optional()
    .expect("the fingerprint row reads")
}

// ── Row 2 — the vectors ─────────────────────────────────────────────────────

/// §3.3 row 2 — VECTORS BYTE-IDENTICAL. PIN, green at base on purpose: an
/// ASCII passphrase and the empty passphrase are already NFKD, so the fix
/// must move none of these seeds. The Trezor #1 hex is `derivation.rs`'s
/// in-tree vector; the two 12-word rows are the published BIP39 answers for
/// the all-`abandon`/`about` phrase, and every row is ALSO checked against the
/// audited crate's normalizing `to_seed`, so a typo'd pin cannot pass.
#[test]
fn bip39_vectors_are_byte_identical_under_normalization() {
    let rows: [(&str, &str, &str); 3] = [
        (
            TREZOR_24,
            "TREZOR",
            "bda85446c68413707090a52022edd26a1c9462295029f2e60cd7c4f2bbd30971\
             70af7a4d73245cafa9c3cca8d561a7c3de6f5d4a10be8ed2a5e608d68f92fcc8",
        ),
        (
            ABANDON_ABOUT,
            "TREZOR",
            "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531\
             f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04",
        ),
        (
            ABANDON_ABOUT,
            "",
            "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc1\
             9a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4",
        ),
    ];
    for (phrase, passphrase, expected) in rows {
        let seed = phrase_seed(phrase, passphrase);
        assert_eq!(
            hex::encode(&seed),
            expected,
            "BIP39 vector ({passphrase:?}) moved — an ASCII/empty passphrase must derive byte-identically"
        );
        assert_eq!(
            seed,
            bip39_reference(phrase, passphrase),
            "the SDK and the audited crate disagree on an ASCII/empty passphrase"
        );
    }
    // A passphrase-less restore (`None`) is the empty passphrase.
    let none = resolve_seed(SeedSource::mnemonic(ABANDON_ABOUT.to_owned(), None))
        .expect("resolves")
        .seed()
        .to_vec();
    assert_eq!(none, phrase_seed(ABANDON_ABOUT, ""));
    // The Generate arm's empty passphrase: a generated wallet's seed is its
    // own phrase's BIP39 seed under "", so a restore of the revealed phrase
    // lands on the same wallet.
    let generated = resolve_seed(SeedSource::generate()).expect("generates");
    let phrase = generated
        .mnemonic()
        .expect("a generated wallet keeps its phrase");
    assert_eq!(generated.seed(), &bip39_reference(phrase, "")[..]);
    assert_eq!(generated.seed(), &phrase_seed(phrase, "")[..]);
}

/// UNLISTED (IT-1 +A) — a non-ASCII passphrase typed ALREADY in NFKD keeps
/// its wallet across the change. PIN, green at base on purpose: the fix moves
/// only a passphrase whose NFKD differs from its raw bytes; a user whose
/// keyboard already produced the decomposed form (`e` + U+0301, Hangul jamo)
/// must restore exactly the wallet they restored before. A fix that
/// normalized to NFC (or any form but NFKD) would move these seeds and red
/// here, while the probe — which only asks that two spellings AGREE — could
/// stay green.
#[test]
fn an_already_nfkd_passphrase_keeps_its_wallet_across_the_change() {
    for passphrase in ["cafe\u{301}", "\u{1112}\u{1161}\u{11ab}"] {
        let seed = phrase_seed(ABANDON_ABOUT, passphrase);
        let raw_pbkdf2 = Mnemonic::parse_in_normalized(Language::English, ABANDON_ABOUT)
            .expect("parses")
            .to_seed_normalized(passphrase)
            .to_vec();
        assert_eq!(
            seed, raw_pbkdf2,
            "an NFKD passphrase {passphrase:?} derived a different seed than its own bytes"
        );
        assert_eq!(seed, bip39_reference(ABANDON_ABOUT, passphrase));
    }
}

// ── Row 3 — compatibility characters ────────────────────────────────────────

/// §3.3 row 3 — COMPATIBILITY CHARACTERS. RED AT BASE: a passphrase carrying
/// a ligature (`ﬁ`, U+FB01), a circled digit (`①`, U+2460) and full-width
/// letters (`Ｚｅｃ`) is, under BIP39's NFKD, the passphrase `fi1Zec`; the
/// base derivation PBKDF2s the raw bytes and recovers a different wallet.
/// Asserted at the WALLET (the default address), not only the seed.
#[test]
fn a_compatibility_character_passphrase_recovers_the_same_wallet() {
    let compat = "\u{fb01}\u{2460}\u{ff3a}\u{ff45}\u{ff43}";
    let decomposed = "fi1Zec";
    let a = phrase_seed(ABANDON_ABOUT, compat);
    let b = phrase_seed(ABANDON_ABOUT, decomposed);
    assert_eq!(
        b,
        bip39_reference(ABANDON_ABOUT, decomposed),
        "control: the ASCII spelling is BIP39's answer"
    );
    assert_eq!(
        a,
        bip39_reference(ABANDON_ABOUT, compat),
        "the compatibility spelling must be BIP39's answer (NFKD before PBKDF2)"
    );
    assert_eq!(
        derive_default_address(Network::Main, &a).expect("derive"),
        derive_default_address(Network::Main, &b).expect("derive"),
        "one passphrase, two spellings, must recover ONE wallet"
    );
    // Each class alone, so a partial normalization (e.g. width folding only)
    // cannot pass on the others' strength.
    for (compat_one, plain_one) in [("\u{fb01}", "fi"), ("\u{2460}", "1"), ("\u{ff3a}", "Z")] {
        assert_eq!(
            phrase_seed(ABANDON_ABOUT, compat_one),
            phrase_seed(ABANDON_ABOUT, plain_one),
            "{compat_one:?} and {plain_one:?} are one BIP39 passphrase"
        );
    }
}

// ── Row 4 — the zeroize site ────────────────────────────────────────────────

/// §3.3 row 4 — THE NORMALIZED COPY IS ZEROIZED. RED AT BASE (there is no
/// normalizing call). A source scan, because the owned NFKD buffer is a local
/// the tests cannot reach: `resolve_seed` calls the crate's
/// `normalize_utf8_cow`, every owned copy it takes of the passphrase
/// (`.into_owned()`) is born inside `Zeroizing::new(…)`, and production code
/// never calls the plain `Mnemonic::to_seed` — which normalizes into a
/// `Cow::Owned` it drops un-zeroized (the HARD-C class the module doc
/// records).
#[test]
fn the_normalized_passphrase_copy_is_zeroized() {
    let src = include_str!("../../derivation.rs");
    let production = src
        .split("#[cfg(test)]\nmod tests")
        .next()
        .expect("derivation.rs has a production half");
    let start = production
        .find("pub(crate) fn resolve_seed(")
        .expect("resolve_seed exists");
    let end = production[start..]
        .find("\nfn parse_mnemonic(")
        .map(|i| start + i)
        .expect("parse_mnemonic follows resolve_seed");
    // Whitespace-free, so rustfmt's line breaks cannot hide or fake a match.
    let body: String = production[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    assert!(
        body.contains("normalize_utf8_cow("),
        "resolve_seed must NFKD the passphrase with the audited crate's normalize_utf8_cow"
    );
    let owned: Vec<usize> = body
        .match_indices(".into_owned()")
        .map(|(i, _)| i)
        .collect();
    assert!(
        !owned.is_empty(),
        "the normalized passphrase must be taken as ONE owned copy (into_owned) into a Zeroizing buffer"
    );
    for at in owned {
        let stmt_start = body[..at].rfind([';', '{']).map_or(0, |i| i + 1);
        assert!(
            body[stmt_start..at].contains("Zeroizing::new("),
            "an owned passphrase copy in resolve_seed is not born inside Zeroizing::new: `{}`",
            &body[stmt_start..at + ".into_owned()".len()]
        );
    }
    let production_nows: String = production.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        !production_nows.contains(".to_seed("),
        "production code calls the plain Mnemonic::to_seed, which drops its normalized Cow un-zeroized"
    );
    // Whether the raw passphrase reaches PBKDF2 is a data-flow question an
    // identifier scan cannot answer (a conforming build shadows `passphrase`
    // with its NFKD buffer); rows 1 and 3 answer it by behaviour.
}

// ── Rows 9–11 — a host-supplied seed is outside the change ─────────────────

/// §3.3 row 9 — A HOST-SUPPLIED SEED DERIVES THE SAME ACCOUNT, BYTE FOR BYTE.
/// PIN, green at base on purpose (recorded there): Relim's frozen 32-byte
/// wallet sub-seed, fed to the raw-bytes path, derives the pinned address —
/// through `resolve_seed` on both networks, AND through the real host-seed
/// create (`SeedPersistence::None`, the seed arriving by the port at import).
/// Closes the gap: Relim's KAT froze only leg 1 and named the SDK's FR-3
/// KAT for leg 2, which pins a different seed. A mutant that routes raw bytes
/// through any normalization reds it.
#[tokio::test]
async fn a_host_supplied_seed_derives_the_hosts_pinned_address_byte_for_byte() {
    let seed = sub_seed();
    assert_eq!(seed.len(), 32, "the host's sub-seed is 32 bytes");

    // The raw-bytes branch hands the bytes through untouched, fresh or not.
    for source in [
        SeedSource::raw_bytes(seed.clone()).expect("valid"),
        SeedSource::raw_bytes_fresh(seed.clone()).expect("valid"),
    ] {
        let payload = resolve_seed(source).expect("resolves");
        assert_eq!(
            payload.seed(),
            &seed[..],
            "raw bytes are the seed, byte for byte"
        );
        assert!(payload.mnemonic().is_none(), "no phrase for a host seed");
        assert_eq!(
            derive_default_address(Network::Main, payload.seed()).expect("mainnet"),
            RELIM_GOLDEN_MAINNET_UA,
            "the host's pinned mainnet address moved"
        );
        assert_eq!(
            derive_default_address(Network::Test, payload.seed()).expect("testnet"),
            RELIM_GOLDEN_TESTNET_UA,
            "the host's pinned testnet address moved"
        );
    }

    // The same bytes through the host-seed create → port → import.
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = Arc::new(crate::seed::testing::StagedSeedPort::new(seed.clone()));
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        &seed,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(testnet_birthday())
        .await
        .expect("import via port");
    assert_eq!(port.pulls(), 1, "the import pulled the host seed once");
    assert_eq!(
        w.current_address().await.expect("address").encoded(),
        RELIM_GOLDEN_TESTNET_UA,
        "the host-seed wallet's address is the pinned one"
    );
    w.close().await.expect("close");
}

/// §3.3 row 10 — THE FINGERPRINT OF EVERY EXISTING HOST-SEED WALLET STILL
/// MATCHES. PIN, green at base on purpose: the fixture is the #357
/// fingerprint row a host-seed create writes for Relim's sub-seed, RECORDED
/// raw at the base commit — "a store provisioned at the base, its row as
/// written then" — so a change that moves the fingerprint's input for the
/// port arm (re-derives, re-normalizes or re-hashes the port bytes) reds the
/// first assertion. Then the wallet opens, imports (the #357 verify runs on
/// the port's bytes) and gets past every seed gate of a sign with the same
/// bytes: never `SeedMismatch`.
#[tokio::test]
async fn a_host_seed_wallet_created_before_the_change_still_matches_its_fingerprint() {
    let seed = sub_seed();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = Arc::new(crate::seed::testing::StagedSeedPort::new(seed.clone()));
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        &seed,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    assert_eq!(
        fingerprint_row(&w).map(hex::encode).as_deref(),
        Some(RELIM_GOLDEN_FINGERPRINT_ROW_HEX),
        "the #357 row for the host's seed is not the row the base commit wrote — \
         every existing host-seed wallet would refuse its own host's seed"
    );
    {
        let aux = w.inner.aux_db.lock().expect("aux db mutex poisoned");
        assert_eq!(
            crate::seed_fingerprint::verify(&aux, &seed),
            crate::seed_fingerprint::Verdict::Match,
            "the port's bytes verify against the stored row"
        );
    }
    // Opens and imports (the sync's first act) with the port's bytes.
    w.import_account(testnet_birthday())
        .await
        .expect("a host-seed wallet imports through its port — no SeedMismatch");
    assert_eq!(
        w.current_address().await.expect("address").encoded(),
        RELIM_GOLDEN_TESTNET_UA
    );
    w.close().await.expect("close");

    // A fresh open with the same port bytes, and a sign past every seed gate:
    // the unsynced handle fail-closes at the anchor guard (`ProposalStale`),
    // AFTER the pull and B1 — never `SeedMismatch`.
    let port2 = Arc::new(crate::seed::testing::StagedSeedPort::new(seed.clone()));
    let w2 = Wallet::open_with_vault_and_seed_port(
        cfg(dir.path(), Network::Test, SeedPersistence::None),
        Arc::clone(&vault),
        Some(port2.clone() as Arc<dyn WalletSeedPort>),
        None,
    )
    .await
    .expect("reopen with the same port bytes");
    assert_eq!(
        fingerprint_row(&w2).map(hex::encode).as_deref(),
        Some(RELIM_GOLDEN_FINGERPRINT_ROW_HEX),
        "an open never rewrites the fingerprint row"
    );
    let id = put_live_proposal(&w2, SpendBinding::mint());
    match w2.sign_proposal(id).await {
        Err(WalletError::ProposalStale) => {}
        Err(WalletError::SeedMismatch) => {
            panic!("the host's own seed was refused as a mismatch after the change")
        }
        other => panic!("expected the unsynced anchor guard's ProposalStale, got {other:?}"),
    }
    assert_eq!(port2.pulls(), 1, "the sign pulled the port once");
    w2.close().await.expect("close");
}

/// §3.3 row 11 — A HOST-SEED RESTORE OR RESCAN NEVER TRIES A SECOND
/// DERIVATION. PIN (revision 3), green at base on purpose: an EMPTY host-seed
/// wallet — the state that would have triggered the dropped legacy fallback —
/// restores (imports at a birthday) and rescans with the port pulled exactly
/// once per import, the SAME account before and after, and the wallet empty.
#[tokio::test]
async fn a_host_seed_restore_or_rescan_never_tries_a_second_derivation() {
    use crate::provision::testing::FakeOracle;
    const ANCHOR: u64 = 280_049;
    const TIP: u64 = 280_080;
    const RESCAN_H: u32 = 280_019;

    let seed = sub_seed();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = Arc::new(crate::seed::testing::StagedSeedPort::new(seed.clone()));
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        &seed,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    assert_eq!(port.pulls(), 0, "no pull before a key-deriving op");

    // The restore: an import at a birthday.
    let birthday = account::birthday_from_treestate(sync::testing::chain_tree_state(ANCHOR))
        .expect("the anchor decodes");
    w.import_account(birthday).await.expect("restore via port");
    assert_eq!(port.pulls(), 1, "the restore pulled the port exactly once");
    let before = w
        .current_address()
        .await
        .expect("address")
        .encoded()
        .to_owned();
    assert_eq!(
        before, RELIM_GOLDEN_TESTNET_UA,
        "the restore derived the host's account"
    );
    let mut chain = sync::testing::FakeChain::to_tip(TIP);
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the restore's pass scans");
    assert_eq!(
        w.balance().await.expect("balance").total.zat(),
        0,
        "an empty wallet"
    );
    assert_eq!(
        port.pulls(),
        1,
        "the scan of an empty wallet asked the port nothing more"
    );

    // The rescan: asks nothing at the rebuild, one pull at the re-import.
    let w = w
        .rescan_from_with_vault(Some(BlockHeight::new(RESCAN_H)), Arc::clone(&vault))
        .await
        .expect("the host-seed wallet rescans");
    assert_eq!(port.pulls(), 1, "the rescan itself pulls nothing");
    let mut oracle = FakeOracle::honest(Network::Test, TIP);
    w.provision_account(&mut oracle, w.inner.birthday)
        .await
        .expect("the first sync after the rescan re-imports through the port");
    assert_eq!(
        port.pulls(),
        2,
        "exactly one pull for the re-import — no second derivation attempt"
    );
    assert_eq!(
        w.current_address().await.expect("address").encoded(),
        before,
        "the rescan re-imported the SAME account"
    );
    let mut chain2 = sync::testing::FakeChain::to_tip(TIP);
    w.sync_once(&mut chain2, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the post-rescan pass scans");
    assert_eq!(
        w.balance().await.expect("balance").total.zat(),
        0,
        "still empty"
    );
    assert_eq!(
        port.pulls(),
        2,
        "the post-rescan scan asked the port nothing more"
    );
    w.close().await.expect("close");
}
