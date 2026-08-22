//! Normalization (docs/architecture/20-query-ir.md): lowers a
//! [`crate::ir::validate::ValidatedQuery`] **rule by rule** into the
//! paper-form conjunctive queries execution consumes — the normalized
//! artifact is a list, one [`NormalizedQuery`] per rule, because the query
//! is a rule list. Each rule lowers exactly as the conjunctive query did:
//! distinct-variable atom
//! occurrences (positive and negated, one table with a [`Role`]), per-atom
//! filters (membership and interval conditions included), and the residual
//! list: cross-atom comparisons, decomposed interval word comparisons, and
//! anti-probe descriptors (`docs/architecture/20-query-ir.md`, Deviation
//! vs paper §2: the paper's all-distinct-variables / pushed-selections
//! assumption is a WLOG; we own the lowering because there is no external
//! optimizer).
//!
//! Infallible: the witness guarantees every input is lowerable.

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
///
/// - `Positive`: joins the plan — the only role
///   [`Role::participates`] admits.
/// - `Negated`: joins no plan node; reached exclusively through its
///   [`AntiProbe`] descriptor (`docs/architecture/20-query-ir.md`,
///   § normalization step 4).
/// - `Eliminated`: a positive occurrence the grounding removed — the mark
///   carries the containment statement that justified it and doubles
///   as the introspection record; no separate eliminated-list exists.
/// - `Folded`: a closed-relation occurrence the grounding **evaluated at
///   prepare** (`plan/ground/evaluate.rs`): its filters ran against the
///   sealed extension and the atom's whole contribution became a
///   plan-constant membership set on its siblings (or nothing at all,
///   for a satisfied check). Unlike `Eliminated`, a folded occurrence
///   may have been negated — the mark records the polarity because the
///   occurrence's own role no longer does. The filters stay on the
///   occurrence (introspection renders them); nothing downstream resolves,
///   probes, or scans them.
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
    /// σ-survivors (`evaluate.rs` `surviving_ids`).
    Positive {
        relation: RelationId,
        survivors: Box<[u64]>,
    },
    /// Same σ-survivors, not the complement attached to sibling binders.
    /// Empty box = deleted anti-probe.
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
    /// The atom source this bind reads — stored relation or derived table.
    #[must_use]
    pub const fn source(self) -> crate::ir::AtomSource {
        match self {
            Self::Edb(relation) => crate::ir::AtomSource::Edb(relation),
            Self::Finished(id) | Self::RecDelta(id) | Self::RecAcc(id) => {
                crate::ir::AtomSource::Interior(id)
            }
        }
    }

    /// Stored relation, if this bind is EDB.
    #[must_use]
    pub const fn edb(self) -> Option<RelationId> {
        match self {
            Self::Edb(relation) => Some(relation),
            Self::Finished(_) | Self::RecDelta(_) | Self::RecAcc(_) => None,
        }
    }

    /// Derived table id, if this bind is not EDB.
    #[must_use]
    pub const fn interior(self) -> Option<crate::ir::InteriorId> {
        match self {
            Self::Edb(_) => None,
            Self::Finished(id) | Self::RecDelta(id) | Self::RecAcc(id) => Some(id),
        }
    }

    /// The bind already sealed on a normalized occurrence.
    #[must_use]
    pub const fn of_occurrence(occurrence: &Occurrence) -> Self {
        occurrence.bind
    }
}

impl Role {
    /// **The** participates-in-planning predicate: whether the
    /// occurrence joins the plan — enters the DP, appears in subatoms,
    /// binds variables, and counts toward plan validity. Negated
    /// occurrences only reject bindings; eliminated and folded
    /// occurrences are proven redundant (`plan/ground.rs`). Every
    /// planner, stats, and witness iteration routes through this one
    /// match.
    #[must_use]
    pub fn participates(&self) -> bool {
        matches!(self, Self::Positive)
    }

    /// Whether the grounding discharged this occurrence from execution
    /// entirely (eliminated or folded): no statistics read, no view, no
    /// image, no filter resolution, no selection probe — the negative
    /// space of [`Role::participates`] that negated occurrences (which
    /// still probe through their anti-probes) do **not** share. Every
    /// execution-side skip routes through this one predicate
    /// (`api/prepared/{bind,build,run_join}.rs`).
    #[must_use]
    pub fn discharged(&self) -> bool {
        matches!(self, Self::Eliminated(_) | Self::Folded(_))
    }
}

