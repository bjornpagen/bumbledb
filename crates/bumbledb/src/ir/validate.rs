//! The single validation boundary (docs/architecture/20-query-ir.md): IR in, [`ValidatedQuery`]
//! witness out. Everything downstream trusts the witness and re-checks
//! nothing (post-mortem §38: v5 validated one plan four times).
//!
//! The roster, transcribed from `docs/architecture/20-query-ir.md` and
//! checked off in code order below — it is exhaustive by contract.
//!
//! The query shape first (rules are validated one at a time; every
//! rule-local diagnostic names a position inside the first failing rule):
//!
//!  0. empty rule set; more than [`crate::ir::MAX_RULES`] rules (counted
//!     independently of the per-rule occurrence cap); head/rule positional
//!     arity, shape, or type mismatch (each rule's find terms align
//!     against the head position by position — rule 0's resolved type row
//!     pins the head's positional types, and every later rule must agree)
//!
//! Between the query shape and the per-rule roster, first the
//! **nesting boundary check**: condition trees deeper than
//! [`crate::ir::MAX_CONDITION_DEPTH`] are the typed
//! `ConditionNestingTooDeep` — judged by an iterative depth walk before
//! any recursive tree walk runs, so hostile nesting is a rejection,
//! never a stack exhaustion (the trust-boundary law). Then **DNF
//! distribution** ([`crate::ir::distribute`]): each rule's condition
//! trees distribute to disjunctive normal form and each disjunct becomes
//! a rule — the structural term count past [`crate::ir::MAX_RULES`] is
//! the typed `DnfExceedsRules { produced, cap }` (judged before
//! materializing), duplicate rules collapse by normalized-form equality,
//! and a query whose every disjunction is empty is the empty union
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
//!     ("always" — write no condition), distinct typed errors for
//!     literal masks
//! 10. constant comparisons (no variable side) and self-comparisons
//! 11. point variables bound only by membership (no enumerable domain)
//! 12. negated-atom variables not bound by any positive atom (negated
//!     atoms bind nothing; a query with no positive atoms is invalid)
//! 13. unbound find variables (Datalog safety; includes aggregate inputs)
//! 14. comparison-only variables
//! 15. empty finds
//! 16. duplicate find terms
//! 17. no positive atoms
//! 18. aggregate input types (Sum/Min/Max integers only; Count nullary)
//! 19. aggregate over a group-key variable
//! 20. planner caps: more than `MAX_OCCURRENCES` atom occurrences —
//!     negated occurrences counted — or more than 128 distinct variables
//!     (rejected here so downstream id widths and bitset sizes are true
//!     invariants)

use std::collections::{BTreeMap, BTreeSet};

use crate::allen::AllenMask;
use crate::error::ValidationError;
use crate::image::view::MaskConst;
use crate::ir::normalize::{LoweredRule, OccId};
use crate::ir::{CmpOp, FindTerm, InteriorId, ParamId, Value, VarId};
use bumbledb_theory::schema::{FieldId, IntervalElement, ValueType};

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
/// and introspection's header. Referenced only by [`InteriorId`], from
/// inside the same [`crate::ir::Query`] — the named-view refusal stands
/// (no stored, named, or cross-query reference exists), and the one
/// reference form is the `Interior` atom, typed against these sealed
/// columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// The signature: one column per head position, in head order.
    pub columns: Box<[SignatureColumn]>,
}

impl std::fmt::Display for Signature {
    /// The signature in one line — introspection's header (`(u64, Sum i64)`:
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
                ValueType::Interval { element, width } => {
                    let element = match element {
                        IntervalElement::U64 => "u64",
                        IntervalElement::I64 => "i64",
                    };
                    match width {
                        None => write!(f, "interval<{element}>")?,
                        Some(w) => write!(f, "interval<{element}, {w}>")?,
                    }
                }
            }
        }
        f.write_str(")")
    }
}

/// One column of the sealed signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureColumn {
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
        mask: AllenMask,
    },
    /// The interval-pair comparison against a constant, the mask sealed
    /// field-on-left (conversed immediately when written constant-first).
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

