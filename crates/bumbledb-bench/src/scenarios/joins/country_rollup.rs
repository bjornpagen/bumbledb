use bumbledb::{AggOp, Atom, FieldId, FindTerm, Query, VarId};

use super::ids;
use super::term::var;

/// j5 — kind/country rollup over the full join: Min(year) and Count per
/// (country) — the aggregate face of join-order stress.
pub(super) fn country_rollup() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::MOVIE_COMPANY,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(3))],
            },
            Atom {
                relation: ids::COMPANY,
                bindings: vec![(FieldId(0), var(3)), (FieldId(2), var(0))],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![(FieldId(0), var(2)), (FieldId(2), var(1))],
            },
        ],
        predicates: vec![],
    }
}
