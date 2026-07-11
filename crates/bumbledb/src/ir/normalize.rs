//! Normalization (docs/architecture/20-query-ir.md): lowers a
//! [`crate::ir::validate::ValidatedQuery`] **rule by rule** into the
//! paper-form conjunctive queries execution consumes — the normalized
//! artifact is a list, one [`NormalizedQuery`] per rule, because the query
//! is a program. Each rule lowers exactly as the conjunctive query did:
//! distinct-variable atom
//! occurrences (positive and negated, one table with a [`Role`]), per-atom
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
use crate::schema::{FieldId, RelationId, StatementId, ValueType};

mod dnf;
mod lower_literal;
#[allow(clippy::module_inception)]
mod normalize;
mod place_comparisons;

pub use dnf::{collapse, disjunct_count, distribute, LoweredRule};
pub(crate) use lower_literal::lower_literal;
pub use normalize::normalize;

/// Dense atom-occurrence id. Everything downstream (plan validity, trie
/// schemas) quantifies over occurrences, never relation names — self-joins
/// are ordinary. Positive occurrences are numbered first, negated after
/// (the same order validation diagnostics use).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OccId(pub u16);

/// An occurrence's planning state — one sum, deliberately: a polarity
/// flag plus an `eliminated: Option<StatementId>` would admit
/// negated ∧ eliminated, a state the chase's conditions forbid
/// (`plan/chase.rs`), and index-shifting removal would move every
/// [`OccId`] downstream. One occurrence table holds all three states;
/// occurrence ids never move.
///
/// - `Positive`: joins the plan — the only role
///   [`Role::participates`] admits.
/// - `Negated`: joins no plan node; reached exclusively through its
///   [`AntiProbe`] descriptor (`docs/architecture/20-query-ir.md`,
///   § normalization step 4).
/// - `Eliminated`: a positive occurrence the chase removed — the mark
///   carries the containment statement that justified it and doubles
///   as the EXPLAIN record; no separate eliminated-list exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Positive,
    Negated,
    Eliminated(StatementId),
}

impl Role {
    /// **The** participates-in-planning predicate: whether the
    /// occurrence joins the plan — enters the DP, appears in subatoms,
    /// binds variables, and counts toward plan validity. Negated
    /// occurrences only reject bindings; eliminated occurrences are
    /// proven redundant (`plan/chase.rs`). Every planner, stats, and
    /// witness iteration routes through this one match.
    #[must_use]
    pub fn participates(self) -> bool {
        matches!(self, Self::Positive)
    }
}

/// One atom occurrence in paper form: distinct variables only, plus the
/// filters lowered out of its bindings. For a negated occurrence, `vars`
/// are the anti-probe's key fields and `filters` are its own filter list,
/// evaluated inside the probe (`docs/architecture/40-execution.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub occ_id: OccId,
    pub relation: RelationId,
    pub role: Role,
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
/// 40-execution doc's job). Scalar single-word semantics: interval
/// comparisons never reach here — interval `Eq`/`Ne` canonicalize to
/// masks ([`PlacedAllen`]) and point containment decomposes into
/// [`PlacedWordComparison`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedComparison {
    pub op: CmpOp,
    pub lhs: VarId,
    pub rhs: VarId,
}

/// A cross-atom measure residual: `Duration(interval) <op> scalar` where
/// the u64 side is another occurrence's variable — the measure always on
/// the left (a comparison written scalar-first mirrors its operator at
/// lowering, so no operand-order flag exists). Evaluated at the earliest
/// plan node binding both variables, exactly where whole-value residuals
/// attach: read the interval variable's two slot words, test the ray
/// (`end == MAX` raises [`crate::Error::MeasureOfRay`] — the engine's
/// one runtime type error), subtract, compare the u64 word. Var-vs-
/// constant and same-atom measure comparisons never reach here — they
/// lower to the occurrence's filter list
/// ([`FilterPredicate::DurationCompare`] /
/// [`FilterPredicate::DurationFieldsCompare`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedDuration {
    /// The measured interval variable (two slot words).
    pub interval: VarId,
    /// The order operator, measure-side-left.
    pub op: CmpOp,
    /// The u64 comparison side.
    pub scalar: VarId,
}

/// A cross-atom `Allen` residual: two interval variables and the mask —
/// four endpoint slot words (each side's pair at its slot base) plus the
/// mask, evaluated classify-then-test at the earliest plan node where
/// both sides are bound, exactly where whole-value residuals attach. The
/// mask stays symbolic ([`crate::ir::MaskTerm`]): a param mask resolves
/// per execution ([`crate::exec::run::Executor::bind_allen_masks`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedAllen {
    pub lhs: VarId,
    pub rhs: VarId,
    pub mask: crate::ir::MaskTerm,
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

/// One word comparison of a decomposed cross-atom point containment
/// (`Contains(a, p)` between different occurrences' variables):
/// `lhs <op> rhs` over binding-slot words — the one fixed composition
/// (`docs/architecture/20-query-ir.md`, § normalization):
///
/// - `Contains(a, p: point)` ≡ `a.start ≤ p AND p < a.end`
///
/// so `op` is always `Lt` or `Le`. Interval-pair predicates are never
/// decomposed — they are [`PlacedAllen`] residuals carrying their mask.
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
    /// The negated occurrence ([`Role::Negated`] in the one occurrence
    /// table).
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
    /// Cross-atom point containments, decomposed into word comparisons
    /// over slot pairs.
    pub word_residuals: Vec<PlacedWordComparison>,
    /// Cross-atom `Allen` residuals: four endpoint slots + mask
    /// (interval `Eq`/`Ne` comparisons canonicalize here too — exactly
    /// one interval-pair form reaches the planner).
    pub allen_residuals: Vec<PlacedAllen>,
    /// Cross-atom measure residuals: two-slot read + ray test +
    /// subtraction feeding the ordinary word comparison
    /// ([`PlacedDuration`]).
    pub duration_residuals: Vec<PlacedDuration>,
    /// Anti-probe descriptors, one per negated occurrence, in occurrence
    /// order.
    pub anti_probes: Vec<AntiProbe>,
    /// Every variable's binding-slot width — the [`SlotWidth`] layout,
    /// exported to the plan witness.
    pub slot_widths: BTreeMap<VarId, SlotWidth>,
}

#[cfg(test)]
mod tests;
