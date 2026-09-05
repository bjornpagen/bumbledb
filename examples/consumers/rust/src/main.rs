//! Packed Rust core consumer (D07/D22): the shared `Learning` schema,
//! ordinary RAII, application-owned `Id128` values generated once before
//! sealing, typed nominal entity IDs, grouped exact float aggregates,
//! `use`-composition of a reusable typed query template, and a witnessed
//! read/modify/write against the same store.
//!
//! Current public spellings: `Db::create(..., work)` by value,
//! `ChangeSet::builder(db.schema(), work.clone())`, `db.apply` /
//! `ApplyOutcome::InvariantRejected`, `db.snapshot(&work)` → `OwnedRead` /
//! `ReadFrame`, `frame.prepare` + `execute_collect`, `Db::close() -> CloseReport`.
//! Typed facts encode through `Fact::append_values`. Query params bind as
//! `BindValue` (the published `BindArgs` surface).
//!
//! Verification: NotRun until packed-consumer qualification.

use std::time::Duration;

use bumbledb::{
    Admission, ApplyExpected, ApplyOutcome, BindValue, ChangeSet, ChangeSetBuilder, CloseReport,
    Db, ExecutionPolicy, F64, Fact, Id128, Interval, WorkContext, start_operation,
};

bumbledb::schema! {
    pub Learning;

    relation Student { id: id128 as StudentId, name: str, budget: u64 }
    relation Attempt {
        id: id128 as AttemptId,
        student: id128 as StudentId,
        score: f64,
        units: u64,
        active: interval<i64>,
    }

    Student(id) -> Student;
    Attempt(id) -> Attempt;
    Attempt(student) <= Student(id);
    Student(id) <=[units]{0..budget} Attempt(student);
}

/// Finite host policy. Zero means none; nothing here invents MAX/year.
fn work() -> WorkContext {
    start_operation(ExecutionPolicy {
        input_bytes: 4_000_000,
        working_bytes: 16_000_000,
        scratch_bytes: 16_000_000,
        result_bytes: 4_000_000,
        rows: 100_000,
        work_units: 10_000_000,
        timeout: Duration::from_secs(10),
    })
    .expect("finite work")
}

/// A budget that cannot pay for a completed two-row answer.
fn tiny_delivery() -> WorkContext {
    start_operation(ExecutionPolicy {
        input_bytes: 4_000_000,
        working_bytes: 16_000_000,
        scratch_bytes: 16_000_000,
        result_bytes: 8,
        rows: 100_000,
        work_units: 10_000_000,
        timeout: Duration::from_secs(2),
    })
    .expect("tiny delivery work")
}

fn insert_fact<'a, F: Fact<'a>>(
    draft: &mut ChangeSetBuilder<'_>,
    fact: &F,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut values = Vec::new();
    fact.append_values(&mut values)?;
    draft.insert(F::RELATION, &values)?;
    Ok(())
}

fn delete_fact<'a, F: Fact<'a>>(
    draft: &mut ChangeSetBuilder<'_>,
    fact: &F,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut values = Vec::new();
    fact.append_values(&mut values)?;
    draft.delete(F::RELATION, &values)?;
    Ok(())
}

