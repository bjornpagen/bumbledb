#![cfg(feature = "trace")] // every test here reads obs captures

use super::*;

/// The view-memo LRU (docs/architecture/40-execution.md): four rotating residual bindings
/// all memoize; a fifth evicts exactly the least recently used.
#[test]
fn residual_bindings_memoize_under_lru() {
    use crate::obs;

    let dir = TempDir::new("prepared-lru-trace");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[
            (1, 7, "a", 10),
            (2, 7, "b", 20),
            (3, 7, "c", 30),
            (4, 7, "d", 40),
            (5, 7, "e", 50),
            (6, 7, "f", 60),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let params = |floor: i64| vec![BindValue::U64(7), BindValue::I64(floor)];
    let windows = [-100, 15, 25, 35];

    let mut run = |floor: i64| -> (usize, usize, Vec<(String, i64)>) {
        obs::start_capture();
        let out = prepared
            .execute_collect(&txn, &cache, &params(floor))
            .expect("execute");
        let events = obs::finish_capture();
        let builds = events
            .iter()
            .filter(|e| e.name == obs::names::VIEW_BUILD)
            .count();
        let hits = events
            .iter()
            .filter(|e| e.name == obs::names::VIEW_MEMO_HIT)
            .count();
        (builds, hits, rows_of(&out))
    };
    let expected = |floor: i64| -> Vec<(String, i64)> {
        let rows = [
            ("a", 10),
            ("b", 20),
            ("c", 30),
            ("d", 40),
            ("e", 50),
            ("f", 60),
        ];
        let mut expected: Vec<(String, i64)> = rows
            .iter()
            .filter(|(_, amount)| *amount >= floor)
            .map(|(memo, amount)| ((*memo).to_owned(), *amount))
            .collect();
        expected.sort_unstable();
        expected
    };

    // First cycle: every window builds once (differentially checked).
    for floor in windows {
        let (builds, _, rows) = run(floor);
        assert_eq!(builds, 1, "first sight of window {floor} builds");
        assert_eq!(rows, expected(floor));
    }
    // Second cycle: every window hits — active or parked.
    for floor in windows {
        let (builds, hits, rows) = run(floor);
        assert_eq!(builds, 0, "window {floor} memoized");
        assert_eq!(hits, 1);
        assert_eq!(rows, expected(floor));
    }
    // A fifth window evicts the least recently used (floor -100).
    let (builds, _, rows) = run(45);
    assert_eq!(builds, 1, "the fifth binding builds");
    assert_eq!(rows, expected(45));
    // The most recent of the old four still hits...
    let (builds, hits, _) = run(35);
    assert_eq!((builds, hits), (0, 1), "most recent old binding kept");
    // ...and the least recent was the eviction victim.
    let (builds, _, rows) = run(-100);
    assert_eq!(builds, 1, "least recent binding was evicted");
    assert_eq!(rows, expected(-100));
}

/// The view memo under the rule loop (docs/architecture/40-execution.md
/// § the rule loop): occurrences of one relation in different rules
/// share the image Arc by construction — one `IMAGE_BUILD` however many
/// rules read the relation — and each occurrence's filtered view
/// memoizes per (generation, resolved filters), so a repeat execution
/// of the whole program rebuilds nothing in any rule.
#[test]
fn rules_share_the_image_and_memoize_every_rules_views() {
    use crate::ir::HeadTerm;
    use crate::obs;

    let dir = TempDir::new("prepared-rules-memo");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 3, "a", 10), (2, 7, "b", 25)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Two rules over the SAME relation, each with a residual filter so
    // real filtered views exist (amount >= literal — resolved filters
    // coincide across executions).
    let rule = |account: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Literal(Value::I64(0)),
        })],
    };
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![rule(3), rule(7)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");

    // Cold: the relation's image builds ONCE (the cache shares the Arc
    // across both rules' occurrences); each occurrence builds its view.
    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let cold = obs::finish_capture();
    assert_eq!(amounts_of(&out), vec![10, 25]);
    assert_eq!(
        cold.iter()
            .filter(|e| e.name == obs::names::IMAGE_BUILD)
            .count(),
        1,
        "one image build across the rules — the Arc is shared by construction"
    );
    assert_eq!(
        cold.iter()
            .filter(|e| e.name == obs::names::VIEW_BUILD)
            .count(),
        2,
        "each rule's occurrence builds its filtered view once"
    );

    // Warm: every rule's occurrence hits its memo — no image, no view.
    obs::start_capture();
    prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let warm = obs::finish_capture();
    let warm_names: Vec<&str> = warm.iter().map(|e| e.name).collect();
    assert!(!warm_names.contains(&obs::names::IMAGE_BUILD));
    assert!(!warm_names.contains(&obs::names::VIEW_BUILD));
    assert_eq!(
        warm.iter()
            .filter(|e| e.name == obs::names::VIEW_MEMO_HIT)
            .count(),
        2,
        "both rules' views memoized"
    );
    // The RULE spans mark the loop under the execute span.
    assert!(warm_names.contains(&obs::names::RULE[0]), "{warm_names:?}");
    assert!(warm_names.contains(&obs::names::RULE[1]), "{warm_names:?}");
}

