//! F3 finding C gate: resource accounting and beyond-memory admission are
//! REAL on the public-to-native call path (`Db::integration_writer` →
//! `WriterSession::prepare` → the store's candidate protocol → the
//! production `SchemaJudge`). Gate anchors: F-RESOURCE, Q-BUDGET, Q-DISK,
//! E-LARGE (local asymptotic half), QRY-002/PERF-002 shape, SDK-011.
//!
//! The load-bearing pair: admission of a small change to a relation far
//! larger than the WORKING allowance succeeds (charged disk carries the
//! grouped judgment state), and the SAME admission with a zero SCRATCH
//! allowance refuses with the typed exhaustion — proving the success ran
//! on accounted disk, not on silent memory. Scale rides
//! `BUMBLEDB_GATE_ROWS` (default keeps CI honest and local runs fast; the
//! F3 storage-qualified runner raises it into the gibibytes).

use std::time::Duration;

use bumbledb::integration::{AttachmentChange, HostChanges, IntegrationError};
use bumbledb::work::{ByteKind, ExecutionPolicy, Resource, WorkContext};
use bumbledb::{Admission, Db, Error, RelationId, Value, WorkError};

mod common;

bumbledb::schema! {
    pub GateBounded;

    relation Doc {
        id: u64 as DocId,
        body: str,
    }

    Doc(body) -> Doc;
}

const DOC: RelationId = RelationId(0);

/// The declared per-relation scale for the large-relation tests. The
/// default (~256 MiB of canonical rows) proves the asymptotic against the
/// 8 MiB working budget below (a 32× gap); raise it to GiB scale via
/// `BUMBLEDB_GATE_ROWS` on a storage-qualified runner.
fn gate_rows() -> u64 {
    std::env::var("BUMBLEDB_GATE_ROWS")
        .ok()
        .and_then(|rows| rows.parse().ok())
        .unwrap_or(200_000)
}

/// Distinct ~1.3 KiB text per row: text-heavy data whose determinants are
/// far beyond the scratch map's inline key bound (the exact-checked bucket
/// path is what carries them on disk).
fn body(row: u64) -> String {
    format!(
        "doc-{row:012}-{}",
        "lorem ipsum dolor sit amet consectetur ".repeat(32)
    )
}

fn bounded(working: u64, scratch: u64) -> WorkContext {
    ExecutionPolicy {
        input_bytes: u64::MAX,
        working_bytes: working,
        scratch_bytes: scratch,
        result_bytes: u64::MAX,
        rows: u64::MAX,
        work_units: u64::MAX,
        timeout: Duration::from_mins(30),
    }
    .start()
    .expect("policy starts")
}

/// One admitted small change (a single new document) as a sealed
/// `ChangeSet` built under the caller's own bounded ledger.
fn small_change(
    db: &Db<GateBounded>,
    work: &WorkContext,
    id: u64,
    text: &str,
) -> bumbledb::ChangeSet {
    let mut builder = bumbledb::ChangeSet::builder(db.schema(), work.clone());
    builder
        .insert(DOC, &[Value::U64(id), Value::String(text.into())])
        .expect("stage");
    builder.finish().expect("seal")
}

fn build_store(dir: &std::path::Path, rows: u64) -> Db<GateBounded> {
    let db = Db::create(dir, GateBounded, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        for row in 0..rows {
            let text = body(row);
            tx.insert([&Doc {
                id: DocId(row),
                body: &text,
            }])?;
        }
        Ok(())
    })
    .expect("bulk load")
    .unwrap();
    db
}

/// THE gate: a large existing relation on disk, a small admitted change,
/// and a working allowance ~32× smaller than the relation. Admission
/// succeeds through the real integration path, the peak charged working
/// bytes stay bounded (a charge past the limit would have refused), and
/// the change durably commits.
#[test]
fn small_change_to_a_large_relation_admits_under_a_small_working_budget() {
    let rows = gate_rows();
    let dir = common::TempDir::new("gate-bounded-large");
    let db = build_store(dir.path(), rows);
    let before = db.generation().expect("generation");

    // First, the control: the SAME admission with a zero scratch allowance
    // refuses with the typed exhaustion — the disk tier is charged, so the
    // success below cannot be hiding an unaccounted memory build.
    let starved = bounded(8 << 20, 0);
    {
        let changes = small_change(&db, &starved, rows + 1, &body(rows + 1));
        let mut session = db.integration_writer(&starved).expect("writer");
        let Err(error) = session.prepare(&changes) else {
            panic!("a zero scratch allowance cannot carry the judgment");
        };
        assert!(
            matches!(
                error,
                IntegrationError::Work(WorkError::Exhausted {
                    resource: Resource::ScratchBytes,
                    ..
                })
            ),
            "typed scratch refusal, got {error:?}"
        );
    }
    assert_eq!(
        db.generation().expect("generation"),
        before,
        "the refused attempt left no partial state"
    );

    // The real admission: 8 MiB of working bytes against a relation two
    // orders of magnitude larger, with the scratch dimension funded.
    let work = bounded(8 << 20, 64 << 30);
    let changes = small_change(&db, &work, rows + 1, &body(rows + 1));
    let mut session = db.integration_writer(&work).expect("writer");
    let prepared = match session.prepare(&changes).expect("prepare") {
        Admission::Accepted(prepared) => prepared,
        Admission::Rejected(violations) => panic!("a lawful change rejected: {violations}"),
    };
    assert_eq!(prepared.application_changes().added, 1);
    assert_eq!(prepared.application_changes().removed, 0);
    let sealed = prepared
        .seal(HostChanges {
            records: &[],
            attachment: AttachmentChange::Keep,
        })
        .expect("seal");
    let commit = sealed.commit().expect("commit");
    assert!(commit.changed, "one durable committed change");
    drop(session);

    let after = db.generation().expect("generation");
    assert_ne!(after, before, "the generation witnessed the change");
    assert!(
        work.used(Resource::WorkingBytes) <= 8 << 20,
        "the working ledger never exceeded its allowance"
    );
    // The admitted document is durably readable through the public path.
    let text = body(rows + 1);
    db.read(common::work(), |snap| {
        assert_eq!(
            snap.get(DocByBody { body: &text })?,
            Some(Doc {
                id: DocId(rows + 1),
                body: &text,
            })
        );
        Ok(())
    })
    .expect("read back");
}

