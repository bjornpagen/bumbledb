//! Free Join plan lowering: `binary2fj` (paper Fig. 7), the conservative `factor`
//! hoist (Fig. 8), the `gj_split` lowering to the GJ end of the spectrum, cover
//! enumeration (§4.4), residual and anti-probe placement, trie schemas (§3.3), and
//! the sealed [`ValidatedPlan`] witness. Free Join: Wang et al., SIGMOD 2023.
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

pub(crate) use binary2fj::binary2fj;
pub(crate) use check_selections::check_selections;
pub(crate) use factor::factor;
pub(crate) use fold_split::fold_split;
pub(crate) use gj_split::gj_split;
pub(crate) use provably_distinct::{
    DistinctWitness, Distinctness, ProjectionDistinctWitness, provably_distinct,
    provably_distinct_projection,
};

pub(crate) use crate::ir::normalize::OccBind;

pub(crate) use split_filters::split_filters;
#[cfg(test)]
pub(crate) use validate::validate;
pub(crate) use validate::validate_with_signatures;

/// A subatom: one occurrence with a subset of its variables. The plan
/// partitions every **positive** occurrence's variables across its
/// subatoms; negated occurrences join no node — they are reached only
/// through anti-probes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Subatom {
    pub occ: OccId,
    pub vars: Vec<VarId>,
}

/// One plan node: a list of subatoms. Executed as: iterate the chosen
/// cover, probe the rest in order. `estimate` is the planner's per-step
/// row count — copied through fold-split, sealed onto [`PlanNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub subatoms: Vec<Subatom>,
    pub estimate: u64,
}

/// A Free Join plan: a list of nodes partitioning the query's positive
/// occurrences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FjPlan {
    pub nodes: Vec<Node>,
}

/// A plan-validation failure. Plans built by `binary2fj` + `factor` are
/// valid by construction; this boundary exists because [`FjPlan`] is plain
/// data anyone can construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlanError {
    /// A participating occurrence's subatoms do not partition its
    BrokenPartition {
        occ: OccId,
    },

    MissingOccurrence {
        occ: OccId,
    },

    UnknownOccurrence {
        node: usize,
        occ: OccId,
    },

    NonParticipatingOccurrenceInNode {
        node: usize,
        occ: OccId,
    },

    DuplicateOccurrenceInNode {
        node: usize,
        occ: OccId,
    },

    NoCover {
        node: usize,
    },

    UnplacedResidual {
        residual: usize,
    },

    UnplacedWordResidual {
        residual: usize,
    },

    UnplacedAllenResidual {
        residual: usize,
    },

    UnplacedAntiProbe {
        anti_probe: usize,
    },

    SelectionOnFilteredField {
        occ: OccId,
    },

    UnplacedPointProbe {
        occ: OccId,
    },
}

/// One probeable equality: `field == value`, the value constant per
/// execution (literal word/byte, param slot, param set, or pending
/// intern — literals and params are the same machine). Selections are
/// the probe-not-scan half of an occurrence's conditions; `filters`
/// keeps the scannable rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
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
pub(crate) struct PointProbe {
    pub occ: OccId,

    /// `(interval field, point var, dense)` — as
    /// [`PlanOccurrence::point_filters`].
    pub filters: Vec<(FieldId, VarId, bool)>,
}

/// One occurrence's execution-facing description — every role lives in
/// the one table ([`OccId`]s are indices): negated occurrences appear in
/// no subatom and are probed through the nodes' `anti_probes`;
/// grounding-eliminated occurrences appear nowhere at all and their view is
/// never built (`plan/ground.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanOccurrence {
    pub occ_id: OccId,

    pub role: Role,

    pub bind: OccBind,

    pub vars: Vec<(FieldId, VarId)>,

    pub selections: Vec<Selection>,

    /// The occurrence's filters that remain after its selections are split
    /// off (`split_filters`).
    pub filters: Vec<FilterPredicate>,

    /// `(interval field, point var, dense)` — dense carries the F64
    /// finite-probe guard into the executor's membership probes.
    pub point_filters: Vec<(FieldId, VarId, bool)>,

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
pub(crate) struct PlanNode {
    pub subatoms: Vec<Subatom>,

