//! The pure-data query IR, validation, and normalization (docs/architecture).
//!
//! Queries are plain data — encodable, inspectable, no behavior
//! (`docs/architecture/20-query-ir.md`, normative). No wildcard variant
//! exists: an unbound field is *absent* from `bindings`, so "wildcard bound
//! to something" is unwritable. Variables carry dense ids only; names are a
//! debugging sidecar the engine never stores.

pub(crate) mod normalize;
pub mod render;
pub mod validate;

use bumbledb_theory::schema::{FieldId, RelationId};

/// The one literal-value sum, shared with statement selections — the
/// normative IR block in `docs/architecture/20-query-ir.md` names it here.
pub use bumbledb_theory::Value;

/// The DNF distribution — the declared decomposition of the input
/// condition grammar ([`ConditionTree`]) into Or-free rules; validation
/// runs it, and it is exported so the differential suite can prove it
/// against the naive model's direct tree evaluation.
pub use normalize::{LoweredRule, distribute};

/// The rule-count cap: each `Interior.rules` list and the main
/// `Query.rules` independently, and the rec SCC as one pool
/// (`base.len() + rec.len()`), rejected at validation
/// (`ValidationError::TooManyRules`). Counted independently of the
/// per-rule occurrence cap ([`crate::plan::planner::MAX_OCCURRENCES`]):
/// rules are planned one at a time, so the roster bounds each
/// rule-list's breadth here and each rule's width there. There is no
/// interior-count cap.
pub const MAX_RULES: usize = 16;

/// The condition-tree nesting cap: a [`ConditionTree`] deeper than this
/// is rejected at validation (`ValidationError::ConditionNestingTooDeep`)
/// — a **boundary check**, not planner hygiene (the trust-boundary law,
/// `docs/architecture/20-query-ir.md`): queries arrive as data, the tree
/// walks (DNF counting, distribution, rendering) recurse by depth, and an
/// unbounded depth would let hostile input exhaust the stack — a crash,
/// not a typed error. Depth is measured **iteratively** (an explicit work
/// list, [`normalize::nesting_depth`]), so the check itself is total; the
/// recursive walks run only on checked trees. The cap is generous: a
/// meaningful tree's depth is bounded by its leaf count, and the DNF
/// blowup cap ([`MAX_RULES`]) already limits leaves per disjunct.
pub const MAX_CONDITION_DEPTH: usize = 64;

/// Dense derived-table id — an index into a [`Query`]'s interiors,
/// with a Reach query's rec occupying `InteriorId(interiors.len())`
/// (`lean/Bumbledb/Query/Syntax.lean: InteriorId`). Same width as
/// [`crate::schema::RelationId`], deliberately: a **separate identity**,
/// never a pun. Statements quantify over stored relations only
/// (`docs/architecture/30-dependencies.md`); no statement form carries
/// an `InteriorId` position. Construction never panics: a derived-table
/// count that does not fit `u32` is [`crate::error::ValidationError::InteriorIdOverflow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InteriorId(pub u32);

impl InteriorId {
    /// Index into the derived-table signature / image slice.
    ///
    /// # Panics
    ///
    /// Never on a 64-bit target: `u32` always fits `usize`. The crate
    /// is 64-bit only; this is a programmer invariant, not an IR
    /// overflow (`InteriorIdOverflow` is judged before any
    /// `InteriorId` is minted).
    #[must_use]
    pub(crate) fn index(self) -> usize {
        usize::try_from(self.0).expect("crate is 64-bit")
    }
}

/// Where an atom draws its facts: a stored (EDB) relation, or a
/// derived table of the same query (an interior, or the rec)
/// (`lean/Bumbledb/Query/Syntax.lean: AtomSource`). An `Interior`
/// atom's bindings address **head positions** positionally —
/// `FieldId(i)` is the target derived head's column `i`, typed by its
/// sealed signature column — through the same `FieldId` reading
/// (`FieldId` is already positional, never nominal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AtomSource {
    Edb(RelationId),
    Interior(InteriorId),
}

impl AtomSource {
    /// The stored relation this source reads, if any.
    #[must_use]
    pub fn edb(self) -> Option<RelationId> {
        match self {
            Self::Edb(relation) => Some(relation),
            Self::Interior(_) => None,
        }
    }

