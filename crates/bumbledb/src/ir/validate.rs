//! The single validation boundary (docs/architecture/20-query-ir.md): IR in, [`ValidatedQuery`]
//! witness out. Everything downstream trusts the witness and re-checks
//! nothing (post-mortem §38: v5 validated one plan four times).
//!
//! The roster, transcribed from `docs/architecture/20-query-ir.md` and
//! checked off in code order below — it is exhaustive by contract.
//!
//! The program shape first (rules are validated one at a time; every
//! rule-local diagnostic names a position inside the first failing rule):
//!
//!  0. empty rule set; more than [`crate::ir::MAX_RULES`] rules (counted
//!     independently of the per-rule occurrence cap); head/rule positional
//!     arity, shape, or type mismatch (each rule's find terms align
//!     against the head position by position — rule 0's resolved type row
//!     pins the head's positional types, and every later rule must agree)
//!
//! Between the program shape and the per-rule roster, first the
//! **nesting boundary guard**: condition trees deeper than
//! [`crate::ir::MAX_CONDITION_DEPTH`] are the typed
//! `ConditionNestingTooDeep` — judged by an iterative depth walk before
//! any recursive tree walk runs, so hostile nesting is a rejection,
//! never a stack exhaustion (the trust-boundary law). Then **DNF
//! distribution** ([`crate::ir::distribute`]): each rule's condition
//! trees distribute to disjunctive normal form and each disjunct becomes
//! a rule — the structural term count past [`crate::ir::MAX_RULES`] is
//! the typed `DnfExceedsRules { produced, cap }` (judged before
//! materializing), duplicate rules collapse by normalized-form equality,
//! and a program whose every disjunction is empty is the empty union
//! (`EmptyRuleSet`). Everything below — and everything downstream —
//! reads the Or-free [`LoweredRule`]s; rule indices in diagnostics and
//! in the witness are **lowered-rule** indices.
//!
//! Then, per rule (a rule validates exactly as a conjunctive query did;
//! variables are rule-scoped, params query-global — param typing unifies
//! across rules after each rule's own fixpoint):
//!
//!  1. unknown relation ids
//!  2. unknown field ids
//!  3. duplicate `FieldId` in one atom's bindings
//!  4. variable type conflicts (structural — interval-field bindings
//!     anchor *bivalently*; see [`Context::resolve_bivalents`])
//!  5. literal-vs-field and param-anchor type mismatches (non-UTF-8
//!     String literals and `start >= end` interval literals included),
//!     and element-typed point literals at the domain ceiling wherever
//!     they meet an interval position — membership bindings and
//!     `PointIn` operands (the point-domain law: points are
//!     `MIN ..= MAX−1`; `MAX` is the ray's ∞ — point *params* get the
//!     same rejection at bind, where the value exists)
//!  6. enum ordinal out of range for the field's variant list (bindings
//!     and comparisons, each precisely diagnosed)
//!  7. param anchor conflicts (an *unanchored* param is unwritable by
//!     construction: every param position is itself an anchor; a mask
//!     param with any value anchor conflicts — a mask is not a
//!     data-model type) and non-dense param ids — dense across scalars,
//!     sets, and masks jointly
//!  8. a `ParamId` used both scalar and set; a `ParamSet` under any
//!     operator but `Eq`; an interval-typed `ParamSet` anchor
//!  9. comparisons violating the type rules (Eq/Ne all types; order ops
//!     U64/U64 and I64/I64 only — an interval operand under an order op
//!     gets its own diagnostic; Allen two intervals of one element type;
//!     `PointIn` interval × element — its interval⊇interval form is
//!     `Allen(COVERS)`, not an operator), and the Allen vacuity rules:
//!     the ∅ mask ("never" — write no query) and the full mask
//!     ("always" — write no condition), distinct typed errors here for
//!     literal masks and at bind for mask params
//! 10. constant comparisons (no variable side) and self-comparisons
//! 11. point variables bound only by membership (no enumerable domain)
//! 12. negated-atom variables not bound by any positive atom (negated
//!     atoms bind nothing; a query with no positive atoms is invalid)
//! 13. unbound find variables (Datalog safety; includes aggregate inputs)
//! 14. comparison-only variables
//! 15. empty finds
//! 16. duplicate find terms
//! 17. no positive atoms
//! 18. aggregate input types (Sum/Min/Max integers only; `CountDistinct`
//!     every type; Count nullary)
//! 19. aggregate over a group-key variable
//! 20. mixed Arg and fold aggregates; Arg terms with differing keys or
//!     directions; a non-orderable Arg key
//! 21. planner caps: more than `MAX_OCCURRENCES` atom occurrences —
//!     negated occurrences counted — or more than 128 distinct variables
//!     (rejected here so downstream id widths and bitset sizes are true
//!     invariants)

