//! P12 adversarial integration: close/drain under load and retained results
//! after close, over the landed successor store's public embedding surface
//! (SDK-002/SDK-005/SDK-007 successor properties at the CORE boundary,
//! RUN-04/RUN-05 shape, E-SNAPSHOT/E-DURABILITY, G11/G12). The Node-side
//! twins live in `ts/test/adversarial-boundary.test.ts`; this file proves
//! the underlying Rust ownership truth those wrappers report.
//!
//! Verification: `NotRun` (F2 authors, does not execute).

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bumbledb::schema::{
    FieldDescriptor, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};
use bumbledb::store::CloseReport;
use bumbledb::{Db, Error, ExecutionPolicy, Value, WorkContext};

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-p12-close-{tag}-{}-{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn theory() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Row".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![],
    }
}

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 100_000_000,
        working_bytes: 100_000_000,
        scratch_bytes: 100_000_000,
        result_bytes: 100_000_000,
        rows: 10_000_000,
        work_units: 1_000_000_000,
        timeout: Duration::from_secs(600),
    }
    .start()
    .expect("work budget starts")
}

fn scan_ids(db: &Db<SchemaDescriptor>) -> Vec<u64> {
    let mut ids = Vec::new();
    db.read(common::work(), |read| {
        for row in read.scan(RelationId(0))? {
            let row = row?;
            if let Some(Value::U64(id)) = row.first() {
                ids.push(*id);
            }
        }
        Ok(())
    })
    .expect("scan reads");
    ids.sort_unstable();
    ids
}

fn insert(db: &Db<SchemaDescriptor>, id: u64) {
    db.write(common::work(), |tx| {
        tx.insert_dyn(RelationId(0), [vec![Value::U64(id)]])?;
        Ok(())
    })
    .expect("write runs")
    .unwrap();
}