    /// The derived table this source reads, if any — an interior or
    /// the rec (`lean/Bumbledb/Query/Syntax.lean: AtomSource.interior?`).
    #[must_use]
    pub fn interior(self) -> Option<InteriorId> {
        match self {
            Self::Edb(_) => None,
            Self::Interior(interior) => Some(interior),
        }
    }
}

/// Dense query-variable id — **rule-scoped**: the same `VarId` in two
/// rules names two unrelated variables (each rule is its own scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarId(pub u16);

/// Dense parameter id; values are supplied positionally at execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParamId(pub u16);

/// One term of an atom binding or comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Var(VarId),
    Param(ParamId),
    /// A param id used as a *set* — bound at execution to a slice of values
    /// of the anchored type; the term denotes *any element* (a binding
    /// position matches iff the field value is in the set). Legal in atom
    /// bindings (positive and negated) and as one side of `Eq`; illegal
    /// under every other operator. A `ParamId` is scalar or set, never both
    /// (`docs/architecture/20-query-ir.md`, § param sets).
    ParamSet(ParamId),
    Literal(Value),
    /// The **measure** of an interval-typed rule variable: `|[s, e)| =
    /// e − s`, type u64 — the one arithmetic the point-set denotation
    /// defines (`docs/architecture/10-data-model.md`; everything else is
    /// endpoint math and stays refused). Legal in exactly one term
    /// position: one side of an order comparison (`Lt`/`Le`/`Gt`/`Ge`)
    /// against a u64-typed term or literal — never in an atom binding
    /// (the measure is a computation, not a bindable value; typed
    /// rejection), never under `Eq`/`Ne`/`Allen`/`PointIn`, never on
    /// both sides. A ray (`end == MAX`) has no finite measure: the
    /// subtraction raises the typed execution error
    /// [`crate::Error::MeasureOfRay`] — hosts exclude rays with an
    /// `Allen` check or a bounded-end filter on the same atom first.
    Measure(VarId),
}

/// One atom: a source with named-field bindings. Absence of a field *is*
/// the wildcard. An atom with zero bindings is legal and means a
/// nonemptiness gate on the source.
///
/// The source position is [`AtomSource`]: an `Edb` atom reads a stored
/// relation exactly as ever; an `Interior` atom reads a derived table
/// of the same [`Query`] (an earlier interior, or the rec from main),
/// its `FieldId`s addressing the target's head positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub source: AtomSource,
    /// Named-field bindings; absence of a field is the wildcard.
    ///
    /// **Membership is a typing rule, not a node**
    /// (`docs/architecture/20-query-ir.md`): a binding `(field, term)`
    /// where the field is `Interval(E)` and the term's type is `E` means
    /// **point membership** — the binding satisfies iff `start ≤ t < end`.
    /// A term of type `Interval(E)` in the same position means interval
    /// **value equality** (identity). `Var`, `Param`, `ParamSet`, and `Literal` all
    /// participate under the same rule. The rule is owned by validation and
    /// lowering; one consequence, enforced there: every point variable must
    /// also be bound by at least one non-membership occurrence (a scalar
    /// field binding), because membership alone gives it no enumerable
    /// domain.
    pub bindings: Vec<(FieldId, Term)>,
}

