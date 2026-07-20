use bumbledb::Theory as _;
use bumbledb::schema::{Generation, StatementDescriptor, ValueType};
use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::Scale;
use crate::differential::{self, Op};
use crate::duralane;
use crate::naive::{Delta, NaiveDb};
use crate::poststate;

use super::{LawSizes, LawfulWorld, corpus, enforcement, ids};

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
