//! Free Join plan lowering: `binary2fj` (paper Fig. 7), the
//! conservative `factor` hoist (Fig. 8), the `gj_split` lowering to
//! the GJ end of the spectrum (ruled 2026-07-23, R19), cover
//! enumeration (§4.4), residual and anti-probe placement, trie schemas
//! (§3.3), and the sealed [`ValidatedPlan`] witness
//! .
//! Plain `Vec`s everywhere — no fixed-capacity silent-drop containers

use crate::image::ColumnSpan;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::VarId;
use crate::ir::normalize::{AntiProbe, OccId, Role, SlotWidth};
use bumbledb_theory::schema::FieldId;

mod binary2fj;
mod check_occurrence_coverage;
mod check_selections;
mod derive_nodes;
mod factor;
mod fold_split;
mod gj_split;
mod provably_distinct;
mod split_filters;
mod validate;

pub use binary2fj::binary2fj;
pub(crate) use check_selections::check_selections;
pub use factor::factor;
pub use fold_split::fold_split;
pub use gj_split::gj_split;
pub(crate) use provably_distinct::{DistinctWitness, Distinctness, provably_distinct};

pub(crate) use crate::ir::normalize::OccBind;

pub(crate) use split_filters::split_filters;
#[cfg(test)]
pub use validate::validate;
pub use validate::validate_with_signatures;

/// A subatom: one occurrence with a subset of its variables. The plan
/// partitions every **positive** occurrence's variables across its
/// subatoms; negated occurrences join no node — they are reached only
/// through anti-probes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subatom {
    pub occ: OccId,
    pub vars: Vec<VarId>,
}

/// One plan node: a list of subatoms. Executed as: iterate the chosen
/// cover, probe the rest in order. `estimate` is the planner's per-step
/// row count — copied through fold-split, sealed onto [`PlanNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub subatoms: Vec<Subatom>,
    pub estimate: u64,
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
    /// A participating occurrence's subatoms do not partition its

    BrokenPartition { occ: OccId },

    MissingOccurrence { occ: OccId },

    UnknownOccurrence { node: usize, occ: OccId },

    NonParticipatingOccurrenceInNode { node: usize, occ: OccId },

    DuplicateOccurrenceInNode { node: usize, occ: OccId },

    NoCover { node: usize },

    UnplacedResidual { residual: usize },

    UnplacedWordResidual { residual: usize },

    UnplacedAllenResidual { residual: usize },

    UnplacedAntiProbe { anti_probe: usize },

    SelectionOnFilteredField { occ: OccId },

    UnplacedPointProbe { occ: OccId },
}

/// One probeable equality: `field == value`, the value constant per
/// execution (literal word/byte, param slot, param set, or pending
/// intern — literals and params are the same machine). Selections are
/// the probe-not-scan half of an occurrence's conditions; `filters`
/// keeps the scannable rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub field: FieldId,
    pub value: Const,
}

/// One placed membership probe: a positive occurrence's var-sourced
/// membership filters, evaluated inside the join once (a) every point
/// variable is bound and (b) the occurrence's trie is fully descended —
/// the current binding, and the binding survives iff **one fact
/// satisfies every filter**. Grouped per
/// occurrence because the conjunction quantifies over one fact:
/// `∃f (P₁(f) ∧ P₂(f))`, never `∃f P₁(f) ∧ ∃f P₂(f)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointProbe {
    pub occ: OccId,

    pub filters: Vec<(FieldId, VarId)>,
}

/// One occurrence's execution-facing description — every role lives in
/// the one table ([`OccId`]s are indices): negated occurrences appear in
/// no subatom and are probed through the nodes' `anti_probes`;
/// grounding-eliminated occurrences appear nowhere at all and their view is
/// never built (`plan/ground.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanOccurrence {
    pub occ_id: OccId,

    pub role: Role,

    pub bind: OccBind,

    pub vars: Vec<(FieldId, VarId)>,

    pub selections: Vec<Selection>,

    /// before the subtraction, `split_filters`); every other one is

    pub filters: Vec<FilterPredicate>,

    pub point_filters: Vec<(FieldId, VarId)>,

    pub spans: Box<[ColumnSpan]>,

    pub trie_schema: Vec<Vec<VarId>>,

    pub key_widths: Vec<u16>,
}

