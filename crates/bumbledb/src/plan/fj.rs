//! Free Join plan lowering (docs/architecture/40-execution.md): `binary2fj` (paper Fig. 7), the
//! conservative `factor()` hoist (Fig. 8), cover enumeration (§4.4),
//! residual and anti-probe placement, trie schemas (§3.3), and the sealed
//! [`ValidatedPlan`] witness (`docs/architecture/40-execution.md`).
//!
//! Plain `Vec`s everywhere — no fixed-capacity silent-drop containers
//! (post-mortem §35: capacity bugs must be impossible, not silent).

use crate::image::view::{Const, FilterPredicate};
use crate::image::ColumnSpan;
use crate::ir::normalize::{AntiProbe, OccId, PlacedComparison, PlacedWordComparison, SlotWidth};
use crate::ir::VarId;
use crate::schema::RelationId;

mod binary2fj;
mod check_occurrence_coverage;
mod check_selections;
mod derive_nodes;
mod factor;
mod provably_distinct;
mod slot_of;
mod split_filters;
mod validate;

#[cfg(test)]
mod occurrence;

pub use binary2fj::binary2fj;
pub(crate) use check_selections::check_selections;
pub use factor::factor;
pub(crate) use split_filters::split_filters;
pub use validate::validate;

/// A subatom: one occurrence with a subset of its variables. The plan
/// partitions every **positive** occurrence's variables across its
/// subatoms; negated occurrences join no node — they are reached only
/// through anti-probes (docs/architecture/40-execution.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subatom {
    pub occ: OccId,
    pub vars: Vec<VarId>,
}

/// One plan node: a list of subatoms. Executed as: iterate the chosen
/// cover, probe the rest in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub subatoms: Vec<Subatom>,
}

/// A Free Join plan: a list of nodes partitioning the query's positive
/// occurrences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FjPlan {
    pub nodes: Vec<Node>,
}

/// A plan-validation failure. Plans built by `binary2fj` + `factor` are
/// valid by construction; this boundary exists because [`FjPlan`] is plain
/// data anyone can construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// A positive occurrence's subatoms do not partition its variable set.
    BrokenPartition { occ: OccId },
    /// A positive occurrence of the query appears in no subatom of any
    /// node. A zero-variable (gate) occurrence dropped this way would
    /// silently skip its nonemptiness check — wrong results on a
    /// validated plan.
    MissingOccurrence { occ: OccId },
    /// A subatom references an occurrence outside the normalized query —
    /// the executor would index past its COLT array.
    UnknownOccurrence { node: usize, occ: OccId },
    /// A subatom references a **negated** occurrence — negated
    /// occurrences join no node; the executor reaches them exclusively
    /// through anti-probes (docs/architecture/40-execution.md).
    NegatedOccurrenceInNode { node: usize, occ: OccId },
    /// Two subatoms of one node share an occurrence.
    DuplicateOccurrenceInNode { node: usize, occ: OccId },
    /// A node has no cover: no subatom contains all its new variables.
    NoCover { node: usize },
    /// A residual comparison's variables are never both bound.
    UnplacedResidual { residual: usize },
    /// A decomposed interval word residual's variables are never both
    /// bound.
    UnplacedWordResidual { residual: usize },
    /// An anti-probe's variable set is never fully bound (validation
    /// guarantees negated-atom variables are positive-atom-bound, so
    /// this names a hand-built plan or query).
    UnplacedAntiProbe { anti_probe: usize },
    /// An occurrence's `filters` still carries an Eq-constant compare —
    /// lowering moves every one into `selections`, so its presence means
    /// a hand-built occurrence bypassed the split (docs/architecture/40-execution.md).
    SelectionOnFilteredField { occ: OccId },
    /// A var-sourced membership filter's point variable is never bound —
    /// validation guarantees point variables are positive-atom-bound, so
    /// this names a hand-built plan or query.
    UnplacedPointProbe { occ: OccId },
}

