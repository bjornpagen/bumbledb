//! The DNF lowering property (PRD ALG-06): for randomized predicate
//! trees over small corpora, the **lowered rule set's union equals naive
//! tree evaluation**. The naive model evaluates the input tree directly
//! from the definition — it never lowers — so the differential *is* the
//! proof of [`bumbledb::ir::distribute`]: distributing to rules and
//! unioning their denotations changes nothing.

use std::collections::BTreeSet;

use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    Atom, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, Query, RelationId, Rule, Term,
    Value, VarId, ir,
};

use crate::corpus_gen::Rng;
use crate::fixture::field;
use crate::naive::{Delta, NaiveDb, Tuple};

/// One relation is enough: the property is about conditions, not joins —
/// Posting(account u64, amount i64), with tiny value domains so random
/// comparisons select real subsets.
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

/// A random predicate tree. Child counts include zero, so the empty
/// conjunction (true) and the empty disjunction (false — the rule lowers
/// to zero rules) are exercised, not just tolerated.
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
        let conditions: Vec<ConditionTree> =
            (0..=rng.range(2)).map(|_| tree(&mut rng, 3)).collect();
        let query = Query::single(posting_rule(conditions));

        let direct = db.query(&query, &[]).expect("no aggregates: no overflow");

        let mut union: BTreeSet<Tuple> = BTreeSet::new();
        for lowered in ir::distribute(&query.rules[0]) {
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

/// Posting widened by a `span` interval column for the measure-leaf
/// quantification — the tree grammar's one partial predicate.
fn interval_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Posting".into(),
            fields: vec![
                field("account", ValueType::U64),
                field("amount", ValueType::I64),
                field(
                    "span",
                    ValueType::Interval {
                        element: bumbledb::schema::IntervalElement::U64,
                        width: None,
                    },
                ),
            ],
        }],
        statements: vec![],
    }
}