/// Aggregate operators (`docs/architecture/20-query-ir.md`, § aggregation).
/// The fold domain of every aggregate is the group's set of distinct full
/// bindings over all query variables; the group key is the values of the
/// non-aggregated find variables. Across rules the domain splits by
/// provenance (ruled 2026-07-23, R2): a DNF-derived rule set keeps the
/// written rule's full binding set (surface `or` is fold-transparent),
/// while a hand-written multi-rule query folds the union of the rules'
/// binding sets projected to the head — the head is the only shared
/// vocabulary — with the fold-free nullary `Count` refused there (R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggOp {
    /// Accumulates in i128 and range-checks the final value once:
    /// Sum(I64)→I64, Sum(U64)→U64; out-of-range is a runtime query error.
    Sum,
    /// The orderable types — U64, I64, and bool, ordered `false < true`
    /// (ruled 2026-07-23, R3: `Min` over bool is **All**); intervals and
    /// closed references stay excluded (R4).
    Min,
    /// The orderable types, as [`AggOp::Min`] — `Max` over bool is
    /// **Any**, the other extreme of the 0/1 encoding.
    Max,
    /// Nullary: |the group's binding set|, result type U64.
    Count,
    /// The coalescing fold (Snodgrass coalesce) over an interval-typed
    /// variable: per group, the result is the set of **maximal disjoint
    /// half-open segments** of the union of the group's interval point
    /// sets. `Pack` is **relation-shaped** — one answer per (group,
    /// maximal segment); the result position is interval-typed
    /// (`docs/architecture/20-query-ir.md` § aggregation). Adjacency
    /// merges (`end == next.start` — the half-open law), a packed ray is
    /// a ray, and identical claims collapse in the coalesce. At most one
    /// `Pack` per head, never beside fold terms — the group variables are
    /// the only companions (validation, each refusal typed).
    Pack,
}

/// A fold over a variable: `Sum`/`Min`/`Max` only. Nullary [`FindTerm::Count`]
/// and coalescing [`FindTerm::Pack`] are sibling constructors — Count-with-
/// variable and Sum-without are unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldOp {
    Sum,
    Min,
    Max,
}

impl FoldOp {
    /// This fold's var-free head shape.
    #[must_use]
    pub fn head_op(self) -> HeadOp {
        match self {
            Self::Sum => HeadOp::Sum,
            Self::Min => HeadOp::Min,
            Self::Max => HeadOp::Max,
        }
    }
}

/// One find term: a projected variable, the measure, nullary count, a
/// fold over a variable, pack, or a fold over the measure. Count cannot
/// carry a variable; folds and pack cannot omit one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindTerm {
    Var(VarId),
    /// The measure at a find position: projects surface `Duration(over)` —
    /// one u64 value per binding, `end − start` of the interval variable
    /// (see [`Term::Measure`]; the variable must be interval-typed and
    /// atom-bound). The projected measure is a group-key position under
    /// aggregation, exactly like a plain variable find. A ray has no finite
    /// measure and raises [`crate::Error::MeasureOfRay`] at evaluation.
    Measure(VarId),
    /// Nullary: |the group's binding set|, result type U64.
    Count,
    /// `Sum`/`Min`/`Max` over a bound variable.
    Aggregate {
        op: FoldOp,
        over: VarId,
    },
    /// The coalescing fold over an interval-typed variable.
    Pack {
        over: VarId,
    },
    /// A fold over the measure: `Sum`/`Min`/`Max` of `Duration(over)`.
    /// Accumulates exactly as the same fold over a u64 variable. A ray
    /// has no finite measure and raises [`crate::Error::MeasureOfRay`].
    AggregateMeasure {
        op: FoldOp,
        over: VarId,
    },
}

impl FindTerm {
    /// The head position this term projects into — its var-free shape.
    /// A measure find is a value position (`HeadTerm::Var`): the head
    /// names shapes, and the positional type row (u64 for a measure)
    /// keeps rules aligned.
    #[must_use]
    pub fn head_term(&self) -> HeadTerm {
        match self {
            Self::Var(_) | Self::Measure(_) => HeadTerm::Var,
            Self::Count => HeadTerm::Aggregate(HeadOp::Count),
            Self::Aggregate { op, .. } | Self::AggregateMeasure { op, .. } => {
                HeadTerm::Aggregate(op.head_op())
            }
            Self::Pack { .. } => HeadTerm::Aggregate(HeadOp::Pack),
        }
    }
}

/// The aggregate-op kind at a head position: [`AggOp`] with its rule-scoped
/// variables stripped. Rules supply the variables; validation checks each
/// rule's find term against the head's op kind position by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadOp {
    Sum,
    Min,
    Max,
    Count,
    Pack,
}

impl AggOp {
    /// This op's var-free head shape.
    #[must_use]
    pub fn head_op(self) -> HeadOp {
        match self {
            Self::Sum => HeadOp::Sum,
            Self::Min => HeadOp::Min,
            Self::Max => HeadOp::Max,
            Self::Count => HeadOp::Count,
            Self::Pack => HeadOp::Pack,
        }
    }
}

