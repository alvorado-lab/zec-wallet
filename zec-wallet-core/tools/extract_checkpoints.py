#!/usr/bin/env python3
"""Deterministic checkpoint extraction — CHECKPOINTS.md steps 1-6, both slices.

Emits src/checkpoints/data.rs and prints the two canonical SHA-256 provenance
hashes.  Proven by regenerating the committed file byte-for-byte at the pinned
commit before it is trusted at any newer one.
"""
import hashlib
import json
import re
import struct
import subprocess
import sys
from pathlib import Path

ACTIVATION = {"mainnet": 419200, "testnet": 280000}

# Every field we take from upstream is written RAW into a Rust string literal by
# `emit`. Without these, one `"` or `\` or `];` in an upstream JSON is arbitrary
# Rust source compiled into a wallet binary that holds keys — and NEITHER pinned
# hash would catch it, because both are computed over the poisoned bytes in the
# same commit as the data. Validate the charset here, at the boundary, so the
# emitter cannot be turned into a code generator for someone else's source.
HEX64 = re.compile(r"\A[0-9a-f]{64}\Z")
FRONTIER_HEX = re.compile(r"\A[0-9a-f]{0,8192}\Z")

# Upstream's JSON shape, pinned so SCHEMA GROWTH IS LOUD. `ironwoodTree` is the
# third shielded pool that appeared upstream in the 2026-09-06 refresh (13
# mainnet rows from 3429810, 25 testnet rows from 4134750). It was DELIBERATELY
# DROPPED until Phase B, because `zcash_client_backend` 0.23.0's `TreeState`
# modelled sapling + orchard only and the pool could not be conveyed to the
# decoder at all. 0.24.0 models it (`TreeState.ironwood_tree`, proto tag 7, and
# a REQUIRED `final_ironwood_tree` on `ChainState::new`), so as of the Ironwood
# pin wave we carry it. That matters more than a field: `TreeState::
# ironwood_tree()` decodes an ABSENT field to an EMPTY tree **silently**, so a
# bundle that still dropped it would compile, run, and anchor every wallet
# provisioned at those heights against a pool the chain already has — the same
# wrong-position class the orchard frontier tests exist to kill.
REQUIRED_KEYS = {"network", "height", "hash", "time", "saplingTree"}
OPTIONAL_KEYS = {"orchardTree", "ironwoodTree"}
NETWORK_FIELD = {"mainnet": "main", "testnet": "test"}
# The REAL NU5 activation heights (zcash_protocol::consensus, verified against
# zcash_protocol-0.9.0 consensus.rs:492/526). Do not substitute "the last
# checkpoint row below NU5" — those are 1680000/1840000, which happen to satisfy
# the assertion below only because the sole row in each gap IS the activation
# height itself. A future bundle with a row in 1680001..1687103 would abort.
NU5_ACTIVATION = {"mainnet": 1687104, "testnet": 1842420}
# The Ironwood / NU6.3 activation heights — the same boundary contract as NU5
# above, one pool later. LITERALS for the reason `checkpoints/mod.rs`'s test
# module carries them as literals: this emitter reads JSON, not `zcash_protocol`.
# `checkpoint_activation_heights_match_zcash_protocol` is what binds the Rust
# side to the crate constants once `Nu6_3` is in `KNOWN_UPGRADES`.
IRONWOOD_ACTIVATION = {"mainnet": 3428143, "testnet": 4134000}


