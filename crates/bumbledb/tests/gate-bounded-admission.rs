//! Public integration admission: exact diagnostics, cancellation, durable
//! publication, and observed allocation cost for an indexed one-group update.
//! Allocation windows are meaningful with `alloc-counter` under nextest's
//! one-process-per-test execution; they are not RSS or physical-read counters.

use bumbledb::integration::{AttachmentChange, HostChanges, IntegrationError};
use bumbledb::work::WorkContext;
use bumbledb::{Admission, Db, RelationId, Value, WorkError};

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
/// default is ~2.6 MiB of canonical rows; raise it to GiB scale via
/// `BUMBLEDB_GATE_ROWS` on a storage-qualified runner.
fn gate_rows() -> u64 {
    std::env::var("BUMBLEDB_GATE_ROWS")
        .ok()
        .and_then(|rows| rows.parse().ok())
        .unwrap_or(2048)
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

/// One admitted small change (a single new document) as a sealed
/// `ChangeSet` built with the caller's cancellation context.
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

/// A small update allocates independently of the large existing relation,
/// then durably commits through the real integration path.
#[test]
fn small_change_to_a_large_relation_avoids_relation_sized_allocations() {
    let rows = gate_rows();
    let dir = common::TempDir::new("gate-bounded-large");
    let db = build_store(dir.path(), rows);
    let before = db.generation(common::work()).expect("generation");

    // The indexed one-group update must not scan or spill the existing relation.
    let work = WorkContext::new();
    let changes = small_change(&db, &work, rows + 1, &body(rows + 1));
    let mut session = db.integration_writer(&work).expect("writer");
    #[cfg(feature = "alloc-counter")]
    let before_alloc = bumbledb::alloc_counter::snapshot().window.alloc_bytes;
    let prepared = match session.prepare(&changes).expect("prepare") {
        Admission::Accepted(prepared) => prepared,
        Admission::Rejected(violations) => panic!("a lawful change rejected: {violations}"),
    };
    assert_eq!(prepared.application_changes().added, 1);
    assert_eq!(prepared.application_changes().removed, 0);
    #[cfg(feature = "alloc-counter")]
    assert!(
        bumbledb::alloc_counter::snapshot().window.alloc_bytes - before_alloc < 64 << 10,
        "indexed admission must not materialize the existing relation"
    );
    let sealed = prepared
        .seal(HostChanges {
            records: &[],
            attachment: AttachmentChange::Keep,
        })
        .expect("seal");
    let commit = sealed.commit().expect("commit");
    assert!(commit.changed, "one durable committed change");
    drop(session);

    let after = db.generation(common::work()).expect("generation");
    assert_ne!(after, before, "the generation witnessed the change");
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

/// Constraint checking delivers COMPLETE rejection
/// diagnostics: a change conflicting with a committed row inside the large
/// relation is rejected with the key statement, BOTH competing rows cited
/// (the committed incumbent and the newcomer), and truncation labeled
/// exactly.
#[test]
fn rejection_diagnostics_include_both_competitors_and_exact_truncation() {
    let rows = 2048;
    let dir = common::TempDir::new("gate-bounded-reject");
    let db = build_store(dir.path(), rows);
    let before = db.generation(common::work()).expect("generation");

    let work = WorkContext::new();
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
        db.generation(common::work()).expect("generation"),
        before,
        "a rejection commits nothing"
    );
}

/// Cancellation stops the judgment with the typed refusal and leaves no
/// partial state: the store answers exactly as before, and a fresh session
/// admits normally afterwards.
#[test]
fn cancellation_leaves_no_partial_state() {
    let rows = 2048;
    let dir = common::TempDir::new("gate-bounded-cancel");
    let db = build_store(dir.path(), rows);
    let before = db.generation(common::work()).expect("generation");

    let work = WorkContext::new();
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
    assert_eq!(db.generation(common::work()).expect("generation"), before);

    // The store is unpoisoned: a fresh context admits the same change.
    let fresh = WorkContext::new();
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
    assert_ne!(db.generation(common::work()).expect("generation"), before);
}

/// Ordinary complete judgment must not spill at a hidden byte threshold.
/// A broken TMPDIR catches accidental scratch creation in a subprocess.
#[test]
fn complete_admission_needs_no_temporary_directory() {
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
            "ordinary_admission_child_helper",
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
    // readable, and exactly the loaded rows: the child aborted its candidate.
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
/// builds the store, completes judgment, then aborts without publishing.
#[test]
#[ignore = "subprocess helper for complete_admission_needs_no_temporary_directory"]
fn ordinary_admission_child_helper() {
    if std::env::var("BUMBLEDB_GATE_CHILD").as_deref() != Ok("1") {
        return;
    }
    let store_dir = std::env::var("BUMBLEDB_GATE_STORE").expect("store dir");
    let db = Db::create(
        std::path::Path::new(&store_dir),
        GateBounded,
        common::work(),
    )
    .expect("create")
    .expect("accepted");
    // Ordinary bulk load also works with the broken TMPDIR.
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
    let before = db.generation(common::work()).expect("generation");

    // Complete judgment visits all groups, without a policy-driven spill.
    let work = WorkContext::new();
    let changes = small_change(&db, &work, 5000, &body(5000));
    // Complete verification must visit every group, unlike incremental prepare.
    let judge = bumbledb::store::SchemaJudge::new(db.schema());
    let mut session = db.integration_store().writer(&work).expect("writer");
    match session
        .prepare(&changes, &bumbledb::store::UnindexedRows, &judge)
        .unwrap()
    {
        bumbledb::store::Prepared::Admitted(prepared) => prepared.abort(),
        bumbledb::store::Prepared::Rejected(violations) => {
            panic!("lawful update rejected: {violations:?}")
        }
    }
    drop(session);
    assert_eq!(
        db.generation(common::work()).expect("generation"),
        before,
        "the aborted candidate publishes nothing"
    );
}
