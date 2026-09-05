//! Chapter 34's Rust core flow as a downstream consumer: the shared
//! `Learning` schema, ordinary RAII, application-owned `Id128` values
//! generated once before sealing, typed nominal entity IDs, grouped
//! exact float aggregates, `use`-composition of a reusable typed query
//! template, and a witnessed read/modify/write against the same store.
//!
//! Everything is the ACTUAL public surface: `Db::create`/`open`,
//! `db.write(|tx| …)` admission with complete `Violations` on rejection,
//! `db.read(|snap| …)` coherent snapshots, `db.prepare`/`execute_collect`
//! and positional `BindValue` parameters. No log/AWS symbol is reachable
//! from this crate graph (the core is log/transport-free by contract).

use std::error::Error;

use bumbledb::{Admission, BindValue, Db, F64, Id128, Interval};

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

fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!("bumbledb-consumer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;

    // Application-owned identity: sixteen bytes chosen by the app, once,
    // before anything is sealed. There is no allocator, FreshRef or
    // reservation anywhere; a real app uses cryptographic entropy.
    let student_id = StudentId(Id128::from_bytes(*b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10"));
    let attempt_id = AttemptId(Id128::from_bytes(*b"\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20"));
    let second_attempt = AttemptId(Id128::from_bytes(*b"\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30"));

    // Explicit creation; open never creates a missing database.
    let db = Db::create(&dir, Learning)?.unwrap();

    // Ordinary admission: `?` is operational failure, the Admission match
    // is semantic judgment with complete statement diagnostics.
    let admitted = db.write(|tx| {
        tx.insert([&Student { id: student_id, name: "Ada", budget: 10 }])?;
        tx.insert([&Attempt {
            id: attempt_id,
            student: student_id,
            score: F64::from(0.9),
            units: 1,
            active: Interval::new(0i64, 60i64).expect("nonempty half-open interval"),
        }])?;
        tx.insert([&Attempt {
            id: second_attempt,
            student: student_id,
            score: F64::from(0.7),
            units: 2,
            active: Interval::new(60i64, 120i64).expect("nonempty half-open interval"),
        }])?;
        Ok(())
    })?;
    match admitted {
        Admission::Accepted(()) => {}
        Admission::Rejected(violations) => return Err(format!("rejected: {violations:?}").into()),
    }

    // A parameterized reusable template: field-name punning binds by name,
    // omitted fields stay existential, params are positional BindValues.
    let attempts_for = bumbledb::query!(Learning {
        (id, score, units) | Attempt(id, student, score, units), student == ?student;
    });

    // A grouped exact-aggregate template, imported downstream with `use`:
    // the same relation-expression composition the TypeScript
    // `match(imported, …)` splice builds.
    let attempt_stats = bumbledb::query!(Learning {
        (student, total: Sum(score), mean: Mean(score)) | Attempt(id, student, score);
    });
    let student_summary = bumbledb::query!(Learning {
        use stats = &attempt_stats;
        (student, name, total, mean) |
            stats(student, total, mean), Student(id: student, name);
    });

    let mut per_student = db.prepare(&attempts_for)?;
    let mut summary = db.prepare(&student_summary)?;
    let no_params: [BindValue<'static>; 0] = [];

    let (rows, summaries) = db.read(|snap| {
        let rows = snap.execute_collect(&mut per_student, &[BindValue::Id128(student_id.0)])?;
        let summaries = snap.execute_collect(&mut summary, &no_params)?;
        Ok((rows, summaries))
    })?;
    assert_eq!(rows.len(), 2, "both attempts are visible through the template");
    assert_eq!(summaries.len(), 1, "one student, one exact grouped summary row");

    // Witnessed correction: read under a short snapshot, keep the copied
    // row, replace it in one admitted delta. Same-fact add wins within a
    // command; separate writes stay ordered by the engine.
    let previous = db.read(|snap| {
        let attempts = snap.scan_facts::<Attempt>()?.collect::<bumbledb::Result<Vec<_>>>()?;
        Ok(attempts
            .into_iter()
            .find(|attempt| attempt.id == attempt_id)
            .expect("the inserted attempt exists"))
    })?;
    let corrected = db.write(|tx| {
        tx.delete([&previous])?;
        tx.insert([&Attempt { score: F64::from(0.95), ..previous }])?;
        Ok(())
    })?;
    match corrected {
        Admission::Accepted(()) => {}
        Admission::Rejected(violations) => return Err(format!("rejected: {violations:?}").into()),
    }

    // Capacity is a law, not advice: an eleventh unit for a ten-unit
    // budget refuses with the violated statement, not a panic or a
    // silent partial write.
    let third = AttemptId(Id128::from_bytes(*b"\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40"));
    let over_budget = db.write(|tx| {
        tx.insert([&Attempt {
            id: third,
            student: student_id,
            score: F64::from(0.5),
            units: 8,
            active: Interval::new(120i64, 180i64).expect("nonempty half-open interval"),
        }])?;
        Ok(())
    })?;
    match over_budget {
        Admission::Rejected(violations) => {
            assert!(!violations.is_empty(), "capacity rejection names its statements");
        }
        Admission::Accepted(()) => return Err("an over-budget attempt was admitted".into()),
    }

    // Dropping db/prepared/snapshot guards releases their resources; the
    // store directory outlives the process like any application data dir.
    drop(db);
    std::fs::remove_dir_all(&dir)?;
    println!("bumbledb rust consumer fixture: OK");
    Ok(())
}