/// The interior typing surface handed to the per-rule roster: sealed
/// column types, indexed by [`InteriorId`]. The phase is the slice
/// extent, not a hole in it: interiors already sealed sit in
/// [`Self::interiors`]; the rec predicate is a second slice (present
/// only when rec arms or main type against it). An `Interior` anchor
/// resolves against the target's sealed column exactly as an `Edb`
/// anchor resolves against its stored field ([`Context::check_atoms`]).
pub(super) struct InteriorSignatures<'a> {
    /// Already-sealed interiors in declaration order. Interior *i*
    /// types against `&sealed[..i]`; rec base types against the full
    /// interiors slice (the rec's own predicate is not yet present).
    interiors: &'a [Signature],
    /// The rec predicate, present when typing rec arms or a reach
    /// query's main. Absent while typing interiors and rec base — a
    /// self-atom in base is [`ValidationError::SelfInBase`] at the
    /// roster, before typing.
    rec: Option<&'a Signature>,
    /// The reading interior's id, if this rule-list is an interior
    /// ([`ValidationError::InteriorNotPrior`]); `None` for rec and main
    /// (out-of-range is [`ValidationError::UnknownInterior`]).
    reader: Option<InteriorId>,
    /// `interiors.len() + rec.is_some()` on the *boundary* query — the
    /// well-formedness screen's address space, independent of how many
    /// signatures have sealed.
    derived_count: usize,
}

impl InteriorSignatures<'_> {
    /// The binding-independent source screen: the target names a real
    /// derived table (a zero-binding `Interior` gate never reaches
    /// [`InteriorSignatures::column`], so the screen runs per atom).
    ///
    /// Two named refusals, one each: [`ValidationError::UnknownInterior`]
    /// iff the id is `>= derived_count`; [`ValidationError::InteriorNotPrior`]
    /// iff the reader is interior *i* and the target is `j >= i` (even
    /// when `j < derived_count`).
    pub(super) fn screen(
        &self,
        atom: usize,
        interior: InteriorId,
    ) -> Result<(), ValidationError> {
        let index = usize::try_from(interior.0).expect("64-bit usize");
        if index >= self.derived_count {
            return Err(ValidationError::UnknownInterior { atom, interior });
        }
        if let Some(at) = self.reader {
            let at_idx = usize::try_from(at.0).expect("64-bit usize");
            if index >= at_idx {
                return Err(ValidationError::InteriorNotPrior { interior, at });
            }
        }
        Ok(())
    }

    /// The sealed predicate for a derived id that [`Self::screen`] has
    /// already admitted. Rec base never looks up the rec slot (self in
    /// base is a roster refusal).
    fn lookup(&self, interior: InteriorId) -> &Signature {
        let index = usize::try_from(interior.0).expect("64-bit usize");
        if index < self.interiors.len() {
            &self.interiors[index]
        } else {
            self.rec.expect("screen admitted this id; rec base never reads self")
        }
    }

    /// The sealed type of one `Interior` binding position. Precondition:
    /// [`Self::screen`] already ran for this atom (the roster's
    /// unknown-id item). Does not re-screen.
    fn column(
        &self,
        atom: usize,
        interior: InteriorId,
        field: FieldId,
    ) -> Result<&ValueType, ValidationError> {
        debug_assert!(
            self.screen(atom, interior).is_ok(),
            "check_atoms screens before column"
        );
        self.lookup(interior)
            .columns
            .get(usize::from(field.0))
            .map(|column| &column.ty)
            .ok_or(ValidationError::InteriorColumnOutOfRange { atom, field })
    }
}

/// One sealed interior: lowered rules, signature, per-rule typing.
/// Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedInterior {
    lowered: Vec<LoweredRule>,
    signature: Signature,
    rules: Vec<RuleTyping>,
}

impl ValidatedInterior {
    /// This interior's sealed signature.
    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Lowered-rule count.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// A nonempty list: first plus rest. Empty rec arms are roster refusals
/// ([`ValidationError::EmptyRecursiveBase`] /
/// [`ValidationError::EmptyRecursiveStep`]); the witness cannot spell them.
#[derive(Debug)]
pub(crate) struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    fn from_vec(items: Vec<T>) -> Option<Self> {
        let mut iter = items.into_iter();
        let first = iter.next()?;
        Some(Self {
            first,
            rest: iter.collect(),
        })
    }

