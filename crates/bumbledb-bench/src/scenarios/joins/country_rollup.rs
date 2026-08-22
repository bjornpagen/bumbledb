use bumbledb::{Atom, FieldId, FindTerm, FoldOp, Query, Rule, VarId};

use super::ids;
use super::term::var;

pub(super) fn country_rollup() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Min,
                over: VarId(1),
            },
            FindTerm::Count,
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MOVIE_COMPANY),
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(3))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::COMPANY),
                bindings: vec![(FieldId(0), var(3)), (FieldId(2), var(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MOVIE),
                bindings: vec![(FieldId(0), var(2)), (FieldId(2), var(1))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}
