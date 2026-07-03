//! Structured per-execution statistics (docs/benchmarks/05): the data
//! behind EXPLAIN, as plain structs — estimates vs actuals, cover
//! choices, probe hit rates, batching, skips — for tooling that wants
//! numbers, not a rendered string. Obtained via `Snapshot::profile`
//! (ANALYZE semantics: the query really executes, with counting
//! instrumentation; allocation-sanctioned exactly like `explain`).

/// One execution's counted statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStats {
    /// Per plan node, in node order (empty for guard probes).
    pub nodes: Vec<NodeStats>,
    /// Bindings emitted to the sink.
    pub emits: u64,
    /// Present iff the query classified as a guard probe.
    pub guard: Option<GuardStats>,
}

/// One node's counted execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeStats {
    /// Node activations (recursion entries).
    pub entries: u64,
    /// Cover batches drawn.
    pub batches: u64,
    /// Entries yielded across those batches (batching engaged ⇔
    /// `batches` ≪ `batch_entries` at batch sizes > 1).
    pub batch_entries: u64,
    /// The planner's estimate for this step.
    pub estimate: u64,
    /// The measured cardinality after this node (entries of the next
    /// node, or sink emits for the last).
    pub actual: u64,
    /// Per subatom, in subatom order.
    pub covers: Vec<CoverStats>,
    /// Residual comparisons that passed.
    pub residual_pass: u64,
    /// Residual comparisons that failed.
    pub residual_fail: u64,
    /// D2 subtree skips propagated through this node.
    pub skips: u64,
}

/// One subatom's counted execution within a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverStats {
    /// The subatom index within its node.
    pub subatom: usize,
    /// Times chosen as the cover with an `Exact` key count.
    pub chosen_exact: u64,
    /// Times chosen as the cover with an `Estimate` key count.
    pub chosen_estimate: u64,
    /// Sibling probes that hit.
    pub probes_hit: u64,
    /// Sibling probes that missed.
    pub probes_miss: u64,
    /// Phase-1 hash computations.
    pub hashes: u64,
}

/// The guard-probe outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardStats {
    /// Whether the probe found a fact.
    pub hit: bool,
}