    fn len(&self) -> usize {
        1 + self.rest.len()
    }

    fn get(&self, index: usize) -> Option<&T> {
        match index {
            0 => Some(&self.first),
            n => self.rest.get(n - 1),
        }
    }
}

/// One lowered rec *base* arm: the rule plus its typing. Base arms
/// cannot name self ([`ValidationError::SelfInBase`]).
#[derive(Debug)]
pub struct ValidatedBaseArm {
    rule: LoweredRule,
    typing: RuleTyping,
}

/// One lowered rec *step* arm: the unique positive self-atom's
/// occurrence ([`Self::self_occ`]) plus the rule and its typing.
/// Missing/nonlinear self are roster refusals; the witness cannot
/// spell them.
#[derive(Debug)]
pub struct ValidatedRecArm {
    self_occ: OccId,
    rule: LoweredRule,
    typing: RuleTyping,
}

impl ValidatedRecArm {
    /// The unique positive self-atom's occurrence — the proof
    /// `rec_roster` found, stored so prepare never re-searches.
    #[must_use]
    pub(crate) fn self_occ(&self) -> OccId {
        self.self_occ
    }
}

/// The sealed rec SCC: base then rec arms, one signature.
/// Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedRec {
    base: NonEmpty<ValidatedBaseArm>,
    rec: NonEmpty<ValidatedRecArm>,
    signature: Signature,
}

impl ValidatedRec {
    /// The rec's sealed signature.
    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Lowered base-arm count.
    #[must_use]
    pub fn base_count(&self) -> usize {
        self.base.len()
    }

    /// Lowered rec-arm count.
    #[must_use]
    pub fn rec_count(&self) -> usize {
        self.rec.len()
    }

    /// One rec step arm, each carrying `self_occ`.
    #[must_use]
    pub(crate) fn arm(&self, index: usize) -> &ValidatedRecArm {
        self.rec.get(index).expect("index in range")
    }

    /// One rec base-arm rule.
    #[must_use]
    pub(crate) fn base_rule<'a>(
        &'a self,
        query: &'a ValidatedQuery,
        index: usize,
    ) -> RuleWitness<'a> {
        let arm = self.base.get(index).expect("index in range");
        RuleWitness {
            rule: &arm.rule,
            typing: &arm.typing,
            query,
        }
    }

    /// One rec step-arm rule.
    #[must_use]
    pub(crate) fn step_rule<'a>(
        &'a self,
        query: &'a ValidatedQuery,
        index: usize,
    ) -> RuleWitness<'a> {
        let arm = self.rec.get(index).expect("index in range");
        RuleWitness {
            rule: &arm.rule,
            typing: &arm.typing,
            query,
        }
    }

    /// Every rec base arm, in declaration order.
    pub(crate) fn base_rules<'a>(
        &'a self,
        query: &'a ValidatedQuery,
    ) -> impl Iterator<Item = RuleWitness<'a>> {
        (0..self.base.len()).map(|index| self.base_rule(query, index))
    }

    /// Every rec step arm, in declaration order.
    pub(crate) fn step_rules<'a>(
        &'a self,
        query: &'a ValidatedQuery,
    ) -> impl Iterator<Item = RuleWitness<'a>> {
        (0..self.rec.len()).map(|index| self.step_rule(query, index))
    }
}

/// The sealed main query: lowered rules, answer signature, per-rule typing.
/// Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedMain {
    lowered: Vec<LoweredRule>,
    signature: Signature,
    rules: Vec<RuleTyping>,
}

