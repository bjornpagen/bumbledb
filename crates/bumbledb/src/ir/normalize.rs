//! Normalization: lowers a
//! [`crate::ir::validate::ValidatedQuery`] **rule by rule** into the
//! paper-form conjunctive queries execution consumes — the normalized
//! artifact is a list, one [`NormalizedQuery`] per rule, because the query
//! is a rule list. Each rule lowers exactly as the conjunctive query did:
//! distinct-variable atom
use std::collections::BTreeMap;

use crate::image::view::FilterPredicate;
use crate::ir::VarId;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId, ValueType};

mod dnf;
mod fold;
mod lower_literal;
#[expect(
    clippy::module_inception,
    reason = "the nested module owns the operation named by its parent"
)]
mod normalize;
mod place_comparisons;

pub use dnf::{LoweredRule, collapse, disjunct_count, distribute, nesting_depth};
#[cfg(test)]
pub use fold::with_fold_disabled;
pub(crate) use fold::{decoded_interval, decoded_scalar, render_const};
pub(crate) use lower_literal::{fixed_bytes_word_buf, lower_literal};
pub use normalize::normalize_rules;

/// Dense atom-occurrence id. Everything downstream (plan validity, trie
/// schemas) quantifies over occurrences, never relation names — self-joins
/// are ordinary. Positive occurrences are numbered first, negated after
/// (the same order validation diagnostics use).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OccId(pub u16);

/// An occurrence's planning state — one sum, deliberately: a polarity
/// flag plus an `eliminated: Option<StatementId>` would admit
/// negated ∧ eliminated, a state the grounding's conditions forbid
/// (`plan/ground.rs`), and index-shifting removal would move every
/// [`OccId`] downstream. One occurrence table holds all four states;
/// occurrence ids never move.
/// - `Positive`: joins the plan — the only role
///   [`Role::participates`] admits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Positive,
    Negated,
    Eliminated(StatementId),
    Folded(FoldedMark),
}

/// The evaluator's mark (`plan/ground/evaluate.rs`): polarity as a sum,
/// σ-survivors (n ≤ 256) and the stored relation the fold ran against.
/// Not `Copy` — the parsed id list is the diagnostic source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldedMark {
    Positive {
        relation: RelationId,
        survivors: Box<[u64]>,
    },

    Negated {
        relation: RelationId,
        survivors: Box<[u64]>,
    },
}

/// How an occurrence binds: stored EDB vs a derived table, with the
/// rec-arm stamp on the derived arms. EDB-with-a-derived-role and
/// Interior-with-no-role are unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccBind {
    Edb(RelationId),
    Finished(crate::ir::InteriorId),
    RecDelta(crate::ir::InteriorId),
    RecAcc(crate::ir::InteriorId),
}

impl OccBind {
    #[must_use]
    pub const fn source(self) -> crate::ir::AtomSource {
        match self {
            Self::Edb(relation) => crate::ir::AtomSource::Edb(relation),
            Self::Finished(id) | Self::RecDelta(id) | Self::RecAcc(id) => {
                crate::ir::AtomSource::Interior(id)
            }
        }
    }

    #[must_use]
    pub const fn edb(self) -> Option<RelationId> {
        match self {
            Self::Edb(relation) => Some(relation),
            Self::Finished(_) | Self::RecDelta(_) | Self::RecAcc(_) => None,
        }
    }

    #[must_use]
    pub const fn interior(self) -> Option<crate::ir::InteriorId> {
        match self {
            Self::Edb(_) => None,
            Self::Finished(id) | Self::RecDelta(id) | Self::RecAcc(id) => Some(id),
        }
    }

    #[must_use]
    pub const fn of_occurrence(occurrence: &Occurrence) -> Self {
        occurrence.bind
    }
}

impl Role {
    #[must_use]
    pub fn participates(&self) -> bool {
        matches!(self, Self::Positive)
    }

    #[must_use]
    pub fn discharged(&self) -> bool {
        matches!(self, Self::Eliminated(_) | Self::Folded(_))
    }
}

/// One atom occurrence in paper form: distinct variables only, plus the
/// filters lowered out of its bindings. For a negated occurrence, `vars`
/// are the anti-probe's key fields and `filters` are its own filter list,
/// evaluated inside the probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub occ_id: OccId,
    pub role: Role,

    pub bind: OccBind,

    pub vars: Vec<(FieldId, VarId)>,

    pub filters: Vec<FilterPredicate>,

    pub point_vars: Vec<(FieldId, VarId)>,
}

impl Occurrence {
    #[must_use]
    pub const fn source(&self) -> crate::ir::AtomSource {
        self.bind.source()
    }
}

/// Which of a variable's binding words a residual side reads (the
/// [`SlotWidth`] layout): `Start` is a scalar variable's single word or an
/// interval variable's start word; `End` is an interval variable's end
/// word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalWord {
    Start,
    End,
}

impl IntervalWord {
    #[must_use]
    pub fn offset(self) -> usize {
        match self {
            Self::Start => 0,
            Self::End => 1,
        }
    }
}

/// A lowered negated atom: the anti-probe residual descriptor. Attached,
/// like residual comparisons, to the earliest plan node where all its
/// variables are bound. The
/// probe: any fact of the occurrence matching `probe_bindings` under the
/// current binding — with the occurrence's own filter list
/// ([`Occurrence::filters`]) evaluated inside the probe — **rejects** the
/// binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiProbe {
    pub occurrence: OccId,

    pub probe_bindings: Vec<(FieldId, VarId)>,
}

/// Binding-slot width of one variable — **the multi-slot layout decision,
/// made here and nowhere else**: an interval-typed variable occupies
/// **two consecutive u64 slots** — (start word, end word), in encoded
/// column-word order — in the VarId-indexed binding-slot array; a
/// `bytes<N>` variable occupies its `⌈N/8⌉` padded-word slots in byte
/// order (the interval two-slot precedent, generalized); every other
/// variable occupies one. Exported through
/// [`NormalizedQuery::slot_widths`] into the plan witness's binding-slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotWidth(u8);

impl SlotWidth {
    pub const ONE: Self = Self(1);

    pub const TWO: Self = Self(2);

    #[must_use]
    pub fn of(value_type: &ValueType) -> Self {
        match value_type {
            ValueType::Interval { .. } | ValueType::FixedInterval { .. } => Self::TWO,
            ValueType::FixedBytes { len } => Self(
                u8::try_from(crate::encoding::fixed_bytes_words(*len))
                    .expect("bytes width is at most 8 words"),
            ),
            _ => Self::ONE,
        }
    }

    #[must_use]
    pub fn slots(self) -> usize {
        usize::from(self.0)
    }
}

/// The paper-form query: occurrences + per-atom filters + the residual
/// list (word comparisons and anti-probes — exactly those; nothing
/// single-occurrence survives to residuals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedQuery {
    pub occurrences: Vec<Occurrence>,

    pub residuals: Vec<FilterPredicate>,

    pub word_residuals: Vec<FilterPredicate>,

    pub allen_residuals: Vec<FilterPredicate>,

    pub anti_probes: Vec<AntiProbe>,

    pub slot_widths: BTreeMap<VarId, SlotWidth>,

    pub dead: Option<String>,
}

#[cfg(test)]
mod tests;