/// A generation bump invalidates every memoized binding, and the
/// rebuilt view reflects the new fact.
#[test]
fn a_generation_bump_invalidates_the_memo() {
    use crate::obs;

    let dir = TempDir::new("prepared-lru-generation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "old", 10)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let params = vec![BindValue::U64(7), BindValue::I64(0)];
    let out = prepared
        .execute_collect(&txn, &cache, &params)
        .expect("execute");
    assert_eq!(out.len(), 1);
    drop(txn);

    insert_postings(&env, &schema, &[(2, 7, "new", 20)]);
    let txn = env.read_txn().expect("txn");
    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &params)
        .expect("execute");
    let events = obs::finish_capture();
    assert!(
        events.iter().any(|e| e.name == obs::names::VIEW_BUILD),
        "the stale binding rebuilds in place"
    );
    assert_eq!(
        rows_of(&out),
        vec![("new".to_owned(), 20), ("old".to_owned(), 10)],
        "the rebuilt view carries the new fact"
    );
}

/// The read-path capture contract (feature `trace`).
#[test]
fn read_path_traces_phases_memo_hits_and_key_probe() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-read");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "rent", -1200), (2, 7, "food", -55)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let names = |events: &[obs::TraceEvent]| -> Vec<&'static str> {
        events.iter().map(|e| e.name).collect()
    };

    // Prepare: the phase spans, exactly.
    obs::start_capture();
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let events = obs::finish_capture();
    let got = names(&events);
    for expected in [
        obs::names::VALIDATE,
        obs::names::NORMALIZE,
        obs::names::CLASSIFY,
        obs::names::STATS,
        obs::names::PLAN_DP,
        obs::names::LOWER,
        obs::names::BUILD_COLTS,
        obs::names::PREPARE,
    ] {
        assert!(got.contains(&expected), "missing {expected} in {got:?}");
    }
    // Containment: every phase inside the outer prepare span.
    let outer = events
        .iter()
        .find(|e| e.name == obs::names::PREPARE)
        .expect("outer");
    for e in &events {
        assert!(e.start_ns >= outer.start_ns);
        assert!(e.start_ns + e.dur_ns <= outer.start_ns + outer.dur_ns);
    }

    // First execute: builds views, no memo hits, row count in a0.
    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("execute");
    let first = obs::finish_capture();
    assert_eq!(out.len(), 2);
    let first_names = names(&first);
    assert!(
        first_names.contains(&obs::names::VIEW_BUILD),
        "{first_names:?}"
    );
    assert!(!first_names.contains(&obs::names::VIEW_MEMO_HIT));
    let exec = first
        .iter()
        .find(|e| e.name == obs::names::EXECUTE)
        .expect("execute span");
    assert_eq!(exec.a0, 2, "execute a0 carries the row count");

    // Second execute, same snapshot + params: memo hits only.
    obs::start_capture();
    prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("execute");
    let second = obs::finish_capture();
    let second_names = names(&second);
    assert!(
        second_names.contains(&obs::names::VIEW_MEMO_HIT),
        "{second_names:?}"
    );
    assert!(!second_names.contains(&obs::names::VIEW_BUILD));
    assert!(!second_names.contains(&obs::names::IMAGE_BUILD));

    // A key-probe-shaped query: key_probe, never join.
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
    obs::start_capture();
    key_probe
        .execute_collect(&txn, &cache, &[BindValue::U64(1)])
        .expect("execute");
    let key_probe_events = obs::finish_capture();
    let key_probe_names = names(&key_probe_events);
    assert!(
        key_probe_names.contains(&obs::names::KEY_PROBE),
        "{key_probe_names:?}"
    );
    assert!(!key_probe_names.contains(&obs::names::JOIN));
    let probe = key_probe_events
        .iter()
        .find(|e| e.name == obs::names::KEY_PROBE)
        .expect("probe");
    assert_eq!(probe.a0, 1, "hit flag");

    // Nothing records without capture.
    prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("execute");
    obs::start_capture();
    assert!(obs::finish_capture().is_empty());
}

