//! Statistics and the DP planner (docs/architecture/40-execution.md): real
//! statistics in, one left-deep atom order out.
//!
//! Statistics are exact row counts (or measured filtered-view survivor
//! counts) plus schema statement knowledge (keys and containments,
//! `docs/architecture/30-dependencies.md`) — nothing else exists: no NDV
//! fields, no histograms, no magic selectivity constants (the post-mortem's
//! central engine finding, §30).

use crate::ir::normalize::OccId;
use crate::ir::VarId;

mod densify;
mod estimate;
mod plan;

pub use plan::plan;

/// Hard cap on occurrences the exhaustive subset DP accepts. The 40-execution doc named
/// 32 (the bitmask width), but 2³² DP states is memory-infeasible; at
/// 2²⁰ the DP table (`Option<State>`, 32 bytes each) is ~32 MB plus a
/// 16 MB per-mask prefix-variables memo — instant, and the doc's own
/// envelope is "≤ ~12 atoms" (amendment recorded in
/// docs/architecture/40-execution.md), where both are kilobytes. The
/// validation-boundary roster cap counts negated occurrences too (they
/// consume plan-time work), but only participating occurrences enter the
/// DP state — negated occurrences never join
/// (docs/architecture/40-execution.md, § search) and chase-eliminated
/// occurrences left planning entirely (`plan/chase.rs`).
pub const MAX_OCCURRENCES: usize = 20;

/// Distinct-variable cap for the planner's dense var bitsets.
pub(crate) const MAX_DISTINCT_VARS: usize = 128;

/// The planner's per-occurrence statistics (docs/architecture/40-execution.md): the
/// selectivity-shaped cardinality estimate, plus the base-relation
/// distinct count of every bound variable's field (from the same
/// ladder — key-exact, image-exact, schema bounds, floor). The
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
/// EXPLAIN (docs/architecture/40-execution.md). Participating occurrences
/// only — negated occurrences join nothing and reach execution as
/// anti-probes, and chase-eliminated occurrences left planning entirely
/// (`plan/chase.rs`).
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
    /// join-step fanout inputs (docs/architecture/40-execution.md).
    var_distincts: Vec<(u128, u64)>,
    /// Var bitsets of `Functionality` statements whose every projection
    /// field is var-bound in this occurrence (statements with any
    /// literal-bound or unbound field are skipped — simple and faithful
    /// to the doc's estimator). The pointwise-key guard lives in the
    /// translation ([`densify`]): a pointwise key's set exists only when
    /// its interval field is bound **by value**, so a join binding just
    /// the scalar prefix never certifies fanout 1.
    key_var_sets: Vec<u128>,
}

#[cfg(test)]
mod tests;
