//! Structured per-execution statistics (docs/architecture/60-validation.md): the data
//! behind plan introspection, as plain structs — estimates vs actuals, cover
//! choices, probe hit rates, batching, skips — for tooling that wants
//! numbers, not a rendered string. Obtained via `ReadInstance::profile`
//! (ANALYZE semantics: the query really executes, with counting
//! instrumentation; allocation-sanctioned exactly like `introspect`).

/// The version shared by rendered and structured plan introspection.
pub const INTROSPECTION_VERSION: u16 = 7;

/// One execution's counted statistics. The body is a sum matching the
/// prepared pipeline: `reach` exists exactly on the Reach arm; interiors
/// exist on both; dead main is `Cq { rules: [] }` plus [`Self::dead`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStats {
    /// The introspection contract version. Any content or ordering change
    /// to either surface increments this value and the rendered marker.
    pub introspection_version: u16,
    /// Bindings emitted to the sink across all rules (the sum of the
    /// per-rule `emitted` on a Cq; the answer count on Reach).
    pub emits: u64,
    /// The rule-disjointness proof (docs/architecture/40-execution.md
    /// § set semantics): `Some` iff the query's rules are provably
    /// pairwise disjoint, naming the witness. `None` for single-rule
    /// programs, Reach, and unproven pairs.
    pub disjoint_rules: Option<DisjointRules>,
    /// Rules the subsumption pass deleted at prepare (`plan/ground.rs`).
    pub subsumed: Vec<SubsumedRule>,
    /// Rules the statically-empty fold refuted at prepare
    /// (`ir/normalize/fold.rs`).
    pub dead: Vec<DeadRule>,
    /// Pipeline-shaped counted body.
    pub body: StatsBody,
}

/// The counted body: interiors in both arms; `reach` only on Reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatsBody {
    Cq {
        rules: Vec<RuleStats>,
        interiors: Vec<InteriorStats>,
    },
    Reach {
        interiors: Vec<InteriorStats>,
        reach: ReachStats,
    },
}

impl ExecutionStats {
    /// Cq per-rule stats; empty on Reach (Reach does not grow a
    /// main-rule table).
    #[must_use]
    pub fn rules(&self) -> &[RuleStats] {
        match &self.body {
            StatsBody::Cq { rules, .. } => rules,
            StatsBody::Reach { .. } => &[],
        }
    }

    /// Named interiors in declaration order.
    #[must_use]
    pub fn interiors(&self) -> &[InteriorStats] {
        match &self.body {
            StatsBody::Cq { interiors, .. } | StatsBody::Reach { interiors, .. } => interiors,
        }
    }

    /// The reach rounds, present exactly on the Reach arm.
    #[must_use]
    pub fn reach(&self) -> Option<&ReachStats> {
        match &self.body {
            StatsBody::Reach { reach, .. } => Some(reach),
            StatsBody::Cq { .. } => None,
        }
    }

    /// Cq per-rule stats, consumed. Empty on Reach.
    #[must_use]
    pub fn into_cq_rules(self) -> Vec<RuleStats> {
        match self.body {
            StatsBody::Cq { rules, .. } => rules,
            StatsBody::Reach { .. } => Vec::new(),
        }
    }

    pub(crate) fn cq(
        rules: Vec<RuleStats>,
        interiors: Vec<InteriorStats>,
        emits: u64,
        disjoint_rules: Option<DisjointRules>,
        subsumed: Vec<SubsumedRule>,
        dead: Vec<DeadRule>,
    ) -> Self {
        Self {
            introspection_version: INTROSPECTION_VERSION,
            emits,
            disjoint_rules,
            subsumed,
            dead,
            body: StatsBody::Cq { rules, interiors },
        }
    }

    pub(crate) fn reach_body(
        interiors: Vec<InteriorStats>,
        reach: ReachStats,
        emits: u64,
        subsumed: Vec<SubsumedRule>,
        dead: Vec<DeadRule>,
    ) -> Self {
        Self {
            introspection_version: INTROSPECTION_VERSION,
            emits,
            disjoint_rules: None,
            subsumed,
            dead,
            body: StatsBody::Reach { interiors, reach },
        }
    }
}

/// One named interior's counted emits. No ghost per-interior rule table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteriorStats {
    /// The interior's [`crate::ir::InteriorId`].
    pub interior: u32,
    /// Bindings emitted to that interior's projection sink.
    pub emits: u64,
}

/// The rec SCC's counted round loop (`api/prepared/reach.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachStats {
    /// The rounds that ran, in order: round 0 is the base arms (no
    /// delta image yet), rounds ≥ 1 the Δ. The last entry is the
    /// converging round — every emission absorbed, or nothing emitted.
    pub rounds: Vec<RoundStats>,
}

