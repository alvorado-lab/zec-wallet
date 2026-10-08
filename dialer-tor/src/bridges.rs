//! Bridge lines: the second hostile input this crate takes, and the only one a
//! user types.
//!
//! A bridge is an entry point the public directory cannot supply, so its text
//! comes from outside the app entirely — a website, a mail autoresponder, a
//! friend. Everything here is size-capped BEFORE it is split or parsed, and no
//! refusal ever echoes what was pasted.
//!
//! # Why a refusal is a CLASS and never a `Display`
//!
//! `BridgeParseError`'s own messages interpolate the offending word: six of the
//! variants this build can produce carry `{word:?}`, and a bridge line's words
//! are its address and its fingerprints. Rendering one of those strings would
//! put a censored user's entry point into a UI string and into every log that
//! string reaches — and device and CI logs are public (`adbd` writes argv). So
//! this module's only output on the refusal path is [`bridge_class`]'s frozen
//! label, and the sentence is written where the copy lives.
//!
//! # What this build does NOT have
//!
//! `pt-client` is off (the workspace `arti-client` entry), so a bridge line
//! naming a pluggable transport is refused rather than silently downgraded —
//! upstream's own `PluggableTransportsNotSupported`, surfaced as
//! [`CLASS_PT_UNSUPPORTED`]. A bridge configured here is an UNLISTED entry
//! point, not a camouflaged one; the two are different harms and the copy that
//! says so is the caller's.

use arti_client::config::{BridgeConfigBuilder, BridgeParseError};

/// How many bridge lines one paste may carry.
///
/// The unit is BRIDGE LINES, not bytes: arti's configuration is a list of
/// bridges and every cost here — a parse, a stored line, a guard sample entry —
/// is per line. Tor's own distributors hand out three at a time (BridgeDB's
/// response, Tor Browser's built-in sets), so eight is more than double what a
/// real user is ever given and still a number a person could type.
pub const MAX_BRIDGE_LINES: usize = 8;

/// The longest one bridge line may be, in bytes.
///
/// A vanilla line is `ADDRESS:PORT FINGERPRINT` — 21 + 1 + 40 for the RSA
/// fingerprint, under 80 bytes. The longest real form is an obfs4 line, which
/// adds a 70-byte `cert=` and an `iat-mode=`: ~180 bytes. 256 is the honest
/// worst case rounded up, and it refuses a line that is really a paragraph.
pub const MAX_BRIDGE_LINE_BYTES: usize = 256;

/// Whole-paste ceiling, applied before the text is split or parsed.
///
/// Symbolized rather than written as a number: moving either
/// component moves this with it.
///
/// **The party who sets the input is not the user.** A bridge line arrives from
/// wherever the user was told to get one, so the adversarial case is a paste the
/// user was handed. What they gain by making it enormous is not a parse cost —
/// it is a DURABLE one: the accepted text is stored and re-read and re-parsed on
/// every unlock, ahead of the messenger being usable, and the store's own
/// ceiling is 8 MiB. This bound is what keeps that quantity a person-sized one.
///
/// **Arti bounds none of this itself** — the first consumer's design recorded
/// that reading as owed, and it is: `BridgeConfigBuilder`'s `FromStr` splits on
/// whitespace and allocates a `String` per offending word with no length,
/// line-count or total bound anywhere in `tor-guardmgr` 0.45.0's
/// `src/bridge/config.rs`. So this is not a second bound over an upstream one;
/// there is no upstream one.
pub const MAX_BRIDGE_CONFIG_BYTES: usize = MAX_BRIDGE_LINES * MAX_BRIDGE_LINE_BYTES;