/// One head position: the find shape every rule must project at this
/// position — a plain variable or an aggregate op. Var-free by
/// construction: variables are rule-scoped, so the head names shapes and
/// each rule's find terms supply the variables (positional alignment; the
/// positional *type* row is computed at validation and pinned in the
/// witness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadTerm {
    Var,
    Aggregate(HeadOp),
}

/// Comparison operators. `Eq`/`Ne` are legal for all six types; order
/// operators only for U64/U64 and I64/I64 (no cross-type comparison, ever
/// — and never intervals). `Allen { mask }` is **the** interval-pair
/// comparison — two interval terms of one element type; satisfied iff
/// `classify(lhs, rhs)` is in the mask (`crate::allen`) — and interval
/// `Eq`/`Ne` are its derived facts (normalization canonicalizes them to
/// `EQUALS` / `¬EQUALS`, so exactly one interval-pair form reaches the
/// planner). `PointIn` is point membership: an element-typed operand
/// against an interval operand; `x PointIn iv` iff `iv.start ≤ x < iv.end`.
/// The ordered IR fields retain the established interval-left, point-right
/// lowering used by the surface `x in iv`. Interval ⊇ interval is instead
/// `Allen(COVERS)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Allen {
        mask: bumbledb_theory::allen::AllenMask,
    },
    PointIn,
}

/// Scalar word comparison: `Eq`/`Ne` and the order operators. Allen and
/// `PointIn` are other kinds — they cannot inhabit this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordCmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl WordCmp {
    /// The word-comparison fragment of [`CmpOp`], if this is not an
    /// interval operator.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn from_cmp(op: CmpOp) -> Option<Self> {
        match op {
            CmpOp::Eq => Some(Self::Eq),
            CmpOp::Ne => Some(Self::Ne),
            CmpOp::Lt => Some(Self::Lt),
            CmpOp::Le => Some(Self::Le),
            CmpOp::Gt => Some(Self::Gt),
            CmpOp::Ge => Some(Self::Ge),
            CmpOp::Allen { .. } | CmpOp::PointIn => None,
        }
    }

    /// Evaluates the operator over ordered operands.
    pub(crate) fn compare<T: Ord>(self, left: &T, right: &T) -> bool {
        match self {
            Self::Eq => left == right,
            Self::Ne => left != right,
            Self::Lt => left < right,
            Self::Le => left <= right,
            Self::Gt => left > right,
            Self::Ge => left >= right,
        }
    }
}

impl From<OrderCmp> for WordCmp {
    fn from(op: OrderCmp) -> Self {
        match op {
            OrderCmp::Lt => Self::Lt,
            OrderCmp::Le => Self::Le,
            OrderCmp::Gt => Self::Gt,
            OrderCmp::Ge => Self::Ge,
        }
    }
}

/// An order operator: `Lt`/`Le`/`Gt`/`Ge`. Equality is a different kind
/// — `DurationCompare { op: Eq }` is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderCmp {
    Lt,
    Le,
    Gt,
    Ge,
}

impl OrderCmp {
    /// Evaluates the operator over ordered operands.
    pub(crate) fn compare<T: Ord>(self, left: &T, right: &T) -> bool {
        WordCmp::from(self).compare(left, right)
    }
}

/// One comparison condition. `Eq` between two variables is unification and
/// obeys identical type rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub op: CmpOp,
    pub lhs: Term,
    pub rhs: Term,
}

/// The *input* condition grammar: any boolean combination of positive
/// comparisons (`docs/architecture/20-query-ir.md`, § the input condition
/// grammar). This is the one place the surface admits a nested OR — and
/// the engine never sees it: validation distributes every rule's trees to
/// DNF, each disjunct becomes a rule ([`distribute`]), and the validated
/// artifact carries only flat [`Comparison`] lists ([`LoweredRule`]).
/// A cross-atom OR *as an execution concept* stays refused — OR is data
/// or it is nothing; DNF lowering recovers the tangled middle as rules.
///
/// Negated atoms and membership stay leaf-level: there is no OR over
/// atoms — atoms disjoin by writing rules, which is what rules are for.
///
/// The empty combinations keep their algebraic readings — `And([])` is
/// the empty conjunction (true: it contributes no leaves) and `Or([])`
/// is the empty disjunction (false: the rule denotes nothing and lowers
/// to zero rules) — accepted exactly as statically contradictory
/// conditions are: the semantics are exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionTree {
    Leaf(Comparison),
    And(Vec<ConditionTree>),
    Or(Vec<ConditionTree>),
}