/// One reach round's counted execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundStats {
    /// Frontier size entering this round's delta image. Zero at round 0.
    pub delta: u64,
    /// Bindings the round's runs emitted to the rec sink.
    pub emitted: u64,
    /// Of those, the re-derivations the spanning seen-set absorbed
    /// (`emitted - absorbed` were new — next round's frontier).
    pub absorbed: u64,
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

/// One rule's counted execution. The sum matches the prepared rule:
/// key-probe fields that must be empty under the probe tag are
/// unrepresentable on that arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleStats {
    /// A key-probe rule: no plan nodes, no grounding marks, no pins.
    KeyProbe {
        /// Whether this rule carries the proof that distinct facts imply
        /// distinct bindings.
        distinct_bindings: bool,
        /// Bindings this rule emitted.
        emitted: u64,
        /// Of those, the ones the spanning seen-set absorbed.
        absorbed: u64,
        /// Whether the probe found a fact.
        hit: bool,
    },
    /// A free-join rule under the shared sink.
    FreeJoin {
        /// Whether this rule carries the proof that distinct facts imply
        /// distinct bindings. A single-rule aggregate spends this witness to
        /// omit its binding seen-set; a union retains its spanning set.
        distinct_bindings: bool,
        /// Per plan node, in node order.
        nodes: Vec<NodeStats>,
        /// Occurrences the grounding eliminated (`plan/ground.rs`), read straight
        /// off the rule plan's `Role::Eliminated` marks — no separate list
        /// exists in the plan; this surface renders the marks.
        eliminated: Vec<EliminatedOccurrence>,
        /// Occurrences the grounding-evaluator folded (`plan/ground/evaluate.rs`),
        /// read straight off the rule plan's `Role::Folded` marks exactly as
        /// `eliminated` reads its own.
        folded: Vec<FoldedOccurrence>,
        /// Per participating occurrence, in occurrence-id order: the
        /// statistics the rule's plan was costed with — every node `estimate`
        /// is estimated from (pinned rows at prepare), so a drifted plan is
        /// visible in one read of this surface (the pull-based signal is
        /// `PreparedQuery::staleness`). Negated and grounding-eliminated
        /// occurrences earned no statistics read at prepare and carry no entry.
        pinned: Vec<PinnedRows>,
        /// Bindings this rule emitted to the shared sink.
        emitted: u64,
        /// Of those, the ones the spanning seen-set absorbed — duplicates
        /// within the rule or re-derivations of an earlier rule's head fact
        /// (`emitted - absorbed` were new). Zero under a single-rule
        /// distinct-bindings proof (nothing can be absorbed).
        absorbed: u64,
    },
}

impl RuleStats {
    /// Whether this rule carries the distinct-bindings proof.
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        match self {
            Self::KeyProbe {
                distinct_bindings, ..
            }
            | Self::FreeJoin {
                distinct_bindings, ..
            } => *distinct_bindings,
        }
    }

    /// Bindings this rule emitted.
    #[must_use]
    pub fn emitted(&self) -> u64 {
        match self {
            Self::KeyProbe { emitted, .. } | Self::FreeJoin { emitted, .. } => *emitted,
        }
    }

    /// Bindings the spanning seen-set absorbed.
    #[must_use]
    pub fn absorbed(&self) -> u64 {
        match self {
            Self::KeyProbe { absorbed, .. } | Self::FreeJoin { absorbed, .. } => *absorbed,
        }
    }

    /// Per-node stats; empty for key probes.
    #[must_use]
    pub fn nodes(&self) -> &[NodeStats] {
        match self {
            Self::FreeJoin { nodes, .. } => nodes,
            Self::KeyProbe { .. } => &[],
        }
    }

    /// Grounding-eliminated occurrences; empty for key probes.
    #[must_use]
    pub fn eliminated(&self) -> &[EliminatedOccurrence] {
        match self {
            Self::FreeJoin { eliminated, .. } => eliminated,
            Self::KeyProbe { .. } => &[],
        }
    }

    /// Grounding-folded occurrences; empty for key probes.
    #[must_use]
    pub fn folded(&self) -> &[FoldedOccurrence] {
        match self {
            Self::FreeJoin { folded, .. } => folded,
            Self::KeyProbe { .. } => &[],
        }
    }

    /// Prepare-time pin record; empty for key probes.
    #[must_use]
    pub fn pinned(&self) -> &[PinnedRows] {
        match self {
            Self::FreeJoin { pinned, .. } => pinned,
            Self::KeyProbe { .. } => &[],
        }
    }

    /// The key-probe outcome, present iff this rule classified as a key probe.
    #[must_use]
    pub fn key_probe(&self) -> Option<KeyProbeStats> {
        match self {
            Self::KeyProbe { hit, .. } => Some(KeyProbeStats { hit: *hit }),
            Self::FreeJoin { .. } => None,
        }
    }
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
    /// The executed row count after this node (entries of the next
    /// node, or sink emits for the last). D2 cancellation may deliberately
    /// stop before enumerating the denotation's full binding set, so this
    /// is an execution-work actual, not always a row-count oracle.
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
