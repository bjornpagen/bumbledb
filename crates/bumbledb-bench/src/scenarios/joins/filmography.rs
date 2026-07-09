use bumbledb::{Atom, FieldId, FindTerm, Query, Value, VarId};

use super::term::{param, var};
use super::{ids, mix, HOT_PEOPLE, PEOPLE};
use crate::gen::Rng;

/// j1 — one hot person, one cold person, one mid, one miss: fan-in skew
/// on a 2-atom FK walk.
pub(super) fn filmography() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(1), param(0)), (FieldId(0), var(2))],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![
                    (FieldId(0), var(2)),
                    (FieldId(1), var(0)),
                    (FieldId(2), var(1)),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    }
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