impl ValidatedMain {
    /// The answer head's sealed signature.
    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Lowered main-rule count.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// The sealed witness: query-global param tables plus a shape sum.
/// Unconstructible outside this module.
///
/// Variables are rule-scoped, so their typing lives per rule
/// ([`RuleTyping`]); params are query-global, so their tables live here
/// once — unified across every interior, rec arm, and main rule.
/// Rec-absence is [`Self::Cq`]; rec-presence is [`Self::Reach`]. Cq
/// callers never see rec accessors.
#[derive(Debug)]
pub enum ValidatedQuery {
    /// No rec: interiors (possibly empty) and main.
    Cq {
        interiors: Vec<ValidatedInterior>,
        main: ValidatedMain,
        param_types: BTreeMap<ParamId, ValueType>,
        /// Param ids bound as sets (`Term::ParamSet`); their entry in
        /// `param_types` is the *element* type.
        set_params: BTreeSet<ParamId>,
        /// Element-typed params meeting an interval position (membership
        /// bindings and `PointIn` operands): their values are points, so the
        /// point-domain law (`docs/architecture/10-data-model.md`) forbids the
        /// domain ceiling — enforced at bind, where the value exists.
        point_params: BTreeSet<ParamId>,
    },
    /// Rec present: interiors, the rec SCC, and main. `rec_id` and
    /// `derived_count` are stored once at validate (engine-003's accepted
    /// half / engine-028).
    Reach {
        interiors: Vec<ValidatedInterior>,
        rec: ValidatedRec,
        main: ValidatedMain,
        rec_id: InteriorId,
        derived_count: u32,
        param_types: BTreeMap<ParamId, ValueType>,
        set_params: BTreeSet<ParamId>,
        point_params: BTreeSet<ParamId>,
    },
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
    /// Closed-bound variables with their sealed extensions' row counts —
    /// the R4 order wall's roster and the dense group domains (049).
    closed_vars: BTreeMap<VarId, u16>,
}

impl ValidatedQuery {
    /// Named interiors in declaration order.
    #[must_use]
    pub fn interiors(&self) -> &[ValidatedInterior] {
        match self {
            Self::Cq { interiors, .. } | Self::Reach { interiors, .. } => interiors,
        }
    }

    /// The main/answer witness.
    #[must_use]
    pub fn main(&self) -> &ValidatedMain {
        match self {
            Self::Cq { main, .. } | Self::Reach { main, .. } => main,
        }
    }

    /// The answer head's sealed signature — [`ValidatedMain`]'s
    /// predicate. Downstream result typing reads this, never an
    /// interior or rec signature.
    #[must_use]
    pub fn signature(&self) -> &Signature {
        self.main().signature()
    }

    /// One **main** rule's slice of the witness — the unit the per-rule
    /// pipeline (normalize → grounding → plan) consumes for the answer
    /// head.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an index at or beyond
    /// the main rule count.
    #[must_use]
    pub fn rule(&self, index: usize) -> RuleWitness<'_> {
        self.main_rule(index)
    }

