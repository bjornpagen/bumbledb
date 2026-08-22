//! The pure-data query IR, validation, and normalization.
//! Queries are plain data — encodable, inspectable, no behavior
//! . No wildcard variant
//! exists: an unbound field is *absent* from `bindings`, so "wildcard bound
//! to something" is unwritable. Variables carry dense ids only; names are a
//! debugging sidecar the engine never stores.

pub(crate) mod normalize;
pub mod render;
pub mod validate;

use bumbledb_theory::schema::{FieldId, RelationId};

/// The one literal-value sum, shared with statement selections — the
/// normative IR block in names it here.
pub use bumbledb_theory::Value;

/// condition grammar ([`ConditionTree`]) into Or-free rules; validation
/// runs it, and it is exported so the differential suite can prove it
/// against the naive model's direct tree evaluation.
pub use normalize::{LoweredRule, distribute};

/// The rule-count cap: each `Interior.rules` list and the main
/// `Query.rules` independently, and the rec SCC as one pool
/// (`base.len + rec.len`), rejected at validation
/// (`ValidationError::TooManyRules`). Counted independently of the
/// per-rule occurrence cap ([`crate::plan::planner::MAX_OCCURRENCES`]):
/// rules are planned one at a time, so the roster bounds each
/// rule-list's breadth here and each rule's width there. There is no
/// interior-count cap.
pub const MAX_RULES: usize = 16;

/// The condition-tree nesting cap: a [`ConditionTree`] deeper than this
/// is rejected at validation (`ValidationError::ConditionNestingTooDeep`)
/// — a **boundary check**, not planner hygiene: queries arrive as data, the tree
/// walks (DNF counting, distribution, rendering) recurse by depth, and an
/// unbounded depth would let hostile input exhaust the stack — a crash,
/// not a typed error. Depth is measured **iteratively** (an explicit work
/// list, [`normalize::nesting_depth`]), so the check itself is total; the
/// recursive walks run only on checked trees. The cap is generous: a
pub const MAX_CONDITION_DEPTH: usize = 64;

/// Dense derived-table id — an index into a [`Query`]'s interiors,
/// with a Reach query's rec occupying `InteriorId(interiors.len)`
/// [`crate::schema::RelationId`], deliberately: a **separate identity**,
/// never a pun. Statements quantify over stored relations only
/// ; no statement form carries
/// an `InteriorId` position. Construction never panics: a derived-table
/// count that does not fit `u32` is [`crate::error::ValidationError::InteriorIdOverflow`].
/// (`lean/Bumbledb/Query/Syntax.lean: InteriorId`). Same width as
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InteriorId(pub u32);

impl InteriorId {

    /// # Panics

    /// is 64-bit only; this is a programmer invariant, not an IR
    /// overflow (`InteriorIdOverflow` is judged before any

    #[must_use]
    pub(crate) fn index(self) -> usize {
        usize::try_from(self.0).expect("crate is 64-bit")
    }
}

/// Where an atom draws its facts: a stored (EDB) relation, or a
/// derived table of the same query (an interior, or the rec)
/// atom's bindings address **head positions** positionally —
/// `FieldId(i)` is the target derived head's column `i`, typed by its
/// (`FieldId` is already positional, never nominal).
/// (`lean/Bumbledb/Query/Syntax.lean: AtomSource`). An `Interior`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AtomSource {
    Edb(RelationId),
    Interior(InteriorId),
}

impl AtomSource {

    #[must_use]
    pub fn edb(self) -> Option<RelationId> {
        match self {
            Self::Edb(relation) => Some(relation),
            Self::Interior(_) => None,
        }
    }

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

    ParamSet(ParamId),
    Literal(Value),
}

/// One atom: a source with named-field bindings. Absence of a field *is*
/// the wildcard. An atom with zero bindings is legal and means a
/// nonemptiness gate on the source.
/// The source position is [`AtomSource`]: an `Edb` atom reads a stored
/// relation exactly as ever; an `Interior` atom reads a derived table
/// of the same [`Query`] (an earlier interior, or the rec from main),
/// its `FieldId`s addressing the target's head positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub source: AtomSource,

    pub bindings: Vec<(FieldId, Term)>,
}

/// Aggregate operators.
/// The fold domain of every aggregate is the group's set of distinct full
/// bindings over all query variables; the group key is the values of the
/// non-aggregated find variables. Across rules the domain splits by
/// provenance (ruled 2026-07-23, R2): a DNF-derived rule set keeps the
/// written rule's full binding set (surface `or` is fold-transparent),
/// while a hand-written multi-rule query folds the union of the rules'
/// vocabulary — with the fold-free nullary `Count` refused there (R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggOp {

    Sum,

    /// (ruled 2026-07-23, R3: `Min` over bool is **All**); intervals and

    Min,

    Max,

    Count,

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

    #[must_use]
    pub fn head_op(self) -> HeadOp {
        match self {
            Self::Sum => HeadOp::Sum,
            Self::Min => HeadOp::Min,
            Self::Max => HeadOp::Max,
        }
    }
}

/// One find term: a projected variable, nullary count, a fold over a
/// variable, or pack. Count cannot carry a variable; folds and pack
/// cannot omit one. Length of an interval is host arithmetic on the
/// endpoints the answer row already carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindTerm {
    Var(VarId),

    Count,

    Aggregate {
        op: FoldOp,
        over: VarId,
    },

    Pack {
        over: VarId,
    },
}

