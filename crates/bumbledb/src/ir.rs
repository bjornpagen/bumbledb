//! The pure-data query IR, validation, and normalization (docs/architecture).
//!
//! Queries are plain data — encodable, inspectable, no behavior
//! (`docs/architecture/20-query-ir.md`, normative). No wildcard variant
//! exists: an unbound field is *absent* from `bindings`, so "wildcard bound
//! to something" is unwritable. Variables carry dense ids only; names are a
//! debugging sidecar the engine never stores.

pub(crate) mod normalize;
pub mod render;
pub(crate) mod validate;

use crate::schema::{FieldId, RelationId};

/// The one literal-value sum, shared with statement selections — the
/// normative IR block in `docs/architecture/20-query-ir.md` names it here.
pub use crate::value::Value;

/// The DNF distribution — the declared decomposition of the input
/// predicate grammar ([`PredicateTree`]) into Or-free rules; validation
/// runs it, and it is exported so the differential suite can prove it
/// against the naive model's direct tree evaluation.
pub use normalize::{distribute, LoweredRule};

/// The rule-count cap: a query is a program of at most this many rules,
/// rejected at validation (`ValidationError::TooManyRules`). Counted
/// independently of the per-rule occurrence cap
/// ([`crate::plan::planner::MAX_OCCURRENCES`]): rules are planned one at a
/// time, so the roster bounds the program's breadth here and each rule's
/// width there.
pub const MAX_RULES: usize = 16;

/// The predicate-tree nesting cap: a [`PredicateTree`] deeper than this
/// is rejected at validation (`ValidationError::PredicateNestingTooDeep`)
/// — a **boundary guard**, not planner hygiene (the trust-boundary law,
/// `docs/architecture/20-query-ir.md`): queries arrive as data, the tree
/// walks (DNF counting, distribution, rendering) recurse by depth, and an
/// unbounded depth would let hostile input exhaust the stack — a crash,
/// not a typed error. Depth is measured **iteratively** (an explicit work
/// list, [`normalize::nesting_depth`]), so the guard itself is total; the
/// recursive walks run only on guarded trees. The cap is generous: a
/// meaningful tree's depth is bounded by its leaf count, and the DNF
/// blowup cap ([`MAX_RULES`]) already limits leaves per disjunct.
pub const MAX_PREDICATE_DEPTH: usize = 64;

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
    /// rejection), never under `Eq`/`Ne`/`Allen`/`Contains`, never on
    /// both sides. A ray (`end == MAX`) has no finite measure: the
    /// subtraction raises the typed execution error
    /// [`crate::Error::MeasureOfRay`] — hosts exclude rays with an
    /// `Allen` guard or a bounded-end filter on the same atom first.
    Duration(VarId),
}

/// One atom: a relation with named-field bindings. Absence of a field *is*
/// the wildcard. An atom with zero bindings is legal and means a
/// nonemptiness gate on the relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub relation: RelationId,
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
/// non-aggregated find variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggOp {
    /// Accumulates in i128 and range-checks the final value once:
    /// Sum(I64)→I64, Sum(U64)→U64; out-of-range is a runtime query error.
    Sum,
    /// U64 and I64 only (the orderable types — intervals excluded).
    Min,
    /// U64 and I64 only, as [`AggOp::Min`].
    Max,
    /// Nullary (`over: None`): |the group's binding set|, result type U64.
    Count,
    /// |the set of distinct values of `over` across the group's binding
    /// set|, result type U64; legal over every type.
    CountDistinct,
    /// Arg-restriction: the group's binding set is first restricted to the
    /// bindings attaining the **maximum** of `key`, and the group's output
    /// rows are projected from that restricted set — a tie yields every
    /// attaining row. `over` is the carried variable; `key` must be
    /// orderable (U64/I64), and all Arg terms in one query share one key
    /// and one direction.
    ArgMax { key: VarId },
    /// Arg-restriction toward the **minimum** of `key`; rules as
    /// [`AggOp::ArgMax`].
    ArgMin { key: VarId },
    /// The coalescing fold (Snodgrass coalesce) over an interval-typed
    /// variable: per group, the result is the set of **maximal disjoint
    /// half-open segments** of the union of the group's interval point
    /// sets. `Pack` is **relation-shaped** — one result row per (group,
    /// maximal segment); the result position is interval-typed
    /// (`docs/architecture/20-query-ir.md` § aggregation). Adjacency
    /// merges (`end == next.start` — the half-open law), a packed ray is
    /// a ray, and identical claims collapse in the coalesce. At most one
    /// `Pack` per head, never beside fold or Arg terms — the group
    /// variables are the only companions (validation, each refusal
    /// typed).
    Pack,
}