impl PlanOccurrence {
    pub(crate) fn source(&self) -> crate::ir::AtomSource {
        self.bind.source()
    }
}

/// One validated node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub subatoms: Vec<Subatom>,

    pub covers: Vec<u8>,

    pub residuals: Vec<FilterPredicate>,
    // REFUSAL, recorded (the representation audit; do not re-litigate):

    // enum begging to exist. The merge is refused: grouped-by-kind IS

    pub word_residuals: Vec<FilterPredicate>,

    /// `residuals` (a fourth grouped-by-kind list, per the refusal above:

    pub allen_residuals: Vec<FilterPredicate>,

    pub anti_probes: Vec<AntiProbe>,

    pub point_probes: Vec<PointProbe>,

    pub new_vars: Vec<VarId>,

    pub suffix_skip: SuffixSkip,

    pub estimate: u64,
}

/// Plan evidence for D2 subtree cancellation. `Licensed` means this node
/// binds only existential variables for the active projection shape;
/// aggregate validation supplies every variable as sink-relevant, so its
/// plans contain only `Forbidden`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuffixSkip {
    Forbidden,
    Licensed,
}

/// The sealed plan witness execution trusts; validated once at
/// construction, nothing downstream re-checks (post-mortem §38).
#[derive(Debug)]
pub struct ValidatedPlan {
    occurrences: Vec<PlanOccurrence>,
    nodes: Vec<PlanNode>,

    slots: Vec<(VarId, SlotWidth)>,

    distinctness: Distinctness,
}

impl ValidatedPlan {
    #[must_use]
    pub fn occurrences(&self) -> &[PlanOccurrence] {
        &self.occurrences
    }

    pub(crate) fn occurrences_mut(&mut self) -> &mut [PlanOccurrence] {
        &mut self.occurrences
    }

    #[must_use]
    pub fn nodes(&self) -> &[PlanNode] {
        &self.nodes
    }

    #[must_use]
    pub fn slots(&self) -> &[(VarId, SlotWidth)] {
        &self.slots
    }

    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.iter().map(|(_, width)| width.slots()).sum()
    }

    #[must_use]
    pub fn is_negated(&self, occ: OccId) -> bool {
        self.occurrences[usize::from(occ.0)].role == Role::Negated
    }

    #[must_use]
    pub(crate) fn distinct_witness(&self) -> Option<DistinctWitness> {
        match self.distinctness {
            Distinctness::Proven(witness) => Some(witness),
            Distinctness::Unproven => None,
        }
    }

    #[must_use]
    pub fn estimates(&self) -> Vec<u64> {
        self.nodes.iter().map(|node| node.estimate).collect()
    }

    /// # Panics

    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        let mut slot = 0;
        for (candidate, width) in &self.slots {
            if *candidate == var {
                return slot;
            }
            slot += width.slots();
        }
        panic!("validated plan binds every variable")
    }

    /// # Panics

    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn width_of(&self, var: VarId) -> usize {
        self.slots
            .iter()
            .find(|(candidate, _)| *candidate == var)
            .map(|(_, width)| width.slots())
            .expect("validated plan binds every variable")
    }

    /// (ruled 2026-07-23, R2). Total by construction: grounding may have

    #[must_use]
    pub fn slot_spans(&self) -> Vec<(VarId, usize, usize)> {
        let mut spans = Vec::with_capacity(self.slots.len());
        let mut slot = 0;
        for (var, width) in &self.slots {
            spans.push((*var, slot, width.slots()));
            slot += width.slots();
        }
        spans.sort_unstable_by_key(|(var, ..)| *var);
        spans
    }

    /// # Panics

    /// On a programmer-invariant violation: an occurrence outside the plan.
    #[cfg(test)]
    #[must_use]
    pub fn occurrence(&self, occ: OccId) -> &PlanOccurrence {
        self.occurrences
            .iter()
            .find(|o| o.occ_id == occ)
            .expect("validated plan covers its occurrences")
    }
}

#[cfg(test)]
mod tests;
