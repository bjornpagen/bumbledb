//! Statistics and the DP planner: real
//! statistics in, one left-deep atom order out.
//! Statistics are exact row counts (or measured filtered-view survivor
//! counts) plus schema statement knowledge — nothing else exists: no NDV
//! fields, no histograms, no magic selectivity constants (the post-mortem's
//! central engine finding, §30).
use crate::ir::VarId;
use crate::ir::normalize::OccId;

mod densify;
mod estimate;
mod plan;

pub use plan::plan;

/// Hard cap on occurrences the exhaustive subset DP accepts. The 40-execution doc named
/// 32 (the bitmask width), but 2³² DP states is memory-infeasible; at
/// 2²⁰ the DP table (`Option<State>`, 32 bytes each) is ~32 MB plus a
/// 16 MB per-mask prefix-variables memo — instant, and the doc's own
/// envelope is "≤ ~12 atoms", where both are kilobytes. The
/// validation-boundary roster cap counts negated occurrences too (they
/// consume plan-time work), but only participating occurrences enter the
/// and grounding-eliminated
pub const MAX_OCCURRENCES: usize = 20;

pub(crate) const MAX_DISTINCT_VARS: usize = 128;

/// The planner's per-occurrence statistics: the
/// selectivity-shaped row-count estimate, plus the base-relation
/// distinct count of every bound variable's field (from the same
/// ladder — key-exact, image-exact, schema bounds, floor). The
/// distincts drive the join-step fanout model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccStats {
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
pub struct JoinOrder {
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
