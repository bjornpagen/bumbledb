mod corpus_digest;
mod digest_hex;
pub mod float_corpus;
pub mod irgen;
mod mandate;
pub mod opgen;
mod range_window;
pub mod rng;
mod row;
mod scale;
mod sizes;
#[cfg(test)]
mod tests;
pub mod theorygen;

pub use corpus_digest::corpus_digest;
pub use digest_hex::digest_hex;
pub use mandate::{MANDATE_SEGMENTS, Segment, mandate_segments};
pub use range_window::range_window;
pub use rng::Rng;
pub use row::{relation_rows, row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Tiny,
    S,
    M,
    L,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenConfig {
    pub seed: u64,
    pub scale: Scale,
}

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

pub const HOT_SHARE_PCT: u64 = 50;

pub const TAG_VARIANTS: u64 = 3;
pub const HOT_TAG_PCT: u64 = 60;

pub const AT_BASE: i64 = 1_700_000_000_000_000;
pub const AT_STEP: i64 = 50;

pub(crate) fn mix(seed: u64, rel: bumbledb::RelationId, row: u64) -> u64 {
    let mut z = seed ^ (u64::from(rel.0) << 56) ^ row;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