/// Constraint checking under pressure delivers COMPLETE rejection
/// diagnostics: a change conflicting with a committed row inside the large
/// relation is rejected with the key statement, BOTH competing rows cited
/// (the committed incumbent and the newcomer), and truncation labeled
/// exactly — all under the same small working budget.
#[test]
fn rejection_diagnostics_are_complete_under_pressure() {
    let rows = 50_000;
    let dir = common::TempDir::new("gate-bounded-reject");
    let db = build_store(dir.path(), rows);
    let before = db.generation().expect("generation");

    let work = bounded(8 << 20, 64 << 30);
    // A NEW document claiming an EXISTING body: the text key refuses.
    let duplicate = body(7);
    let changes = small_change(&db, &work, rows + 9, &duplicate);
    let mut session = db.integration_writer(&work).expect("writer");
    let violations = match session.prepare(&changes).expect("prepare completes") {
        Admission::Rejected(violations) => violations,
        Admission::Accepted(_) => panic!("a key conflict admitted"),
    };
    assert_eq!(violations.len(), 1, "exactly the text key is violated");
    assert!(!violations.examples_truncated(0), "two rows, budget four");
    let cited = violations.cited_facts(0);
    assert_eq!(cited.len(), 2, "both competing rows are evidence");
    let mut ids = Vec::new();
    for fact in cited {
        assert_eq!(
            fact.values()[1],
            Value::String(duplicate.clone().into_boxed_str()),
            "each cited row carries the contested text"
        );
        ids.push(fact.values()[0].clone());
    }
    ids.sort_by_key(|value| match value {
        Value::U64(id) => *id,
        other => panic!("u64 ids only, got {other:?}"),
    });
    assert_eq!(
        ids,
        vec![Value::U64(7), Value::U64(rows + 9)],
        "the committed incumbent AND the newcomer"
    );
    drop(session);
    assert_eq!(
        db.generation().expect("generation"),
        before,
        "a rejection commits nothing"
    );
}

/// Cancellation stops the judgment with the typed refusal and leaves no
/// partial state: the store answers exactly as before, and a fresh session
/// admits normally afterwards.
#[test]
fn cancellation_leaves_no_partial_state() {
    let rows = 20_000;
    let dir = common::TempDir::new("gate-bounded-cancel");
    let db = build_store(dir.path(), rows);
    let before = db.generation().expect("generation");

    let work = bounded(8 << 20, 64 << 30);
    let changes = small_change(&db, &work, rows + 1, &body(rows + 1));
    let mut session = db.integration_writer(&work).expect("writer");
    work.cancel();
    let Err(error) = session.prepare(&changes) else {
        panic!("cancelled work must refuse");
    };
    assert!(
        matches!(error, IntegrationError::Work(WorkError::Cancelled)),
        "typed cancellation, got {error:?}"
    );
    drop(session);
    assert_eq!(db.generation().expect("generation"), before);

    // The store is unpoisoned: a fresh ledger admits the same change.
    let fresh = bounded(8 << 20, 64 << 30);
    let changes = small_change(&db, &fresh, rows + 1, &body(rows + 1));
    let mut session = db.integration_writer(&fresh).expect("writer");
    match session.prepare(&changes).expect("prepare") {
        Admission::Accepted(prepared) => {
            prepared
                .seal(HostChanges {
                    records: &[],
                    attachment: AttachmentChange::Keep,
                })
                .expect("seal")
                .commit()
                .expect("commit");
        }
        Admission::Rejected(violations) => panic!("lawful change rejected: {violations}"),
    }
    assert_ne!(db.generation().expect("generation"), before);
}