/// One probeable equality: `field == value`, the value constant per
/// execution (literal word/byte, param slot, param set, or pending
/// intern — literals and params are the same machine). Selections are
/// the probe-not-scan half of an occurrence's predicates; `filters`
/// keeps the scannable rest (docs/architecture/40-execution.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub field: crate::schema::FieldId,
    pub value: Const,
}

/// One placed membership probe: a positive occurrence's var-sourced
/// membership filters, evaluated inside the join once (a) every point
/// variable is bound and (b) the occurrence's trie is fully descended —
/// its remaining positions are then exactly the facts consistent with
/// the current binding, and the binding survives iff **one fact
/// satisfies every filter** (the point-membership scan,
/// docs/architecture/40-execution.md § access paths). Grouped per
/// occurrence because the conjunction quantifies over one fact:
/// `∃f (P₁(f) ∧ P₂(f))`, never `∃f P₁(f) ∧ ∃f P₂(f)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointProbe {
    pub occ: OccId,
    /// The interval field and bound point variable of each filter.
    pub filters: Vec<(crate::schema::FieldId, VarId)>,
}

/// One occurrence's execution-facing description — positive and negated
/// occurrences alike live in the one table ([`OccId`]s are indices);
/// polarity is a plan-shape fact: negated occurrences appear in no
/// subatom and are probed through the nodes' `anti_probes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanOccurrence {
    pub occ_id: OccId,
    pub relation: RelationId,
    /// The field each variable reads from.
    pub vars: Vec<(crate::schema::FieldId, VarId)>,
    /// Probeable equalities, ordered by field id (deterministic plans).
    /// Always empty for a negated occurrence: its Eq-constants stay in
    /// `filters` (below).
    pub selections: Vec<Selection>,
    /// Residual per-occurrence filters (evaluated at the source view):
    /// non-Eq compares, every `FieldsCompare`, and the interval
    /// compositions — never an Eq-constant on a positive occurrence,
    /// which lowering routes into `selections`. A **negated** occurrence
    /// keeps its whole lowered filter list here, Eq-constants included:
    /// the anti-probe runs against the ordinary filtered view, memoized
    /// per (generation, resolved filters), and an empty view just means
    /// the probe never rejects (docs/architecture/40-execution.md,
    /// § anti-probe filters). Var-sourced membership filters live in
    /// `point_filters`, never here — a view is built per execution, a
    /// variable binds per join row.
    pub filters: Vec<FilterPredicate>,
    /// Var-sourced membership filters (`PointIn` whose point is a bound
    /// variable), stripped out of `filters` at plan validation. For a
    /// positive occurrence they execute through the node's
    /// [`PlanNode::point_probes`]; for a negated occurrence the
    /// anti-probe evaluates them inside the probe — a binding is
    /// rejected only if a matching fact **also** satisfies every
    /// membership.
    pub point_filters: Vec<(crate::schema::FieldId, VarId)>,
    /// The field→column map (docs/architecture/50-storage.md image
    /// layout): one [`ColumnSpan`] per field of the relation, in
    /// declaration order — an interval field spans two word columns;
    /// consumers dispatch on spans, never raw field indices.
    pub spans: Box<[ColumnSpan]>,
    /// The trie schema: for a positive occurrence, its subatom var-lists
    /// in node order (§3.3; under COLT laziness there is no trailing `[]`
    /// level — the build-phase question dissolves, 40-execution). For a
    /// negated occurrence, one probe level holding all its variables in
    /// binding order (the order they appear in the probing node's
    /// binding) — derived per §3.3 exactly as a fully-hoisted positive
    /// lookup would be.
    pub trie_schema: Vec<Vec<VarId>>,
    /// Words per trie level: the sum of the level's variables' slot
    /// widths. An interval-typed join variable is one variable keyed by
    /// its two-word pair (docs/architecture/40-execution.md) — the COLT
    /// wordmap keys tuples, and this is the key-width bookkeeping it
    /// reads.
    pub key_widths: Vec<u16>,
}

