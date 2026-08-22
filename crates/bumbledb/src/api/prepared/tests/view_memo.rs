#![cfg(feature = "trace")] // every test here reads obs captures

use super::*;
use crate::ir::Rec;

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
/// of the whole query rebuilds nothing in any rule.
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

    // Cold: the relation's image builds ONCE (the cache shares the Arc
    // across both rules' occurrences); each occurrence builds its view.
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

    // Warm: every rule's occurrence hits its memo — no image, no view.
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
        events.iter().any(|e| e.point() == obs::names::VIEW_BUILD),
        "the stale binding rebuilds in place"
    );
    assert_eq!(
        answers_of(&out),
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

    let names = |events: &[obs::TraceEvent]| -> Vec<obs::TracePoint> {
        events.iter().map(|e| e.point()).collect()
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
        .find(|e| e.point() == obs::names::PREPARE)
        .expect("outer");
    for e in &events {
        assert!(e.start_ns() >= outer.start_ns());
        assert!(e.start_ns() + e.dur_ns() <= outer.start_ns() + outer.dur_ns());
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
        .find(|e| e.point() == obs::names::EXECUTE)
        .expect("execute span");
    assert_eq!(exec.a0(), 2, "execute a0 carries the row count");

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

    // Q(id, units) :- Currency(id, units) — one occurrence, no params.
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

    // First execution: one image synthesis, one view build, both axioms.
    let (builds, _, image_builds, rows) = run(&txn);
    assert_eq!((builds, image_builds, rows), (1, 1, 2));
    drop(txn);

    // A state-changing commit advances the storage generation; evict
    // everything — the harshest commit hook (the lineage-disabled twin
    // of the `advance` `Db` wires), which still cannot touch a closed
    // slot.
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

    // Second execution at the new generation: zero rebuilds — the memo
    // binding hits at the sentinel and the image Arc never moved.
    let txn = env.read_txn().expect("txn");
    let (builds, hits, image_builds, rows) = run(&txn);
    assert_eq!((builds, hits, image_builds, rows), (0, 1, 0, 2));
}

/// Lane I2 — the planner and its selectivity ladder, formerly dark under
/// the single `PLAN_DP`/`STATS` spans: the DP interior records one
/// densify span and one table-fill span carrying the counted candidate
/// work (never a span per candidate), and every planner row-count read
/// and distinct-ladder resolution surfaces as a point event. Presence
/// AND containment: the DP spans nest inside `PLAN_DP`, the ladder events
/// inside `STATS`, all inside the outer `PREPARE`.
#[test]
fn prepare_lights_the_planner_dp_and_selectivity_ladder() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-planner");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Four rows, three under account 7 — the row count the reads pin.
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

    // The DP interior, under PLAN_DP. A single-atom query has one
    // participating occurrence and a trivial DP — no popcount-≥2
    // subproblem, so the fill pass honestly counts zero candidate work.
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

    // The selectivity reads, under STATS. One planner row-count read of
    // Posting pins the four stored rows; the distinct ladder resolves the
    // floor rung (no key, no resident image at prepare, no containment)
    // for every field it touches.
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

    // Everything under the outer PREPARE span.
    let prepare_span = one(obs::names::PREPARE);
    for e in &events {
        within(e, prepare_span);
    }
}

/// Lane I2 — the normalization sub-passes, formerly dark under the single
/// `NORMALIZE` span: comparison placement and the statically-empty
/// constant fold each record their own span, nested inside `NORMALIZE`.
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
    // A live single-atom rule folds to nothing dead.
    let fold = events
        .iter()
        .find(|e| e.point() == obs::names::NORMALIZE_FOLD)
        .expect("fold span");
    assert_eq!(fold.a0(), 0, "the rule is not statically empty");
}

/// Lane I2 — validation's interior, formerly dark under the single
/// `VALIDATE` span: the query path records one rule-set lowering span
/// and one strict per-rule pass span, both nested inside `VALIDATE`,
/// each charged its rule work; the query-only passes never fire here.
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
    // The stratify span is gone. SEAL still runs (declaration-order
    // interior sealing) even when interiors are empty.
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

/// Lane I2 — a rec query records declaration-order sealing beside the
/// lowering and strict-rule passes, all nested inside VALIDATE.
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

/// Lane I2 — the columnar batch decode (formerly invisible inside
/// `IMAGE_BUILD`) and the predicate-scan filter kernels (attributable
/// only as a phase bucket before). The first execution builds Posting's
/// image: one `DECODE_BATCH` over all four rows, nested inside
/// `IMAGE_BUILD`; the view build then runs the fixed-width scan kernel
/// over the whole `account` column — one `KERNEL_FILTER` at batch
/// granularity, its lane count the row count, never a per-lane event.
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
    // account 7 AND amount >= 0: rows a(100) and d(50).
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

    // At least one kernel scan swept the whole column — lanes = rows.
    let full_sweep = events
        .iter()
        .filter(|e| e.point() == obs::names::KERNEL_FILTER)
        .any(|e| e.a0() == 4);
    assert!(full_sweep, "a fixed-width kernel scanned all four lanes");
}

/// The occurrence dedup (docs/architecture/40-execution.md): a
/// self-join whose plan orients two same-relation occurrences
/// identically rebuilds ONE view — every same-shaped sibling clones the
/// canonical's bound state (view and forced root, `view_dedup`) instead
/// of re-scanning the image and re-forcing the same trie — and the
/// star's answers come out exactly right.
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
    // Account 7 holds memos {a, b} (2³ triples), account 8 holds {c}.
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

/// Q(m1, m2, m3) :- Posting(account = x, memo = m1), Posting(account =
/// x, memo = m2), Posting(account = x, memo = m3) — the star self-join
/// the occurrence dedup serves: the plan hangs the second and third
/// occurrences off the shared `x` node with identical orientation, so
/// their views AND forced tries coincide.
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