/// One rule: a conjunctive body projecting its find terms against the
/// query's head. The rule's denotation is the set of distinct bindings of
/// its variables satisfying every positive atom, every condition, and no
/// negated atom, projected through `finds`.
///
/// A rule is its **own variable scope**: `VarId`s never cross rules — the
/// same id in two rules names two unrelated variables (they may even
/// resolve to different types). Params, by contrast, are query-global.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// One term per head position; the shape (var vs aggregate-op kind)
    /// and the positional type must match the head, checked at validation.
    pub finds: Vec<FindTerm>,
    /// At least one atom; conjunctive, positive.
    pub atoms: Vec<Atom>,
    /// Anti-join atoms (`docs/architecture/20-query-ir.md`, § negation).
    /// A binding satisfies a negated atom iff **no fact** of its relation
    /// matches the atom's bindings under that assignment — plain anti-join
    /// over sets; no null trick, no three-valued logic. **Safety rule:**
    /// every variable occurring in a negated atom must also occur in a
    /// positive atom — a negated atom **binds nothing, only rejects**.
    /// Literals, params, param sets, and membership bindings are all legal
    /// here; negation is a *position* in the rule, not a kind of atom, so
    /// the list reuses [`Atom`] unchanged.
    pub negated: Vec<Atom>,
    /// The condition trees, conjoined — the list is an `And`, so the flat
    /// conjunctive rule is written without wrapping. Any nested OR is
    /// distributed away at validation ([`ConditionTree`]); downstream of
    /// the boundary a rule's conditions are a flat comparison list
    /// ([`LoweredRule`]).
    pub conditions: Vec<ConditionTree>,
}

impl Rule {
    /// The head shape this rule's find terms project — the degenerate
    /// one-rule query's head is exactly this row.
    #[must_use]
    pub fn head(&self) -> Vec<HeadTerm> {
        self.finds.iter().map(FindTerm::head_term).collect()
    }
}

/// A nonempty list: first plus rest. Empty is unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmpty<T> {
    pub first: T,
    pub rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    /// A singleton.
    #[must_use]
    pub fn one(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// First plus the remaining items.
    #[must_use]
    pub fn new(first: T, rest: Vec<T>) -> Self {
        Self { first, rest }
    }

    /// `None` when `items` is empty.
    #[must_use]
    pub fn from_vec(items: Vec<T>) -> Option<Self> {
        let mut iter = items.into_iter();
        let first = iter.next()?;
        Some(Self {
            first,
            rest: iter.collect(),
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    /// A [`NonEmpty`] is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        match index {
            0 => Some(&self.first),
            n => self.rest.get(n - 1),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        std::iter::once(&mut self.first).chain(self.rest.iter_mut())
    }
}

impl<T> IntoIterator for NonEmpty<T> {
    type Item = T;
    type IntoIter = std::iter::Chain<std::iter::Once<T>, std::vec::IntoIter<T>>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.first).chain(self.rest)
    }
}

impl<T> std::ops::Index<usize> for NonEmpty<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.get(index).expect("index out of bounds")
    }
}

/// One interior rule: bound-variable finds only. Aggregates and the
/// measure are unrepresentable — the creation-quarantine law
/// (`lean/Bumbledb/Query/Syntax.lean: Interior` / `Rule.finds : List VarId`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRule {
    pub finds: Vec<VarId>,
    pub atoms: Vec<Atom>,
    pub negated: Vec<Atom>,
    pub conditions: Vec<ConditionTree>,
}

impl ProjectionRule {
    /// Lower to a main [`Rule`]: every find is a projected variable.
    #[must_use]
    pub fn to_rule(&self) -> Rule {
        Rule {
            finds: self.finds.iter().copied().map(FindTerm::Var).collect(),
            atoms: self.atoms.clone(),
            negated: self.negated.clone(),
            conditions: self.conditions.clone(),
        }
    }

