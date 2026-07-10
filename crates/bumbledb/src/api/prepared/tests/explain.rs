use super::*;

#[test]
fn explain_reports_the_join_plan_with_actuals() {
    let dir = TempDir::new("prepared-explain");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 1), (2, 7, "b", 2)]);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let (rows, report) = prepared
        .explain(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("explain");
    assert_eq!(rows.len(), 2);
    assert!(report.contains("free join"));
    assert!(report.contains("emitted bindings: 2"));
}

/// The stats surface carries the pin record — golden on one EXPLAIN
/// report: every node estimate is "estimated from (pinned rows at
/// prepare)", and a guard probe (which reads no statistics) pins
/// nothing.
#[test]
fn the_stats_surface_carries_the_pinned_rows() {
    let dir = TempDir::new("prepared-pinned-rows");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 7, "a", 1), (2, 7, "b", 2), (3, 9, "c", 3)],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let (_, stats) = prepared
        .profile(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("profile");
    assert_eq!(stats.pinned.len(), 1, "one participating occurrence");
    let pin = &stats.pinned[0];
    assert_eq!(pin.occurrence, 0);
    assert_eq!(pin.relation, "Posting");
    assert_eq!(pin.rows, 3, "the S count read at prepare");
    assert!(
        pin.survivors.is_some(),
        "the account selection + amount range make a filtered view"
    );

    let (_, report) = prepared
        .explain(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("explain");
    assert!(
        report.contains("estimated from (pinned rows at prepare): 3"),
        "{report}"
    );

    // A guard probe classifies before statistics: nothing is pinned.
    let guard_query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let mut guard = prepare(&txn, &cache, &schema, &guard_query).expect("prepare");
    let (_, stats) = guard
        .profile(&txn, &cache, &[BindValue::U64(1)])
        .expect("profile");
    assert!(stats.pinned.is_empty(), "guard probes read no statistics");
}

#[test]
fn profile_returns_structured_stats_matching_the_execution() {
    let dir = TempDir::new("prepared-profile");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[
            (1, 7, "rent", -1200),
            (2, 7, "food", -55),
            (3, 9, "gym", -80),
        ],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let (rows, stats) = prepared
        .profile(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("profile");
    assert_eq!(rows.len(), 2);
    assert_eq!(stats.emits, 2);
    assert!(stats.guard.is_none());
    assert!(!stats.nodes.is_empty());
    let last = stats.nodes.last().expect("nodes");
    assert_eq!(last.actual, stats.emits, "last node's actual = emits");
    assert!(
        stats.nodes[0].batches >= 1 && stats.nodes[0].batch_entries >= stats.nodes[0].batches,
        "batching counters populated: {stats:?}"
    );

    // The rendered explain is built from the same struct — spot-pin
    // the format so the golden contract holds.
    let (_, report) = prepared
        .explain(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("explain");
    assert!(report.contains("access path: free join"), "{report}");
    assert!(report.contains("emitted bindings: 2"), "{report}");

    // Guard profile: no nodes, a hit flag.
    let guard_query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let mut guard = prepare(&txn, &cache, &schema, &guard_query).expect("prepare");
    let (rows, stats) = guard
        .profile(&txn, &cache, &[BindValue::U64(1)])
        .expect("profile");
    assert_eq!(rows.len(), 1);
    assert!(stats.nodes.is_empty());
    assert_eq!(
        stats.guard,
        Some(crate::api::stats::GuardStats { hit: true })
    );
    let (_, stats) = guard
        .profile(&txn, &cache, &[BindValue::U64(999)])
        .expect("profile");
    assert_eq!(
        stats.guard,
        Some(crate::api::stats::GuardStats { hit: false })
    );
}
