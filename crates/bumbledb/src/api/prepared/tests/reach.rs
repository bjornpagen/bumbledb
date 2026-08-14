//! Interiors-only vs rec body: the one-engine locks.

use super::*;

fn interiors_only() -> Query {
    Query {
        interiors: vec![Interior {
            head: vec![HeadTerm::Var],
            rules: vec![Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    source: AtomSource::Edb(POSTING),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                }],
                negated: vec![],
                conditions: vec![],
            }],
        }],
        rec: None,
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
        !matches!(prepared.body, PreparedBody::Reach(_)),
        "interiors-only must not build ReachDriver"
    );
    assert!(
        matches!(prepared.body, PreparedBody::Rules(_)),
        "interiors-only body is Rules"
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