/// The whole paste is longer than [`MAX_BRIDGE_CONFIG_BYTES`].
pub const CLASS_CONFIG_TOO_LONG: &str = "bridge-config-too-long";
/// More than [`MAX_BRIDGE_LINES`] non-blank lines.
pub const CLASS_TOO_MANY_LINES: &str = "bridge-too-many-lines";
/// One line is longer than [`MAX_BRIDGE_LINE_BYTES`].
pub const CLASS_LINE_TOO_LONG: &str = "bridge-line-too-long";
/// A bridge line naming a pluggable transport, on a build whose `pt-client` is
/// off. The arm where a user meets this build's stated limit rather than an
/// error nobody sees; its sentence says which half of the capability exists and
/// offers no download.
pub const CLASS_PT_UNSUPPORTED: &str = "bridge-pluggable-transport-unsupported";
/// The `#[non_exhaustive]` tail: a refusal this build has no sentence for. It
/// names the state rather than the nearest neighbour's meaning.
pub const CLASS_UNUSABLE: &str = "bridge-line-unusable";

/// One frozen label per refusal, mapped 1:1 from upstream's variant.
///
/// Nothing is re-grouped and no Relim taxonomy is minted: this is the mapping
/// the first consumer's design said is owed, and the wildcard is compulsory
/// because the upstream enum is `#[non_exhaustive]`.
///
/// The three `#[cfg(feature = "pt-client")]` variants are deliberately absent —
/// naming one would not compile on this build, which is the check. The guard
/// that keeps it that way is `tests/carve_out.rs`, which refuses `tor-ptmgr` in
/// the lockfile.
#[must_use]
pub fn bridge_class(err: &BridgeParseError) -> &'static str {
    match err {
        BridgeParseError::Empty => "bridge-line-empty",
        BridgeParseError::InvalidPtOrAddr { .. } => "bridge-invalid-transport-or-address",
        BridgeParseError::InvalidIpAddrOrPt { .. } => "bridge-invalid-address",
        BridgeParseError::InvalidIdentityOrParameter { .. } => "bridge-invalid-identity",
        BridgeParseError::MultipleIdentitiesOfSameType { .. } => "bridge-duplicate-identity",
        BridgeParseError::UnsupportedIdentityType { .. } => "bridge-unsupported-identity-type",
        BridgeParseError::UnsupportedChannelMethod { .. } => "bridge-unsupported-channel-method",
        BridgeParseError::DirectParametersNotAllowed => "bridge-direct-parameters-not-allowed",
        BridgeParseError::NoRsaIdentity => "bridge-no-rsa-identity",
        BridgeParseError::PluggableTransportsNotSupported { .. } => CLASS_PT_UNSUPPORTED,
        BridgeParseError::BridgesNotSupported => "bridge-support-disabled",
        _ => CLASS_UNUSABLE,
    }
}

/// Bridge lines that have been size-checked and parsed.
///
/// Not `Debug`, not `Display`, not `Clone`: it holds parsed bridge addresses and
/// relay fingerprints, which are exactly the metadata a censor wants and
/// exactly what a stray `{:?}` would publish. `Default` is the no-bridge case
/// and is what a user who supplies nothing gets — arti's own directory
/// bootstrap, untouched.
#[derive(Default)]
pub struct BridgeLines {
    parsed: Vec<BridgeConfigBuilder>,
}