/// Ray-bearing corpus: one row in four carries `[start, ∞)`, so measure
/// leaves render `Ray` verdicts, not just booleans.
fn corpus_with_spans(rng: &mut Rng, rows: u64) -> NaiveDb {
    let mut db = NaiveDb::new(&interval_schema());
    let inserts = (0..rows)
        .map(|_| {
            let start = rng.range(8);
            let span = if rng.chance(1, 4) {
                bumbledb::Interval::<u64>::ray(start).expect("start is in the point domain")
            } else {
                bumbledb::Interval::<u64>::new(start, start + 1 + rng.range(9))
                    .expect("nonempty by construction")
            };
            (
                POSTING,
                vec![
                    Value::U64(rng.range(ACCOUNT_DOMAIN)),
                    Value::I64(i64::try_from(rng.range(AMOUNT_SPREAD)).expect("small") - 3),
                    Value::IntervalU64(span),
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

/// The measure leaf — `|span|` under an order operator against a small
/// duration literal; the only leaf whose verdict can be `Ray`.
fn measure_leaf(rng: &mut Rng) -> ConditionTree {
    let literal = Term::Literal(Value::U64(rng.range(12)));
    let measure = Term::Measure(VarId(2));
    let op = match rng.range(4) {
        0 => CmpOp::Lt,
        1 => CmpOp::Le,
        2 => CmpOp::Gt,
        _ => CmpOp::Ge,
    };
    let (lhs, rhs) = if rng.chance(1, 2) {
        (measure, literal)
    } else {
        (literal, measure)
    };
    ConditionTree::Leaf(Comparison { op, lhs, rhs })
}

/// A random tree over the widened leaf pool: scalar comparisons plus
/// the measure — one leaf in three is partial.
fn measured_tree(rng: &mut Rng, depth: u64) -> ConditionTree {
    if depth == 0 || rng.chance(2, 5) {
        return if rng.chance(1, 3) {
            measure_leaf(rng)
        } else {
            leaf(rng)
        };
    }
    let children = (0..rng.range(4))
        .map(|_| measured_tree(rng, depth - 1))
        .collect();
    if rng.chance(1, 2) {
        ConditionTree::And(children)
    } else {
        ConditionTree::Or(children)
    }
}

fn span_rule(conditions: Vec<ConditionTree>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions,
    }
}

/// The Kleene-verdict face of the property (ruled 2026-07-23, R6):
/// distributing a tree with measure leaves to DNF and re-evaluating the
/// distributed form — `Or` over each disjunct's conjunction — moves no
/// verdict, over ray-bearing rows where leaves genuinely render `Ray`.
/// The `Result` comparison quantifies the raise itself: a binding
/// raises iff the order-insensitive fold is `Ray`
/// (`lean/Bumbledb/Query/Aggregates.lean: Verdict3.and_or_distrib`),
/// never "iff evaluation reached the measure first".
#[test]
fn distribution_preserves_the_kleene_verdict() {
    let mut rays_raised = 0u64;
    for seed in 0..300 {
        let mut rng = Rng::new(seed ^ 0x5EED_D0F0);
        let rows = 1 + rng.range(24);
        let db = corpus_with_spans(&mut rng, rows);
        let conditions: Vec<ConditionTree> = (0..=rng.range(2))
            .map(|_| measured_tree(&mut rng, 3))
            .collect();
        let query = Query::single(span_rule(conditions));

        let direct = db.query(&query, &[]);

        let distributed = ConditionTree::Or(
            ir::distribute(&query.rules[0])
                .into_iter()
                .map(|lowered| {
                    ConditionTree::And(
                        lowered
                            .conditions
                            .into_iter()
                            .map(ConditionTree::Leaf)
                            .collect(),
                    )
                })
                .collect(),
        );
        let redistributed = db.query(&Query::single(span_rule(vec![distributed])), &[]);

        if direct == Err(crate::naive::query::QueryError::MeasureOfRay) {
            rays_raised += 1;
        }
        assert_eq!(
            direct, redistributed,
            "seed {seed}: distribution moved the verdict"
        );
    }
    // The quantification is real: a healthy share of seeds raise.
    assert!(rays_raised >= 30, "only {rays_raised} seeds raised");
}

/// The finding-024 reproduction, pinned: the same denotation with the
/// `Or`'s children in both orders — the verdict must not flip on child
/// order (the retired `Cell<bool>` poison flipped it).
#[test]
fn the_ray_verdict_is_child_order_blind() {
    let mut db = NaiveDb::new(&interval_schema());
    db.apply(&Delta {
        deletes: vec![],
        inserts: vec![(
            POSTING,
            vec![
                Value::U64(3),
                Value::I64(0),
                Value::IntervalU64(bumbledb::Interval::<u64>::ray(3).expect("in domain")),
            ],
        )],
    })
    .expect("fixture facts commit");
    let scalar = ConditionTree::Leaf(Comparison {
        op: CmpOp::Lt,
        lhs: Term::Var(VarId(0)),
        rhs: Term::Literal(Value::U64(5)),
    });
    let measure = ConditionTree::Leaf(Comparison {
        op: CmpOp::Lt,
        lhs: Term::Measure(VarId(2)),
        rhs: Term::Literal(Value::U64(10)),
    });
    // Or: the scalar child holds, so the binding is saved — the sibling
    // ray is not demanded, in either child order.
    for children in [
        vec![scalar.clone(), measure.clone()],
        vec![measure.clone(), scalar.clone()],
    ] {
        let query = Query::single(span_rule(vec![ConditionTree::Or(children)]));
        let answers = db.query(&query, &[]).expect("the holding child saves it");
        assert_eq!(answers.len(), 1);
    }
    // And: the scalar child holds, so the conjunction demands the
    // measure — `Ray` in either order.
    for children in [vec![scalar.clone(), measure.clone()], vec![measure, scalar]] {
        let query = Query::single(span_rule(vec![ConditionTree::And(children)]));
        assert_eq!(
            db.query(&query, &[]),
            Err(crate::naive::query::QueryError::MeasureOfRay),
            "the demanded ray raises, order-blind"
        );
    }
}