    fn projection_head(finds: &[VarId]) -> Vec<HeadTerm> {
        finds.iter().map(|_| HeadTerm::Var).collect()
    }
}

/// A named interior: a finite CQ (union of conjunctive rules), evaluated
/// **once**, not an lfp (`lean/Bumbledb/Query/Syntax.lean: Interior`).
/// Declaration order is topological order: interior `i` may read
/// `Interior(j)` only for `j < i`. Head width is `rules[0].finds.len()`;
/// there is no separate head — aggregates cannot be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interior {
    /// At least one rule, at most [`MAX_RULES`]; union; bodies: EDB ∪
    /// earlier interiors. Empty is [`crate::error::ValidationError::EmptyInterior`].
    pub rules: Vec<ProjectionRule>,
}

impl Interior {
    /// Bound-variable head derived from the first rule's finds.
    #[must_use]
    pub fn head(&self) -> Vec<HeadTerm> {
        self.rules
            .first()
            .map(|rule| ProjectionRule::projection_head(&rule.finds))
            .unwrap_or_default()
    }
}

/// A base arm of a linear rec: negation is unrepresentable, and there
/// is no self field (`lean/Bumbledb/Query/Syntax.lean: RecRule`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecRule {
    pub finds: Vec<VarId>,
    pub atoms: Vec<Atom>,
    pub conditions: Vec<ConditionTree>,
}

impl RecRule {
    /// Lower to a [`Rule`]. Negation stays unrepresentable.
    #[must_use]
    pub fn to_rule(&self) -> Rule {
        Rule {
            finds: self.finds.iter().copied().map(FindTerm::Var).collect(),
            atoms: self.atoms.clone(),
            negated: vec![],
            conditions: self.conditions.clone(),
        }
    }
}

/// A step arm of a linear rec: `self_bindings` IS the unique positive
/// self-atom. Remaining atoms are non-self. Negation is unrepresentable
/// (`lean/Bumbledb/Query/Syntax.lean: RecStep`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecStep {
    pub finds: Vec<VarId>,
    pub self_bindings: Vec<(bumbledb_theory::schema::FieldId, Term)>,
    pub atoms: Vec<Atom>,
    pub conditions: Vec<ConditionTree>,
}

impl RecStep {
    /// Reconstruct the unique positive self-atom as the first atom so
    /// missing/nonlinear self cannot be written, and `self_occ` is
    /// [`crate::ir::normalize::OccId`](0) after lowering.
    #[must_use]
    pub fn to_rule(&self, rec_id: InteriorId) -> Rule {
        let mut atoms = Vec::with_capacity(1 + self.atoms.len());
        atoms.push(Atom {
            source: AtomSource::Interior(rec_id),
            bindings: self.self_bindings.clone(),
        });
        atoms.extend(self.atoms.clone());
        Rule {
            finds: self.finds.iter().copied().map(FindTerm::Var).collect(),
            atoms,
            negated: vec![],
            conditions: self.conditions.clone(),
        }
    }

    /// Written-order reconstruction: remaining atoms, then the self-atom.
    /// Render and host goldens use this so var numbering on reparse
    /// matches the body walk; execution lowering stays [`Self::to_rule`].
    #[must_use]
    pub fn to_written_rule(&self, rec_id: InteriorId) -> Rule {
        let mut atoms = Vec::with_capacity(self.atoms.len() + 1);
        atoms.extend(self.atoms.clone());
        atoms.push(Atom {
            source: AtomSource::Interior(rec_id),
            bindings: self.self_bindings.clone(),
        });
        Rule {
            finds: self.finds.iter().copied().map(FindTerm::Var).collect(),
            atoms,
            negated: vec![],
            conditions: self.conditions.clone(),
        }
    }
}

/// One linear recursive SCC: nonempty base arms (no self atom) and
/// nonempty rec arms (exactly one positive self-atom each, reified as
/// [`RecStep::self_bindings`]) (`lean/Bumbledb/Query/Syntax.lean: Rec`).
/// The rec's `InteriorId` is `interiors.len()` after the overflow check.
/// `base.len() + rec.len()` is one pool against [`MAX_RULES`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rec {
    pub base: NonEmpty<RecRule>,
    pub rec: NonEmpty<RecStep>,
}