fn outcome_name(outcome: &ApplyOutcome) -> &'static str {
    match outcome {
        ApplyOutcome::Accepted { .. } => "accepted",
        ApplyOutcome::NoChange { .. } => "no-change",
        ApplyOutcome::InvariantRejected { .. } => "invariant-rejected",
        ApplyOutcome::Moved { .. } => "moved",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("bumbledb-consumer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let work = work();

    let student_id = StudentId(Id128::from_bytes(*b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10"));
    let attempt_id = AttemptId(Id128::from_bytes(*b"\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20"));
    let second_attempt = AttemptId(Id128::from_bytes(*b"\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30"));

    let db = match Db::create(&dir, Learning, work.clone())? {
        Admission::Accepted(db) => db,
        Admission::Rejected(violations) => {
            return Err(format!("empty Learning must admit: {violations:?}").into());
        }
    };

    let mut draft = ChangeSet::builder(db.schema(), work.clone());
    insert_fact(
        &mut draft,
        &Student {
            id: student_id,
            name: "Ada",
            budget: 10,
        },
    )?;
    insert_fact(
        &mut draft,
        &Attempt {
            id: attempt_id,
            student: student_id,
            score: F64::from(0.9),
            units: 1,
            active: Interval::new(0i64, 60i64).expect("nonempty half-open interval"),
        },
    )?;
    insert_fact(
        &mut draft,
        &Attempt {
            id: second_attempt,
            student: student_id,
            score: F64::from(0.7),
            units: 2,
            active: Interval::new(60i64, 120i64).expect("nonempty half-open interval"),
        },
    )?;
    match db.apply(&draft.finish()?, ApplyExpected::Any, &work)? {
        ApplyOutcome::Accepted { .. } | ApplyOutcome::NoChange { .. } => {}
        other => return Err(format!("insert apply refused: {}", outcome_name(&other)).into()),
    }

    let attempts_for = bumbledb::query!(Learning {
        (id, score, units) | Attempt(id, student, score, units), student == ?student;
    });
    let attempt_stats = bumbledb::query!(Learning {
        (student, total: Sum(score), mean: Mean(score)) | Attempt(id, student, score);
    });
    let student_summary = bumbledb::query!(Learning {
        use stats = &attempt_stats;
        (student, name, total, mean) |
            stats(student, total, mean), Student(id: student, name);
    });

    let (rows, summaries, previous, witness) = {
        let snapshot = db.snapshot(&work)?;
        let frame = snapshot.frame(&work);
        let mut attempts = frame.prepare(&attempts_for)?;
        let rows = frame.execute_collect(&mut attempts, &[BindValue::Id128(student_id.0)])?;
        let mut summary = frame.prepare(&student_summary)?;
        let summaries = frame.execute_collect(&mut summary, &[] as &[BindValue])?;
        let previous = snapshot
            .get(attempt_id, &work)?
            .expect("the inserted attempt exists");
        (rows, summaries, previous, snapshot.witness())
    };
    assert_eq!(rows.len(), 2, "both attempts are visible through the template");
    assert_eq!(summaries.len(), 1, "one student, one exact grouped summary row");

    // Tiny delivery work is a fresh frame budget: it does not inherit the snapshot.
    let tiny = tiny_delivery();
    let snapshot = db.snapshot(&work)?;
    let frame = snapshot.frame(&tiny);
    let mut oversized = frame.prepare(&attempts_for)?;
    assert!(
        frame
            .execute_collect(&mut oversized, &[BindValue::Id128(student_id.0)])
            .is_err(),
        "D07: a result-bytes cap of 8 must refuse a two-row collect"
    );
    drop(snapshot);

    let mut correction = ChangeSet::builder(db.schema(), work.clone());
    delete_fact(&mut correction, &previous)?;
    insert_fact(
        &mut correction,
        &Attempt {
            score: F64::from(0.95),
            ..previous
        },
    )?;
    match db.apply(&correction.finish()?, ApplyExpected::Exact(witness), &work)? {
        ApplyOutcome::Accepted { .. } | ApplyOutcome::NoChange { .. } => {}
        other => {
            return Err(format!("witnessed correction refused: {}", outcome_name(&other)).into());
        }
    }

    let third = AttemptId(Id128::from_bytes(*b"\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40"));
    let mut over = ChangeSet::builder(db.schema(), work.clone());
    insert_fact(
        &mut over,
        &Attempt {
            id: third,
            student: student_id,
            score: F64::from(0.5),
            units: 8,
            active: Interval::new(120i64, 180i64).expect("nonempty half-open interval"),
        },
    )?;
    match db.apply(&over.finish()?, ApplyExpected::Any, &work)? {
        ApplyOutcome::InvariantRejected { violations } => {
            assert!(
                !violations.is_empty(),
                "capacity rejection names its statements"
            );
        }
        other => {
            return Err(format!(
                "an over-budget attempt was admitted: {}",
                outcome_name(&other)
            )
            .into());
        }
    }

    match db.close() {
        CloseReport::Closed => {}
        CloseReport::Incomplete {
            live_transactions, ..
        } => {
            return Err(format!("joined close left {live_transactions} live transactions").into());
        }
    }
    std::fs::remove_dir_all(&dir)?;
    println!("bumbledb rust consumer fixture: OK");
    Ok(())
}