impl FindTerm {

    #[must_use]
    pub fn head_term(&self) -> HeadTerm {
        match self {
            Self::Var(_) => HeadTerm::Var,
            Self::Count => HeadTerm::Aggregate(HeadOp::Count),
            Self::Aggregate { op, .. } => HeadTerm::Aggregate(op.head_op()),
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

/// One comparison condition. `Eq` between two variables is unification and
/// obeys identical type rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub op: CmpOp,
    pub lhs: Term,
    pub rhs: Term,
}

/// The *input* condition grammar: any boolean combination of positive
/// comparisons. This is the one place the surface admits a nested OR — and
/// the engine never sees it: validation distributes every rule's trees to
/// DNF, each disjunct becomes a rule ([`distribute`]), and the validated
/// artifact carries only flat [`Comparison`] lists ([`LoweredRule`]).
/// A cross-atom OR *as an execution concept* stays refused — OR is data
/// or it is nothing; DNF lowering recovers the tangled middle as rules.
/// Negated atoms and membership stay leaf-level: there is no OR over
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
/// A rule is its **own variable scope**: `VarId`s never cross rules — the
/// same id in two rules names two unrelated variables (they may even
/// resolve to different types). Params, by contrast, are query-global.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {

    pub finds: Vec<FindTerm>,

    pub atoms: Vec<Atom>,

    pub negated: Vec<Atom>,

    pub conditions: Vec<ConditionTree>,
}

impl Rule {

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

    #[must_use]
    pub fn one(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    #[must_use]
    pub fn new(first: T, rest: Vec<T>) -> Self {
        Self { first, rest }
    }

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
/// (`lean/Bumbledb/Query/Syntax.lean: Interior` / `Rule.finds: List VarId`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRule {
    pub finds: Vec<VarId>,
    pub atoms: Vec<Atom>,
    pub negated: Vec<Atom>,
    pub conditions: Vec<ConditionTree>,
}

impl ProjectionRule {

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
/// Declaration order is topological order: interior `i` may read
/// `Interior(j)` only for `j < i`. Head width is `rules[0].finds.len`;
/// there is no separate head — aggregates cannot be written.
/// **once**, not an lfp (`lean/Bumbledb/Query/Syntax.lean: Interior`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interior {

    pub rules: Vec<ProjectionRule>,
}

impl Interior {

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
/// `base.len + rec.len` is one pool against [`MAX_RULES`].
/// [`RecStep::self_bindings`]) (`lean/Bumbledb/Query/Syntax.lean: Rec`).
/// The rec's `InteriorId` is `interiors.len` after the overflow check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rec {
    pub base: NonEmpty<RecRule>,
    pub rec: NonEmpty<RecStep>,
}

impl Rec {

    #[must_use]
    pub fn head(&self) -> Vec<HeadTerm> {
        ProjectionRule::projection_head(&self.base.first.finds)
    }
}

/// A query: named interiors (a DAG, eval once), then either a finite
/// .
/// **Denotation:** interiors then (Reach: rec lfp) then main
/// means there is exactly one union per rule-list — no bag distinction
/// exists or is representable. Disjunction is data, never an execution
/// node.
/// The single-rule query is the conjunctive query unchanged
/// ([`Query::single`]): empty-prefix CQ (`rec` is `None`).
/// (`lean/Bumbledb/Query/Denotation.lean: evalQuery`). Set semantics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {

    pub interiors: Vec<Interior>,

    pub head: Vec<HeadTerm>,

    pub rules: Vec<Rule>,

    pub rec: Option<Rec>,
}

impl Query {

    #[must_use]
    pub fn cq(interiors: Vec<Interior>, head: Vec<HeadTerm>, rules: Vec<Rule>) -> Self {
        Self {
            interiors,
            head,
            rules,
            rec: None,
        }
    }

    #[must_use]
    pub fn reach(
        interiors: Vec<Interior>,
        rec: Rec,
        head: Vec<HeadTerm>,
        rules: Vec<Rule>,
    ) -> Self {
        Self {
            interiors,
            head,
            rules,
            rec: Some(rec),
        }
    }

    #[must_use]
    pub fn single(rule: Rule) -> Self {
        Self::cq(vec![], rule.head(), vec![rule])
    }

    #[must_use]
    pub fn rec(&self) -> Option<&Rec> {
        self.rec.as_ref()
    }

    #[must_use]
    pub fn interiors(&self) -> &[Interior] {
        &self.interiors
    }

    #[must_use]
    pub fn head(&self) -> &[HeadTerm] {
        &self.head
    }

    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn interiors_mut(&mut self) -> &mut Vec<Interior> {
        &mut self.interiors
    }

    pub fn head_mut(&mut self) -> &mut Vec<HeadTerm> {
        &mut self.head
    }

    pub fn rules_mut(&mut self) -> &mut Vec<Rule> {
        &mut self.rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::FoldOp;
    use bumbledb_theory::Interval;

    #[test]
    fn point_lookup_by_fresh_key() {

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
                    bindings: vec![], 
                },
            ],
            negated: vec![],
            conditions: vec![],
        });
        assert!(query.rules()[0].atoms[1].bindings.is_empty());
    }

    #[test]
    fn anti_join_with_param_set_shape() {

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

        let values = [
            Value::Bool(true),
            Value::U64(u64::MAX),
            Value::I64(i64::MIN),
            Value::String(Box::from("text")),
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
