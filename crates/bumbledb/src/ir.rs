//! The pure-data query IR, validation, and normalization (docs/architecture).
//!
//! Queries are plain data — serializable, inspectable, no behavior
//! (`docs/architecture/20-query-ir.md`, normative). No wildcard variant
//! exists: an unbound field is *absent* from `bindings`, so "wildcard bound
//! to something" is unwritable. Variables carry dense ids only; names are a
//! debugging sidecar the engine never stores.

pub(crate) mod normalize;
pub(crate) mod validate;

use crate::schema::{FieldId, RelationId};

/// The one literal-value sum, shared with statement selections — the
/// normative IR block in `docs/architecture/20-query-ir.md` names it here.
pub use crate::value::Value;

/// Dense query-variable id.
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
}

/// One find term: a projected variable or an aggregate. `over` is `None`
/// for the nullary `Count`, `Some(counted var)` for `CountDistinct`, the
/// aggregated variable for `Sum`/`Min`/`Max`, and the *carried* variable
/// for `ArgMax`/`ArgMin` (the key rides in the op).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindTerm {
    Var(VarId),
    Aggregate { op: AggOp, over: Option<VarId> },
}

/// Comparison operators. `Eq`/`Ne` are legal for all seven types; order
/// operators only for U64/U64 and I64/I64 (no cross-type comparison, ever
/// — and never intervals). `Overlaps` requires two interval terms of one
/// element type: satisfied iff the point-sets intersect. `Contains`
/// requires an interval left side and either an interval of the same
/// element type (⊇ of point-sets) or an element-typed right side (point
/// membership as a predicate, for terms already bound elsewhere).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Overlaps,
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
            // normalization decomposes them into fixed word-comparison
            // shapes over the interval's start/end (`ir::normalize`).
            Self::Overlaps | Self::Contains => {
                unreachable!("interval operators are decomposed at lowering")
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

/// A conjunctive query: the logical solution is the set of distinct
/// bindings of all query variables; projection returns the set of
/// projected facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// At least one term; duplicates rejected at validation.
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
    /// here; negation is a *position* in the query, not a kind of atom, so
    /// the list reuses [`Atom`] unchanged.
    pub negated: Vec<Atom>,
    pub predicates: Vec<Comparison>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interval::Interval;

    // These constructions double as documentation of the doc's example
    // query shapes over the ledger schema (Account, Posting, ...).

    #[test]
    fn point_lookup_by_serial_key() {
        // Account(id = ?0, holder = h, status = s) — a single atom binding
        // the serial key to a param.
        let query = Query {
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
        };
        assert_eq!(query.atoms.len(), 1);
    }

    #[test]
    fn containment_walk_join_with_range_predicate() {
        // Posting(account = a, amount = amt, at = t), Account(id = a):
        // a containment walk joined on `a`, with t >= <timestamp>.
        let query = Query {
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
            predicates: vec![Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Literal(Value::I64(1_700_000_000_000_000)),
            }],
        };
        assert_eq!(query.atoms.len(), 2);
        assert_eq!(query.predicates.len(), 1);
    }

    #[test]
    fn aggregate_balance_by_account() {
        // finds: [account, Sum(amount), Count] — group key from output;
        // Count is nullary (over: None).
        let query = Query {
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
        };
        assert!(matches!(
            query.finds[1],
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(_)
            }
        ));
    }

    #[test]
    fn zero_binding_atom_is_a_nonemptiness_gate() {
        let query = Query {
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
        };
        assert!(query.atoms[1].bindings.is_empty());
    }

    #[test]
    fn anti_join_with_param_set_shape() {
        // Account(id = a, region ∈ ?set0), ¬Posting(account = a):
        // accounts in a region set with no postings. The negated atom
        // reuses `a` (the safety rule) and binds nothing.
        let query = Query {
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
        };
        assert_eq!(query.negated.len(), 1);
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
            Value::Enum(3),
            Value::String(Box::from(&b"text"[..])),
            Value::Bytes(Box::from(&[0xDEu8, 0xAD][..])),
            Value::IntervalU64(0, u64::MAX),
            Value::IntervalI64(i64::MIN, i64::MAX),
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