def load(net_dir: Path, net: str):
    rows = []
    for p in sorted(net_dir.glob("*.json")):
        d = json.loads(p.read_text())

        keys = set(d)
        missing = REQUIRED_KEYS - keys
        unknown = keys - REQUIRED_KEYS - OPTIONAL_KEYS
        assert not missing, f"{p.name}: missing required key(s) {sorted(missing)}"
        assert not unknown, (
            f"{p.name}: UNKNOWN upstream key(s) {sorted(unknown)} — upstream's schema "
            f"grew. Decide what the new field means for the bundled anchor before "
            f"extracting; do not drop it silently."
        )

        # The directory is not a network binding — a stray testnet file in the
        # mainnet dir would merge silently whenever it sorts monotonically,
        # putting a testnet hash and frontier in the mainnet trusted anchors.
        assert d["network"] == NETWORK_FIELD[net], (
            f"{p.name}: network is {d['network']!r}, expected "
            f"{NETWORK_FIELD[net]!r} for the {net} directory"
        )

        block_hash = d["hash"]
        sapling = d["saplingTree"]
        orchard = d.get("orchardTree", "") or ""
        ironwood = d.get("ironwoodTree", "") or ""
        assert HEX64.match(block_hash), f"{p.name}: hash is not 64 lowercase hex chars"
        for name, v in (
            ("saplingTree", sapling),
            ("orchardTree", orchard),
            ("ironwoodTree", ironwood),
        ):
            assert FRONTIER_HEX.match(v), f"{p.name}: {name} is not bounded lowercase hex"

        rows.append(
            (int(d["height"]), int(d["time"]), block_hash, sapling, orchard, ironwood)
        )
    rows.sort(key=lambda r: r[0])

    # Step 4 invariant: heights AND times strictly increasing, first row is activation.
    assert rows, f"{net}: no checkpoint JSONs found"
    assert rows[0][0] == ACTIVATION[net], f"{net}: first row {rows[0][0]} != activation"
    for a, b in zip(rows, rows[1:]):
        assert b[0] > a[0], f"{net}: height not strictly increasing at {a[0]} -> {b[0]}"
        assert b[1] > a[1], f"{net}: time not strictly increasing at {a[0]} -> {b[0]}"
    # The Orchard frontier is ABSENT below the NU5 activation and PRESENT from
    # the activation onward (the activation row itself carries the empty tree
    # "000000", which is present-but-empty — not the same as absent).
    for h, _t, _hash, _sap, orch, _iw in rows:
        if h < NU5_ACTIVATION[net]:
            assert orch == "", f"{net}: orchard frontier present below NU5 at {h}"
        else:
            assert orch != "", f"{net}: orchard frontier absent at/after NU5 at {h}"
    # Same contract for the Ironwood frontier, one upgrade later. This is not
    # decoration: an absent field decodes to an EMPTY tree with no error, so
    # "missing at a height where the pool is live" is precisely the failure that
    # never announces itself. Asserting it here means a refresh that regresses
    # the field aborts the extraction instead of shipping silent zeros.
    for h, _t, _hash, _sap, _orch, iw in rows:
        if h < IRONWOOD_ACTIVATION[net]:
            assert iw == "", f"{net}: ironwood frontier present below NU6.3 at {h}"
        else:
            assert iw != "", f"{net}: ironwood frontier absent at/after NU6.3 at {h}"
    return rows


def time_hash(mainnet, testnet):
    """MAINNET then TESTNET, each row = u32 LE height || u64 LE time."""
    buf = bytearray()
    for rows in (mainnet, testnet):
        for h, t, *_ in rows:
            buf += struct.pack("<I", h) + struct.pack("<Q", t)
    return hashlib.sha256(bytes(buf)).hexdigest()


def treestate_hash(mainnet, testnet):
    """MAINNET_TREESTATES then TESTNET_TREESTATES, each row = u32 LE height ||
    for each of (hash, saplingTree, orchardTree, ironwoodTree): u32 LE byte-len
    || ASCII hex.

    The canonical form gained `ironwoodTree` with the Ironwood pin wave, so this
    hash necessarily differs from the pre-Phase-B pin over the SAME upstream
    commit. That is the intended signal: the bundle's content really did change.
    """
    buf = bytearray()
    for rows in (mainnet, testnet):
        for h, _t, blockhash, sap, orch, iw in rows:
            buf += struct.pack("<I", h)
            for s in (blockhash, sap, orch, iw):
                b = s.encode("ascii")
                buf += struct.pack("<I", len(b)) + b
    return hashlib.sha256(bytes(buf)).hexdigest()


HEADER = """//! Bundled checkpoint anchors (spec §3.6 + §3.2f). GENERATED — do not hand-edit;
//! see `CHECKPOINTS.md` for the reproducible-provenance discipline (the
//! `checkmate`-generated set the ECC mobile SDKs ship; this is the exact data
//! Zashi/Zodl use).
//!
//! Source: Electric-Coin-Company/zcash-android-wallet-sdk
//!   sdk-lib/src/main/assets/co.electriccoin.zcash/checkpoint/{{mainnet,testnet}}/*.json
//!   commit {commit}
//!
//! TWO row-aligned slices of the SAME source rows (one source of truth).
//! `MAINNET` / `TESTNET` carry `(height, time)` for `estimate_birthday` (§3.6).
//! `MAINNET_TREESTATES` / `TESTNET_TREESTATES` carry `(height, block_hash_hex,
//! sapling_frontier_hex, orchard_frontier_hex, ironwood_frontier_hex)` for
//! `bundled_treestate` — the TRUSTED birthday anchor (§3.2f, iv-c). Each slice
//! carries its OWN pinned content hash (the §4.6 M2 integrity gate).
//!
//! Invariant (verified at extraction AND re-checked at load, §3.6): heights and
//! times STRICTLY increasing; first row == the network's Sapling activation;
//! the two slices are row-aligned (`treestate_and_time_slices_are_row_aligned`).
"""


