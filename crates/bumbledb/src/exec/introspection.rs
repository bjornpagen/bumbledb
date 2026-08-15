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
//! Sections have fixed order; rules retain query order, nodes retain plan
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
mod into_stats;
mod reach_counters;
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

/// Driver-level counters for a reach execution
/// (docs/architecture/40-execution.md § the linear reach driver): the
/// per-round delta sizes and union accounting the driver reports through
/// the `Counters` seam's fixpoint hooks. Node-level methods are
/// deliberate no-ops — the driver runs many differently shaped plan
/// units under one counter, so the counted surface here is the round
/// structure (`stats.reach`), not per-node row counts.
#[derive(Debug, Default)]
pub struct ReachCounters {
    emits: u64,
    pending_delta: u64,
    rounds: Vec<crate::api::stats::RoundStats>,
}

/// The introspection report: a pipeline-shaped body plus the counted
/// execution. `Display` formats lazily — nothing here ran inside the
/// hot loops.
#[derive(Debug)]
pub struct IntrospectionReport<'p> {
    /// Query and signature header for the public artifact. Low-level
    /// executor tests omit it while retaining the same versioned body.
    pub header: Option<IntrospectionHeader>,
    /// Pipeline-shaped plans: Cq plans align with `stats.rules()`;
    /// Reach units carry their labels and no per-unit counted stats.
    pub body: ReportBody<'p>,
    pub stats: crate::api::stats::ExecutionStats,
}

/// Plans matching the prepared pipeline. Reach labels are
/// `reach base {i}`, `reach rec {i} (delta occ {d})`, `main {i}`.
#[derive(Debug)]
pub enum ReportBody<'p> {
    Cq {
        plans: Vec<RulePlan<'p>>,
    },
    Reach {
        rec_id: crate::ir::InteriorId,
        units: Vec<(String, RulePlan<'p>)>,
    },
}

/// Owned public header rendered before the plan sections.
#[derive(Debug)]
pub struct IntrospectionHeader {
    pub query: String,
    /// [`crate::api::prepared::PreparedQuery::signature()`], rendered.
    pub signature: String,
    pub pending_literal: Option<String>,
}

/// One rule's access path (docs/architecture/40-execution.md).
#[derive(Debug)]
pub enum RulePlan<'p> {
    /// The rule classified as a point lookup.
    KeyProbe(&'p KeyProbePlan),
    /// The Free Join engine.
    FreeJoin(&'p ValidatedPlan),
    /// The statically-empty query (`ir/normalize/fold.rs`): every
    /// rule refuted on constants at prepare — nothing runs, and the
    /// per-rule killing conditions print from `stats.dead`.
    Empty,
}
