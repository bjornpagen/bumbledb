//! The ten gated read families (docs/architecture/50-validation.md): exact IR, exact
//! param policy, hand-written SQL golden, gate classification. This file
//! of queries **is** the benchmark's identity — `digest()` keys the
//! verify stamp and every report on it.

use bumbledb::{Query, Value};

use crate::gen::GenConfig;

mod digest;
mod read;
mod render_queries_md;
mod write;
#[cfg(test)]
mod tests;

pub use digest::digest;
pub use read::all;
pub use render_queries_md::render_queries_md;
pub use write::write_families;

/// Whether a family gates the suite (loses ⇒ the run fails) or merely
/// reports. All ten read families gate (`00-product.md`: every family
/// must win).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gate,
    Report,
}

/// One read family: the benchmark's unit of identity.
pub struct Family {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,
    /// The seeded param sets — verify and bench call this with the same
    /// `GenConfig` and therefore see identical sets.
    pub params: fn(&GenConfig) -> Vec<Vec<Value>>,
    /// Hand-written (docs/architecture/50-validation.md) — never regenerated from the
    /// translator; pinned equal to `translate` output by test.
    pub golden_sql: &'static str,
    /// The documented param policy, rendered into the versioned query
    /// list.
    pub param_policy: &'static str,
}

/// The two tag ids the generator guarantees on hot accounts: tag 0 (every
/// hot account's `k = 0` pair) and tag 97 (hot account 0's `k = 1` pair,
/// `(0 + 97) % 256`).
pub const SKEW_HOT_TAGS: [u64; 2] = [0, 97];

/// One write/cold family (docs/architecture/50-validation.md): a name, its report-only
/// classification, and its write-appropriate protocol. The runners live
/// in `writebench` — these are identities, not closures.
pub struct WriteFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub protocol: crate::harness::Protocol,
}
