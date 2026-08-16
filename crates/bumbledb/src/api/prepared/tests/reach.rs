//! Interiors-only vs rec body: the one-engine locks.

use super::*;
use crate::api::stats::StatsBody;
use crate::ir::{CmpOp, NonEmpty, ProjectionRule, Rec, RecRule, RecStep};

fn interiors_only() -> Query {
    Query::Cq {
        interiors: vec![Interior {
            rules: vec![ProjectionRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(POSTING),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                }],
                negated: vec![],
                conditions: vec![],
            }],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

#[test]
fn an_interiors_only_query_does_not_enter_reach() {
    let dir = TempDir::new("prepared-interiors-only-body");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 100)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let prepared = prepare(&txn, &cache, &schema, &interiors_only()).expect("prepare");
    assert!(
        !matches!(prepared.pipeline, PreparedPipeline::Reach { .. }),
        "interiors-only must not build ReachDriver"
    );
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::Cq { .. }),
        "interiors-only pipeline is Cq"
    );
}

#[test]
fn a_tight_tuple_budget_trips_on_an_interiors_only_query() {
    let dir = TempDir::new("prepared-interiors-tuple-budget");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 7, "a", 100), (2, 7, "b", 200), (3, 8, "c", 300)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &interiors_only()).expect("prepare");
    prepared.set_derived_budget(0, 0);
    let err = prepared
        .execute_collect(&txn, &cache, &[])
        .expect_err("zero tuples trips a nonempty interior");
    assert!(
        matches!(
            err,
            Error::DerivedBudgetExceeded {
                rounds: 0,
                tuples
            } if tuples > 0
        ),
        "expected DerivedBudgetExceeded {{ rounds: 0 }}, got {err:?}"
    );
    prepared.set_derived_budget(0, u64::MAX);
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("tight rounds alone must not trip interiors-only");
    assert_eq!(out.len(), 3);
}

#[test]
fn dead_main_with_live_interiors_still_reports_interior_emits() {
    let dir = TempDir::new("prepared-dead-main-live-interiors");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 100), (2, 7, "b", 200)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    // Main is an EDB rule whose constant conditions refute themselves —
    // the known fold kernel (`score > 5 ∧ score < 3` on i64). Interiors
    // stay live; the pipeline is Cq with empty main rules.
    let query = Query::Cq {
        interiors: interiors_only().interiors().to_vec(),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Edb(POSTING),
                bindings: vec![(FieldId(3), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![
                ConditionTree::Leaf(Comparison {
                    op: CmpOp::Gt,
                    lhs: Term::Var(VarId(0)),
                    rhs: Term::Literal(Value::I64(5)),
                }),
                ConditionTree::Leaf(Comparison {
                    op: CmpOp::Lt,
                    lhs: Term::Var(VarId(0)),
                    rhs: Term::Literal(Value::I64(3)),
                }),
            ],
        }],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    match &prepared.pipeline {
        PreparedPipeline::Cq { interiors, rules } => {
            assert!(
                !interiors.is_empty(),
                "expected live interiors, got {}",
                interiors.len()
            );
            assert!(
                rules.is_empty(),
                "expected dead main, got {} rules; dead={:?}",
                rules.len(),
                prepared.dead
            );
        }
        PreparedPipeline::Reach { .. } => panic!("expected Cq, got Reach"),
    }
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    match &stats.body {
        StatsBody::Cq { rules, interiors } => {
            assert!(rules.is_empty(), "dead main is rules: [] plus stats.dead");
            assert_eq!(interiors.len(), 1);
            assert!(
                interiors[0].emits > 0,
                "live interiors still report emits when main is dead: {stats:?}"
            );
        }
        StatsBody::Reach { .. } => panic!("dead-main-with-interiors is Cq, got Reach"),
    }
}

fn rec_query() -> Query {
    Query::Reach {
        interiors: vec![],
        rec: Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(POSTING),
                    bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0)],
                self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                atoms: vec![],
                conditions: vec![],
            }),
        },
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

#[test]
fn interiors_only_profile_is_the_cq_arm() {
    let dir = TempDir::new("prepared-interiors-only-stats");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 100)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &interiors_only()).expect("prepare");
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    match &stats.body {
        StatsBody::Cq { interiors, .. } => {
            assert_eq!(interiors.len(), 1);
            assert!(
                interiors[0].emits > 0,
                "interiors-only reports emits: {stats:?}"
            );
        }
        StatsBody::Reach { .. } => panic!("interiors-only profile must be Cq"),
    }
}

#[test]
fn reach_profile_is_the_reach_arm() {
    let dir = TempDir::new("prepared-reach-stats");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 100)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &rec_query()).expect("prepare");
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    match &stats.body {
        StatsBody::Reach { interiors, reach } => {
            assert!(interiors.is_empty(), "this rec has no named interiors");
            assert!(!reach.rounds.is_empty(), "reach reports rounds: {stats:?}");
            assert!(
                stats.rules().is_empty(),
                "Reach does not grow a main-rule table"
            );
        }
        StatsBody::Cq { .. } => panic!("rec profile must be Reach"),
    }
}