/// An injected scratch-storage failure (the judge's temporary environment
/// cannot be created) surfaces as the typed I/O condition — never a
/// fabricated rejection, never a partial commit — and the store survives
/// unharmed. Runs in a subprocess so the broken TMPDIR touches only the
/// child.
#[test]
fn injected_scratch_failure_leaves_no_partial_state() {
    if std::env::var("BUMBLEDB_GATE_CHILD").as_deref() == Ok("1") {
        return; // the child runs only the helper below
    }
    let dir = common::TempDir::new("gate-bounded-inject");
    std::fs::create_dir_all(dir.path()).expect("store dir");
    // A FILE where the scratch root must be a directory: every temporary
    // scratch environment creation under it fails with real I/O.
    let broken = dir.path().join("not-a-directory");
    std::fs::write(&broken, b"scratch root impostor").expect("impostor");
    let store_dir = dir.path().join("store");

    let status = std::process::Command::new(std::env::current_exe().expect("self"))
        .args([
            "scratch_failure_child_helper",
            "--exact",
            "--nocapture",
            "--include-ignored",
        ])
        .env("BUMBLEDB_GATE_CHILD", "1")
        .env("BUMBLEDB_GATE_STORE", &store_dir)
        .env("TMPDIR", &broken)
        .status()
        .expect("spawn child");
    assert!(status.success(), "the child's assertions all held");

    // The parent (with a healthy TMPDIR) reopens the child's store: intact,
    // readable, and exactly the loaded rows — no partial answer survived
    // the injected failure.
    let db = Db::open(&store_dir, GateBounded, common::work()).expect("reopen");
    let text = body(3);
    db.read(common::work(), |snap| {
        assert_eq!(
            snap.get(DocByBody { body: &text })?,
            Some(Doc {
                id: DocId(3),
                body: &text,
            })
        );
        assert_eq!(
            snap.get(DocByBody {
                body: "never-admitted"
            })?,
            None
        );
        Ok(())
    })
    .expect("read back");
}

/// The child half of the injection test (spawned with a broken TMPDIR):
/// builds the store, then watches the bounded admission fail with the
/// typed storage error while the committed state stays exact.
#[test]
#[ignore = "subprocess helper for injected_scratch_failure_leaves_no_partial_state"]
fn scratch_failure_child_helper() {
    if std::env::var("BUMBLEDB_GATE_CHILD").as_deref() != Ok("1") {
        return;
    }
    let store_dir = std::env::var("BUMBLEDB_GATE_STORE").expect("store dir");
    let db = Db::create(std::path::Path::new(&store_dir), GateBounded, common::work())
        .expect("create")
        .expect("accepted");
    // The embedded bulk load never needs scratch (its ledger is the host's
    // own): loading works even with the broken TMPDIR.
    db.write(common::work(), |tx| {
        for row in 0..3000u64 {
            let text = body(row);
            tx.insert([&Doc {
                id: DocId(row),
                body: &text,
            }])?;
        }
        Ok(())
    })
    .expect("bulk load")
    .unwrap();
    let before = db.generation().expect("generation");

    // A working budget small enough to demand the disk tier; its creation
    // fails on the injected TMPDIR, and the failure is a typed storage
    // error — not a rejection, not a truncated verdict, not a commit.
    let work = bounded(64 << 10, 64 << 30);
    let changes = small_change(&db, &work, 5000, &body(5000));
    let mut session = db.integration_writer(&work).expect("writer");
    let Err(error) = session.prepare(&changes) else {
        panic!("scratch creation cannot succeed under the broken TMPDIR");
    };
    match &error {
        IntegrationError::Core(Error::Store(store)) => {
            assert!(
                matches!(**store, bumbledb::store::StoreError::Io(_)),
                "typed I/O condition, got {store:?}"
            );
        }
        other => panic!("expected the typed storage failure, got {other:?}"),
    }
    drop(session);
    assert_eq!(
        db.generation().expect("generation"),
        before,
        "no partial state from the failed judgment"
    );
}

/// Ownership-transfer and early-error accounting through the public work
/// ledger: a reservation charges once, travels with its owner, and refunds
/// exactly once at drop; a refused reservation charges nothing.
#[test]
fn reservations_refund_exactly_once_and_refusals_charge_nothing() {
    let work = bounded(1000, 0);
    let held = work
        .reserve(ByteKind::Working, 600)
        .expect("within allowance");
    assert_eq!(work.used(Resource::WorkingBytes), 600);
    let refused = work.reserve(ByteKind::Working, 500).expect_err("beyond");
    assert!(matches!(
        refused,
        WorkError::Exhausted {
            resource: Resource::WorkingBytes,
            used: 600,
            requested: 500,
            limit: 1000,
        }
    ));
    assert_eq!(
        work.used(Resource::WorkingBytes),
        600,
        "no charge on refusal"
    );
    // The owner moves; the charge moves with it (not with the context).
    let moved = held;
    drop(work.clone());
    assert_eq!(work.used(Resource::WorkingBytes), 600);
    drop(moved);
    assert_eq!(work.used(Resource::WorkingBytes), 0, "exactly one refund");
    drop(work.reserve(ByteKind::Working, 1000).expect("whole again"));
    assert_eq!(work.used(Resource::WorkingBytes), 0);
}