use std::collections::{BTreeMap, BTreeSet};

use crate::image::view::MaskConst;
use crate::ir::normalize::LoweredRule;
use crate::ir::{CmpOp, FindTerm, MaskTerm, ParamId, Value, VarId};
use crate::schema::{IntervalElement, ValueType};

mod context;
mod finds;
#[expect(
    clippy::module_inception,
    reason = "the nested module owns the operation named by its parent"
)]
mod validate;

pub use validate::validate;

/// The predicate a query defines — anonymous (names live in the host,
/// exactly like relations pre-`as`), its typed output signature derived
/// ONCE at validation and sealed. The single authority for sink
/// construction, result-buffer typing, finalize's all-words decision,
/// and EXPLAIN's header. Referenced by NOTHING — the named-view refusal
/// stands; a reference to a predicate is the recursion trigger firing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicate {
    /// The signature: one column per head position, in head order.
    pub columns: Box<[PredicateColumn]>,
}

impl std::fmt::Display for Predicate {
    /// The signature in one line — EXPLAIN's header (`(u64, Sum i64)`:
    /// declaration type spellings, rule-notation fold names).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        for (index, column) in self.columns.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            if let Some(op) = column.op {
                write!(f, "{op} ")?;
            }
            match &column.ty {
                ValueType::Bool => f.write_str("bool")?,
                ValueType::U64 => f.write_str("u64")?,
                ValueType::I64 => f.write_str("i64")?,
                ValueType::String => f.write_str("string")?,
                ValueType::FixedBytes { len } => write!(f, "bytes<{len}>")?,
                ValueType::Interval { element } => match element {
                    IntervalElement::U64 => f.write_str("interval<u64>")?,
                    IntervalElement::I64 => f.write_str("interval<i64>")?,
                },
            }
        }
        f.write_str(")")
    }
}

/// One column of the predicate's signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateColumn {
    /// The RESULT type — what lands in the buffer. Count is U64 here
    /// whatever it counted; Duration's measure is U64; Min/Max/Sum
    /// carry their input's type; Pack carries the interval type; the
    /// Arg forms carry the projected payload's type.
    pub ty: ValueType,
    /// None = plain projection; Some = the fold producing the column.
    /// Kept together deliberately: the sink needs both jointly, and a
    /// signature-only split would re-create a parallel table (decided
    /// here, not inherited from the sketch).
    pub op: Option<AggKind>,
}

/// The fold producing a predicate column, by kind alone: an Arg key is a
/// rule-scoped variable outside the signature's vocabulary, so the head
/// owns the payload-free kind (a projected measure is a plain column —
/// `None` — while a folded measure carries its fold's kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggKind {
    /// [`crate::ir::AggOp::Sum`].
    Sum,
    /// [`crate::ir::AggOp::Min`].
    Min,
    /// [`crate::ir::AggOp::Max`].
    Max,
    /// [`crate::ir::AggOp::Count`].
    Count,
    /// [`crate::ir::AggOp::CountDistinct`].
    CountDistinct,
    /// [`crate::ir::AggOp::ArgMax`], key elided.
    ArgMax,
    /// [`crate::ir::AggOp::ArgMin`], key elided.
    ArgMin,
    /// [`crate::ir::AggOp::Pack`].
    Pack,
}

impl std::fmt::Display for AggKind {
    /// The rule notation's fold names (`ir/render.rs`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Sum => "Sum",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Count => "Count",
            Self::CountDistinct => "CountDistinct",
            Self::ArgMax => "ArgMax",
            Self::ArgMin => "ArgMin",
            Self::Pack => "Pack",
        })
    }
}

