use bumbledb::{Atom, FieldId, FindTerm, Query, VarId};

use super::ids;
use super::term::{param, var};

/// j6 — keyword neighborhood: movies sharing any keyword with a
/// person's movies — the fan-out explosion a bad order makes fatal.
pub(super) fn keyword_neighborhood() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(1), param(0)), (FieldId(0), var(1))],
            },
            Atom {
                relation: ids::MOVIE_KEYWORD,
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(2))],
            },
            Atom {
                relation: ids::MOVIE_KEYWORD,
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(2))],
            },
        ],
        predicates: vec![],
    }
}
