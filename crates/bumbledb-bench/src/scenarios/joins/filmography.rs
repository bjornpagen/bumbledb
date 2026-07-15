use bumbledb::{Atom, FieldId, FindTerm, Query, Rule, Value, VarId};

use super::term::{param, var};
use super::{HOT_PEOPLE, PEOPLE, ids, mix};
use crate::corpus_gen::Rng;

/// j1 — one hot person, one cold person, one mid, one miss: fan-in skew
/// on a 2-atom containment walk.
pub(super) fn filmography() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::CAST_INFO),
                bindings: vec![(FieldId(1), param(0)), (FieldId(0), var(2))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MOVIE),
                bindings: vec![
                    (FieldId(0), var(2)),
                    (FieldId(1), var(0)),
                    (FieldId(2), var(1)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

pub(super) fn filmography_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 900, 1));
    vec![
        vec![Value::U64(rng.range(HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(PEOPLE + 1_000_000)],
    ]
}