/// One find term: a projected variable or an aggregate. `over` is `None`
/// for the nullary `Count`, `Some(counted var)` for `CountDistinct`, the
/// aggregated variable for `Sum`/`Min`/`Max`, and the *carried* variable
/// for `ArgMax`/`ArgMin` (the key rides in the op).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindTerm {
    Var(VarId),
    Aggregate {
        op: AggOp,
        over: Option<VarId>,
    },
    /// The measure at a find position: projects `Duration(over)` — one
    /// u64 value per binding, `end − start` of the interval variable
    /// (see [`Term::Duration`]; the variable must be interval-typed and
    /// atom-bound). The projected measure is a group-key position under
    /// aggregation, exactly like a plain variable find.
    Duration(VarId),
    /// A fold over the measure: `Sum`/`Min`/`Max` of `Duration(over)` —
    /// the only three ops the measure admits (`Count` is nullary;
    /// `CountDistinct` and the Arg ops are typed rejections). Accumulates
    /// exactly as `Sum`/`Min`/`Max` over a u64 variable — Sum in the wide
    /// accumulator with the single finalize range check.
    AggregateDuration {
        op: AggOp,
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
            Self::Var(_) | Self::Duration(_) => HeadTerm::Var,
            Self::Aggregate { op, .. } | Self::AggregateDuration { op, .. } => {
                HeadTerm::Aggregate(op.head_op())
            }
        }
    }
}

/// The aggregate-op kind at a head position: [`AggOp`] with its rule-scoped
/// variables stripped (an Arg key is a rule variable; the head is
/// var-free). Rules supply the variables; validation checks each rule's
/// find term against the head's op kind position by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadOp {
    Sum,
    Min,
    Max,
    Count,
    CountDistinct,
    ArgMax,
    ArgMin,
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
            Self::CountDistinct => HeadOp::CountDistinct,
            Self::ArgMax { .. } => HeadOp::ArgMax,
            Self::ArgMin { .. } => HeadOp::ArgMin,
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

/// The `Allen` comparison's mask position: a literal mask, or a param
/// resolved at bind (`Value::AllenMask` / [`crate::BindValue::AllenMask`])
/// — the temporal relation as a bind-time argument. A two-variant sum, not
/// a [`Term`]: a variable or set mask is unrepresentable, not rejected.
/// Both surfaces reject the vacuous ∅/full masks with distinct typed
/// errors — validation for literals, bind for params.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskTerm {
    Literal(crate::allen::AllenMask),
    Param(ParamId),
}

/// Comparison operators. `Eq`/`Ne` are legal for all seven types; order
/// operators only for U64/U64 and I64/I64 (no cross-type comparison, ever
/// — and never intervals). `Allen { mask }` is **the** interval-pair
/// comparison — two interval terms of one element type; satisfied iff
/// `classify(lhs, rhs)` is in the mask (`crate::allen`) — and interval
/// `Eq`/`Ne` are its derived facts (normalization canonicalizes them to
/// `EQUALS` / `¬EQUALS`, so exactly one interval-pair form reaches the
/// planner). `Contains` is point membership as a predicate — an interval
/// left side, an element-typed right side (the predicate form of the
/// membership binding rule, for terms already bound elsewhere); its
/// interval⊇interval form is gone — that predicate is `Allen(COVERS)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Allen { mask: MaskTerm },
    Contains,
}

impl CmpOp {
    /// Evaluates the operator over ordered operands (execution-level
    /// readers: filtered views and residual evaluation).
    pub(crate) fn compare<T: Ord>(self, left: &T, right: &T) -> bool {
        match self {
            Self::Eq => left == right,
            Self::Ne => left != right,
            Self::Lt => left < right,
            Self::Le => left <= right,
            Self::Gt => left > right,
            Self::Ge => left >= right,
            // Interval operators never reach single-word evaluation:
            // normalization lowers `Allen` to mask-carrying shapes and
            // `Contains` to endpoint compositions (`ir::normalize`).
            Self::Allen { .. } | Self::Contains => {
                unreachable!("interval operators are lowered before evaluation")
            }
        }
    }
}

/// One comparison predicate. `Eq` between two variables is unification and
/// obeys identical type rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub op: CmpOp,
    pub lhs: Term,
    pub rhs: Term,
}

/// The *input* predicate grammar: any boolean combination of positive
/// comparisons (`docs/architecture/20-query-ir.md`, § the input predicate
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
/// predicates are: the semantics are exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateTree {
    Leaf(Comparison),
    And(Vec<PredicateTree>),
    Or(Vec<PredicateTree>),
}

/// One rule: a conjunctive body projecting its find terms against the
/// query's head. The rule's denotation is the set of distinct bindings of
/// its variables satisfying every positive atom, every predicate, and no
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
    /// The predicate trees, conjoined — the list is an `And`, so the flat
    /// conjunctive rule is written without wrapping. Any nested OR is
    /// distributed away at validation ([`PredicateTree`]); downstream of
    /// the boundary a rule's predicates are a flat comparison list
    /// ([`LoweredRule`]).
    pub predicates: Vec<PredicateTree>,
}

impl Rule {
    /// The head shape this rule's find terms project — the degenerate
    /// one-rule query's head is exactly this row.
    #[must_use]
    pub fn head(&self) -> Vec<HeadTerm> {
        self.finds.iter().map(FindTerm::head_term).collect()
    }
}

