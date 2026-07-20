use bumbledb::Theory as _;
use bumbledb::schema::{Generation, StatementDescriptor, ValueType};
use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::Scale;
use crate::differential::{self, Op};
use crate::duralane::{self, DurabilityLane};
use crate::harness::{Measurement, Protocol};
use crate::json::Value as Json;
use crate::naive::{Delta, NaiveDb, Violation as Cited};
use crate::poststate;

use super::lanes::{self, LawCursor};
use super::{
    Attempt, LawAttemptId, LawSizes, LawSteerId, LawTaskId, LawfulWorld, SteerScope, corpus,
    enforcement, ids,
};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-lawful-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The declared world seals with every statement family the mandate
/// names, by count: 4 declared keys + 3 fresh auto-keys (+ 3 closed
/// auto-keys), 7 containments exactly one of which selects its target
/// (the ψ-selected steer-scope law), 1 cardinality window ({0..8}),
/// and a payload-bearing closed vocabulary (`Outcome.terminal`). The id
/// constants pin declaration order.
#[test]
fn the_lawful_schema_validates_and_carries_every_statement_family() {
    let schema = super::schema();
    for (rel, name) in [
        (ids::TASK, "Task"),
        (ids::ATTEMPT, "Attempt"),
        (ids::VERDICT, "Verdict"),
        (ids::STEER, "Steer"),
        (ids::STEER_SCOPE, "SteerScope"),
        (ids::TASK_KINDS, "TaskKinds"),
        (ids::STEER_KINDS, "SteerKinds"),
        (ids::OUTCOME, "Outcome"),
    ] {
        assert_eq!(schema.relation(rel).name(), name, "declaration order");
    }

    let descriptor = LawfulWorld.descriptor();
    let declared_keys = descriptor
        .statements
        .iter()
        .filter(|statement| matches!(statement, StatementDescriptor::Functionality { .. }))
        .count();
    assert_eq!(declared_keys, 4, "the four declared keys");
    let fresh_autos = descriptor
        .relations
        .iter()
        .flat_map(|relation| &relation.fields)
        .filter(|field| field.generation == Generation::Fresh)
        .count();
    assert_eq!(fresh_autos, 3, "Task.id, Attempt.id, Steer.id");
    let closed_autos = descriptor
        .relations
        .iter()
        .filter(|relation| relation.extension.is_some())
        .count();
    assert_eq!(closed_autos, 3, "the three closed vocabularies");
    assert_eq!(
        schema.keys().len(),
        declared_keys + fresh_autos + closed_autos,
        "every key family materialized"
    );

    assert_eq!(schema.containments().len(), 7, "the seven containments");
    let selected = schema
        .containments()
        .iter()
        .filter(|containment| !containment.target.selection.is_empty())
        .count();
    assert_eq!(selected, 1, "exactly one ψ-selected containment");

    assert_eq!(schema.windows().len(), 1, "the one cardinality window");
    let window = &schema.windows()[0];
    assert_eq!((window.lo, window.hi), (0, Some(8)), "the {{0..8}} window");

    assert!(
        schema
            .relation(ids::OUTCOME)
            .fields()
            .iter()
            .any(|field| &*field.name == "terminal" && matches!(field.value_type, ValueType::Bool)),
        "Outcome carries the terminal payload column"
    );
}

/// The enforcement map is TOTAL over the materialized statement list —
/// an engine law without a `SQLite` row is a failing count here, never
/// a silent parity gap — and every notation is unique (the map's key).
#[test]
fn the_enforcement_map_is_total_over_the_materialized_statements() {
    let materialized = LawfulWorld.descriptor().materialized_statements();
    assert_eq!(
        enforcement::MAP.len(),
        materialized.len(),
        "one enforcement row per materialized statement"
    );
    let notations: std::collections::BTreeSet<&str> =
        enforcement::MAP.iter().map(|row| row.notation).collect();
    assert_eq!(
        notations.len(),
        enforcement::MAP.len(),
        "notations are unique"
    );
}