impl BridgeLines {
    /// Size-cap, split, and parse. `Err` is the frozen refusal class.
    ///
    /// Blank lines and `#` comments are dropped before anything is counted:
    /// people paste what a distributor emailed them, and a paste that is three
    /// bridges and two blank lines is three bridges.
    ///
    /// Empty (or all-blank) text is NOT a refusal — it is the no-bridge case,
    /// which is the default and must stay reachable so that *carries a bridge*
    /// never becomes *requires a bridge*.
    ///
    /// # Errors
    ///
    /// The class label, never a message: see this module's header.
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        // BEFORE the split, so nothing allocates proportional to what was
        // pasted. `len()` is bytes and the cap is in bytes.
        if text.len() > MAX_BRIDGE_CONFIG_BYTES {
            return Err(CLASS_CONFIG_TOO_LONG);
        }
        let mut parsed = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            // BEFORE the blank/comment skip, so no line escapes the per-line
            // bound by wearing a `#`.
            if line.len() > MAX_BRIDGE_LINE_BYTES {
                return Err(CLASS_LINE_TOO_LONG);
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if parsed.len() == MAX_BRIDGE_LINES {
                return Err(CLASS_TOO_MANY_LINES);
            }
            parsed.push(line.parse::<BridgeConfigBuilder>().map_err(|e| {
                // The class, and only the class — `e` itself echoes the line.
                tracing::warn!(target: "dialer_tor::censor", class = bridge_class(&e), "a pasted bridge line was refused");
                bridge_class(&e)
            })?);
        }
        Ok(Self { parsed })
    }

    /// How many bridges this configures. Zero is the ordinary default.
    #[must_use]
    pub fn len(&self) -> usize {
        self.parsed.len()
    }

    /// No bridges — arti's own directory bootstrap.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parsed.is_empty()
    }

    pub(crate) fn into_parsed(self) -> Vec<BridgeConfigBuilder> {
        self.parsed
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_not_impl_any;

    use super::*;

    assert_not_impl_any!(BridgeLines: core::fmt::Debug, core::fmt::Display, Clone);

    /// A well-formed vanilla bridge line. Address and fingerprint are the
    /// documentation-reserved `192.0.2.0/24` and a fabricated hex digest — this
    /// is not a real bridge and could never reach one.
    pub(crate) const VANILLA: &str = "192.0.2.55:38114 316E643333645F6D79216558614D3931657A5F5F";

    /// Every refusal a paste can actually reach on this feature set, driven
    /// through the REAL upstream parser rather than by constructing the enum.
    ///
    /// The list is not a guess: it is every `BridgeParseError` construction site
    /// in `tor-guardmgr` 0.45.0 (`src/bridge/config.rs` and
    /// `src/bridge_disabled.rs` — the only two files in the whole registry that
    /// construct one), minus the sites inside a `#[cfg(feature = "pt-client")]`
    /// block and minus the module gated `#[cfg(not(feature = "bridge-client"))]`.
    ///
    /// `Empty` is reachable only through the `Bridge` KEYWORD with nothing after
    /// it: an empty paste is the no-bridge default and never reaches the parser
    /// at all, which is the row below.
    fn reachable() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Bridge", "bridge-line-empty"),
            ("1.2.3.4:notaport ABCD", "bridge-invalid-address"),
            ("obfs4 192.0.2.55:38114", CLASS_PT_UNSUPPORTED),
            (
                "192.0.2.55:38114 316E643333645F6D79216558614D3931657A5F5F \
                 316E643333645F6D79216558614D3931657A5F5F",
                "bridge-duplicate-identity",
            ),
            (
                "192.0.2.55:38114 zzzznotafingerprint",
                "bridge-invalid-identity",
            ),
            (
                "192.0.2.55:38114 316E643333645F6D79216558614D3931657A5F5F iat-mode=0",
                "bridge-direct-parameters-not-allowed",
            ),
            ("192.0.2.55:38114", "bridge-no-rsa-identity"),
        ]
    }

    /// The taxonomy a user can MEET, enumerated from upstream's producers and
    /// driven one line at a time. It is what decides how many sentences the copy
    /// owes, so a row that silently merged two would under-budget it.
    #[test]
    fn every_reachable_bridge_refusal_has_its_own_class_and_none_falls_to_the_tail() {
        let mut seen = std::collections::BTreeSet::new();
        for (line, want) in reachable() {
            let got = BridgeLines::parse(line)
                .err()
                .unwrap_or_else(|| panic!("{line:?} was ACCEPTED; it must not parse"));
            assert_eq!(got, want, "{line:?} reached the wrong class");
            assert_ne!(
                got, CLASS_UNUSABLE,
                "{line:?} fell to the non-exhaustive tail, so the mapping lost a variant"
            );
            seen.insert(got);
        }
        assert_eq!(
            seen.len(),
            reachable().len(),
            "two reachable refusals collapsed onto one class: {seen:?}"
        );
        // The COUNT is the assertion, not the rows: it is what the copy budget
        // is denominated in, and the first consumer's design stated a
        // different number (10) derived from the DECLARED variants rather than
        // from the producers. Seven is what a paste can actually reach on this
        // feature set; four compiled variants have no producer here.
        assert_eq!(seen.len(), 7, "the reachable bridge taxonomy moved");
    }

    /// Anti-vacuity for the row above, and the D13 falsifier that matters most:
    /// a valid line is ACCEPTED and produces no refusal at all.
    #[test]
    fn a_valid_bridge_line_is_accepted_and_no_bridges_is_not_a_refusal() {
        let one = BridgeLines::parse(VANILLA).expect("a well-formed vanilla bridge line parses");
        assert_eq!(one.len(), 1);

        // *Carries a bridge* must not become *requires a bridge*: a user who
        // supplies nothing gets exactly what they get today.
        for nothing in ["", "   ", "\n\n", "# just a comment\n"] {
            let none = BridgeLines::parse(nothing)
                .unwrap_or_else(|c| panic!("{nothing:?} was refused as {c}"));
            assert!(none.is_empty());
        }
    }

    /// `bridge-client` is ON. If it were off, `BridgesNotSupported` is what
    /// every parse returns, so this is the row that proves the feature reached
    /// the build rather than the manifest.
    #[test]
    fn bridge_support_is_compiled_in() {
        assert!(BridgeLines::parse(VANILLA).is_ok());
        assert_ne!(
            BridgeLines::parse("192.0.2.55:38114").err(),
            Some("bridge-support-disabled"),
            "`bridge-client` is off: every bridge line is refused before it is read"
        );
    }

    /// The WHOLE-PASTE ceiling, both polarities, one byte either side. A cap
    /// tested only in the refusing direction is a cap that can be zero.
    ///
    /// The fixture is one long comment line rather than bridges, so the byte
    /// bound is what refuses; the two inner bounds get their own rows below,
    /// with fixtures the paste bound cannot reach first.
    #[test]
    fn the_paste_ceiling_refuses_one_byte_over_and_accepts_one_byte_under() {
        // Newline-separated so the per-line bound is never the one that fires.
        let unit = format!("#{}\n", "x".repeat(MAX_BRIDGE_LINE_BYTES - 2));
        assert_eq!(unit.len(), MAX_BRIDGE_LINE_BYTES);
        let over = unit.repeat(MAX_BRIDGE_CONFIG_BYTES / unit.len()) + "y";
        assert_eq!(over.len(), MAX_BRIDGE_CONFIG_BYTES + 1);
        assert_eq!(BridgeLines::parse(&over).err(), Some(CLASS_CONFIG_TOO_LONG));

        // Anti-vacuity: exactly at the ceiling is ACCEPTED, so the refusal
        // above is the bound and not the content.
        let at = &over[..MAX_BRIDGE_CONFIG_BYTES];
        assert_eq!(at.len(), MAX_BRIDGE_CONFIG_BYTES);
        assert!(BridgeLines::parse(at).is_ok());
    }

    /// The per-line bound, driven with a line short enough that the whole-paste
    /// ceiling is not what refuses — otherwise this row grades the wrong bound.
    #[test]
    fn the_line_length_bound_fires_before_the_paste_bound_does() {
        let long_line = format!("#{}", "x".repeat(MAX_BRIDGE_LINE_BYTES));
        assert_eq!(long_line.len(), MAX_BRIDGE_LINE_BYTES + 1);
        assert!(long_line.len() < MAX_BRIDGE_CONFIG_BYTES);
        assert_eq!(
            BridgeLines::parse(&long_line).err(),
            Some(CLASS_LINE_TOO_LONG)
        );
        // Anti-vacuity: exactly at the bound is accepted.
        assert!(BridgeLines::parse(&long_line[..MAX_BRIDGE_LINE_BYTES]).is_ok());
        // And the bound is not escaped by the blank/comment skip: this line IS
        // a comment, so a length check placed after that skip would let it
        // through and this row would go green on the wrong ordering.
        assert!(long_line.starts_with('#'));
    }

    /// The line-count refusal, with lines that are bridges rather than
    /// comments — a comment is dropped before it is counted, so counting
    /// comments would grade nothing.
    #[test]
    fn more_bridges_than_the_line_bound_is_refused_and_the_bound_itself_is_accepted() {
        let short_bridge = "192.0.2.1:1 316E643333645F6D79216558614D3931657A5F5F";
        let at = format!("{short_bridge}\n").repeat(MAX_BRIDGE_LINES);
        assert!(
            at.len() <= MAX_BRIDGE_CONFIG_BYTES,
            "the fixture must not trip the paste ceiling first"
        );
        let held = BridgeLines::parse(&at).expect("exactly the bound is accepted");
        assert_eq!(held.len(), MAX_BRIDGE_LINES);

        let over = format!("{short_bridge}\n").repeat(MAX_BRIDGE_LINES + 1);
        assert!(over.len() <= MAX_BRIDGE_CONFIG_BYTES);
        assert_eq!(BridgeLines::parse(&over).err(), Some(CLASS_TOO_MANY_LINES));
    }

    /// Every class this module can emit is distinct. Two refusals sharing a
    /// label is two next steps sharing one sentence, which is the silent-no-op
    /// defect wearing a label.
    #[test]
    fn no_two_bridge_classes_collide() {
        let all = [
            CLASS_CONFIG_TOO_LONG,
            CLASS_TOO_MANY_LINES,
            CLASS_LINE_TOO_LONG,
            CLASS_PT_UNSUPPORTED,
            CLASS_UNUSABLE,
            "bridge-line-empty",
            "bridge-invalid-transport-or-address",
            "bridge-invalid-address",
            "bridge-invalid-identity",
            "bridge-duplicate-identity",
            "bridge-unsupported-identity-type",
            "bridge-unsupported-channel-method",
            "bridge-direct-parameters-not-allowed",
            "bridge-no-rsa-identity",
            "bridge-support-disabled",
        ];
        let distinct: std::collections::BTreeSet<&str> = all.iter().copied().collect();
        assert_eq!(distinct.len(), all.len(), "two bridge classes collide");
    }

    /// No class this module emits carries any of the pasted text. The check is
    /// the whole point of the class-not-`Display` rule, and it is driven rather
    /// than argued: upstream's `Display` for these variants DOES contain the
    /// word, so the row goes red the moment somebody renders one.
    #[test]
    fn no_refusal_class_echoes_the_pasted_line() {
        let secret = "192.0.2.55:38114";
        for line in [
            format!("obfs4 {secret}"),
            format!("{secret} zzzznotafingerprint"),
            format!("{secret} 316E643333645F6D79216558614D3931657A5F5F iat-mode=0"),
            secret.to_string(),
        ] {
            let class = BridgeLines::parse(&line).err().expect("refused");
            assert!(
                !class.contains(secret) && !class.contains("38114") && !class.contains("192.0.2"),
                "the refusal class {class:?} carries the pasted endpoint"
            );
        }
        // Anti-vacuity: upstream's own rendering DOES carry the pasted word, so
        // the absence above is this module's doing and not a property of the
        // input. Six of the eleven variants this build compiles interpolate
        // `{word:?}`; this drives one of them.
        let Err(echoed) = format!("{secret} zzzznotafingerprint").parse::<BridgeConfigBuilder>()
        else {
            panic!("upstream now ACCEPTS a line with a bad fingerprint");
        };
        let echoed = echoed.to_string();
        assert!(
            echoed.contains("zzzznotafingerprint"),
            "upstream stopped echoing the pasted word, so this row no longer \
             proves that the class mapping is what suppresses it: {echoed}"
        );
    }
}