impl Rec {
    /// Bound-variable head derived from the first base arm's finds.
    #[must_use]
    pub fn head(&self) -> Vec<HeadTerm> {
        ProjectionRule::projection_head(&self.base.first.finds)
    }
}

/// A query: named interiors (a DAG, eval once), then either a finite
/// CQ main or one linear rec SCC plus main
/// (`docs/architecture/20-query-ir.md`,
/// `lean/Bumbledb/Query/Syntax.lean: Query`).
///
/// **Denotation:** interiors then (Reach: rec lfp) then main
/// (`lean/Bumbledb/Query/Denotation.lean: evalQuery`). Set semantics
/// means there is exactly one union per rule-list — no bag distinction
/// exists or is representable. Disjunction is data, never an execution
/// node.
///
/// The single-rule query is the conjunctive query unchanged
/// ([`Query::single`]): empty-prefix [`Query::Cq`].
#[expect(
    clippy::large_enum_variant,
    reason = "Reach carries Rec by value; boxing would split the public Query shape"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// Finite CQ: interiors (possibly empty) and main. Rec is
    /// unrepresentable on this arm.
    Cq {
        /// DAG, declaration order; no count cap. Empty is legal.
        interiors: Vec<Interior>,
        /// The find shape (arity + aggregate ops) every **main** rule
        /// aligns against, position by position; at least one term,
        /// duplicates within a rule rejected at validation. The
        /// positional type row is computed at validation and pinned in
        /// the witness.
        head: Vec<HeadTerm>,
        /// Main rules: at least one, at most [`MAX_RULES`]. Empty main
        /// is [`crate::error::ValidationError::EmptyRuleSet`].
        rules: Vec<Rule>,
    },
    /// Reach: interiors, one rec SCC by value, then main.
    Reach {
        /// DAG, declaration order; no count cap. Empty is legal.
        interiors: Vec<Interior>,
        /// The one linear rec SCC. Stacked rec is unrepresentable
        /// (`Rec` does not contain a [`Query`]).
        rec: Rec,
        /// The find shape every **main** rule aligns against.
        head: Vec<HeadTerm>,
        /// Main rules: at least one, at most [`MAX_RULES`]. Empty main
        /// is [`crate::error::ValidationError::EmptyRuleSet`].
        rules: Vec<Rule>,
    },
}

impl Query {
    /// The conjunctive query — empty interiors, head derived from the
    /// rule's own find shape. Constructs [`Query::Cq`].
    #[must_use]
    pub fn single(rule: Rule) -> Self {
        Self::Cq {
            interiors: vec![],
            head: rule.head(),
            rules: vec![rule],
        }
    }

    /// Named interiors in declaration order.
    #[must_use]
    pub fn interiors(&self) -> &[Interior] {
        match self {
            Self::Cq { interiors, .. } | Self::Reach { interiors, .. } => interiors,
        }
    }

    /// The find shape every main rule aligns against.
    #[must_use]
    pub fn head(&self) -> &[HeadTerm] {
        match self {
            Self::Cq { head, .. } | Self::Reach { head, .. } => head,
        }
    }

