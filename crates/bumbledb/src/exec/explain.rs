//! EXPLAIN (docs/architecture/40-execution.md): the debugging surface — an instrumented execution of
//! the same plan through the `Counters` seam, never a runtime mode
//! (`docs/architecture/40-execution.md`, observability).
//!
//! The normal path instantiates `NoopCounters` (zero-sized, compiled to
//! nothing); the EXPLAIN entry point instantiates [`CountingCounters`] and
//! executes the real query — ANALYZE semantics. Counter methods are plain
//! increments into plan-sized arrays: no formatting, no allocation in the
//! join loops. Output shape is OPEN per the architecture README; this
//! rendering is plain and stable-ish.
//!
//! Chase-eliminated occurrences (`plan/chase.rs`) surface here too, read
//! directly from the plan's `Role::Eliminated` marks — no separate list
//! exists. The marks' readers are exactly this surface (EXPLAIN and the
//! structured stats, which render each mark with its relation name and
//! its licensing statement through `schema/render.rs`) and the DP, which
//! sees a smaller problem because eliminated occurrences never enter it.

use crate::exec::dispatch::GuardPlan;
use crate::plan::fj::ValidatedPlan;

mod counters;
mod counting_counters;
mod display;
mod into_stats;
#[cfg(test)]
mod tests;

/// Plan-sized counters: every method is an increment, sized once at
/// construction (node count x max subatoms per node).
#[derive(Debug)]
pub struct CountingCounters {
    stride: usize,
    node_entries: Vec<u64>,
    /// Per (node, subatom): times chosen as cover with an `[Exact,
    /// Estimate]` count label — aggregated per node, not per entry.
    cover_choices: Vec<[u64; 2]>,
    /// Per (node, subatom): probe `[hit, miss]`.
    probes: Vec<[u64; 2]>,
    /// Per (node, subatom): phase-1 hash computations.
    hashes: Vec<u64>,
    /// Per node: residual `[pass, fail]`.
    residuals: Vec<[u64; 2]>,
    /// Per node: anti-probe `[miss (binding survives), hit (binding
    /// rejected)]` — probed is the sum (docs/architecture/40-execution.md,
    /// § anti-probe filters).
    anti_probes: Vec<[u64; 2]>,
    /// Per node: D2 subtree skips propagated through it.
    skips: Vec<u64>,
    /// Per node: `[batches drawn, entries yielded]` — batching engaged
    /// means batches ≪ entries at batch sizes > 1.
    batches: Vec<[u64; 2]>,
    emits: u64,
}

/// The EXPLAIN report: the plan rendering plus (for the join engine) the
/// counted execution. `Display` formats lazily — nothing here ran inside
/// the hot loops.
#[derive(Debug)]
pub enum Report<'p> {
    /// The query classified as a point lookup (docs/architecture/40-execution.md).
    GuardProbe { plan: &'p GuardPlan },
    /// The Free Join engine, with its counted execution.
    FreeJoin {
        plan: &'p ValidatedPlan,
        stats: crate::api::stats::ExecutionStats,
    },
}