/// A comparison's proven legal shape — validation's classification,
/// sealed. Constructed at the exact point the typed comparison rules
/// prove legality (`context.rs`: the proof and the seal are the same
/// lines), carried per rule on the witness
/// ([`RuleWitness::classified_comparisons`]), and consumed by
/// normalization's placement (`ir/normalize/place_comparisons.rs`) with
/// a **total** match — no shape is re-derived downstream, so no
/// defensive arm exists. The fifth sealed finding, alongside the
/// witness's typing tables, `ResolvableFilter`, `SinkSpec`, and
/// `ParamSpec`.
///
/// The variants are exactly the comparison language validation accepts —
/// nothing aspirational — and each carries the RESOLVED facts placement
/// needs: the rule-variable ids, the sealed constant or param handle,
/// the operator sealed variable-on-left / measure-on-left, the mask
/// sealed field-on-left. Interval `Eq`/`Ne` canonicalize here (the
/// `EQUALS` mask / its complement), so exactly one interval-pair form
/// leaves validation.
///
/// Pipeline-internal: never part of `ir.rs`'s input language, never in
/// the public API, never serialized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClassifiedComparison {
    /// Scalar var-vs-var under `Eq`/`Ne`/order — one shared non-interval
    /// type (interval equality seals as [`Self::AllenVarVar`]).
    VarVar { op: CmpOp, lhs: VarId, rhs: VarId },
    /// Scalar var-vs-constant under `Eq`/`Ne`/order, the operator sealed
    /// variable-on-left (a constant-first order comparison mirrors).
    VarConst {
        op: CmpOp,
        var: VarId,
        value: SealedConst,
    },
    /// `Eq` against the set marker (`Term::ParamSet` — legal under `Eq`
    /// alone): a selection-level word-set membership at execution.
    VarInSet { var: VarId, set: ParamId },
    /// The interval-pair comparison over two variables, the mask
    /// symbolic in written operand order.
    AllenVarVar {
        lhs: VarId,
        rhs: VarId,
        mask: MaskTerm,
    },
    /// The interval-pair comparison against a constant, the mask sealed
    /// field-on-left ([`MaskConst`]: conversed immediately for a literal
    /// written constant-first, `ConversedParam` for a param).
    AllenVarConst {
        var: VarId,
        other: SealedConst,
        mask: MaskConst,
    },
    /// Point containment between variables: `interval-var ∋ point-var`.
    PointInVarVar { interval: VarId, point: VarId },
    /// Point containment of a constant point: `interval-var ∋ point`.
    PointInVarPoint { interval: VarId, point: SealedConst },
    /// Point containment in a constant interval: `outer ∋ scalar-var`.
    VarWithin { var: VarId, outer: SealedConst },
    /// The measure comparison, the operator sealed measure-on-left:
    /// `Duration(interval) <op> other`.
    Duration {
        interval: VarId,
        op: CmpOp,
        other: DurationOperand,
    },
}

/// A classified comparison's sealed constant side: the bind-time param
/// handle, or the literal exactly as written (encoding to column words
/// stays normalization's job — the seal is IR-algebra only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SealedConst {
    Param(ParamId),
    Literal(Value),
}

/// The measure's comparison side: another rule variable (u64-resolved),
/// or a sealed constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DurationOperand {
    Var(VarId),
    Const(SealedConst),
}

/// The sealed witness: the query plus the derived tables downstream layers
/// trust. Unconstructible outside this module.
///
/// Variables are rule-scoped, so their typing lives per rule
/// ([`RuleWitness`]); params are query-global, so their tables live here
/// once — unified across the rules' own typing fixpoints.
#[derive(Debug)]
pub struct ValidatedQuery {
    /// The lowered program: Or-free rules, one per DNF disjunct of the
    /// input rules (duplicates collapsed) — the artifact everything
    /// downstream reads. No `Or` survives validation.
    lowered: Vec<LoweredRule>,
    /// The predicate the query defines, derived once from rule 0 after
    /// every rule was checked to derive the same signature (the per-rule
    /// positional alignment below).
    predicate: Predicate,
    /// Per rule, in rule order: its variable typing and group key.
    rules: Vec<RuleTyping>,
    param_types: BTreeMap<ParamId, ValueType>,
    /// Param ids bound as sets (`Term::ParamSet`); their entry in
    /// `param_types` is the *element* type.
    set_params: BTreeSet<ParamId>,
    /// Element-typed params meeting an interval position (membership
    /// bindings and `PointIn` operands): their values are points, so the
    /// point-domain law (`docs/architecture/10-data-model.md`) forbids the
    /// domain ceiling — enforced at bind, where the value exists.
    point_params: BTreeSet<ParamId>,
    /// Params in `Allen` mask positions ([`crate::ir::MaskTerm::Param`]):
    /// bound as [`crate::BindValue::AllenMask`], with the ∅/full vacuity
    /// rejection at bind. Disjoint from `param_types` — a mask is not a
    /// data-model type.
    mask_params: BTreeSet<ParamId>,
}

