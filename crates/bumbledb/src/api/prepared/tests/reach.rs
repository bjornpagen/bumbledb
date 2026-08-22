//! Interiors-only vs rec body: the one-engine locks.

use super::*;
use crate::ir::{CmpOp, ProjectionRule};

fn interiors_only() -> Query {
    Query {
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
        rec: None,
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
    let query = Query {
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
        rec: None,
    };
    let prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    match &prepared.pipeline {
        PreparedPipeline::Cq { interiors, rules } => {
            assert!(
                !interiors.is_empty(),
                "expected live interiors, got {}",
                interiors.len()
            );
            assert!(
                rules.is_empty(),
                "expected dead main, got {} rules",
                rules.len()
            );
        }
        PreparedPipeline::Reach { .. } => panic!("expected Cq, got Reach"),
        PreparedPipeline::PointProbe { .. } => panic!("expected Cq, got PointProbe"),
    }
}