    /// Main rules.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        match self {
            Self::Cq { rules, .. } | Self::Reach { rules, .. } => rules,
        }
    }

    /// Named interiors, mutably.
    pub fn interiors_mut(&mut self) -> &mut Vec<Interior> {
        match self {
            Self::Cq { interiors, .. } | Self::Reach { interiors, .. } => interiors,
        }
    }

    /// The find shape, mutably.
    pub fn head_mut(&mut self) -> &mut Vec<HeadTerm> {
        match self {
            Self::Cq { head, .. } | Self::Reach { head, .. } => head,
        }
    }

    /// Main rules, mutably.
    pub fn rules_mut(&mut self) -> &mut Vec<Rule> {
        match self {
            Self::Cq { rules, .. } | Self::Reach { rules, .. } => rules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::FoldOp;
    use bumbledb_theory::Interval;

    // These constructions double as documentation of the doc's example
    // query shapes over the ledger schema (Account, Posting, ...).

    #[test]
    fn point_lookup_by_fresh_key() {
        // Account(id = ?0, holder = h, status = s) — a single atom binding
        // the fresh key to a param.
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Param(ParamId(0))),
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        });
        assert_eq!(query.rules()[0].atoms.len(), 1);
    }

    #[test]
    fn containment_walk_join_with_range_condition() {
        // Posting(account = a, amount = amt, at = t), Account(id = a):
        // a containment walk joined on `a`, with t >= <timestamp>.
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    source: crate::ir::AtomSource::Edb(RelationId(4)),
                    bindings: vec![
                        (FieldId(2), Term::Var(VarId(0))),
                        (FieldId(4), Term::Var(VarId(1))),
                        (FieldId(5), Term::Var(VarId(2))),
                    ],
                },
                Atom {
                    source: crate::ir::AtomSource::Edb(RelationId(1)),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                },
            ],
            negated: vec![],
            conditions: vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Literal(Value::I64(1_700_000_000_000_000)),
            })],
        });
        assert_eq!(query.rules()[0].atoms.len(), 2);
        assert_eq!(query.rules()[0].conditions.len(), 1);
    }

    #[test]
    fn aggregate_balance_by_account() {
        // finds: [account, Sum(amount), Count] — group key from output;
        // Count is a sibling constructor, not `over: None`.
        let query = Query::single(Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: FoldOp::Sum,
                    over: VarId(1),
                },
                FindTerm::Count,
            ],
            atoms: vec![Atom {
                source: crate::ir::AtomSource::Edb(RelationId(4)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(0))),
                    (FieldId(4), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        });
        assert!(matches!(
            query.rules()[0].finds[1],
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: _
            }
        ));
    }

    #[test]
    fn zero_binding_atom_is_a_nonemptiness_gate() {
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                Atom {
                    source: crate::ir::AtomSource::Edb(RelationId(0)),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                },
                Atom {
                    source: crate::ir::AtomSource::Edb(RelationId(7)),
                    bindings: vec![], // gate: Cartesian with the rest
                },
            ],
            negated: vec![],
            conditions: vec![],
        });
        assert!(query.rules()[0].atoms[1].bindings.is_empty());
    }

    #[test]
    fn anti_join_with_param_set_shape() {
        // Account(id = a, region ∈ ?set0), ¬Posting(account = a):
        // accounts in a region set with no postings. The negated atom
        // reuses `a` (the safety rule) and binds nothing.
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(3), Term::ParamSet(ParamId(0))),
                ],
            }],
            negated: vec![Atom {
                source: crate::ir::AtomSource::Edb(RelationId(4)),
                bindings: vec![(FieldId(2), Term::Var(VarId(0)))],
            }],
            conditions: vec![],
        });
        assert_eq!(query.rules()[0].negated.len(), 1);
    }

    #[test]
    fn value_covers_every_data_model_type() {
        // The anti-Bytes-hole assertion (post-mortem §13): one variant per
        // 10-data-model type, constructed here so a missing one cannot
        // compile.
        let values = [
            Value::Bool(true),
            Value::U64(u64::MAX),
            Value::I64(i64::MIN),
            Value::String(Box::from(&b"text"[..])),
            Value::FixedBytes(Box::from(&[0xDEu8, 0xAD][..])),
            Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(0, u64::MAX).expect("nonempty interval"),
            ),
            Value::IntervalI64(
                bumbledb_theory::Interval::<i64>::new(i64::MIN, i64::MAX)
                    .expect("nonempty interval"),
            ),
        ];
        assert_eq!(values.len(), 7);
    }

    #[test]
    fn interval_converts_through_the_checked_type() {
        // `From<Interval<_>>`: same halves, no re-check needed — the
        // checked type already holds `start < end`.
        let iv = Interval::<i64>::new(-5, 9).expect("valid bounds");
        assert_eq!(
            Value::from(iv),
            Value::IntervalI64(
                bumbledb_theory::Interval::<i64>::new(-5, 9).expect("nonempty interval")
            )
        );
        let iv = Interval::<u64>::new(3, 7).expect("valid bounds");
        assert_eq!(
            Value::from(iv),
            Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(3, 7).expect("nonempty interval")
            )
        );
    }
}
