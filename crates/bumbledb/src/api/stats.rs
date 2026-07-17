//! Structured per-execution statistics (docs/architecture/60-validation.md): the data
//! behind plan introspection, as plain structs — estimates vs actuals, cover
//! choices, probe hit rates, batching, skips — for tooling that wants
//! numbers, not a rendered string. Obtained via `Snapshot::profile`
//! (ANALYZE semantics: the query really executes, with counting
//! instrumentation; allocation-sanctioned exactly like `introspect`).

/// The version shared by rendered and structured plan introspection.
pub const INTROSPECTION_VERSION: u16 = 3;

/// One execution's counted statistics: per-rule node stats under the
/// head-level union accounting (docs/architecture/40-execution.md § the
/// rule loop — one sink hears every rule; its seen-set spanning rules is
/// the union). The single-rule program is the one-element list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStats {
    /// The introspection contract version. Any content or ordering change
    /// to either surface increments this value and the rendered marker.
    pub introspection_version: u16,
    /// Per rule, in rule order.
    pub rules: Vec<RuleStats>,
    /// Bindings emitted to the sink across all rules (the sum of the
    /// per-rule `emitted`).
    pub emits: u64,
    /// The rule-disjointness proof (docs/architecture/40-execution.md
    /// § set semantics): `Some` iff the program's rules are provably
    /// pairwise disjoint, naming the witness. `None` for single-rule
    /// programs and unproven pairs. This is diagnostic knowledge; the
    /// spanning seen-set stays in either case.
    pub disjoint_rules: Option<DisjointRules>,
    /// Rules the subsumption pass deleted at prepare (`plan/ground.rs`):
    /// after per-rule elimination the subsuming rule's normalized body
    /// contains the deleted rule's, so the union loses nothing. Indices
    /// are lowered-rule indices (the DNF-distributed program validation
    /// diagnostics use) — the per-rule list above holds only survivors,
    /// in order.
    pub subsumed: Vec<SubsumedRule>,
    /// Rules the statically-empty fold refuted at prepare
    /// (`ir/normalize/fold.rs`): each carries its killing condition —
    /// introspection's `statically empty: rule N: <picture>` line. Indices are
    /// lowered-rule indices, exactly as `subsumed`; a program of only
    /// dead rules represented by an empty prepared program.
    pub dead: Vec<DeadRule>,
    /// The fixpoint driver's counted rounds
    /// (docs/architecture/40-execution.md § the fixpoint driver): one
    /// entry per recursive stratum, in condensation order. Empty for
    /// query-shaped programs (no round loop exists), and populated on
    /// counted paths only — the release executor's `NoopCounters`
    /// records nothing.
    pub strata: Vec<StratumStats>,
}

/// One recursive stratum's counted round loop (`api/prepared/fixpoint.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StratumStats {
    /// The stratum's condensation index.
    pub stratum: u16,
    /// The rounds that ran, in order: round 0 is the stratum's
    /// non-recursive rules (no delta images exist yet), rounds ≥ 1 the
    /// delta-variant runs. The last entry is the converging round —
    /// every emission absorbed, or nothing emitted.
    pub rounds: Vec<RoundStats>,
}

/// One fixpoint round's counted execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundStats {
    /// Per stratum predicate with a plan unit this round, in `PredId`
    /// order: the frontier rows its delta image carried into the
    /// round's variants. Empty at round 0.
    pub deltas: Vec<DeltaRows>,
    /// Bindings the round's runs emitted to the predicates' sinks.
    pub emitted: u64,
    /// Of those, the re-derivations the spanning seen-sets absorbed
    /// (`emitted - absorbed` were new — next round's frontier).
    pub absorbed: u64,
}

/// One predicate's per-round delta size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeltaRows {
    /// The predicate's `PredId` index.
    pub predicate: u16,
    /// The frontier rows entering this round's delta image.
    pub rows: u64,
}

/// One statically-empty rule (`ir/normalize/fold.rs`): its constant
/// conditions are mutually unsatisfiable, so it was deleted at prepare
/// with the killing condition as the record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadRule {
    /// The dead rule's lowered-rule index.
    pub rule: u16,
    /// The killing condition, rendered in the rule notation's value
    /// formats (e.g. `R: a ∈ [8, 19] ∧ a == 3`).
    pub rendered: String,
}

/// One deleted rule with its subsumer (introspection's `subsumed: rule D by
/// rule K`). Both indices are lowered-rule indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubsumedRule {
    /// The deleted rule's index.
    pub rule: u16,
    /// The subsuming rule's index.
    pub by: u16,
}

/// The disjointness witness, rendered by name: the relation and field
/// whose differing pinned literals make the rules' head answers
/// collision-free (introspection's `disjoint_rules: proven (R.f)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisjointRules {
    /// The witness relation's name.
    pub relation: String,
    /// The pinned discriminator field's name.
    pub field: String,
}

