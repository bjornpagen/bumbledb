//! The deterministic corpus generator (docs/architecture/60-validation.md):
//! seeded, streaming, skewed ledger data at three scales. Identical
//! config ⇒ identical bytes, forever — corpora are never stored, always
//! regenerated.
//!
//! Every row's content derives from a per-row RNG seeded by
//! `(seed, relation, row index)`, so streams are restartable, random-
//! access, and independent across relations by construction. `Mandate`
//! histories are **valid under the pointwise key by construction**
//! ([`mandate_segments`]): sequential non-overlapping segments per
//! account, mixing abutting and gapped boundaries, with the ray end
//! (`end == MAX_END` = `[s, ∞)`, "currently active") on every even account.

mod corpus_digest;
mod digest_hex;
mod mandate;
mod range_window;
pub mod rng;
mod row;
mod scale;
mod sizes;
#[cfg(test)]
mod tests;

pub use corpus_digest::corpus_digest;
pub use digest_hex::digest_hex;
pub use mandate::{MANDATE_SEGMENTS, Segment, mandate_segments};
pub use range_window::range_window;
pub use rng::Rng;
pub use row::{relation_rows, row};

/// Corpus scale points (docs/architecture/60-validation.md: 10⁵–10⁷),
/// plus `Tiny` — the fuzz-iteration point: sized so one full
/// build-store → ops → oracles pass is milliseconds, first-class under
/// the same invariants ([`Sizes::of`] owns the ladder's size table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Tiny,
    S,
    M,
    L,
}

/// The corpus identity: seed + scale. Everything else derives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenConfig {
    pub seed: u64,
    pub scale: Scale,
}

/// Derived per-relation row counts (the documented size table). Fields
/// are public so unit-scale harnesses (the naive differential slice) can
/// shrink every axis without a second generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sizes {
    pub postings: u64,
    pub entries: u64,
    pub accounts: u64,
    pub holders: u64,
    pub instruments: u64,
    pub orgs: u64,
    pub org_parents: u64,
    pub posting_tags: u64,
    pub mandates: u64,
}

/// Share of postings routed to the hot set, in percent.
pub const HOT_SHARE_PCT: u64 = 50;

/// `PostingTag.tag` has three variants; tag 0 (`Fee`) carries the skew:
/// [`HOT_TAG_PCT`]% of tagged postings draw it as their first tag.
pub const TAG_VARIANTS: u64 = 3;
pub const HOT_TAG_PCT: u64 = 60;

/// Timestamps: base + `i × AT_STEP` + jitter in `0..AT_STEP`; the range
/// family's fixed window ([`range_window`]) selects ≈2% of postings.
pub const AT_BASE: i64 = 1_700_000_000_000_000;
pub const AT_STEP: i64 = 50;

/// splitmix-style avalanche over `(seed, relation, row)` — the per-row
/// seed every generator function derives from.
pub(crate) fn mix(seed: u64, rel: bumbledb::RelationId, row: u64) -> u64 {
    let mut z = seed ^ (u64::from(rel.0) << 56) ^ row;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
