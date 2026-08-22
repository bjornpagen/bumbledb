#![cfg(feature = "trace")] 

use super::*;
use crate::ir::Rec;

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
            .filter(|e| e.point() == obs::names::VIEW_BUILD)
            .count();
        let hits = events
            .iter()
            .filter(|e| e.point() == obs::names::VIEW_MEMO_HIT)
            .count();
        (builds, hits, answers_of(&out))
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

    for floor in windows {
        let (builds, _, rows) = run(floor);
        assert_eq!(builds, 1, "first sight of window {floor} builds");
        assert_eq!(rows, expected(floor));
    }

    for floor in windows {
        let (builds, hits, rows) = run(floor);
        assert_eq!(builds, 0, "window {floor} memoized");
        assert_eq!(hits, 1);
        assert_eq!(rows, expected(floor));
    }

    let (builds, _, rows) = run(45);
    assert_eq!(builds, 1, "the fifth binding builds");
    assert_eq!(rows, expected(45));

    let (builds, hits, _) = run(35);
    assert_eq!((builds, hits), (0, 1), "most recent old binding kept");

    let (builds, _, rows) = run(-100);
    assert_eq!(builds, 1, "least recent binding was evicted");
    assert_eq!(rows, expected(-100));
}

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

    let rule = |account: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
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
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![rule(3), rule(7)],
        rec: None,
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");

    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let cold = obs::finish_capture();
    assert_eq!(amounts_of(&out), vec![10, 25]);
    assert_eq!(
        cold.iter()
            .filter(|e| e.point() == obs::names::IMAGE_BUILD)
            .count(),
        1,
        "one image build across the rules — the Arc is shared by construction"
    );
    assert_eq!(
        cold.iter()
            .filter(|e| e.point() == obs::names::VIEW_BUILD)
            .count(),
        2,
        "each rule's occurrence builds its filtered view once"
    );

    obs::start_capture();
    prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let warm = obs::finish_capture();
    let warm_names: Vec<obs::TracePoint> = warm.iter().map(|e| e.point()).collect();
    assert!(!warm_names.contains(&obs::names::IMAGE_BUILD));
    assert!(!warm_names.contains(&obs::names::VIEW_BUILD));
    assert_eq!(
        warm.iter()
            .filter(|e| e.point() == obs::names::VIEW_MEMO_HIT)
            .count(),
        2,
        "both rules' views memoized"
    );

    assert!(warm_names.contains(&obs::names::RULE[0]), "{warm_names:?}");
    assert!(warm_names.contains(&obs::names::RULE[1]), "{warm_names:?}");
}

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
        events.iter().any(|e| e.point() == obs::names::VIEW_BUILD),
        "the stale binding rebuilds in place"
    );
    assert_eq!(
        answers_of(&out),
        vec![("new".to_owned(), 20), ("old".to_owned(), 10)],
        "the rebuilt view carries the new fact"
    );
}

#[test]
fn read_path_traces_phases_memo_hits_and_key_probe() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-read");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "rent", -1200), (2, 7, "food", -55)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let names = |events: &[obs::TraceEvent]| -> Vec<obs::TracePoint> {
        events.iter().map(|e| e.point()).collect()
    };

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

    let outer = events
        .iter()
        .find(|e| e.point() == obs::names::PREPARE)
        .expect("outer");
    for e in &events {
        assert!(e.start_ns() >= outer.start_ns());
        assert!(e.start_ns() + e.dur_ns() <= outer.start_ns() + outer.dur_ns());
    }

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
        .find(|e| e.point() == obs::names::EXECUTE)
        .expect("execute span");
    assert_eq!(exec.a0(), 2, "execute a0 carries the row count");

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

    let key_probe_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
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
        .find(|e| e.point() == obs::names::KEY_PROBE)
        .expect("probe");
    assert_eq!(probe.a0(), 1, "hit flag");

    prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100_000)])
        .expect("execute");
    obs::start_capture();
    assert!(obs::finish_capture().is_empty());
}

#[test]
fn closed_relation_views_stay_warm_across_generations() {
    use crate::obs;

    let dir = TempDir::new("prepared-closed-memo");

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
                    bumbledb_theory::schema::Row {
                        handle: "Usd".into(),
                        values: Box::new([Value::U64(2)]),
                    },
                    bumbledb_theory::schema::Row {
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

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(currency),
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
        let out = prepared
            .execute_collect(txn, &cache, &[] as &[BindValue])
            .expect("execute");
        let events = obs::finish_capture();
        let count = |name| events.iter().filter(|e| e.point() == name).count();
        (
            count(obs::names::VIEW_BUILD),
            count(obs::names::VIEW_MEMO_HIT),
            count(obs::names::IMAGE_BUILD),
            out.len(),
        )
    };

    let (builds, _, image_builds, rows) = run(&txn);
    assert_eq!((builds, image_builds, rows), (1, 1, 2));
    drop(txn);

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
    let report = commit(delta, &env).expect("commit").expect("admitted");
    assert!(report.changed());
    cache.advance(report.generation(), &[RelationId(0)], &[]);

    let txn = env.read_txn().expect("txn");
    let (builds, hits, image_builds, rows) = run(&txn);
    assert_eq!((builds, hits, image_builds, rows), (0, 1, 0, 2));
}

