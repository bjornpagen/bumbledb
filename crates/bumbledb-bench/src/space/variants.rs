//! SPACE-02: the before/after layout-variant matrix.
//!
//! Each variant is a full storage build over identical generated data with
//! identical semantics, measured through the same census. Baselines are kept
//! **distinct**: the audited tree uses 8-byte fresh IDs and 32-byte
//! membership digests; the superseded earlier proposal used 28-byte IDs.
//! Comparing the successor's 16-byte IDs against both is legitimate;
//! combining the two baselines into one fictitious net saving is not, and the
//! arithmetic here refuses it by carrying the baseline in every delta.

/// Which historical baseline a delta is measured against. There is no
/// "combined" variant on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Baseline {
    /// The audited 0.x tree: 8-byte fresh IDs, 32-byte membership digest,
    /// immortal dictionary.
    Audited0x,
    /// The superseded proposal that carried 28-byte IDs. Only ID-width
    /// comparisons may cite it, and only as "superseded".
    Superseded28ByteIds,
}

/// One measured axis of SPACE-02.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// 32 → 16-byte local membership fingerprint, exact collision handling
    /// added: −16 raw bytes per fact at the membership entry.
    FingerprintWidth,
    /// 8 → 16-byte application-owned IDs against the audited tree
    /// (+8 bytes per occurrence), 28 → 16 against the superseded proposal
    /// (−12 per occurrence). Never netted together.
    IdWidth,
    /// Immortal dictionary versus inline text: repeated long strings can
    /// amortize interning; unique short strings and deleted historical
    /// strings do badly. Both populations are in the matrix.
    TextLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantCell {
    pub axis: Axis,
    pub baseline: Baseline,
    pub name: &'static str,
    /// The workload regimes each cell must report (chapter 41 SPACE-02):
    /// disk (raw/compacted), RSS, peak scratch, CRUD and warm/post-write/
    /// cold/>RAM costs — encoded as a requirement, checked by the manifest.
    pub regimes: &'static [&'static str],
}

pub const REQUIRED_REGIMES: [&str; 7] = [
    "disk-raw",
    "disk-compacted",
    "rss",
    "peak-scratch",
    "crud",
    "warm-post-write-cold",
    "beyond-ram",
];

/// The complete SPACE-02 matrix.
#[must_use]
pub fn matrix() -> Vec<VariantCell> {
    vec![
        VariantCell {
            axis: Axis::FingerprintWidth,
            baseline: Baseline::Audited0x,
            name: "membership digest 32B -> exact-checked fingerprint 16B",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::IdWidth,
            baseline: Baseline::Audited0x,
            name: "fresh 8B IDs -> application-owned 16B IDs (+8B per occurrence)",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::IdWidth,
            baseline: Baseline::Superseded28ByteIds,
            name: "superseded 28B IDs -> 16B IDs (-12B per occurrence; historical)",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::TextLayout,
            baseline: Baseline::Audited0x,
            name: "immortal dictionary -> inline text, repeated-label population",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::TextLayout,
            baseline: Baseline::Audited0x,
            name: "immortal dictionary -> inline text, unique-churn population",
            regimes: &REQUIRED_REGIMES,
        },
    ]
}

/// Raw-byte delta per fact for the fingerprint change: exactly the 16
/// truncated digest bytes at the membership entry (chapter 41's illustrative
/// 9.6%-of-167 / 7.0%-of-228 figures divide by the historical totals — they
/// are arithmetic, not measured file reductions, and the census must measure
/// the end-to-end effect including changed node shapes and collision fetches).
pub const FINGERPRINT_SAVING_PER_FACT: u64 = 16;

/// Signed per-occurrence ID-width delta against a named baseline.
#[must_use]
pub const fn id_width_delta(baseline: Baseline) -> i64 {
    match baseline {
        Baseline::Audited0x => 8,
        Baseline::Superseded28ByteIds => -12,
    }
}

/// Illustrative fingerprint savings at population scale, decimal units
/// (16 MB per million facts; 1.6 GB per 100 million) — arithmetic used by the
/// report prose, cross-checked in tests so the doc numbers cannot drift.
#[must_use]
pub const fn fingerprint_saving_bytes(facts: u64) -> u64 {
    facts * FINGERPRINT_SAVING_PER_FACT
}