/// One rule's derived typing tables — rule-scoped by definition.
#[derive(Debug)]
struct RuleTyping {
    var_types: BTreeMap<VarId, ValueType>,
    /// Non-aggregated find variables — the group key under aggregation.
    group_key: BTreeSet<VarId>,
    /// The rule's comparisons, classified — one sealed proof per
    /// condition, in condition order ([`ClassifiedComparison`]).
    classified: Vec<ClassifiedComparison>,
}

impl ValidatedQuery {
    /// The predicate this query defines (see [`Predicate`]): the sealed
    /// signature every downstream consumer reads — no other derivation
    /// of the output row types exists.
    #[must_use]
    pub fn predicate(&self) -> &Predicate {
        &self.predicate
    }

    /// One rule's slice of the witness — the unit the per-rule pipeline
    /// (normalize → grounding → plan) consumes.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an index at or beyond
    /// [`Self::rule_count`].
    #[must_use]
    pub fn rule(&self, index: usize) -> RuleWitness<'_> {
        RuleWitness {
            rule: &self.lowered[index],
            typing: &self.rules[index],
            query: self,
        }
    }

    /// Every rule's witness slice, in rule order.
    pub fn rules(&self) -> impl Iterator<Item = RuleWitness<'_>> {
        (0..self.rules.len()).map(|index| self.rule(index))
    }

    /// The resolved type of a scalar param (for a set param this is the
    /// *element* type). Query-global: one binding surface, any rule may
    /// reference any param.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an unknown `ParamId` (the
    /// witness anchored every param).
    #[must_use]
    pub fn param_type(&self, param: ParamId) -> &ValueType {
        &self.param_types[&param]
    }

    /// Every param with its resolved type, in id order (bind-time checking,
    /// The 40-execution doc). A set param's type is its *element* type.
    pub fn param_types(&self) -> impl Iterator<Item = (ParamId, &ValueType)> {
        self.param_types.iter().map(|(p, t)| (*p, t))
    }

    /// The params bound as sets (`Term::ParamSet`) — bind-time expects a
    /// slice of values of the element type for each.
    #[must_use]
    pub fn set_params(&self) -> &BTreeSet<ParamId> {
        &self.set_params
    }

    /// The point-position params: element-typed at an interval position
    /// (a membership binding or a `PointIn` operand). Bind-time rejects
    /// their domain ceiling — points are `MIN ..= MAX−1`; `MAX` is the
    /// ray's ∞ (the point-domain law).
    #[must_use]
    pub fn point_params(&self) -> &BTreeSet<ParamId> {
        &self.point_params
    }

    /// The mask params (`Allen` mask positions): bind-time expects an
    /// Allen mask for each, rejecting the vacuous ∅/full masks. Absent
    /// from [`Self::param_types`] — a mask is not a data-model type.
    #[must_use]
    pub fn mask_params(&self) -> &BTreeSet<ParamId> {
        &self.mask_params
    }
}

/// One rule of the witness: the lowered (Or-free) rule plus its own
/// typing tables, with the query-global param tables reachable through
/// it. Everything downstream of validation runs per rule and consumes
/// exactly this view.
#[derive(Clone, Copy)]
pub struct RuleWitness<'a> {
    rule: &'a LoweredRule,
    typing: &'a RuleTyping,
    query: &'a ValidatedQuery,
}

