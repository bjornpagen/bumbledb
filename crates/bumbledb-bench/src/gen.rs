//! The deterministic corpus generator (docs/architecture/50-validation.md):
//! seeded, streaming, skewed ledger data at three scales. Identical
//! config ⇒ identical bytes, forever — corpora are never stored, always
//! regenerated.
//!
//! Every row's content derives from a per-row RNG seeded by
//! `(seed, relation, row index)`, so streams are restartable, random-
//! access, and independent across relations by construction.

mod account_tag_pair;
mod corpus_digest;
mod digest_hex;
mod range_window;
mod rng;
mod row;
mod scale;
mod sizes;
#[cfg(test)]
mod tests;

pub use account_tag_pair::account_tag_pair;
pub use corpus_digest::corpus_digest;
pub use digest_hex::digest_hex;
pub use range_window::range_window;
pub use row::{relation_rows, row};

/// The house LCG (the engine's test constants): deterministic, fast, and
/// dependency-free.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

/// Corpus scale points (docs/architecture/50-validation.md: 10⁵–10⁷).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
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

/// Derived per-relation row counts (the documented size table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sizes {
    pub postings: u64,
    pub transfers: u64,
    pub accounts: u64,
    pub holders: u64,
    pub instruments: u64,
    pub currencies: u64,
    pub tags: u64,
    pub account_tags: u64,
    pub tag_notes: u64,
}

/// Share of postings routed to the hot set, in percent.
pub const HOT_SHARE_PCT: u64 = 50;

/// The memo vocabulary size (interning realism); 1-in-[`UNIQUE_MEMO_DEN`]
/// postings carry a never-repeated memo instead.
pub const MEMO_VOCAB: u64 = 4096;
pub const UNIQUE_MEMO_DEN: u64 = 64;

/// Timestamps: base + `i × AT_STEP` + jitter in `0..AT_STEP`; the range
/// family's fixed window ([`range_window`]) selects ≈2% of postings.
pub const AT_BASE: i64 = 1_700_000_000_000_000;
pub const AT_STEP: i64 = 50;
