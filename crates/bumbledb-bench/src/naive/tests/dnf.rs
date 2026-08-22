use std::collections::BTreeSet;

use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    Atom, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, Query, RelationId, Rule, Term,
    Value, VarId, ir,
};

use crate::corpus_gen::Rng;
use crate::fixture::field;
use crate::naive::{Delta, NaiveDb, Tuple};

fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Posting".into(),
            fields: vec![
                field("account", ValueType::U64),
                field("amount", ValueType::I64),
            ],
        }],
        statements: vec![],
    }
}

const POSTING: RelationId = RelationId(0);
const ACCOUNT_DOMAIN: u64 = 5;
const AMOUNT_SPREAD: u64 = 7;

fn corpus(rng: &mut Rng, rows: u64) -> NaiveDb {
    let mut db = NaiveDb::new(&schema());
    let inserts = (0..rows)
        .map(|_| {
            (
                POSTING,
                vec![
                    Value::U64(rng.range(ACCOUNT_DOMAIN)),
                    Value::I64(i64::try_from(rng.range(AMOUNT_SPREAD)).expect("small") - 3),
                ],
            )
        })
        .collect();
    db.apply(&Delta {
        deletes: vec![],
        inserts,
    })
    .expect("fixture facts commit (no statements declared)");
    db
}

fn leaf(rng: &mut Rng) -> ConditionTree {
    let (var, literal) = if rng.chance(1, 2) {
        (VarId(0), Value::U64(rng.range(ACCOUNT_DOMAIN)))
    } else {
        (
            VarId(1),
            Value::I64(i64::try_from(rng.range(AMOUNT_SPREAD)).expect("small") - 3),
        )
    };
    let op = match rng.range(6) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        _ => CmpOp::Ge,
    };
    let (lhs, rhs) = if rng.chance(1, 2) {
        (Term::Var(var), Term::Literal(literal))
    } else {
        (Term::Literal(literal), Term::Var(var))
    };
    ConditionTree::Leaf(Comparison { op, lhs, rhs })
}

fn tree(rng: &mut Rng, depth: u64) -> ConditionTree {
    if depth == 0 || rng.chance(2, 5) {
        return leaf(rng);
    }
    let children = (0..rng.range(4)).map(|_| tree(rng, depth - 1)).collect();
    if rng.chance(1, 2) {
        ConditionTree::And(children)
    } else {
        ConditionTree::Or(children)
    }
}

fn posting_rule(conditions: Vec<ConditionTree>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions,
    }
}

#[test]
fn lowered_rule_set_union_equals_naive_tree_evaluation() {
    for seed in 0..300 {
        let mut rng = Rng::new(seed);
        let rows = 1 + rng.range(24);
        let db = corpus(&mut rng, rows);
        let conditions: Vec<ConditionTree> =
            (0..=rng.range(2)).map(|_| tree(&mut rng, 3)).collect();
        let query = Query::single(posting_rule(conditions));

        let direct = db.query(&query, &[]).expect("no aggregates: no overflow");

        let mut union: BTreeSet<Tuple> = BTreeSet::new();
        for lowered in ir::distribute(&query.rules()[0]) {
            let ir::LoweredRule {
                finds,
                atoms,
                negated,
                conditions,
                written: _,
                minted: _,
            } = lowered;
            let conjunctive = Query::single(Rule {
                finds,
                atoms,
                negated,
                conditions: conditions.into_iter().map(ConditionTree::Leaf).collect(),
            });
            union.extend(
                db.query(&conjunctive, &[])
                    .expect("no aggregates: no overflow"),
            );
        }

        assert_eq!(
            direct, union,
            "seed {seed}: lowering changed the denotation"
        );
    }
}
