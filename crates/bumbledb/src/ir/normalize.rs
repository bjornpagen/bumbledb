//! Normalization (docs/architecture/20-query-ir.md): lowers a [`ValidatedQuery`] into the paper-form
//! conjunctive query execution consumes — distinct-variable atom
//! occurrences (positive and negated, one table with a polarity), per-atom
//! filters (membership and interval predicates included), and the residual
//! list: cross-atom comparisons, decomposed interval word comparisons, and
//! anti-probe descriptors (`docs/architecture/20-query-ir.md`, Deviation
//! vs paper §2: the paper's all-distinct-variables / pushed-selections
//! assumption is a WLOG; we own the lowering because there is no external
//! optimizer).
//!
//! Infallible: the witness guarantees every input is lowerable.

use std::collections::BTreeMap;

use crate::image::view::FilterPredicate;
use crate::ir::{CmpOp, VarId};
use crate::schema::{FieldId, RelationId, ValueType};

mod lower_literal;
#[allow(clippy::module_inception)]
mod normalize;
mod place_comparisons;

pub use normalize::normalize;

/// Dense atom-occurrence id. Everything downstream (plan validity, trie
/// schemas) quantifies over occurrences, never relation names — self-joins
/// are ordinary. Positive occurrences are numbered first, negated after
/// (the same order validation diagnostics use).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OccId(pub u16);

/// Whether an occurrence joins the plan or only rejects bindings. One
/// occurrence table holds both — plan validity quantifies over **positive**
/// occurrences only; a negated occurrence joins no plan node and is reached
/// exclusively through its [`AntiProbe`] descriptor
/// (`docs/architecture/20-query-ir.md`, § normalization step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    Positive,
    Negated,
}

/// One atom occurrence in paper form: distinct variables only, plus the
/// filters lowered out of its bindings. For a negated occurrence, `vars`
/// are the anti-probe's key fields and `filters` are its own filter list,
/// evaluated inside the probe (`docs/architecture/40-execution.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub occ_id: OccId,
    pub relation: RelationId,
    pub polarity: Polarity,
    /// Distinct variables with the field each is read from (a repeated
    /// variable keeps its first field; later positions became filters).
    /// A membership-bound point variable is **not** a variable of the
    /// occurrence — its binding lowered to a filter
    /// ([`FilterPredicate::PointIn`] / [`FilterPredicate::FieldsContainPoint`]).
    pub vars: Vec<(FieldId, VarId)>,
    /// Per-occurrence filters, evaluated at the source (filtered view).
    pub filters: Vec<FilterPredicate>,
}

/// A comparison whose sides are variables — evaluated inside the join at
/// the earliest plan node where both are bound (placement is the
/// 40-execution doc's job). Whole-value semantics: an interval-typed
/// variable compares **pairwise** over its two slot words (`Eq`/`Ne` are
/// the only operators that reach here for intervals — `Overlaps`/
/// `Contains`/membership decompose into [`PlacedWordComparison`]s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedComparison {
    pub op: CmpOp,
    pub lhs: VarId,
    pub rhs: VarId,
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
    /// Slot offset from the variable's first slot.
    #[must_use]
    pub fn offset(self) -> usize {
        match self {
            Self::Start => 0,
            Self::End => 1,
        }
    }
}

/// One residual operand: a bound variable's word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarWord {
    pub var: VarId,
    pub word: IntervalWord,
}

/// One word comparison of a decomposed cross-atom interval residual
/// (`Overlaps`/`Contains`/membership between different occurrences'
/// variables): `lhs <op> rhs` over binding-slot words. The three
/// compositions (`docs/architecture/20-query-ir.md`, § normalization):
///
/// - `Overlaps(a, b)` ≡ `a.start < b.end AND b.start < a.end`
/// - `Contains(a, b: interval)` ≡ `a.start ≤ b.start AND b.end ≤ a.end`
/// - `Contains(a, p: point)` ≡ `a.start ≤ p AND p < a.end`
///
/// so `op` is always `Lt` or `Le`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedWordComparison {
    pub op: CmpOp,
    pub lhs: VarWord,
    pub rhs: VarWord,
}

/// A lowered negated atom: the anti-probe residual descriptor. Attached,
/// like residual comparisons, to the earliest plan node where all its
/// variables are bound (the attachment computation is plan-time — PRD 15;
/// normalization produces the descriptor with its variable set). The
/// probe: any fact of the occurrence matching `probe_bindings` under the
/// current binding — with the occurrence's own filter list
/// ([`Occurrence::filters`]) evaluated inside the probe — **rejects** the
/// binding (`docs/architecture/40-execution.md`, § anti-probe filters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiProbe {
    /// The negated occurrence (polarity [`Polarity::Negated`] in the one
    /// occurrence table).
    pub occurrence: OccId,
    /// The occurrence's variable bindings — the probe's key fields, and
    /// the variable set the plan attaches by.
    pub probe_bindings: Vec<(FieldId, VarId)>,
}

/// Binding-slot width of one variable — **the two-slot interval layout
/// decision, made here and nowhere else**: an interval-typed variable
/// occupies **two consecutive u64 slots** — (start word, end word), in
/// encoded column-word order — in the VarId-indexed binding-slot array;
/// every other variable occupies one. Exported through
/// [`NormalizedQuery::slot_widths`] into the plan witness's binding-slot
/// layout and consumed everywhere slots are addressed: residual word
/// comparisons ([`VarWord`] selects within the pair via
/// [`IntervalWord::offset`]), the executor's slot arrays and probe keys,
/// and the sinks' binding reads (PRDs 15/16/17/18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotWidth {
    One,
    Two,
}

impl SlotWidth {
    /// The width of a variable of this type (see the type-level comment —
    /// the one place the layout is decided).
    #[must_use]
    pub fn of(value_type: &ValueType) -> Self {
        match value_type {
            ValueType::Interval { .. } => Self::Two,
            _ => Self::One,
        }
    }

    /// Number of consecutive u64 slots.
    #[must_use]
    pub fn slots(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }
}

/// The paper-form query: occurrences + per-atom filters + the residual
/// list (word comparisons and anti-probes — exactly those; nothing
/// single-occurrence survives to residuals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedQuery {
    /// Positive occurrences first, then negated — [`OccId`]s are indices.
    pub occurrences: Vec<Occurrence>,
    /// Cross-atom whole-value comparisons.
    pub residuals: Vec<PlacedComparison>,
    /// Cross-atom interval predicates, decomposed into word comparisons
    /// over slot pairs.
    pub word_residuals: Vec<PlacedWordComparison>,
    /// Anti-probe descriptors, one per negated occurrence, in occurrence
    /// order.
    pub anti_probes: Vec<AntiProbe>,
    /// Every variable's binding-slot width — the [`SlotWidth`] layout,
    /// exported to the plan witness.
    pub slot_widths: BTreeMap<VarId, SlotWidth>,
}

#[cfg(test)]
mod tests;