/// One rule's counted execution under the shared sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleStats {
    /// Whether this rule carries the proof that distinct facts imply
    /// distinct bindings. A single-rule aggregate spends this witness to
    /// omit its binding seen-set; a union retains its spanning set.
    pub distinct_bindings: bool,
    /// Per plan node, in node order (empty for key-probe rules).
    pub nodes: Vec<NodeStats>,
    /// Occurrences the grounding eliminated (`plan/ground.rs`), read straight
    /// off the rule plan's `Role::Eliminated` marks — no separate list
    /// exists in the plan; this surface renders the marks. Empty for
    /// key probes (single-atom queries have nothing to pair).
    pub eliminated: Vec<EliminatedOccurrence>,
    /// Occurrences the grounding-evaluator folded (`plan/ground/evaluate.rs`),
    /// read straight off the rule plan's `Role::Folded` marks exactly as
    /// `eliminated` reads its own. Empty for key probes.
    pub folded: Vec<FoldedOccurrence>,
    /// Per participating occurrence, in occurrence-id order: the
    /// statistics the rule's plan was costed with — every node `estimate`
    /// is estimated from (pinned rows at prepare), so a drifted plan is
    /// visible in one read of this surface (the pull-based signal is
    /// `PreparedQuery::staleness`). Empty for key probes (they read
    /// no statistics); negated and grounding-eliminated occurrences earned
    /// no statistics read at prepare and carry no entry.
    pub pinned: Vec<PinnedRows>,
    /// Bindings this rule emitted to the shared sink.
    pub emitted: u64,
    /// Of those, the ones the spanning seen-set absorbed — duplicates
    /// within the rule or re-derivations of an earlier rule's head fact
    /// (`emitted - absorbed` were new). Zero under a single-rule
    /// distinct-bindings proof (nothing can be absorbed).
    pub absorbed: u64,
    /// Present iff this rule classified as a key probe.
    pub key_probe: Option<KeyProbeStats>,
}

/// One grounding-eliminated occurrence: never joined, its view never built —
/// the plan solved a smaller problem (`plan/ground.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EliminatedOccurrence {
    /// The occurrence index (`OccId`) in the normalized occurrence table.
    pub occurrence: u16,
    /// The eliminated occurrence's relation name.
    pub relation: String,
    /// The containment statement licensing the elimination.
    pub statement: bumbledb_theory::schema::StatementId,
    /// The statement rendered in the `schema!` algebra notation
    /// (`schema/render.rs`), e.g. `Posting(account) <= Account(id)`.
    pub rendered: String,
}

/// One grounding-folded occurrence (`plan/ground/evaluate.rs`): a closed
/// atom evaluated against its sealed extension at prepare — never
/// joined, its view never bound, its image never built; the surviving
/// id-set rides the siblings' selection machinery as a plan constant.
/// introspection's line: `folded: Kind{mastered == true} → {DirectPass,
/// JudgedPass}` (negated: `folded: !Kind{…} → {…} rejected` — the
/// attached set is then the complement). The handle set IS the payload:
/// handles are the vocabulary's names, and `|S|` is its length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldedOccurrence {
    /// The occurrence index (`OccId`) in the normalized occurrence table.
    pub occurrence: u16,
    /// The folded occurrence's relation name.
    pub relation: String,
    /// The evaluated atom's picture — relation and filters in the rule
    /// notation's value formats (e.g. `Currency{minor_units == 0}`;
    /// a word at the id position prints its handle).
    pub rendered: String,
    /// `S` as handles — the sealed extension rows that satisfied the
    /// filters, in declaration (row-id) order.
    pub handles: Vec<String>,
    /// Whether the folded occurrence was negated: the attached
    /// membership is then the complement (extension minus `S`), and the
    /// `handles` rows are what the deleted anti-probe would have
    /// rejected.
    pub negated: bool,
}

/// One occurrence's pinned prepare-time statistics: what the plan was
/// costed with (docs/architecture/20-query-ir.md, pin-at-prepare) —
/// est-vs-actual honesty for a plan whose data has moved since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedRows {
    /// The occurrence index (`OccId`) in the normalized occurrence table.
    pub occurrence: u16,
    /// The occurrence's relation name.
    pub relation: String,
    /// The `S`-counter row count read at prepare.
    pub rows: u64,
    /// The filtered view's survivor count as measured at prepare, where
    /// the occurrence carries filters (exact where a resident image was
    /// measured; documented bounds and floors otherwise —
    /// `plan/selectivity.rs`). `None` = unfiltered.
    pub survivors: Option<u64>,
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
    /// The executed cardinality after this node (entries of the next
    /// node, or sink emits for the last). D2 cancellation may deliberately
    /// stop before enumerating the denotation's full binding set, so this
    /// is an execution-work actual, not always a cardinality oracle.
    pub actual: u64,
    /// Per subatom, in subatom order.
    pub covers: Vec<CoverStats>,
    /// Residual comparisons that passed.
    pub residual_pass: u64,
    /// Residual comparisons that failed.
    pub residual_fail: u64,
    /// Anti-probes issued for surviving bindings at this node
    /// (docs/architecture/40-execution.md, § anti-probe filters).
    pub anti_probe_probed: u64,
    /// Anti-probes that hit — bindings rejected. Selectivity is
    /// `rejected / probed`.
    pub anti_probe_rejected: u64,
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
    /// Hashes actually computed for map probes (phase 1). Pinned-row
    /// siblings probe by field equality and compute none.
    pub hashes: u64,
}

/// The key-probe outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyProbeStats {
    /// Whether the probe found a fact.
    pub hit: bool,
}
