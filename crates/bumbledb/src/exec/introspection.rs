//! Plan introspection (EXPLAIN, colloquially) is the debugging surface: an instrumented execution of
//! the same plan through the `Counters` seam, never a runtime mode
//! (`docs/architecture/40-execution.md`, observability).
//!
//! The normal path instantiates `NoopCounters` (zero-sized, compiled to
//! nothing); the introspection entry point instantiates [`CountingCounters`] and
//! executes the real query — ANALYZE semantics. Counter methods are plain
//! increments into plan-sized arrays: no formatting, no allocation in the
//! join loops.
//!
//! The rendered artifact and structured statistics are versioned together.
//! Within one version, identical schema fingerprint, canonical query,
//! parameter types, and feature set produce byte-identical output. Any
//! content or ordering change must increment `INTROSPECTION_VERSION`.
//! Sections have fixed order; rules retain program order, nodes retain plan
//! order, and dead, subsumed, and unresolved-literal diagnostics retain
//! statement order. No unordered collection feeds the rendered surface.
//!
//! Grounding-eliminated occurrences (`plan/ground.rs`) surface here too, read
//! directly from the plan's `Role::Eliminated` marks — no separate list
//! exists. The marks' readers are exactly this surface (introspection and the
//! structured stats, which render each mark with its relation name and
//! its licensing statement through `schema/render.rs`) and the DP, which
//! sees a smaller problem because eliminated occurrences never enter it.

use crate::exec::dispatch::KeyProbePlan;
use crate::plan::fj::ValidatedPlan;

mod counters;
mod counting_counters;
mod display;
mod fixpoint_counters;
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

/// Driver-level counters for a fixpoint execution
/// (docs/architecture/40-execution.md § the fixpoint driver): the
/// per-stratum, per-round delta sizes and union accounting the driver
/// reports through the `Counters` seam's fixpoint hooks. Node-level
/// methods are deliberate no-ops — the driver runs many differently
/// shaped plan units under one counter, so the counted surface here is
/// the round structure, not per-node cardinalities.
#[derive(Debug, Default)]
pub struct FixpointCounters {
    emits: u64,
    /// Deltas reported since the last round closed (`fixpoint_round`
    /// bundles them into that round's record).
    pending_deltas: Vec<crate::api::stats::DeltaRows>,
    strata: Vec<crate::api::stats::StratumStats>,
}

/// The introspection report: per-rule plan renderings plus the counted
/// execution — per-rule node stats under the head-level union
/// accounting (docs/architecture/40-execution.md § the rule loop).
/// `Display` formats lazily — nothing here ran inside the hot loops.
#[derive(Debug)]
pub struct IntrospectionReport<'p> {
    /// Query and predicate header for the public artifact. Low-level
    /// executor tests omit it while retaining the same versioned body.
    pub header: Option<IntrospectionHeader>,
    /// Per plan unit, aligned with `stats.rules` for query-shaped
    /// programs. A fixpoint program's units (every predicate's rules, a
    /// recursive rule as its delta variants) carry labels below and no
    /// per-unit counted stats — the counted surface is `stats.strata`.
    pub rules: Vec<RulePlan<'p>>,
    /// Fixpoint unit labels, parallel to `rules`
    /// (`predicate p0 rule 1 delta variant 0`); empty for query-shaped
    /// programs, whose label is the rule index.
    pub unit_labels: Vec<String>,
    pub stats: crate::api::stats::ExecutionStats,
}

/// Owned public header rendered before the plan sections.
#[derive(Debug)]
pub struct IntrospectionHeader {
    pub query: String,
    pub predicate: String,
    pub pending_literal: Option<String>,
}

/// One rule's access path (docs/architecture/40-execution.md).
#[derive(Debug)]
pub enum RulePlan<'p> {
    /// The rule classified as a point lookup.
    KeyProbe(&'p KeyProbePlan),
    /// The Free Join engine.
    FreeJoin(&'p ValidatedPlan),
    /// The statically-empty program (`ir/normalize/fold.rs`): every
    /// rule refuted on constants at prepare — nothing runs, and the
    /// per-rule killing conditions print from `stats.dead`.
    Empty,
}