    /// One main rule.
    #[must_use]
    pub(crate) fn main_rule(&self, index: usize) -> RuleWitness<'_> {
        let main = self.main();
        RuleWitness {
            rule: &main.lowered[index],
            typing: &main.rules[index],
            query: self,
        }
    }

    /// One interior rule.
    #[must_use]
    pub(crate) fn interior_rule(&self, interior: usize, index: usize) -> RuleWitness<'_> {
        let inner = &self.interiors()[interior];
        RuleWitness {
            rule: &inner.lowered[index],
            typing: &inner.rules[index],
            query: self,
        }
    }

    /// Every **main** rule's witness slice, in rule order.
    pub fn rules(&self) -> impl Iterator<Item = RuleWitness<'_>> {
        let n = self.main().rules.len();
        (0..n).map(|index| self.main_rule(index))
    }

    /// Every rule of one interior, in rule order.
    pub(crate) fn interior_rules(&self, interior: usize) -> impl Iterator<Item = RuleWitness<'_>> {
        let n = self.interiors()[interior].rules.len();
        (0..n).map(move |index| self.interior_rule(interior, index))
    }

    fn param_types_map(&self) -> &BTreeMap<ParamId, ValueType> {
        match self {
            Self::Cq { param_types, .. } | Self::Reach { param_types, .. } => param_types,
        }
    }

    fn set_params_set(&self) -> &BTreeSet<ParamId> {
        match self {
            Self::Cq { set_params, .. } | Self::Reach { set_params, .. } => set_params,
        }
    }

    fn point_params_set(&self) -> &BTreeSet<ParamId> {
        match self {
            Self::Cq { point_params, .. } | Self::Reach { point_params, .. } => point_params,
        }
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
        &self.param_types_map()[&param]
    }

    /// Every param with its resolved type, in id order (bind-time checking,
    /// The 40-execution doc). A set param's type is its *element* type.
    pub fn param_types(&self) -> impl Iterator<Item = (ParamId, &ValueType)> {
        self.param_types_map().iter().map(|(p, t)| (*p, t))
    }

    /// The params bound as sets (`Term::ParamSet`) — bind-time expects a
    /// slice of values of the element type for each.
    #[must_use]
    pub fn set_params(&self) -> &BTreeSet<ParamId> {
        self.set_params_set()
    }

    /// The point-position params: element-typed at an interval position
    /// (a membership binding or a `PointIn` operand). Bind-time rejects
    /// their domain ceiling — points are `MIN ..= MAX−1`; `MAX` is the
    /// ray's ∞ (the point-domain law).
    #[must_use]
    pub fn point_params(&self) -> &BTreeSet<ParamId> {
        self.point_params_set()
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

    /// This lowered rule's written-rule provenance (ruled 2026-07-23,
    /// R2): `Some(idx)` iff the disjunct was minted from written rule
    /// `idx` alone — the union sink's regime split reads it (a
    /// surviving rule set carrying ONE shared index is DNF-derived and
    /// re-keys the dedup on the shared slot arrays).
    #[must_use]
    pub fn written(&self) -> Option<u16> {
        self.rule.written
    }

    /// The full mint set — every written rule this disjunct belongs to
    /// (`written`'s uncompressed form; a cross-written collapse erases
    /// `written` but unions here). The ray-probe verdict fold (R6)
    /// groups disjuncts by it.
    #[must_use]
    pub fn minted(&self) -> &[u16] {
        &self.rule.minted
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
    /// and the `SuffixSkip::Forbidden` evidence itself encodes the illegality —
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

    /// A variable's schema-proven dense domain (finding 049): the
    /// sealed extension's row count for a closed-bound variable (its
    /// words are the declaration indices `0..N`,
    /// containment-enforced in-domain), 2 for bool (the strict 0/1
    /// encoding), `None` for every open domain.
    #[must_use]
    pub fn dense_domain(&self, var: VarId) -> Option<u16> {
        if let Some(rows) = self.typing.closed_vars.get(&var) {
            return Some(*rows);
        }
        matches!(self.var_type(var), ValueType::Bool).then_some(2)
    }
}

/// One inference slot: collapsed to a single structural type, or still
/// bivalent (see [`Context::resolve_bivalents`], the resolution rule).
#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeSlot {
    /// Named by at least one monovalent anchor.
    Mono(ValueType),
    /// Anchored only by interval-field positions so far: the term is
    /// either the position's interval type (value equality — the
    /// field's exact family member, width included) or `element`-typed
    /// (membership; a point carries no width).
    Bivalent {
        element: IntervalElement,
        /// The anchoring field's width: `None` general, `Some(w)`
        /// fixed — part of the equality reading's type.
        width: Option<u64>,
    },
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
    /// Variables bound at a closed-reference position (a field whose
    /// declared containment targets a closed relation's id, or the
    /// closed relation's own id field — `ir/render`'s `ClosedRefs`
    /// table), each with the sealed extension's row count. Their words
    /// are declaration-order accidents, so order positions —
    /// `Lt`-family comparisons and `Sum`/`Min`/`Max` folds —
    /// refuse them (ruled 2026-07-23, R4); the row count is the proven
    /// dense group domain (finding 049: closed ids are declaration
    /// indices `0..N`, containment-enforced in-domain).
    closed_vars: BTreeMap<VarId, u16>,
    /// Variables with at least one positive *scalar*-field binding — the
    /// enumerable-domain witnesses for the membership-only rule.
    scalar_bound_vars: BTreeSet<VarId>,
    /// Variables occurring in negated atoms (the negation safety rule).
    negated_vars: BTreeSet<VarId>,
    /// Params anchored at interval positions (membership bindings and
    /// `PointIn` operands); those that resolve element-typed are the
    /// witness's point params.
    interval_position_params: BTreeSet<ParamId>,
}

#[cfg(test)]
mod tests;