#[test]
fn prepare_lights_the_planner_dp_and_selectivity_ladder() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-planner");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");

    insert_postings(
        &env,
        &schema,
        &[
            (1, 7, "a", 100),
            (2, 7, "b", -50),
            (3, 9, "c", 200),
            (4, 7, "d", 50),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    obs::start_capture();
    let _prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let events = obs::finish_capture();

    let one = |name: obs::TracePoint| -> &obs::TraceEvent {
        let hits: Vec<&obs::TraceEvent> = events.iter().filter(|e| e.point() == name).collect();
        assert_eq!(hits.len(), 1, "exactly one {name}");
        hits[0]
    };
    let within = |inner: &obs::TraceEvent, outer: &obs::TraceEvent| {
        assert!(
            inner.start_ns() >= outer.start_ns()
                && inner.start_ns() + inner.dur_ns() <= outer.start_ns() + outer.dur_ns(),
            "{} nests inside {}",
            inner.name(),
            outer.name(),
        );
    };

    let plan_dp = one(obs::names::PLAN_DP);
    let densify = one(obs::names::PLAN_DENSIFY);
    let fill = one(obs::names::PLAN_FILL);
    assert_eq!(densify.a0(), 1, "one participating occurrence densified");
    assert_eq!(
        (fill.a0(), fill.a1()),
        (0, 0),
        "trivial DP evaluates no candidate"
    );
    within(densify, plan_dp);
    within(fill, plan_dp);

    let stats = one(obs::names::STATS);
    let rows = one(obs::names::RELATION_ROWS);
    assert_eq!(
        (rows.a0(), rows.a1()),
        (u64::from(POSTING.0), 4),
        "Posting's stored rows"
    );
    within(rows, stats);
    let ladder: Vec<&obs::TraceEvent> = events
        .iter()
        .filter(|e| e.point() == obs::names::DISTINCT_LADDER)
        .collect();
    assert!(
        !ladder.is_empty(),
        "the distinct ladder resolved at least once"
    );
    for rung in &ladder {
        assert_eq!(
            rung.a0(),
            3,
            "the floor rung — cold prepare, no key/image/containment"
        );
        within(rung, stats);
    }

    let prepare_span = one(obs::names::PREPARE);
    for e in &events {
        within(e, prepare_span);
    }
}

#[test]
fn prepare_lights_the_normalization_sub_passes() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-normalize");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 100)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    obs::start_capture();
    let _prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let events = obs::finish_capture();

    let outer = events
        .iter()
        .find(|e| e.point() == obs::names::NORMALIZE)
        .expect("the NORMALIZE span");
    for name in [obs::names::PLACE_COMPARISONS, obs::names::NORMALIZE_FOLD] {
        let sub = events
            .iter()
            .find(|e| e.point() == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        assert!(
            sub.start_ns() >= outer.start_ns()
                && sub.start_ns() + sub.dur_ns() <= outer.start_ns() + outer.dur_ns(),
            "{name} nests inside NORMALIZE",
        );
    }

    let fold = events
        .iter()
        .find(|e| e.point() == obs::names::NORMALIZE_FOLD)
        .expect("fold span");
    assert_eq!(fold.a0(), 0, "the rule is not statically empty");
}

#[test]
fn prepare_lights_the_validation_interior() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-validate");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 100)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    obs::start_capture();
    let _prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let events = obs::finish_capture();

    let outer = events
        .iter()
        .find(|e| e.point() == obs::names::VALIDATE)
        .expect("the VALIDATE span");
    let one = |name: obs::TracePoint| -> &obs::TraceEvent {
        let hits: Vec<&obs::TraceEvent> = events.iter().filter(|e| e.point() == name).collect();
        assert_eq!(hits.len(), 1, "exactly one {name}");
        hits[0]
    };
    for name in [obs::names::VALIDATE_LOWER, obs::names::VALIDATE_RULES] {
        let sub = one(name);
        assert_eq!(sub.a0(), 1, "{name}: the one-rule query's rule work");
        assert!(
            sub.start_ns() >= outer.start_ns()
                && sub.start_ns() + sub.dur_ns() <= outer.start_ns() + outer.dur_ns(),
            "{name} nests inside VALIDATE",
        );
    }

    assert!(
        events.iter().all(|e| e.name() != "validate_stratify"),
        "the stratify span must not appear",
    );
    let seal = events
        .iter()
        .find(|e| e.point() == obs::names::VALIDATE_SEAL)
        .expect("VALIDATE_SEAL runs over interior count");
    assert_eq!(seal.a0(), 0, "no interiors");
}