def emit(mainnet, testnet, commit: str) -> str:
    # One string per top-level item; a blank line separates them (and the header).
    sections = [HEADER.format(commit=commit)]

    for name, rows in (("MAINNET", mainnet), ("TESTNET", testnet)):
        body = "".join(f"    ({h}, {t}),\n" for h, t, *_ in rows)
        sections.append(
            f"/// {name.capitalize()} (height, unix_time), Sapling activation first.\n"
            f"pub(super) const {name}: &[(u32, u64)] = &[\n"
            f"{body}"
            f"];\n"
        )

    for name, base, rows in (
        ("MAINNET_TREESTATES", "MAINNET", mainnet),
        ("TESTNET_TREESTATES", "TESTNET", testnet),
    ):
        body = "".join(
            f'    ({h}, "{bh}", "{sap}", "{orch}", "{iw}"),\n'
            for h, _t, bh, sap, orch, iw in rows
        )
        sections.append(
            f"/// {base.capitalize()} `(height, block_hash_hex, sapling_frontier_hex, "
            f"orchard_frontier_hex, ironwood_frontier_hex)`,\n"
            f"/// row-aligned with `{base}`. Orchard frontier ABSENT below the "
            f"NU5 activation (height < {NU5_ACTIVATION[base.lower()]});\n"
            f"/// Ironwood frontier ABSENT below the NU6.3 activation "
            f"(height < {IRONWOOD_ACTIVATION[base.lower()]}).\n"
            f"#[rustfmt::skip]\n"
            f"pub(super) const {name}: &[(u32, &str, &str, &str, &str)] = &[\n"
            f"{body}"
            f"];\n"
        )

    return "\n".join(sections)


def main():
    # EVERY check in this file — the charset guards that stop upstream JSON being
    # written as arbitrary Rust source, the unknown-key guard, the network binding,
    # monotonicity, and both activation boundaries — is an `assert`. `python -O` or
    # `PYTHONOPTIMIZE=1` in the environment removes all of them, and the tool then
    # runs to completion and prints plausible hashes over unvalidated bytes. NEITHER
    # pinned hash catches that: both are computed by this same run, over the same
    # poisoned bytes, in the same commit as the data. Refuse to run instead.
    if not __debug__:
        raise SystemExit(
            "extract_checkpoints: refusing to run with assertions disabled "
            "(python -O / PYTHONOPTIMIZE). Every input validation in this file is "
            "an `assert`; without them this emitter writes unvalidated upstream "
            "bytes straight into a Rust source file that is compiled into a wallet "
            "binary holding keys."
        )

    src = Path(sys.argv[1])
    commit = sys.argv[2]
    dest = Path(sys.argv[3]) if len(sys.argv) > 3 else None

    # The commit is written into data.rs's header — the anchor a future auditor
    # re-clones from — and it is covered by NEITHER pinned hash (both hash row
    # data only). Left as free text it can say anything with CI still green, so
    # bind it to the tree we are actually reading.
    head = subprocess.run(
        ["git", "-C", str(src), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    assert head == commit, (
        f"provenance mismatch: the source tree is at {head}, but the commit "
        f"argument says {commit}. The header line must name the tree it came from."
    )

    mainnet = load(src / "mainnet", "mainnet")
    testnet = load(src / "testnet", "testnet")

    th = time_hash(mainnet, testnet)
    tsh = treestate_hash(mainnet, testnet)
    print(f"commit          {commit}")
    print(f"mainnet rows    {len(mainnet)}  {mainnet[0][0]} @ {mainnet[0][1]} .. {mainnet[-1][0]} @ {mainnet[-1][1]}")
    print(f"testnet rows    {len(testnet)}  {testnet[0][0]} @ {testnet[0][1]} .. {testnet[-1][0]} @ {testnet[-1][1]}")
    print(f"time-slice      sha256 {th}")
    print(f"treestate-slice sha256 {tsh}")

    if dest:
        dest.write_text(emit(mainnet, testnet, commit))
        print(f"wrote           {dest} ({dest.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
