//! Free Join plan lowering (docs/architecture/30-execution.md): `binary2fj` (paper Fig. 7), the
//! conservative `factor()` hoist (Fig. 8), cover enumeration (§4.4),
//! residual placement, trie schemas (§3.3), and the sealed
//! [`ValidatedPlan`] witness (`docs/architecture/30-execution.md`).
//!
//! Plain `Vec`s everywhere — no fixed-capacity silent-drop containers
//! (post-mortem §35: capacity bugs must be impossible, not silent).

use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{OccId, PlacedComparison};
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
pub use factor::factor;
pub use validate::validate;
pub(crate) use check_selections::check_selections;
pub(crate) use split_filters::split_filters;

/// A subatom: one occurrence with a subset of its variables. The plan
/// partitions every occurrence's variables across its subatoms.
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

/// A Free Join plan: a list of nodes partitioning the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FjPlan {
    pub nodes: Vec<Node>,
}

/// A plan-validation failure. Plans built by `binary2fj` + `factor` are
/// valid by construction; this boundary exists because [`FjPlan`] is plain
/// data anyone can construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// An occurrence's subatoms do not partition its variable set.
    BrokenPartition { occ: OccId },
    /// An occurrence of the query appears in no subatom of any node. A
    /// zero-variable (gate) occurrence dropped this way would silently
    /// skip its nonemptiness check — wrong results on a validated plan.
    MissingOccurrence { occ: OccId },
    /// A subatom references an occurrence outside the normalized query —
    /// the executor would index past its COLT array.
    UnknownOccurrence { node: usize, occ: OccId },
    /// Two subatoms of one node share an occurrence.
    DuplicateOccurrenceInNode { node: usize, occ: OccId },
    /// A node has no cover: no subatom contains all its new variables.
    NoCover { node: usize },
    /// A residual comparison's variables are never both bound.
    UnplacedResidual { residual: usize },
    /// An occurrence's `filters` still carries an Eq-constant compare —
    /// lowering moves every one into `selections`, so its presence means
    /// a hand-built occurrence bypassed the split (docs/architecture/30-execution.md).
    SelectionOnFilteredField { occ: OccId },
}

/// One probeable equality: `field == value`, the value constant per
/// execution (literal word/byte, param slot, or pending intern —
/// literals and params are the same machine). Selections are the
/// probe-not-scan half of an occurrence's predicates; `filters` keeps
/// the scannable rest (docs/architecture/30-execution.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub field: crate::schema::FieldId,
    pub value: Const,
}

/// One occurrence's execution-facing description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanOccurrence {
    pub occ_id: OccId,
    pub relation: RelationId,
    /// The field each variable reads from (field index = column index).
    pub vars: Vec<(crate::schema::FieldId, VarId)>,
    /// Probeable equalities, ordered by field id (deterministic plans).
    pub selections: Vec<Selection>,
    /// Residual per-occurrence filters (evaluated at the source view):
    /// non-Eq compares and every `FieldsCompare` — never an Eq-constant,
    /// which lowering routes into `selections`.
    pub filters: Vec<FilterPredicate>,
    /// The trie schema: this occurrence's subatom var-lists in node order
    /// (§3.3). Under COLT laziness there is no trailing `[]` level — the
    /// build-phase question dissolves (30-execution).
    pub trie_schema: Vec<Vec<VarId>>,
}

/// One validated node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub subatoms: Vec<Subatom>,
    /// Indices into `subatoms` of the valid covers (every subatom
    /// containing all variables new to this node) — the runtime chooses
    /// among them magnitude-first by key count (§4.4, docs/architecture/30-execution.md).
    pub covers: Vec<u8>,
    /// Residual comparisons evaluated at this node (both sides bound here
    /// for the first time).
    pub residuals: Vec<PlacedComparison>,
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
    /// Dense binding-slot layout: `slots[i]` is the variable stored in
    /// slot `i`; `slot_of` maps a `VarId` to its slot.
    slots: Vec<VarId>,
    /// Provably-distinct-bindings: every occurrence's bound fields cover a
    /// unique constraint, so distinct facts imply distinct bindings and the
    /// aggregate sink may skip its seen-set (30-execution, elision).
    distinct_bindings: bool,
    /// Every node binds a sink-relevant variable (docs/perf/ PRD 09):
    /// `Flow::SkipSuffix` can never cross a node, so the pipelined
    /// executor's cross-node batching needs no cancellation machinery.
    skip_free: bool,
    /// The planner's per-step estimates (EXPLAIN's reader, the 30-execution doc).
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

    /// Slot order: the variable stored in each binding slot.
    #[must_use]
    pub fn slots(&self) -> &[VarId] {
        &self.slots
    }

    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        self.distinct_bindings
    }

    /// Whether a suffix skip can never cross a node — the pipelined
    /// executor's eligibility (docs/perf/ PRD 09).
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
