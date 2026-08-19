use bumbledb::schema::{SealedBound, SealedWeight, ValidateDescriptor as _};
use bumbledb::{Db, FieldId, Theory as _, Value};

use crate::differential::{self, Op, Verdict};
use crate::naive::{Delta, NaiveDb};
use crate::writebench::write_protocol;

use super::{Mass, PARENTS, calendar, calendar_rows, ids, power, power_baseline, power_rows};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-capacity-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn pool(id: u64, supply: u64) -> (bumbledb::RelationId, Vec<Value>) {
    (ids::PARENT, vec![Value::U64(id), Value::U64(supply)])
}

fn device(id: u64, pool: u64, watts: u64) -> (bumbledb::RelationId, Vec<Value>) {
    (
        ids::CHILD,
        vec![Value::U64(id), Value::U64(pool), Value::U64(watts)],
    )
}

fn room(id: u64, start: u64, end: u64) -> (bumbledb::RelationId, Vec<Value>) {
    (
        ids::PARENT,
        vec![
            Value::U64(id),
            Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty")),
        ],
    )
}

fn booking(id: u64, room: u64, start: u64, end: u64) -> (bumbledb::RelationId, Vec<Value>) {
    (
        ids::CHILD,
        vec![
            Value::U64(id),
            Value::U64(room),
            Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty")),
        ],
    )
}

/// Seed one twin's unit mass as differential write ops (32-fact chunks,
/// the windowed precedent).
fn seed_ops(
    mass: Mass,
    rows: fn(Mass, bumbledb::RelationId) -> Box<dyn Iterator<Item = Vec<Value>>>,
) -> Vec<Op> {
    let mut ops = Vec::new();
    for rel in [ids::PARENT, ids::CHILD] {
        let mut delta = Delta::default();
        for row in rows(mass, rel) {
            delta.inserts.push((rel, row));
            if delta.inserts.len() == 32 {
                ops.push(Op::Write(std::mem::take(&mut delta)));
            }
        }
        if !delta.inserts.is_empty() {
            ops.push(Op::Write(std::mem::take(&mut delta)));
        }
    }
    ops
}

fn write(
    deletes: Vec<(bumbledb::RelationId, Vec<Value>)>,
    inserts: Vec<(bumbledb::RelationId, Vec<Value>)>,
) -> Op {
    Op::Write(Delta { deletes, inserts })
}

/// The three twin theories validate; the power pair differs exactly in
/// the declared statements, and both weighted laws carry the ruled
/// shapes — weight column, dependent ceiling, `Duration` pair —
/// asserted against the capacity arena with `Bound` terms (never a
/// unit downgrade passing silently).
#[test]
fn the_twin_theories_validate_and_pin_the_weighted_shapes() {
    let budgeted = power::PowerWorld
        .descriptor()
        .validate()
        .expect("the power twin validates");
    let control = power_baseline::UnbudgetedWorld
        .descriptor()
        .validate()
        .expect("the control twin validates");
    assert_eq!(budgeted.capacities().len(), 1, "the power budget");
    assert_eq!(control.capacities().len(), 0, "the control carries none");
    let budget = &budgeted.capacities()[0];
    assert_eq!(budget.weight, SealedWeight::Field(FieldId(2)), "[watts]");
    assert_eq!(
        (budget.lo, budget.hi),
        (0, SealedBound::TargetField(FieldId(1))),
        "{{0..supply}} — the dependent ceiling"
    );

    let rooms = calendar::CalendarCapacityWorld
        .descriptor()
        .validate()
        .expect("the calendar twin validates");
    assert_eq!(rooms.capacities().len(), 1, "the calendar law");
    let law = &rooms.capacities()[0];
    assert!(
        matches!(
            law.weight,
            SealedWeight::Duration { field, .. } if field == FieldId(2)
        ),
        "[Duration(booked)]"
    );
    assert!(
        matches!(
            law.hi,
            SealedBound::Duration { field, .. } if field == FieldId(1)
        ),
        "{{0..Duration(span)}}"
    );
}

/// The power-budget delta stream — the oracle gate's shared fixture:
/// the legal sample, the over-budget burst, the zero-weight commits
/// (the § 6 Sum-vs-Count split live), the dependent-bound lowering
/// (a bound change alone re-judges the group), the exact budget, and
/// one watt over it.
fn power_stream(mass: Mass) -> Vec<Op> {
    let base = mass.parents * mass.children_per_parent;
    let mut ops = seed_ops(mass, power_rows);
    ops.extend([
        // A legal sample under a seeded pool.
        write(vec![], vec![device(base, 1, 1)]),
        // A fresh tightly-budgeted pool.
        write(vec![], vec![pool(100, 5)]),
        // The over-budget burst: 3 + 3 > 5 — MUST abort on every
        // oracle, witnessed measure 6 (the full walk, C14).
        write(
            vec![],
            vec![device(base + 1, 100, 3), device(base + 2, 100, 3)],
        ),
        // Half the budget commits.
        write(vec![], vec![device(base + 3, 100, 3)]),
        // Zero-weight devices spend nothing (Sum, not Count).
        write(
            vec![],
            vec![device(base + 4, 100, 0), device(base + 5, 100, 0)],
        ),
        // Lowering the supply on the parent's own row re-judges the
        // group: 3 > 2 — the bound change ALONE convicts.
        write(vec![pool(100, 5)], vec![pool(100, 2)]),
        // Raising it releases.
        write(vec![pool(100, 5)], vec![pool(100, 10)]),
        // The exact budget: 3 + 7 = 10 commits.
        write(vec![], vec![device(base + 6, 100, 7)]),
        // One watt over convicts, measure 11.
        write(vec![], vec![device(base + 7, 100, 1)]),
    ]);
    ops
}