/// One atom occurrence in paper form: distinct variables only, plus the
/// filters lowered out of its bindings. For a negated occurrence, `vars`
/// are the anti-probe's key fields and `filters` are its own filter list,
/// evaluated inside the probe (`docs/architecture/40-execution.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub occ_id: OccId,
    pub role: Role,
    /// How this occurrence binds: stored EDB or a derived table, with
    /// the rec-arm stamp on Reach self-reads. The atom source is
    /// [`OccBind::source`].
    pub bind: OccBind,
    /// Distinct variables with the field each is read from (a repeated
    /// variable keeps its first field; later positions became filters).
    /// A membership-bound point variable is **not** a variable of the
    /// occurrence — its binding lowered to a filter
    /// ([`FilterPredicate::PointIn`] / [`FilterPredicate::FieldsPointIn`]).
    pub vars: Vec<(FieldId, VarId)>,
    /// Per-occurrence filters, evaluated at the source (filtered view).
    /// Var-sourced membership is [`Self::point_vars`], not a filter —
    /// the view evaluator never sees a staging token.
    pub filters: Vec<FilterPredicate>,
    /// Var-sourced point membership (`interval-field ∋ var`) lifted by
    /// plan validation into [`crate::plan::fj::PointProbe`]. Not a view
    /// filter.
    pub point_vars: Vec<(FieldId, VarId)>,
}

impl Occurrence {
    /// The atom source this occurrence reads.
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
    /// Slot offset from the variable's first slot.
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

/// Binding-slot width of one variable — **the multi-slot layout decision,
/// made here and nowhere else**: an interval-typed variable occupies
/// **two consecutive u64 slots** — (start word, end word), in encoded
/// column-word order — in the VarId-indexed binding-slot array; a
/// `bytes<N>` variable occupies its `⌈N/8⌉` padded-word slots in byte
/// order (the interval two-slot precedent, generalized); every other
/// variable occupies one. Exported through
/// [`NormalizedQuery::slot_widths`] into the plan witness's binding-slot
/// layout and consumed everywhere slots are addressed: residual word
/// comparisons ([`VarWord`] selects within an interval pair via
/// [`IntervalWord::offset`]), the executor's slot arrays and probe keys,
/// and the sinks' binding reads (PRDs 15/16/17/18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotWidth(u8);

impl SlotWidth {
    /// The scalar width (every single-word type).
    pub const ONE: Self = Self(1);
    /// The interval width: (start word, end word).
    pub const TWO: Self = Self(2);

    /// The width of a variable of this type (see the type-level comment —
    /// the one place the layout is decided).
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

    /// Number of consecutive u64 slots.
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
    /// Positive occurrences first, then negated — [`OccId`]s are indices.
    pub occurrences: Vec<Occurrence>,
    /// Cross-atom whole-value comparisons (`FieldsCompare` over variables).
    pub residuals: Vec<FilterPredicate>,
    /// Cross-atom point memberships, decomposed into word comparisons
    /// over slot pairs (`FieldsCompare` with [`OperandAddr::var_word`]).
    pub word_residuals: Vec<FilterPredicate>,
    /// Cross-atom `Allen` residuals: four endpoint slots + mask
    /// (interval `Eq`/`Ne` comparisons canonicalize here too — exactly
    /// one interval-pair form reaches the planner).
    pub allen_residuals: Vec<FilterPredicate>,
    /// Anti-probe descriptors, one per negated occurrence, in occurrence
    /// order — minus the ones the grounding-evaluator folded away
    /// (`plan/ground/evaluate.rs` deletes a folded negated occurrence's
    /// descriptor: the rejection it encoded became a plan-constant
    /// complement membership on the siblings, or provably never fired).
    pub anti_probes: Vec<AntiProbe>,
    /// Every variable's binding-slot width — the [`SlotWidth`] layout,
    /// exported to the plan witness.
    pub slot_widths: BTreeMap<VarId, SlotWidth>,
    /// The statically-empty verdict: `Some` iff the rule provably
    /// denotes ∅ on constants alone — the rendered killing condition
    /// (e.g. `R: a ∈ [8, 19] ∧ a == 3`), because introspection must print what
    /// refuted the rule. Two writers, one channel: the normalization
    /// fold (`fold.rs`, mutually unsatisfiable constant conditions) and
    /// the grounding-evaluator (`plan/ground/evaluate.rs`, a closed atom
    /// whose prepare-time evaluation empties — `folded to ∅: …`). A dead
    /// rule is deleted at prepare (`api/prepared/build.rs`); a query
    /// of only dead rules prepares to `PreparedPipeline::Cq` with an
    /// empty main rule list.
    pub dead: Option<String>,
}

#[cfg(test)]
mod tests;