    pub covers: Vec<u8>,

    /// Residuals are grouped by kind (whole-variable compares, one-word
    /// compares, Allen compares), so each executor pass reads one kind.
    pub residuals: Vec<FilterPredicate>,

    pub word_residuals: Vec<FilterPredicate>,

    pub allen_residuals: Vec<FilterPredicate>,

    pub anti_probes: Vec<AntiProbe>,

    pub point_probes: Vec<PointProbe>,

    pub new_vars: Vec<VarId>,

    pub suffix_skip: SuffixSkip,

    pub estimate: u64,
}

/// Plan evidence for subtree cancellation. `Licensed` means this node
/// binds only existential variables for the active projection shape;
/// aggregate validation supplies every variable as sink-relevant, so its
/// plans contain only `Forbidden`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SuffixSkip {
    Forbidden,
    Licensed,
}

/// The sealed plan witness execution trusts; validated once at
/// construction, nothing downstream re-checks (post-mortem §38).
#[derive(Debug)]
pub(crate) struct ValidatedPlan {
    occurrences: Vec<PlanOccurrence>,
    nodes: Vec<PlanNode>,

    slots: Vec<(VarId, SlotWidth)>,

    distinctness: Distinctness,
}

impl ValidatedPlan {
    /// A physical traversal contract, not a schema multiplicity proof.
    /// Every cover must enumerate distinct new-variable tuples, including
    /// terminal COLT groups. One unique prefix then extends uniquely at
    /// every node; sibling probes and residuals only remove bindings.
    pub(crate) fn scalar_set_traversal(&self) -> Option<ScalarSetTraversal> {
        (self.distinct_witness().is_none()
            && self.slots.iter().all(|(_, width)| width.slots() == 1)
            && self.occurrences.iter().all(|occurrence| {
                occurrence.bind.edb().is_some()
                    && occurrence.role != Role::Negated
                    && occurrence.point_filters.is_empty()
            })
            && self.nodes.iter().all(|node| {
                node.point_probes.is_empty()
                    && node.anti_probes.is_empty()
                    && node.allen_residuals.is_empty()
                    && node.word_residuals.is_empty()
            }))
        .then_some(ScalarSetTraversal(()))
    }

    #[must_use]
    pub(crate) fn occurrences(&self) -> &[PlanOccurrence] {
        &self.occurrences
    }

    #[must_use]
    pub(crate) fn nodes(&self) -> &[PlanNode] {
        &self.nodes
    }

    #[must_use]
    pub(crate) fn slots(&self) -> &[(VarId, SlotWidth)] {
        &self.slots
    }

    #[must_use]
    pub(crate) fn slot_count(&self) -> usize {
        self.slots.iter().map(|(_, width)| width.slots()).sum()
    }

    #[must_use]
    pub(crate) fn is_negated(&self, occ: OccId) -> bool {
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
    pub(crate) fn estimates(&self) -> Vec<u64> {
        self.nodes.iter().map(|node| node.estimate).collect()
    }

    /// # Panics
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub(crate) fn slot_of(&self, var: VarId) -> usize {
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
    pub(crate) fn width_of(&self, var: VarId) -> usize {
        self.slots
            .iter()
            .find(|(candidate, _)| *candidate == var)
            .map(|(_, width)| width.slots())
            .expect("validated plan binds every variable")
    }

    /// Every planned variable's `(var, first slot, width)` in slot order: the
    /// DNF-derived union regime's dedup key layout.
    #[must_use]
    pub(crate) fn slot_spans(&self) -> Vec<(VarId, usize, usize)> {
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
    pub(crate) fn occurrence(&self, occ: OccId) -> &PlanOccurrence {
        self.occurrences
            .iter()
            .find(|o| o.occ_id == occ)
            .expect("validated plan covers its occurrences")
    }
}

/// Valid only while the resident executor forces distinct cover iteration.
/// Unlike `DistinctWitness`, this cannot license raw source multiplicities,
/// fallback execution, or unioning multiple rule traversals.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScalarSetTraversal(());

#[cfg(test)]
mod tests;
