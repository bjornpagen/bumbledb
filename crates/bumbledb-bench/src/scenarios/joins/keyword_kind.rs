use bumbledb::{
    Atom, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, Query, Rule, Value, VarId,
};

use super::corpus::s;
use super::term::{param, var};
use super::{HOT_KEYWORDS, KEYWORDS, ids, mix};
use crate::corpus_gen::Rng;

/// j3 — keyword × kind: two interned-string/enum-selective dimensions
/// pinching a 3-way join from both sides.
pub(super) fn keyword_kind() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::KEYWORD,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), param(0))],
            },
            Atom {
                relation: ids::MOVIE_KEYWORD,
                bindings: vec![(FieldId(1), var(2)), (FieldId(0), var(3))],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![
                    (FieldId(0), var(3)),
                    (FieldId(1), var(0)),
                    (FieldId(2), var(1)),
                    (FieldId(3), var(4)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(1),
            rhs: param(1),
        })],
    })
}

pub(super) fn keyword_kind_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 900, 3));
    let kw = |k: u64| s(format!("kw-{k:05}"));
    vec![
        vec![kw(rng.range(HOT_KEYWORDS)), Value::I64(1980)],
        vec![
            kw(HOT_KEYWORDS + rng.range(KEYWORDS - HOT_KEYWORDS)),
            Value::I64(1960),
        ],
        vec![
            kw(HOT_KEYWORDS + rng.range(KEYWORDS - HOT_KEYWORDS)),
            Value::I64(2000),
        ],
        vec![s("kw-never-a-keyword".to_owned()), Value::I64(1980)],
    ]
}