/// A query: a non-recursive Datalog program — one head, a non-empty set
/// of conjunctive rules (`docs/architecture/20-query-ir.md`, normative).
///
/// **Denotation: the set union of the rules' denotations.** Set semantics
/// means there is exactly one union — no bag distinction exists or is
/// representable. Disjunction is data, never an execution node: a mask
/// inside a predicate, a set inside a position, rules at the top — the
/// three confinements; a cross-atom OR inside one rule is refused
/// representation (DNF lowering recovers it as rules).
///
/// The single-rule query is the degenerate case and embeds the
/// conjunctive query unchanged ([`Query::single`]). Rules are one step
/// short of the fixpoint on purpose — recursion stays an `OPEN` item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The find shape (arity + aggregate ops) every rule aligns against,
    /// position by position; at least one term, duplicates within a rule
    /// rejected at validation. The positional type row is computed at
    /// validation and pinned in the witness.
    pub head: Vec<HeadTerm>,
    /// At least one rule, at most [`MAX_RULES`].
    pub rules: Vec<Rule>,
}

impl Query {
    /// The degenerate one-rule program — the conjunctive query, with the
    /// head derived from the rule's own find shape.
    #[must_use]
    pub fn single(rule: Rule) -> Self {
        Self {
            head: rule.head(),
            rules: vec![rule],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interval::Interval;

    // These constructions double as documentation of the doc's example
    // query shapes over the ledger schema (Account, Posting, ...).

    #[test]
    fn point_lookup_by_fresh_key() {
        // Account(id = ?0, holder = h, status = s) — a single atom binding
        // the fresh key to a param.
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Param(ParamId(0))),
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            predicates: vec![],
        });
        assert_eq!(query.rules[0].atoms.len(), 1);
    }

    #[test]
    fn containment_walk_join_with_range_predicate() {
        // Posting(account = a, amount = amt, at = t), Account(id = a):
        // a containment walk joined on `a`, with t >= <timestamp>.
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    relation: RelationId(4),
                    bindings: vec![
                        (FieldId(2), Term::Var(VarId(0))),
                        (FieldId(4), Term::Var(VarId(1))),
                        (FieldId(5), Term::Var(VarId(2))),
                    ],
                },
                Atom {
                    relation: RelationId(1),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                },
            ],
            negated: vec![],
            predicates: vec![PredicateTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Literal(Value::I64(1_700_000_000_000_000)),
            })],
        });
        assert_eq!(query.rules[0].atoms.len(), 2);
        assert_eq!(query.rules[0].predicates.len(), 1);
    }

    #[test]
    fn aggregate_balance_by_account() {
        // finds: [account, Sum(amount), Count] — group key from output;
        // Count is nullary (over: None).
        let query = Query::single(Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: AggOp::Sum,
                    over: Some(VarId(1)),
                },
                FindTerm::Aggregate {
                    op: AggOp::Count,
                    over: None,
                },
            ],
            atoms: vec![Atom {
                relation: RelationId(4),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(0))),
                    (FieldId(4), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            predicates: vec![],
        });
        assert!(matches!(
            query.rules[0].finds[1],
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(_)
            }
        ));
    }

    #[test]
    fn zero_binding_atom_is_a_nonemptiness_gate() {
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                Atom {
                    relation: RelationId(0),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                },
                Atom {
                    relation: RelationId(7),
                    bindings: vec![], // gate: Cartesian with the rest
                },
            ],
            negated: vec![],
            predicates: vec![],
        });
        assert!(query.rules[0].atoms[1].bindings.is_empty());
    }

    #[test]
    fn anti_join_with_param_set_shape() {
        // Account(id = a, region ∈ ?set0), ¬Posting(account = a):
        // accounts in a region set with no postings. The negated atom
        // reuses `a` (the safety rule) and binds nothing.
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(3), Term::ParamSet(ParamId(0))),
                ],
            }],
            negated: vec![Atom {
                relation: RelationId(4),
                bindings: vec![(FieldId(2), Term::Var(VarId(0)))],
            }],
            predicates: vec![],
        });
        assert_eq!(query.rules[0].negated.len(), 1);
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
            Value::IntervalU64(0, u64::MAX),
            Value::IntervalI64(i64::MIN, i64::MAX),
            // Plus the one non-field value shape: the Allen mask (a
            // param's bind-time payload, never a stored type).
            Value::AllenMask(crate::allen::AllenMask::DISJOINT),
        ];
        assert_eq!(values.len(), 8);
    }

    #[test]
    fn interval_converts_through_the_checked_type() {
        // `From<Interval<_>>`: same halves, no re-check needed — the
        // checked type already holds `start < end`.
        let iv = Interval::<i64>::new(-5, 9).expect("valid bounds");
        assert_eq!(Value::from(iv), Value::IntervalI64(-5, 9));
        let iv = Interval::<u64>::new(3, 7).expect("valid bounds");
        assert_eq!(Value::from(iv), Value::IntervalU64(3, 7));
    }
}