/// Naive parity for the weighted judge — the semantic oracle gate the
/// timed rows sit behind: verdicts, citations, and witnessed measures
/// (C14) compared whole through the differential runner.
#[test]
fn the_power_budget_verdicts_agree_with_the_naive_model() {
    let dir = scratch("power-naive");
    let mass = Mass::unit();
    let db = Db::create(&dir, power::PowerWorld)
        .expect("create")
        .expect("accepted");
    let mut naive = NaiveDb::new(&power::PowerWorld.descriptor());
    let ops = power_stream(mass);
    let summary = differential::run(&db, &mut naive, &ops).expect("verdict parity");
    assert_eq!(
        summary.aborts, 3,
        "the burst, the lowered bound, the watt over"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Naive parity for the calendar lane (C15: the fresh twin world) —
/// the `Duration` weight and the `Duration` ceiling on both oracles.
#[test]
fn the_calendar_verdicts_agree_with_the_naive_model() {
    let dir = scratch("calendar-naive");
    let mass = Mass::unit();
    let db = Db::create(&dir, calendar::CalendarCapacityWorld)
        .expect("create")
        .expect("accepted");
    let mut naive = NaiveDb::new(&calendar::CalendarCapacityWorld.descriptor());
    let base = mass.parents * mass.children_per_parent;
    let mut ops = seed_ops(mass, calendar_rows);
    ops.extend([
        // A legal sample high in a seeded room's span.
        write(vec![], vec![booking(base, 1, 500_000, 500_001)]),
        // A ten-unit room.
        write(vec![], vec![room(100, 0, 10)]),
        // Six of ten commit.
        write(vec![], vec![booking(base + 1, 100, 0, 6)]),
        // Six more blow the measure: 12 > 10 — witnessed whole.
        write(vec![], vec![booking(base + 2, 100, 6, 12)]),
        // The exact measure commits: 6 + 4 = 10. Overlap is NOT the
        // law — the booked TIME is the budget.
        write(vec![], vec![booking(base + 3, 100, 0, 4)]),
    ]);
    let summary = differential::run(&db, &mut naive, &ops).expect("verdict parity");
    assert_eq!(summary.aborts, 1, "the overspent room");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// One power-stream delta against the `SQLite` twin: deletes then
/// inserts inside one IMMEDIATE transaction (the engine's own delta
/// order), any refusal rolling the whole delta back — accept/abort is
/// the compared verdict.
fn sqlite_verdict(conn: &rusqlite::Connection, delta: &Delta) -> bool {
    conn.execute_batch("BEGIN IMMEDIATE").expect("begin");
    let mut ok = true;
    for (rel, fact) in &delta.deletes {
        let sql = match *rel {
            ids::PARENT => "DELETE FROM \"Pool\" WHERE \"id\" = ?1 AND \"supply\" = ?2",
            _ => "DELETE FROM \"Device\" WHERE \"id\" = ?1 AND \"pool\" = ?2 AND \"watts\" = ?3",
        };
        let params: Vec<i64> = fact
            .iter()
            .map(|v| match v {
                Value::U64(n) => i64::try_from(*n).expect("fixture values fit i64"),
                other => panic!("power facts are u64-only, got {other:?}"),
            })
            .collect();
        if conn
            .execute(sql, rusqlite::params_from_iter(params))
            .is_err()
        {
            ok = false;
            break;
        }
    }
    if ok {
        for (rel, fact) in &delta.inserts {
            let sql = match *rel {
                ids::PARENT => "INSERT INTO \"Pool\" (\"id\", \"supply\") VALUES (?1, ?2)",
                _ => "INSERT INTO \"Device\" (\"id\", \"pool\", \"watts\") VALUES (?1, ?2, ?3)",
            };
            let params: Vec<i64> = fact
                .iter()
                .map(|v| match v {
                    Value::U64(n) => i64::try_from(*n).expect("fixture values fit i64"),
                    other => panic!("power facts are u64-only, got {other:?}"),
                })
                .collect();
            if conn
                .execute(sql, rusqlite::params_from_iter(params))
                .is_err()
            {
                ok = false;
                break;
            }
        }
    }
    if ok {
        conn.execute_batch("COMMIT").expect("commit");
    } else {
        let _ = conn.execute_batch("ROLLBACK");
    }
    ok
}

/// The SUM-trigger twin renders the SAME accept/abort verdicts as the
/// engine over the whole power stream — the one place `SQLite` speaks
/// a weighted capacity law (enforcement, the lawful pattern; the
/// polarity table decides the trigger set: BEFORE INSERT on the
/// weighed side AND on the bound-carrying side, deletes trigger-free
/// because a non-negative ceiling is insert-violable only). Verdict
/// parity only — nothing here is timed.
#[test]
fn the_sqlite_sum_trigger_agrees_with_the_engine() {
    let dir = scratch("power-sqlite");
    let mass = Mass::unit();
    let db = Db::create(&dir, power::PowerWorld)
        .expect("create")
        .expect("accepted");
    let conn = rusqlite::Connection::open_in_memory().expect("twin");
    conn.execute_batch("PRAGMA foreign_keys=ON").expect("fk");
    for statement in super::sqlite::DDL {
        conn.execute_batch(statement).expect("twin ddl");
    }
    // The engine judges final states; the twin's FK is immediate, so
    // the bound-lowering delete+reinsert defers FK checks to COMMIT —
    // the lawful lane's recorded asymmetry, resolved per-transaction.
    let mut aborts = 0u64;
    for op in power_stream(mass) {
        let Op::Write(delta) = op else {
            panic!("the power stream is write-only")
        };
        conn.execute_batch("PRAGMA defer_foreign_keys=ON")
            .expect("defer");
        let theirs = sqlite_verdict(&conn, &delta);
        let ours = matches!(differential::engine_write(&db, &delta), Verdict::Committed);
        assert_eq!(
            ours, theirs,
            "engine and SQLite twin disagreed on {delta:?}"
        );
        if !ours {
            aborts += 1;
        }
    }
    assert_eq!(aborts, 3, "the stream exercises both verdicts");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The three timed rows run their full protocols on seeded twins and
/// every measured commit is legal (the runners measure the judge —
/// bound read plus weighted walk — never refusals).
#[test]
fn the_capacity_rows_run_their_protocols() {
    let dir = scratch("rows");
    let budgeted = Db::create(&dir.join("power"), power::PowerWorld)
        .expect("create")
        .expect("accepted");
    super::load(&budgeted, Mass::BENCH, power_rows).expect("load power");
    let control = Db::create(&dir.join("baseline"), power_baseline::UnbudgetedWorld)
        .expect("create control")
        .expect("accepted");
    super::load(&control, Mass::BENCH, power_rows).expect("load control");
    let rooms = Db::create(&dir.join("calendar"), calendar::CalendarCapacityWorld)
        .expect("create calendar")
        .expect("accepted");
    super::load(&rooms, Mass::BENCH, calendar_rows).expect("load calendar");

    let sum =
        super::commit_capacity_sum(&budgeted, write_protocol("commit_capacity_sum")).expect("sum");
    assert_eq!(sum.work, 64, "one row per sample");
    assert!(sum.stats.min > 0);
    let baseline =
        super::commit_capacity_baseline(&control, write_protocol("commit_capacity_baseline"))
            .expect("baseline");
    assert_eq!(baseline.work, 64);
    let duration =
        super::commit_capacity_duration(&rooms, write_protocol("commit_capacity_duration"))
            .expect("duration");
    assert_eq!(duration.work, 64);
    let _ = PARENTS;
    drop(budgeted);
    drop(control);
    drop(rooms);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The traced capacity path (`bench --trace`): the weighted-capacity
/// judgment lane lands its traced solo sample as a parseable
/// Chrome+folded pair beside the read-family traces, the judgment and
/// commit spans reach the artifact, and the flame embed rides the same
/// report list the read families fill. Ephemeral twin, one selected
/// family: a smoke test, not a measurement.
#[cfg(feature = "obs")]
#[test]
fn traced_capacity_lands_the_judgment_spans() {
    let dir = scratch("traced");
    let trace_dir = dir.join("trace");
    let mut flames = Vec::new();
    let rows = super::write_families(
        crate::corpus_gen::GenConfig {
            seed: 1,
            scale: crate::corpus_gen::Scale::Tiny,
        },
        &dir.join("scratch"),
        &|name| name == "commit_capacity_sum",
        crate::storemode::StoreMode::Ephemeral,
        Some(&trace_dir),
        &mut flames,
    )
    .expect("the traced capacity lane");
    assert_eq!(rows.len(), 1, "one selected row");
    assert_eq!(flames.len(), 1, "one flame embed per traced family");
    assert_eq!(flames[0].name, "commit_capacity_sum");
    let json_path = trace_dir.join("commit_capacity_sum.json");
    let text = std::fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("{}: {e}", json_path.display()));
    assert!(
        text.starts_with("[\n") && text.ends_with("\n]\n"),
        "{} parses as a Chrome array",
        json_path.display()
    );
    assert!(
        text.contains("judgment"),
        "the capacity judgment spans reach the artifact"
    );
    assert!(
        text.contains(bumbledb::obs::names::LMDB_COMMIT.label()),
        "the LMDB commit span reaches the artifact"
    );
    let folded = std::fs::read_to_string(trace_dir.join("commit_capacity_sum.folded"))
        .expect("the folded twin lands beside the json");
    assert!(!folded.is_empty(), "a non-degenerate fold");
    let _ = std::fs::remove_dir_all(&dir);
}