/// One validated node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub subatoms: Vec<Subatom>,
    /// Indices into `subatoms` of the valid covers (every subatom
    /// containing all variables new to this node) — the runtime chooses
    /// among them magnitude-first by key count (§4.4, docs/architecture/40-execution.md).
    pub covers: Vec<u8>,
    /// Whole-value residual comparisons evaluated at this node (both
    /// sides bound here for the first time).
    pub residuals: Vec<PlacedComparison>,
    /// Decomposed interval word residuals (cross-atom `Overlaps`/
    /// `Contains`/membership) evaluated at this node — same placement
    /// rule as `residuals`.
    pub word_residuals: Vec<PlacedWordComparison>,
    /// Anti-probes evaluated at this node: each negated occurrence
    /// attaches to the earliest node binding its whole variable set —
    /// its probe keys **and** its point-filter variables
    /// (docs/architecture/40-execution.md, § anti-probe filters); a
    /// zero-variable emptiness gate attaches to the root.
    pub anti_probes: Vec<AntiProbe>,
    /// Membership probes evaluated at this node: each positive
    /// occurrence's var-sourced membership filters attach to the
    /// earliest node where every point variable is bound and the
    /// occurrence's trie is fully descended ([`PointProbe`]).
    pub point_probes: Vec<PointProbe>,
    /// Variables first bound by this node.
    pub new_vars: Vec<VarId>,
    /// Whether this node binds any sink-relevant (projected) variable —
    /// the D2 subtree-skip unwind stops at the first `true` node
    /// (precomputed here; the executor just reads the bit).
    pub sink_relevant: bool,
}

/// The sealed plan witness execution trusts; validated once at
/// construction, nothing downstream re-checks (post-mortem §38).
#[derive(Debug)]
pub struct ValidatedPlan {
    occurrences: Vec<PlanOccurrence>,
    nodes: Vec<PlanNode>,
    /// The binding-slot layout in binding order: each variable with its
    /// slot width — an interval-typed variable occupies two consecutive
    /// u64 slots (start word, end word; the layout decision lives at
    /// [`SlotWidth`]), every other variable one. `slot_of` maps a
    /// `VarId` to its **first** slot.
    slots: Vec<(VarId, SlotWidth)>,
    /// Provably-distinct-bindings: every positive occurrence's bound
    /// fields cover a key (`Functionality` statement) of its relation, so
    /// distinct facts imply distinct bindings and the aggregate sink may
    /// skip its seen-set (40-execution, elision).
    distinct_bindings: bool,
    /// Every node binds a sink-relevant variable:
    /// `Flow::SkipSuffix` can never cross a node, so the pipelined
    /// executor's cross-node batching needs no cancellation machinery.
    skip_free: bool,
    /// The planner's per-step estimates (EXPLAIN's reader, the 40-execution doc).
    estimates: Vec<u64>,
}

impl ValidatedPlan {
    #[must_use]
    pub fn occurrences(&self) -> &[PlanOccurrence] {
        &self.occurrences
    }

    #[must_use]
    pub fn nodes(&self) -> &[PlanNode] {
        &self.nodes
    }

    /// The binding-slot layout: each variable in binding order with its
    /// slot width.
    #[must_use]
    pub fn slots(&self) -> &[(VarId, SlotWidth)] {
        &self.slots
    }

    /// Total u64 slot count of the layout (interval variables count
    /// twice).
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.iter().map(|(_, width)| width.slots()).sum()
    }

    /// Whether an occurrence is negated — the plan-shape fact: a negated
    /// occurrence appears in no subatom and is reached exclusively
    /// through the nodes' `anti_probes`.
    #[must_use]
    pub fn is_negated(&self, occ: OccId) -> bool {
        !self
            .nodes
            .iter()
            .flat_map(|node| &node.subatoms)
            .any(|subatom| subatom.occ == occ)
    }

    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        self.distinct_bindings
    }

    /// Whether a suffix skip can never cross a node — the pipelined
    /// executor's eligibility.
    #[must_use]
    pub fn skip_free(&self) -> bool {
        self.skip_free
    }

    #[must_use]
    pub fn estimates(&self) -> &[u64] {
        &self.estimates
    }
}

#[cfg(test)]
mod tests;
