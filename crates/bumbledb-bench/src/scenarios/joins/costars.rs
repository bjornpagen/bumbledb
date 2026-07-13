use bumbledb::{Atom, FieldId, FindTerm, Query, Rule, VarId};

use super::ids;
use super::term::{param, var};

/// j2 — costars: the self-join through a shared movie, hot vs cold.
pub(super) fn costars() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), param(0))],
            },
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}
