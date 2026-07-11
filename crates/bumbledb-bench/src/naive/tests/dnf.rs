//! The DNF lowering property (PRD ALG-06): for randomized predicate
//! trees over small corpora, the **lowered rule set's union equals naive
//! tree evaluation**. The naive model evaluates the input tree directly
//! from the definition — it never lowers — so the differential *is* the
//! proof of [`bumbledb::ir::distribute`]: distributing to rules and
//! unioning their denotations changes nothing.

use std::collections::BTreeSet;

use bumbledb::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
};
use bumbledb::{
    ir, Atom, CmpOp, Comparison, FieldId, FindTerm, PredicateTree, Query, RelationId, Rule, Term,
    Value, VarId,
};

use crate::gen::Rng;
use crate::naive::{Delta, NaiveDb, Tuple};

/// One relation is enough: the property is about predicates, not joins —
/// Posting(account u64, amount i64), with tiny value domains so random
/// comparisons select real subsets.
fn schema() -> SchemaDescriptor {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
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
const AMOUNT_SPREAD: u64 = 7; // amounts in -3..=3

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

/// One random comparison: a variable side (account or amount) against a
/// literal drawn from the same small domain, under a random operator.
fn leaf(rng: &mut Rng) -> PredicateTree {
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
    PredicateTree::Leaf(Comparison { op, lhs, rhs })
}

/// A random predicate tree. Child counts include zero, so the empty
/// conjunction (true) and the empty disjunction (false — the rule lowers
/// to zero rules) are exercised, not just tolerated.
fn tree(rng: &mut Rng, depth: u64) -> PredicateTree {
    if depth == 0 || rng.chance(2, 5) {
        return leaf(rng);
    }
    let children = (0..rng.range(4)).map(|_| tree(rng, depth - 1)).collect();
    if rng.chance(1, 2) {
        PredicateTree::And(children)
    } else {
        PredicateTree::Or(children)
    }
}

fn posting_rule(predicates: Vec<PredicateTree>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates,
    }
}

/// The property, quantified over seeds: distribute the tree query to
/// Or-free rules, evaluate each rule naively as the conjunctive query it
/// is, union the results — and compare against evaluating the *tree*
/// naively, which never touched the lowering.
#[test]
fn lowered_rule_set_union_equals_naive_tree_evaluation() {
    for seed in 0..300 {
        let mut rng = Rng::new(seed);
        let rows = 1 + rng.range(24);
        let db = corpus(&mut rng, rows);
        let predicates: Vec<PredicateTree> =
            (0..=rng.range(2)).map(|_| tree(&mut rng, 3)).collect();
        let query = Query::single(posting_rule(predicates));

        let direct = db.query(&query, &[]).expect("no aggregates: no overflow");

        let mut union: BTreeSet<Tuple> = BTreeSet::new();
        for lowered in ir::distribute(&query.rules[0]) {
            let ir::LoweredRule {
                finds,
                atoms,
                negated,
                predicates,
            } = lowered;
            let conjunctive = Query::single(Rule {
                finds,
                atoms,
                negated,
                predicates: predicates.into_iter().map(PredicateTree::Leaf).collect(),
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
