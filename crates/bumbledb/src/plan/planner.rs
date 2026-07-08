//! Statistics and the DP planner (docs/architecture/30-execution.md): real statistics in, one
//! left-deep atom order out (`docs/architecture/30-execution.md`).
//!
//! Statistics are exact row counts (or measured filtered-view survivor
//! counts) plus schema constraint knowledge — nothing else exists: no NDV
//! fields, no histograms, no magic selectivity constants (the post-mortem's
//! central engine finding, §30).

use crate::ir::normalize::OccId;
use crate::ir::VarId;

mod densify;
mod estimate;
mod plan;

pub use plan::plan;

/// Hard cap on occurrences the exhaustive subset DP accepts. The 30-execution doc named
/// 32 (the bitmask width), but 2³² DP states is memory-infeasible; at
/// 2²⁰ the DP table (`Option<State>`, 32 bytes each) is ~32 MB plus a
/// 16 MB per-mask prefix-variables memo — instant, and the doc's own
/// envelope is "≤ ~12 atoms" (amendment recorded in
/// docs/architecture/30-execution.md), where both are kilobytes.
pub const MAX_OCCURRENCES: usize = 20;

/// Distinct-variable cap for the planner's dense var bitsets.
pub(crate) const MAX_DISTINCT_VARS: usize = 128;

/// The planner's per-occurrence statistics (docs/architecture/30-execution.md): the
/// selectivity-shaped cardinality estimate, plus the base-relation
/// distinct count of every bound variable's field (from the same
/// ladder — unique-exact, image-exact, schema bounds, floor). The
/// distincts drive the join-step fanout model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccStats {
    pub occ_id: OccId,
    /// Estimated cardinality after this occurrence's own predicates.
    pub rows: u64,
    /// `(var, distinct count of its field over the base relation)`.
    pub var_distincts: Vec<(VarId, u64)>,
}

/// The chosen left-deep join order, with per-step estimates retained for
/// EXPLAIN (docs/architecture/30-execution.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOrder {
    /// Occurrences in join order (first = the iterated relation).
    pub order: Vec<OccId>,
    /// The estimator's cardinality after each step; `estimates[0]` is the
    /// first occurrence's row count.
    pub estimates: Vec<u64>,
}

/// One DP table entry: cheapest left-deep plan covering the mask.
#[derive(Clone, Copy)]
struct State {
    cost: u64,
    est: u64,
    last: u8,
}

/// Per-occurrence planning inputs, densified.
struct OccInfo {
    rows: u64,
    /// This occurrence's variables as a dense bitset.
    vars: u128,
    /// `(var bit, base-relation distinct count of its field)` — the
    /// join-step fanout inputs (docs/architecture/30-execution.md).
    var_distincts: Vec<(u128, u64)>,
    /// Var bitsets of unique constraints whose every field is var-bound in
    /// this occurrence (constraints with literal-bound fields are skipped —
    /// simple and faithful to the doc's estimator).
    unique_var_sets: Vec<u128>,
}

#[cfg(test)]
mod tests;