#[test]
fn rec_prepare_lights_sealing_and_the_rule_passes() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-validate-rec");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 100)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            base: crate::ir::NonEmpty::one(crate::ir::RecRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(POSTING),
                    bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
                }],
                conditions: vec![],
            }),
            rec: crate::ir::NonEmpty::one(crate::ir::RecStep {
                finds: vec![VarId(0)],
                self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                atoms: vec![],
                conditions: vec![],
            }),
        }),
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
    };

    obs::start_capture();
    let _prepared: PreparedQuery<()> =
        super::prepare(&txn, &cache, &schema, &query).expect("prepare");
    let events = obs::finish_capture();

    let outer = events
        .iter()
        .find(|e| e.point() == obs::names::VALIDATE)
        .expect("VALIDATE span");
    let one = |name: obs::TracePoint| -> &obs::TraceEvent {
        let hits: Vec<&obs::TraceEvent> = events.iter().filter(|e| e.point() == name).collect();
        assert_eq!(hits.len(), 1, "exactly one {name}");
        hits[0]
    };
    let within = |inner: &obs::TraceEvent| {
        assert!(
            inner.start_ns() >= outer.start_ns()
                && inner.start_ns() + inner.dur_ns() <= outer.start_ns() + outer.dur_ns(),
            "{} nests inside VALIDATE",
            inner.name(),
        );
    };

    let seal = one(obs::names::VALIDATE_SEAL);
    within(seal);
    assert!(
        events.iter().all(|e| e.name() != "validate_stratify"),
        "the stratify span must not appear",
    );
}

#[test]
fn execute_lights_the_batch_decode_and_filter_kernel() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-kernel");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[
            (1, 7, "a", 100),
            (2, 7, "b", -50),
            (3, 9, "c", 200),
            (4, 7, "d", 50),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");

    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("execute");
    let events = obs::finish_capture();

    assert_eq!(out.len(), 2);

    let image_build = events
        .iter()
        .find(|e| e.point() == obs::names::IMAGE_BUILD)
        .expect("the cold build");
    let decode = events
        .iter()
        .find(|e| e.point() == obs::names::DECODE_BATCH)
        .expect("the batch decode");
    assert_eq!(decode.a0(), 4, "every stored row decoded");
    assert!(
        decode.start_ns() >= image_build.start_ns()
            && decode.start_ns() + decode.dur_ns() <= image_build.start_ns() + image_build.dur_ns(),
        "the batch decode nests inside IMAGE_BUILD",
    );

    let full_sweep = events
        .iter()
        .filter(|e| e.point() == obs::names::KERNEL_FILTER)
        .any(|e| e.a0() == 4);
    assert!(full_sweep, "a fixed-width kernel scanned all four lanes");
}

#[test]
fn same_shaped_occurrences_dedup_the_cold_rebuild() {
    use crate::obs;

    let dir = TempDir::new("prepared-dedup-trace");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 7, "a", 10), (2, 7, "b", 20), (3, 8, "c", 30)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &memo_star_query()).expect("prepare");

    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let events = obs::finish_capture();

    let builds = events
        .iter()
        .filter(|e| e.point() == obs::names::VIEW_BUILD)
        .count();
    let dedups = events
        .iter()
        .filter(|e| e.point() == obs::names::VIEW_DEDUP)
        .count();
    assert!(dedups >= 1, "at least one sibling clones the bound state");
    assert_eq!(builds + dedups, 3, "every occurrence binds exactly once");
    let selections = events
        .iter()
        .find(|e| e.point() == obs::names::SELECTIONS)
        .expect("the batched selection-probe span");
    assert_eq!(selections.a1(), 1, "no probe short-circuited");

    let mut triples: Vec<(String, String, String)> = (0..out.len())
        .map(|answer| {
            let (AnswerValue::String(m1), AnswerValue::String(m2), AnswerValue::String(m3)) =
                (out.get(answer, 0), out.get(answer, 1), out.get(answer, 2))
            else {
                panic!("star columns are strings");
            };
            (m1.to_owned(), m2.to_owned(), m3.to_owned())
        })
        .collect();
    triples.sort();

    let mut expected: Vec<(String, String, String)> = Vec::new();
    for m1 in ["a", "b"] {
        for m2 in ["a", "b"] {
            for m3 in ["a", "b"] {
                expected.push((m1.to_owned(), m2.to_owned(), m3.to_owned()));
            }
        }
    }
    expected.push(("c".into(), "c".into(), "c".into()));
    expected.sort();
    assert_eq!(triples, expected, "the shared-account memo triples");
}

fn memo_star_query() -> Query {
    let posting = |memo: u16| Atom {
        source: crate::ir::AtomSource::Edb(POSTING),
        bindings: vec![
            (FieldId(1), Term::Var(VarId(0))),
            (FieldId(2), Term::Var(VarId(memo))),
        ],
    };
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
            FindTerm::Var(VarId(3)),
        ],
        atoms: vec![posting(1), posting(2), posting(3)],
        negated: vec![],
        conditions: vec![],
    })
}