/// Both durability lanes load value-identical twins at `Tiny`, judged
/// by the shared post-state comparator over all five ordinary
/// relations (Verdict included — empty on both sides).
#[test]
fn the_lawful_twins_load_value_identical_at_tiny() {
    let sizes = LawSizes::of(Scale::Tiny);
    for lane in duralane::ALL {
        let dir = scratch(&format!("twin-{}", lane.label()));
        let (db, conn) = super::load::load_stores(&dir, 7, sizes, lane).unwrap_or_else(|e| {
            panic!("{}: {e}", lane.label());
        });
        for (rel, expected) in [
            (ids::TASK, sizes.tasks),
            (ids::ATTEMPT, sizes.tasks * sizes.attempts_per_task),
            (ids::VERDICT, 0),
            (ids::STEER, sizes.steers),
            (ids::STEER_SCOPE, sizes.steers / 2),
        ] {
            let name = super::schema().relation(rel).name();
            let ours = poststate::engine_rows(&db, rel).expect("engine rows");
            let theirs =
                poststate::sqlite_rows(&conn, super::schema().relation(rel)).expect("mirror rows");
            assert_eq!(ours.len() as u64, expected, "{name}: engine row count");
            assert_eq!(theirs.len() as u64, expected, "{name}: mirror row count");
            poststate::assert_identical("lawful", name, ours, theirs).expect(name);
        }
        drop((db, conn));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Naive parity — the semantic oracle for the full law roster: both
/// oracles preloaded with the Tiny corpus (outside the runner, so the
/// summary counts the judged ops alone), then two legal writes (one
/// single-insert, one 4-row cluster continuing the fresh sequences) and
/// the four violating deltas — a duplicate `(task, n)` key, an absent
/// task reference, an over-cap attempt burst, and a scope under an
/// Observe steer — verdicts, statement ids, and directions compared
/// whole by the differential runner.
#[test]
fn the_lawful_verdicts_agree_with_the_naive_model() {
    let dir = scratch("naive");
    let sizes = LawSizes::of(Scale::Tiny);
    let db = Db::create(&dir, LawfulWorld).expect("create");
    let mut naive = NaiveDb::new(&LawfulWorld.descriptor());

    let mut seed = Delta::default();
    for rel in [ids::TASK, ids::STEER, ids::ATTEMPT, ids::STEER_SCOPE] {
        db.bulk_load_dyn(rel, corpus::relation_rows(sizes, rel))
            .expect("engine corpus");
        for row in corpus::relation_rows(sizes, rel) {
            seed.inserts.push((rel, row));
        }
    }
    naive.apply(&seed).expect("the corpus is legal");

    let write = |inserts: Vec<(RelationId, Vec<Value>)>| {
        Op::Write(Delta {
            deletes: vec![],
            inserts,
        })
    };
    // The seeded fresh frontiers: attempt ids 0..attempts, steer ids
    // 0..steers.
    let attempts = sizes.tasks * sizes.attempts_per_task;
    let ops = vec![
        // One legal attempt on task 1 (n = 2, above the seeded {0, 1}).
        write(vec![(
            ids::ATTEMPT,
            vec![Value::U64(attempts), Value::U64(1), Value::U64(2)],
        )]),
        // One legal 4-row cluster, ids continuing both fresh sequences:
        // an attempt, its verdict (Accepted), a Repartition steer, and
        // its scope — judged as one final state on both oracles.
        write(vec![
            (
                ids::ATTEMPT,
                vec![Value::U64(attempts + 1), Value::U64(2), Value::U64(2)],
            ),
            (ids::VERDICT, vec![Value::U64(attempts + 1), Value::U64(1)]),
            (
                ids::STEER,
                vec![Value::U64(sizes.steers), Value::U64(1), Value::U64(3)],
            ),
            (
                ids::STEER_SCOPE,
                vec![Value::U64(sizes.steers), Value::U64(0)],
            ),
        ]),
        // The duplicate (task, n) key: the first op's determinant under
        // a new id — MUST abort on both.
        write(vec![(
            ids::ATTEMPT,
            vec![Value::U64(attempts + 2), Value::U64(1), Value::U64(2)],
        )]),
        // An attempt under an absent task id — MUST abort on both.
        write(vec![(
            ids::ATTEMPT,
            vec![
                Value::U64(attempts + 3),
                Value::U64(sizes.tasks + 9),
                Value::U64(0),
            ],
        )]),
        // The window trip: 7 more attempts on task 5 blow the {0..8}
        // cap (2 seeded + 7 = 9 > 8) — MUST abort on both.
        write(
            (0..7)
                .map(|k| {
                    (
                        ids::ATTEMPT,
                        vec![
                            Value::U64(attempts + 4 + k),
                            Value::U64(5),
                            Value::U64(2 + k),
                        ],
                    )
                })
                .collect(),
        ),
        // A scope under steer 0 (Observe): the ψ selection rejects the
        // target — MUST abort on both.
        write(vec![(ids::STEER_SCOPE, vec![Value::U64(0), Value::U64(5)])]),
    ];

    let summary = differential::run(&db, &mut naive, &ops).expect("verdict parity");
    assert_eq!(summary.commits, 2, "the single insert and the cluster");
    assert_eq!(
        summary.aborts, 4,
        "the key, the containment, the window, and the ψ selection"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The tiny per-family protocol: 1 warmup + 2 measured samples = 3
/// closure invocations (the crud test-protocol precedent).
const TINY_PROTO: Protocol = Protocol {
    warmups: 1,
    samples: 2,
};

/// Total closure invocations under [`TINY_PROTO`].
const COUNT: usize = 3;

/// The post-state judgment over all five ordinary relations — the one
/// fold every lawful write test ends on (Verdict included: the cluster
/// family is its only writer).
fn assert_twins_identical(db: &Db<LawfulWorld>, conn: &rusqlite::Connection) {
    for rel in [
        ids::TASK,
        ids::ATTEMPT,
        ids::VERDICT,
        ids::STEER,
        ids::STEER_SCOPE,
    ] {
        let name = super::schema().relation(rel).name();
        let ours = poststate::engine_rows(db, rel).expect("engine rows");
        let theirs =
            poststate::sqlite_rows(conn, super::schema().relation(rel)).expect("mirror rows");
        poststate::assert_identical("lawful", name, ours, theirs).expect(name);
    }
}

/// Both LEGAL commit families run their engine runner then their
/// `SQLite` runner over the ONE shared op stream (sliced in registry
/// order, per-task n counters continuing across the boundary) on a
/// Durable twin pair, and the twins end value-identical on all five
/// ordinary relations — the representation verdict's proof. Each
/// measurement's work is the family's rows-per-sample × samples.
#[test]
fn every_lawful_commit_family_leaves_the_twins_value_identical() {
    let sizes = LawSizes::of(Scale::Tiny);
    let dir = scratch("legal-families");
    let (db, conn) =
        super::load::load_stores(&dir, 7, sizes, DurabilityLane::Durable).expect("load");
    let mut ours_cursor = LawCursor::at_base(sizes);
    let mut theirs_cursor = LawCursor::at_base(sizes);
    let stream = lanes::attempt_ops(sizes, COUNT * 2);
    let (attempt_stream, cluster_stream) = stream.split_at(COUNT);

    let ours = lanes::commit_attempt_engine(&db, TINY_PROTO, attempt_stream, &mut ours_cursor)
        .expect("attempt engine");
    let theirs =
        lanes::commit_attempt_sqlite(&conn, TINY_PROTO, attempt_stream, &mut theirs_cursor)
            .expect("attempt sqlite");
    assert_eq!(ours.work, 2, "attempt: one row per measured sample");
    assert_eq!(theirs.work, 2, "attempt: mirror work");

    let ours = lanes::commit_cluster_engine(&db, TINY_PROTO, cluster_stream, &mut ours_cursor)
        .expect("cluster engine");
    let theirs =
        lanes::commit_cluster_sqlite(&conn, TINY_PROTO, cluster_stream, &mut theirs_cursor)
            .expect("cluster sqlite");
    assert_eq!(ours.work, 8, "cluster: four rows per measured sample");
    assert_eq!(theirs.work, 8, "cluster: mirror work");
    assert_eq!(ours_cursor, theirs_cursor, "the cursors end in lockstep");

    assert_twins_identical(&db, &conn);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every rejection lane refuses on BOTH engines and commits NOTHING:
/// after the untimed window setup, each of the four rejection families
/// runs both runners to completion (the engine runner completing `Ok`
/// IS the proof its closure observed `Error::CommitRejected` with the
/// expected citation — anything else aborts it), the engine generation
/// stands still, and every mirror table's `COUNT(*)` is unchanged.
#[test]
fn every_rejection_lane_refuses_on_both_engines_and_commits_nothing() {
    let sizes = LawSizes::of(Scale::Tiny);
    let dir = scratch("rejections");
    let (db, conn) =
        super::load::load_stores(&dir, 7, sizes, DurabilityLane::Durable).expect("load");
    let mut ours_cursor = LawCursor::at_base(sizes);
    let mut theirs_cursor = LawCursor::at_base(sizes);
    lanes::fill_window_target_engine(&db, sizes, &mut ours_cursor).expect("window setup engine");
    lanes::fill_window_target_sqlite(&conn, sizes, &mut theirs_cursor)
        .expect("window setup sqlite");
    assert_eq!(ours_cursor, theirs_cursor, "the setup keeps lockstep");

    let counts = |conn: &rusqlite::Connection| -> Vec<i64> {
        ["Task", "Attempt", "Verdict", "Steer", "SteerScope"]
            .iter()
            .map(|table| {
                conn.query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |row| {
                    row.get(0)
                })
                .expect("count")
            })
            .collect()
    };
    let assert_refuses =
        |name: &str,
         engine: &dyn Fn() -> Result<Measurement, String>,
         sqlite: &dyn Fn() -> Result<Measurement, String>| {
            let generation = db.generation().expect("generation");
            let before = counts(&conn);
            let ours = engine().unwrap_or_else(|e| panic!("{name} engine: {e}"));
            let theirs = sqlite().unwrap_or_else(|e| panic!("{name} sqlite: {e}"));
            assert_eq!(ours.work, 2, "{name}: one refusal per measured sample");
            assert_eq!(theirs.work, 2, "{name}: mirror work");
            assert_eq!(
                db.generation().expect("generation"),
                generation,
                "{name}: a refused commit must move nothing on the engine"
            );
            assert_eq!(
                counts(&conn),
                before,
                "{name}: a refused insert must land nothing on the mirror"
            );
        };

    assert_refuses(
        "law_reject_key",
        &|| lanes::reject_key_engine(&db, TINY_PROTO),
        &|| lanes::reject_key_sqlite(&conn, TINY_PROTO),
    );
    assert_refuses(
        "law_reject_containment",
        &|| lanes::reject_containment_engine(&db, TINY_PROTO, sizes),
        &|| lanes::reject_containment_sqlite(&conn, TINY_PROTO, sizes),
    );
    assert_refuses(
        "law_reject_window",
        &|| lanes::reject_window_engine(&db, TINY_PROTO),
        &|| lanes::reject_window_sqlite(&conn, TINY_PROTO),
    );
    assert_refuses(
        "law_reject_scope",
        &|| lanes::reject_scope_engine(&db, TINY_PROTO),
        &|| lanes::reject_scope_sqlite(&conn, TINY_PROTO),
    );

    assert_twins_identical(&db, &conn);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The four rejection shapes cite the expected violation kinds — one
/// direct `db.write` per shape, the sealed sets normalized through
/// [`differential::cited`]: Functionality for the duplicate key,
/// Containment for the absent task, Cardinality for the over-cap
/// window, and Containment ON THE ψ STATEMENT for the Observe-steer
/// scope.
#[test]
fn the_rejection_shapes_cite_the_expected_violation_kinds() {
    let dir = scratch("citations");
    let sizes = LawSizes::of(Scale::Tiny);
    let db = Db::create(&dir, LawfulWorld).expect("create");
    for rel in [ids::TASK, ids::STEER, ids::ATTEMPT, ids::STEER_SCOPE] {
        db.bulk_load_dyn(rel, corpus::relation_rows(sizes, rel))
            .expect("corpus");
    }
    let mut cursor = LawCursor::at_base(sizes);
    lanes::fill_window_target_engine(&db, sizes, &mut cursor).expect("window setup");

    let rejected = |what: &str,
                    violate: &dyn Fn(
        &mut bumbledb::WriteTx<'_, LawfulWorld>,
    ) -> bumbledb::Result<()>|
     -> Vec<Cited> {
        match db.write(|tx| violate(tx)) {
            Err(bumbledb::Error::CommitRejected { violations }) => differential::cited(&violations),
            Ok(()) => panic!("{what}: the violating commit was accepted"),
            Err(other) => panic!("{what}: expected CommitRejected, the engine said {other:?}"),
        }
    };

    let base = lanes::REJECT_ID_BASE;
    let cited = rejected("duplicate (task, n) key", &|tx| {
        tx.insert(&Attempt {
            id: LawAttemptId(base),
            task: LawTaskId(1),
            n: 0,
        })
        .map(|_| ())
    });
    assert!(
        cited
            .iter()
            .any(|v| matches!(v, Cited::Functionality { .. })),
        "expected a Functionality citation: {cited:?}"
    );

    let cited = rejected("absent task reference", &|tx| {
        tx.insert(&Attempt {
            id: LawAttemptId(base + 1),
            task: LawTaskId(sizes.tasks + 1_000_000),
            n: 0,
        })
        .map(|_| ())
    });
    assert!(
        cited.iter().any(|v| matches!(v, Cited::Containment { .. })),
        "expected a Containment citation: {cited:?}"
    );

    let cited = rejected("over-cap attempt on task 0", &|tx| {
        tx.insert(&Attempt {
            id: LawAttemptId(base + 2),
            task: LawTaskId(0),
            n: lanes::WINDOW_CAP,
        })
        .map(|_| ())
    });
    assert!(
        cited.iter().any(|v| matches!(v, Cited::Cardinality { .. })),
        "expected a Cardinality citation: {cited:?}"
    );

    let cited = rejected("scope under an Observe steer", &|tx| {
        tx.insert(&SteerScope {
            steer: LawSteerId(0),
            grp: 0,
        })
        .map(|_| ())
    });
    assert!(
        cited
            .iter()
            .any(|v| matches!(v, Cited::Containment { statement, .. }
            if *statement == lanes::psi_statement())),
        "expected a Containment citation on the ψ statement: {cited:?}"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The full tiny run renders both artifacts: the markdown carries the
/// enforcement map, both lane sections, and all six family names; the
/// JSON parses with one enforcement row per map entry, one lane row per
/// (family × lane), and the certified post-state claim.
#[test]
fn the_full_lawful_run_renders_the_enforcement_map_and_both_lanes() {
    let dir = scratch("full-run");
    let (markdown, json) = super::run::run_with(&dir, 7, LawSizes::of(Scale::Tiny), Some(2), None)
        .expect("the tiny lawful run");
    assert!(markdown.contains("enforcement map"), "{markdown}");
    for lane in duralane::ALL {
        assert!(
            markdown.contains(&format!("## lane `{}`", lane.label())),
            "missing the {} lane section:\n{markdown}",
            lane.label()
        );
    }
    for family in super::families() {
        assert!(
            markdown.contains(family.name),
            "missing {}:\n{markdown}",
            family.name
        );
    }
    let parsed = crate::json::parse(&json).expect("valid JSON");
    assert_eq!(parsed.get("world").and_then(Json::as_str), Some("lawful"));
    assert_eq!(parsed.get("seed").and_then(Json::as_f64), Some(7.0));
    let enforcement_rows = parsed
        .get("enforcement")
        .and_then(Json::as_arr)
        .expect("enforcement");
    assert_eq!(enforcement_rows.len(), enforcement::MAP.len());
    let lane_rows = parsed.get("lanes").and_then(Json::as_arr).expect("lanes");
    assert_eq!(
        lane_rows.len(),
        super::families().len() * duralane::ALL.len(),
        "one row per (family × lane)"
    );
    assert_eq!(parsed.get("poststate").and_then(Json::as_str), Some("ok"));
    let _ = std::fs::remove_dir_all(&dir);
}
