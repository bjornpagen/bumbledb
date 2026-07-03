//! The pure-data query IR, validation, and normalization (PRDs 13-15).
//!
//! Queries are plain data — serializable, inspectable, no behavior
//! (`docs/architecture/20-query-ir.md`, normative). No wildcard variant
//! exists: an unbound field is *absent* from `bindings`, so "wildcard bound
//! to something" is unwritable. Variables carry dense ids only; names are a
//! debugging sidecar the engine never stores.

pub(crate) mod normalize;
pub(crate) mod validate;

use crate::schema::{FieldId, RelationId};

/// Dense query-variable id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarId(pub u16);

/// Dense parameter id; values are supplied positionally at execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParamId(pub u16);

/// A literal value. Exactly one variant per data-model type — no universal
/// integer (U64 and I64 literals are exact-typed; out-of-range is
/// unrepresentable rather than truncated), and Bytes exists by construction
/// (the v5 hole, post-mortem §13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    /// Declaration-order ordinal; range-checked against the bound field's
    /// variant list at validation.
    Enum(u8),
    /// Raw UTF-8 bytes; interning is the engine's job (resolved to an
    /// intern id per execution — a dictionary miss means empty result).
    String(Box<[u8]>),
    /// Raw bytes; interning as above.
    Bytes(Box<[u8]>),
}

/// One term of an atom binding or comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Var(VarId),
    Param(ParamId),
    Literal(Value),
}

/// One atom: a relation with named-field bindings. Absence of a field *is*
/// the wildcard. An atom with zero bindings is legal and means a
/// nonemptiness gate on the relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub relation: RelationId,
    pub bindings: Vec<(FieldId, Term)>,
}

/// Aggregate operators. `Count` is nullary (`over: None`): it counts the
/// group's binding set, exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggOp {
    Sum,
    Min,
    Max,
    Count,
}

/// One find term: a projected variable or an aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindTerm {
    Var(VarId),
    Aggregate { op: AggOp, over: Option<VarId> },
}

/// Comparison operators. `Eq`/`Ne` are legal for all six types; order
/// operators only for U64/U64 and I64/I64 (no cross-type comparison, ever).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
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
    /// At least one atom; conjunctive.
    pub atoms: Vec<Atom>,
    pub predicates: Vec<Comparison>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // These constructions double as documentation of the doc's example
    // query shapes over the ledger schema (Account, Posting, ...).

    #[test]
    fn point_lookup_by_unique_key() {
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
            predicates: vec![],
        };
        assert_eq!(query.atoms.len(), 1);
    }

    #[test]
    fn fk_walk_join_with_range_predicate() {
        // Posting(account = a, amount = amt, at = t), Account(id = a):
        // an FK walk joined on `a`, with t >= <timestamp>.
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
            predicates: vec![],
        };
        assert!(query.atoms[1].bindings.is_empty());
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
        ];
        assert_eq!(values.len(), 6);
    }
}
