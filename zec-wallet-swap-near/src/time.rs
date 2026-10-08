//! Strict RFC 3339 **UTC-only** timestamp codec for the 1Click wire
//! (`deadline`, `updatedAt`, …): `YYYY-MM-DDTHH:MM:SS[.fff…]Z`.
//!
//! Why hand-rolled and not a date crate: the wire uses exactly ONE fixed
//! format, the value feeds the deposit-deadline funds gate (so it must parse
//! EXACTLY or fail typed — no lenient fallback), and a calendar dependency
//! is a poor trade for ~40 lines of exact integer arithmetic. This is
//! arithmetic, not crypto — pinned by KATs below and cross-checked by
//! property tests against the inverse.
//!
//! Deliberately rejected shapes (every byte hostile, §4.6): numeric offsets
//! (`+02:00` — the provider speaks Z; an offset is drift we want to SEE),
//! lowercase `t`/`z`, missing seconds, years outside 1970–9999.

/// Civil date → days since the Unix epoch (exact; Howard Hinnant's
/// `days_from_civil`, public-domain algorithm, valid far beyond our range).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m as i64 + 9) % 12; // Mar=0 … Feb=11
    let doy = (153 * mp + 2) / 5 + d as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Fixed-width decimal field; `None` on any non-digit or width mismatch.
fn fixed(s: &[u8], at: usize, width: usize) -> Option<u64> {
    let field = s.get(at..at + width)?;
    let mut v: u64 = 0;
    for &b in field {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v * 10 + u64::from(b - b'0');
    }
    Some(v)
}

/// Parse a strict RFC 3339 UTC timestamp to unix seconds. Fractional
/// seconds are accepted (the wire sends millis) and TRUNCATED — the funds
/// gate already subtracts a whole-seconds safety margin, so sub-second
/// precision buys nothing. `None` = malformed (caller maps to a typed
/// provider-protocol error; the raw string is never logged).
pub(crate) fn parse_rfc3339_utc(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    // shortest valid: YYYY-MM-DDTHH:MM:SSZ = 20 bytes
    if b.len() < 20 {
        return None;
    }
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let (y, m, d) = (
        fixed(b, 0, 4)?,
        fixed(b, 5, 2)? as u32,
        fixed(b, 8, 2)? as u32,
    );
    let (hh, mm, ss) = (fixed(b, 11, 2)?, fixed(b, 14, 2)?, fixed(b, 17, 2)?);
    // tail: optional ".digits", then a mandatory 'Z' ending the string
    let mut i = 19;
    if b[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == frac_start {
            return None; // "." with no digits
        }
    }
    if i + 1 != b.len() || b[i] != b'Z' {
        return None;
    }
    let y = y as i64;
    if !(1970..=9999).contains(&y) || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    // 60 rejected: RFC 3339 leap seconds don't occur in unix time, and a
    // deadline one second conservative is strictly safe
    if hh > 23 || mm > 59 || ss > 59 {
        return None;
    }
    let days = days_from_civil(y, m, d) as u64; // ≥ 0 by the year bound
    Some(days * 86_400 + hh * 3_600 + mm * 60 + ss)
}

/// Unix seconds → `YYYY-MM-DDTHH:MM:SS.000Z` (the request `deadline` field;
/// millis kept at zero to match the provider's own rendering). Exact inverse
/// of [`parse_rfc3339_utc`] on whole seconds (property-tested). Input past
/// year 9999 (a wildly corrupt device clock — not attacker-reachable; we
/// only format `now + a small window`) renders a >4-digit year, which the
/// provider and our own parser both reject — fail-closed at the next gate.
pub(crate) fn format_rfc3339_utc(unix_secs: u64) -> String {
    // civil_from_days — the exact inverse of days_from_civil (same source)
    let z = (unix_secs / 86_400) as i64 + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    let rem = unix_secs % 86_400;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        y,
        m,
        d,
        rem / 3_600,
        (rem / 60) % 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// KATs: known unix timestamps (cross-checked against `date -u`).
    #[test]
    fn rfc3339_known_answers() {
        let cases = [
            ("1970-01-01T00:00:00Z", 0),
            ("2026-06-13T00:00:00Z", 1_781_308_800),
            ("2026-06-14T12:00:00.000Z", 1_781_438_400),
            // leap-year boundary
            ("2024-02-29T23:59:59Z", 1_709_251_199),
            ("2024-03-01T00:00:00Z", 1_709_251_200),
            // non-leap century year 2100 (the /100 rule)
            ("2100-02-28T23:59:59Z", 4_107_542_399),
            ("2100-03-01T00:00:00Z", 4_107_542_400),
            ("9999-12-31T23:59:59Z", 253_402_300_799),
        ];
        for (s, expect) in cases {
            assert_eq!(parse_rfc3339_utc(s), Some(expect), "case {s}");
        }
        // fractional seconds truncate
        assert_eq!(
            parse_rfc3339_utc("2026-06-13T00:31:30.560Z"),
            Some(1_781_310_690)
        );
    }

    #[test]
    fn rfc3339_rejects_malformed_and_non_utc() {
        for s in [
            "",
            "2026-06-13",                // date only
            "2026-06-13T00:00:00",       // no Z
            "2026-06-13t00:00:00Z",      // lowercase t
            "2026-06-13T00:00:00z",      // lowercase z
            "2026-06-13T00:00:00+02:00", // offset — drift we want to SEE
            "2026-06-13T00:00:00.Z",     // empty fraction
            "2026-06-13T00:00:00.123",   // fraction, no Z
            "2026-06-13T24:00:00Z",      // hour 24
            "2026-06-13T00:60:00Z",      // minute 60
            "2026-06-13T00:00:60Z",      // leap second
            "2026-02-30T00:00:00Z",      // impossible day
            "2025-02-29T00:00:00Z",      // Feb 29 in a non-leap year
            "2100-02-29T00:00:00Z",      // Feb 29 in a non-leap CENTURY year
            "1969-12-31T23:59:59Z",      // pre-epoch
            "10000-01-01T00:00:00Z",     // year width
            "2026-06-13T00:00:00ZZ",     // trailing junk
            "2026-06-13T00:00:00Z ",     // trailing space
        ] {
            assert_eq!(parse_rfc3339_utc(s), None, "must reject {s:?}");
        }
    }

    proptest! {
        /// format∘parse is the identity on whole seconds across the full
        /// supported range — the two civil-date algorithms agree everywhere.
        #[test]
        fn rfc3339_roundtrip(secs in 0u64..=253_402_300_799) {
            let s = format_rfc3339_utc(secs);
            prop_assert_eq!(parse_rfc3339_utc(&s), Some(secs));
        }

        /// Never panics on arbitrary input.
        #[test]
        fn rfc3339_parse_never_panics(s in "\\PC{0,64}") {
            let _ = parse_rfc3339_utc(&s);
        }
    }
}
