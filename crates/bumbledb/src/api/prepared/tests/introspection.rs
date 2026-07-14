use super::*;

#[test]
fn introspection_reports_the_join_plan_with_actuals() {
    let dir = TempDir::new("prepared-introspect");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 1), (2, 7, "b", 2)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let (answers, report) = prepared
        .introspect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("introspect");
    assert_eq!(answers.len(), 2);
    assert!(report.contains("free join"));
    assert!(report.contains("emitted bindings: 2"));
}

/// The report's header renders the predicate the query defines — the
/// signature authority (`ir/validate`), one column per head position,
/// fold kinds by their rule-notation names.
#[test]
fn the_introspection_header_renders_the_predicate() {
    let dir = TempDir::new("prepared-introspect-predicate");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 1), (2, 7, "b", 2)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let (_, report) = prepared
        .introspect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("introspect");
    assert!(report.contains("predicate: (string, i64)"), "{report}");

    // The fold-bearing head: the column renders its producing kind.
    let count_query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: crate::ir::AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &count_query).expect("prepare");
    let (_, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert!(report.contains("predicate: (u64, Count u64)"), "{report}");
}

/// The stats surface carries the pin record — golden on one introspection
/// report: every node estimate is "estimated from (pinned rows at
/// prepare)", and a key probe (which reads no statistics) pins
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
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let (_, stats) = prepared
        .profile(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("profile");
    assert_eq!(
        stats.rules[0].pinned.len(),
        1,
        "one participating occurrence"
    );
    let pin = &stats.rules[0].pinned[0];
    assert_eq!(pin.occurrence, 0);
    assert_eq!(pin.relation, "Posting");
    assert_eq!(pin.rows, 3, "the S count read at prepare");
    assert!(
        pin.survivors.is_some(),
        "the account selection + amount range make a filtered view"
    );

    let (_, report) = prepared
        .introspect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("introspect");
    assert!(
        report.contains("estimated from (pinned rows at prepare): 3"),
        "{report}"
    );

    // A key probe classifies before statistics: nothing is pinned.
    let key_probe_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut key_probe = prepare(&txn, &cache, &schema, &key_probe_query).expect("prepare");
    let (_, stats) = key_probe
        .profile(&txn, &cache, &[BindValue::U64(1)])
        .expect("profile");
    assert!(
        stats.rules[0].pinned.is_empty(),
        "key probes read no statistics"
    );
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
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let (answers, stats) = prepared
        .profile(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("profile");
    assert_eq!(answers.len(), 2);
    assert_eq!(stats.emits, 2);
    let rule = &stats.rules[0];
    assert!(rule.key_probe.is_none());
    assert_eq!(rule.emitted, 2);
    assert_eq!(rule.absorbed, 0, "distinct rows: nothing absorbed");
    assert!(!rule.nodes.is_empty());
    let last = rule.nodes.last().expect("nodes");
    assert_eq!(last.actual, stats.emits, "last node's actual = emits");
    assert!(
        rule.nodes[0].batches >= 1 && rule.nodes[0].batch_entries >= rule.nodes[0].batches,
        "batching counters populated: {stats:?}"
    );

    // The rendered introspect is built from the same struct — spot-pin
    // the format so the golden contract holds.
    let (_, report) = prepared
        .introspect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("introspect");
    assert!(report.contains("access path: free join"), "{report}");
    assert!(report.contains("emitted bindings: 2"), "{report}");

    // KeyProbe profile: no nodes, a hit flag.
    let key_probe_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut key_probe = prepare(&txn, &cache, &schema, &key_probe_query).expect("prepare");
    let (answers, stats) = key_probe
        .profile(&txn, &cache, &[BindValue::U64(1)])
        .expect("profile");
    assert_eq!(answers.len(), 1);
    assert!(stats.rules[0].nodes.is_empty());
    assert_eq!(
        stats.rules[0].key_probe,
        Some(crate::api::stats::KeyProbeStats { hit: true })
    );
    let (_, stats) = key_probe
        .profile(&txn, &cache, &[BindValue::U64(999)])
        .expect("profile");
    assert_eq!(
        stats.rules[0].key_probe,
        Some(crate::api::stats::KeyProbeStats { hit: false })
    );
}