/// Close success means real reclamation, and close honesty means reporting
/// live readers instead of pretending quiescence: an in-flight read lease
/// makes `close` report `Incomplete` with a nonzero live count; the retained
/// lease keeps reading its exact coherent snapshot; after the lease ends the
/// repeated close drains to `Closed`, every new verb refuses typed, and the
/// released directory admits a successor open.
#[test]
fn close_under_load_reports_reality_then_drains_and_releases() {
    let dir = temp_dir("load");
    let db = Db::create(&dir, theory(), work()).expect("create store").unwrap();
    insert(&db, 1);

    let (entered_tx, entered_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    std::thread::scope(|scope| {
        // Move the channel endpoints into the reader thread (`Receiver` is
        // Send but not Sync); the store handle stays a shared borrow.
        let db_ref = &db;
        scope.spawn(move || {
            db_ref
                .read(common::work(), |read| {
                    let mut count = 0usize;
                    for row in read.scan(RelationId(0))? {
                        row?;
                        count += 1;
                    }
                    assert_eq!(count, 1, "the lease reads its coherent snapshot");
                    entered_tx.send(()).expect("signal entered");
                    release_rx
                        .recv_timeout(Duration::from_secs(30))
                        .expect("released");
                    // Still inside the SAME lease while close is pending: the
                    // retained reader keeps its exact rows.
                    let mut again = 0usize;
                    for row in read.scan(RelationId(0))? {
                        row?;
                        again += 1;
                    }
                    assert_eq!(again, 1, "a retained lease reads exactly during closing");
                    Ok(())
                })
                .expect("parked read completes");
        });

        entered_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("reader entered");
        match db.integration_store().close(&work()) {
            CloseReport::Incomplete {
                live_transactions, ..
            } => {
                assert!(live_transactions >= 1, "close reports the live lease");
            }
            CloseReport::Closed => panic!("close must not claim quiescence over a live lease"),
        }
        release_tx.send(()).expect("release the reader");
    });

    // Drained: the repeated close converges to Closed.
    let start = Instant::now();
    loop {
        match db.integration_store().close(&work()) {
            CloseReport::Closed => break,
            CloseReport::Incomplete { .. } => {
                assert!(start.elapsed() < Duration::from_secs(30), "close drains");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    // Every new verb refuses typed — never a panic, never a silent no-op.
    let read_refused = db.read(common::work(), |_read| Ok(()));
    assert!(
        matches!(read_refused, Err(Error::Store(_))),
        "reads after close refuse with the typed store error: {read_refused:?}"
    );
    let write_refused = db.write(common::work(), |tx| {
        tx.insert_dyn(RelationId(0), [vec![Value::U64(2)]])?;
        Ok(())
    });
    assert!(
        matches!(write_refused, Err(Error::Store(_))),
        "writes after close refuse typed: {write_refused:?}"
    );
    // Real reclamation: dropping the closed owner releases the kernel lock
    // and a successor opens the same directory with the durable facts.
    drop(db);
    let successor = Db::open(&dir, theory(), common::work()).expect("the released directory reopens");
    assert_eq!(scan_ids(&successor), vec![1]);
    drop(successor);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A result collected before close is OWNED: closing (and dropping) the
/// native store afterwards cannot mutate, truncate or invalidate it — the
/// retained-result-after-close property the TS `CompleteResult` contract
/// also promises (Q-LIFETIME/API-07 shape at the core boundary).
#[test]
fn retained_owned_results_survive_close_byte_for_byte() {
    let dir = temp_dir("retained");
    let db = Db::create(&dir, theory(), work()).expect("create store").unwrap();
    for id in [3u64, 1, 2] {
        insert(&db, id);
    }
    let collected = scan_ids(&db);
    assert_eq!(collected, vec![1, 2, 3]);

    let start = Instant::now();
    loop {
        match db.integration_store().close(&work()) {
            CloseReport::Closed => break,
            CloseReport::Incomplete { .. } => {
                assert!(start.elapsed() < Duration::from_secs(30));
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    drop(db);
    // The owned result is untouched by the native teardown.
    assert_eq!(collected, vec![1, 2, 3], "owned results outlive the store");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Writers hammering the store while it closes: every submission either
/// commits whole or refuses typed; no panic, no torn state. The reopened
/// store passes the full offline sweep (the production judge re-run), so
/// no admission raced the teardown into corruption (G06 close family,
/// SDK-002 "close revokes admission" at the core boundary).
#[test]
fn writers_racing_close_refuse_typed_and_leave_a_coherent_store() {
    let dir = temp_dir("race");
    let db = Db::create(&dir, theory(), work()).expect("create store").unwrap();

    std::thread::scope(|scope| {
        for lane in 0..2u64 {
            let db = &db;
            scope.spawn(move || {
                for step in 0..200u64 {
                    let id = lane * 10_000 + step;
                    let outcome = db.write(common::work(), |tx| {
                        tx.insert_dyn(RelationId(0), [vec![Value::U64(id)]])?;
                        Ok(())
                    });
                    match outcome {
                        Ok(_admission) => {}
                        Err(Error::Store(_)) => break, // close revoked admission, typed
                        Err(other) => panic!("only the typed store refusal is lawful: {other:?}"),
                    }
                }
            });
        }
        // Close mid-flight; writers between prepare and commit are drained
        // or refused, never torn.
        let _ = db.integration_store().close(&work());
    });

    let start = Instant::now();
    loop {
        match db.integration_store().close(&work()) {
            CloseReport::Closed => break,
            CloseReport::Incomplete { .. } => {
                assert!(start.elapsed() < Duration::from_secs(30));
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    drop(db);

    let successor = Db::open(&dir, theory(), common::work()).expect("reopen after racing close");
    let report = successor.verify_store().expect("offline sweep runs");
    assert_eq!(
        report.verdict,
        bumbledb::StoreVerdict::Coherent,
        "no admission raced the teardown into physical or semantic corruption"
    );
    drop(successor);
    let _ = std::fs::remove_dir_all(&dir);
}