impl<'a> RuleWitness<'a> {
    /// The lowered rule, verbatim — at the witness's own lifetime, so a
    /// caller can outlive the `RuleWitness` handle itself (the
    /// disjointness analysis collects every rule's finds at once).
    #[must_use]
    pub fn rule(&self) -> &'a LoweredRule {
        self.rule
    }

    /// The resolved structural type of one of this rule's variables.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an unknown `VarId` (the witness
    /// resolved every variable of the rule).
    #[must_use]
    pub fn var_type(&self, var: VarId) -> &ValueType {
        &self.typing.var_types[&var]
    }

    /// Every variable of this rule with its resolved type, in id order
    /// (the slot-layout roster — normalization builds the binding-slot
    /// widths from it).
    pub fn var_types(&self) -> impl Iterator<Item = (VarId, &ValueType)> {
        self.typing.var_types.iter().map(|(v, t)| (*v, t))
    }

    /// The rule's comparisons, classified — validation's sealed proof of
    /// each comparison's legal shape, in condition order. Placement
    /// (`ir/normalize/place_comparisons.rs`) consumes exactly this;
    /// nothing downstream re-derives a comparison's shape from its terms.
    #[must_use]
    pub(crate) fn classified_comparisons(&self) -> &'a [ClassifiedComparison] {
        &self.typing.classified
    }

    /// The resolved type of a param — query-global
    /// ([`ValidatedQuery::param_type`]).
    ///
    /// # Panics
    ///
    /// As [`ValidatedQuery::param_type`].
    #[must_use]
    pub fn param_type(&self, param: ParamId) -> &ValueType {
        self.query.param_type(param)
    }

    /// The rule's plan's sink-relevance set (the D2 gating bits' source).
    /// For a pure projection it is the group key — the suffix skip may
    /// cross nodes binding nothing projected. For an aggregate-bearing
    /// head it is **every** variable of the rule: the fold is defined over
    /// the distinct full binding set, so no node's bindings are skippable,
    /// and the `sink_relevant` bits themselves encode the illegality —
    /// any `SkipSuffix` a future sink ever signaled under an aggregate
    /// plan is absorbed at the node that produced it.
    #[must_use]
    pub fn sink_vars(&self) -> BTreeSet<VarId> {
        let has_aggregate = self
            .rule
            .finds
            .iter()
            .any(|term| matches!(term, FindTerm::Aggregate { .. }));
        if has_aggregate {
            self.typing.var_types.keys().copied().collect()
        } else {
            self.typing.group_key.clone()
        }
    }

    /// The group key: non-aggregated find variables (test observability;
    /// production reads it only through [`Self::sink_vars`]).
    #[cfg(test)]
    #[must_use]
    pub fn group_key(&self) -> &BTreeSet<VarId> {
        &self.typing.group_key
    }
}

/// One inference slot: collapsed to a single structural type, or still
/// bivalent (see [`Context::resolve_bivalents`], the resolution rule).
#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeSlot {
    /// Named by at least one monovalent anchor.
    Mono(ValueType),
    /// Anchored only by interval-field positions so far: the term is
    /// either `Interval(element)` (value equality) or `element`-typed
    /// (membership).
    Bivalent(IntervalElement),
}

/// How a param id is used: a scalar (`Term::Param`) or a set
/// (`Term::ParamSet`) — one or the other, never both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParamKind {
    Scalar,
    Set,
}

/// Accumulated typing state while walking the query.
#[derive(Default)]
struct Context {
    /// Variable inference slots — the pre-resolution state. CONSUMED by
    /// [`Context::resolve_bivalents`] into [`Context::var_types`]: the
    /// phase change is a type change, so no post-resolution reader can
    /// see (or re-match) an unresolved slot.
    var_slots: BTreeMap<VarId, TypeSlot>,
    /// Every variable's resolved structural type — the resolution's
    /// product, empty until [`Context::resolve_bivalents`] runs.
    var_types: BTreeMap<VarId, ValueType>,
    /// Param inference slots. Params stay slots past resolution: the
    /// typed comparison pass still anchors them.
    param_slots: BTreeMap<ParamId, TypeSlot>,
    /// Every param seen, with its scalar-vs-set role (doubles as the
    /// density-check roster).
    param_kinds: BTreeMap<ParamId, ParamKind>,
    /// Variables bound by at least one positive atom (any field kind).
    atom_vars: BTreeSet<VarId>,
    /// Variables with at least one positive *scalar*-field binding — the
    /// enumerable-domain witnesses for the membership-only rule.
    scalar_bound_vars: BTreeSet<VarId>,
    /// Variables occurring in negated atoms (the negation safety rule).
    negated_vars: BTreeSet<VarId>,
    /// Params anchored at interval positions (membership bindings and
    /// `PointIn` operands); those that resolve element-typed are the
    /// witness's point params.
    interval_position_params: BTreeSet<ParamId>,
    /// Params in `Allen` mask positions (never in `param_slots` — the
    /// conflict is checked, not represented).
    mask_params: BTreeSet<ParamId>,
}

#[cfg(test)]
mod tests;