/// A closed relation's view binds at the sentinel generation
/// (`view_memo::GENERATION_CLOSED`): bind → commit → bind rebuilds
/// nothing — the image slot is never evicted, the memo binding is never
/// reaped (the sentinel is maximal), and the second execution is a pure
/// memo hit across the storage-generation advance.
#[test]
fn closed_relation_views_stay_warm_across_generations() {
    use crate::obs;

    let dir = TempDir::new("prepared-closed-memo");
    // R(x u64 fresh) drives generations; the closed Currency lives
    // outside them.
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![FieldDescriptor {
                    name: "x".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                }],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    crate::schema::Row {
                        handle: "Usd".into(),
                        values: Box::new([Value::U64(2)]),
                    },
                    crate::schema::Row {
                        handle: "Eur".into(),
                        values: Box::new([Value::U64(0)]),
                    },
                ])),
                name: "Currency".into(),
                fields: vec![FieldDescriptor {
                    name: "minor_units".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                }],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let currency = RelationId(1);
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(id, units) :- Currency(id, units) — one occurrence, no params.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: currency,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");

    let mut run = |txn: &crate::storage::env::ReadTxn<'_>| {
        obs::start_capture();
        let out = prepared.execute_collect(txn, &cache, &[]).expect("execute");
        let events = obs::finish_capture();
        let count = |name: &'static str| events.iter().filter(|e| e.name == name).count();
        (
            count(obs::names::VIEW_BUILD),
            count(obs::names::VIEW_MEMO_HIT),
            count(obs::names::IMAGE_BUILD),
            out.len(),
        )
    };

    // First execution: one image synthesis, one view build, both axioms.
    let (builds, _, image_builds, rows) = run(&txn);
    assert_eq!((builds, image_builds, rows), (1, 1, 2));
    drop(txn);

    // A state-changing commit advances the storage generation; the write
    // path evicts the cache exactly as `Db` wires it.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let mut bytes = Vec::new();
    encode_fact(
        &[ValueRef::U64(1)],
        schema.relation(RelationId(0)).layout(),
        &mut bytes,
    );
    delta.insert(&view, RelationId(0), &bytes).expect("insert");
    drop(view);
    let report = commit(delta, &env).expect("commit");
    assert!(report.changed);
    cache.evict_older_than(report.new_generation);

    // Second execution at the new generation: zero rebuilds — the memo
    // binding hits at the sentinel and the image Arc never moved.
    let txn = env.read_txn().expect("txn");
    let (builds, hits, image_builds, rows) = run(&txn);
    assert_eq!((builds, hits, image_builds, rows), (0, 1, 0, 2));
}
