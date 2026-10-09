//! Subset-DP join ordering from per-occurrence row estimates, schema key
//! proofs, and distinct counts for shared variables. The selectivity ladder
//! supplies exact resident-column counts, schema bounds, or documented floors;
//! these are cost estimates, not answer cardinality proofs.
use crate::ir::VarId;
use crate::ir::normalize::OccId;

mod densify;
mod estimate;
mod plan;

pub(crate) use plan::plan;

/// Hard cap on occurrences the exhaustive subset DP accepts: at 2²⁰ subsets the
/// DP table (`Option<State>`, 32 bytes each) is about 32 MB plus a 16 MB
/// per-mask prefix-variables memo, while ordinary queries of about 12 atoms
/// need kilobytes. Only participating occurrences enter the DP.
pub(crate) const MAX_OCCURRENCES: usize = 20;

pub(crate) const MAX_DISTINCT_VARS: usize = 128;

/// Selectivity-shaped rows and base-relation distinct estimates for shared
/// variables only. Unshared output variables cannot affect join fanout and
/// do not request statistics. Own-condition selectivity is included in rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OccStats {
    pub occ_id: OccId,
    /// Estimated row count after this occurrence's own conditions.
    pub rows: u64,

    pub var_distincts: Vec<(VarId, u64)>,
}

/// The chosen left-deep join order, with per-step estimates retained for
/// introspection. Participating occurrences
/// anti-probes, and grounding-eliminated occurrences left planning entirely
/// (`plan/ground.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JoinOrder {
    pub order: Vec<OccId>,
    /// The estimator's row count after each step; `estimates[0]` is the
    pub estimates: Vec<u64>,
}

#[derive(Clone, Copy)]
struct State {
    cost: u64,
    est: u64,
    last: u8,
}

struct OccInfo {
    rows: u64,

    vars: u128,

    var_distincts: Vec<(u128, u64)>,

    key_var_sets: Vec<u128>,
}

struct AllenKeep {
    vars: u128,

    keep_num: u64,
    keep_den: u64,
}

#[cfg(test)]
mod tests;
