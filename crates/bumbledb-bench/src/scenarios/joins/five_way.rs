use bumbledb::{Atom, CmpOp, Comparison, FieldId, FindTerm, Query, Value, VarId};

use super::ids;
use super::term::{param, var};

/// j4 — the JOB-shaped 5-way: fact table pinched by three dimension
/// filters (gender, country, year window) on alternating sides.
pub(super) fn five_way() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(3))],
            },
            Atom {
                relation: ids::PERSON,
                bindings: vec![
                    (FieldId(0), var(3)),
                    (FieldId(1), var(0)),
                    (FieldId(2), param(0)),
                ],
            },
            Atom {
                relation: ids::MOVIE_COMPANY,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(4))],
            },
            Atom {
                relation: ids::COMPANY,
                bindings: vec![
                    (FieldId(0), var(4)),
                    (FieldId(1), var(1)),
                    (FieldId(2), param(1)),
                ],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![(FieldId(0), var(2)), (FieldId(2), var(5))],
            },
        ],
        negated: vec![],
        predicates: vec![
            Comparison {
                op: CmpOp::Ge,
                lhs: var(5),
                rhs: param(2),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(5),
                rhs: param(3),
            },
        ],
    }
}

pub(super) fn five_way_params(_: u64) -> Vec<Vec<Value>> {
    // Gender enum, country enum, year window: tight, mid, wide, empty.
    vec![
        vec![
            Value::Enum(0),
            Value::Enum(2),
            Value::I64(1990),
            Value::I64(1995),
        ],
        vec![
            Value::Enum(1),
            Value::Enum(0),
            Value::I64(1970),
            Value::I64(1990),
        ],
        vec![
            Value::Enum(2),
            Value::Enum(5),
            Value::I64(1930),
            Value::I64(2020),
        ],
        vec![
            Value::Enum(0),
            Value::Enum(7),
            Value::I64(2020),
            Value::I64(1930),
        ],
    ]
}
