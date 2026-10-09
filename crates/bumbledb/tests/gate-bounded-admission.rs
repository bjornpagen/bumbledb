//! Batch decision through the host writer: exact diagnostics, cancellation,
//! durable publication, and allocation independent of relation size. The
//! allocation window assumes nextest's one process per test.

use bumbledb::host::{HostChanges, Judged};
use bumbledb::work::WorkContext;
use bumbledb::{Db, RelationId, Value};

mod common;

#[global_allocator]
static GLOBAL: bumbledb::alloc_counter::CountingAllocator =
    bumbledb::alloc_counter::CountingAllocator;

bumbledb::schema! {
    pub GateBounded;

    relation Doc {
        id: u64 as DocId,
        body: str,
    }

    Doc(body) -> Doc;
}

const DOC: RelationId = RelationId(0);
const ROWS: u64 = 2048;

/// Distinct ~1.3 KiB text per row, so the relation is megabytes of rows.
fn body(row: u64) -> String {
    format!(
        "doc-{row:012}-{}",
        "lorem ipsum dolor sit amet consectetur ".repeat(32)
    )
}

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

fn build_store(dir: &std::path::Path) -> Db<GateBounded> {
    let db = Db::create(dir, GateBounded, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        for row in 0..ROWS {
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

fn commit(db: &Db<GateBounded>, work: &WorkContext, changes: &bumbledb::ChangeSet) {
    let mut session = db.host_writer(work).expect("writer");
    let prepared = session
        .apply_decided(std::slice::from_ref(changes))
        .expect("apply");
    let commit = prepared
        .seal(HostChanges::NONE)
        .expect("seal")
        .commit()
        .expect("commit");
    assert!(commit.changed);
}

/// Deciding a one-row insert allocates independently of the megabytes
/// already in the relation, and the decided set then commits durably.
#[test]
fn deciding_a_small_change_allocates_independently_of_the_relation() {
    let dir = common::TempDir::new("gate-bounded-large");
    let db = build_store(dir.path());
    let before = db.generation(common::work()).expect("generation");

    let work = WorkContext::new();
    let changes = small_change(&db, &work, ROWS + 1, &body(ROWS + 1));
    let mut session = db.host_writer(&work).expect("writer");
    let before_alloc = bumbledb::alloc_counter::snapshot().window.alloc_bytes;
    let decided = session
        .decide_all(std::slice::from_ref(&changes))
        .expect("decide");
    let allocated = bumbledb::alloc_counter::snapshot().window.alloc_bytes - before_alloc;
    assert!(
        allocated < 64 << 10,
        "deciding one row allocated {allocated} bytes"
    );
    let [Judged::Accepted(applied)] = decided.as_slice() else {
        panic!("a lawful change was not accepted: {decided:?}");
    };
    assert_eq!((applied.added, applied.removed), (1, 0));
    drop(session);

    commit(&db, &work, &changes);
    assert_ne!(db.generation(common::work()).expect("generation"), before);
    let text = body(ROWS + 1);
    db.read(common::work(), |snap| {
        assert_eq!(
            snap.get(DocByBody { body: &text })?,
            Some(Doc {
                id: DocId(ROWS + 1),
                body: &text,
            })
        );
        Ok(())
    })
    .expect("read back");
}

/// A key conflict with a committed row cites both competitors, with exact
/// truncation, and commits nothing.
#[test]
fn rejection_cites_both_competitors() {
    let dir = common::TempDir::new("gate-bounded-reject");
    let db = build_store(dir.path());
    let before = db.generation(common::work()).expect("generation");

    let work = WorkContext::new();
    let duplicate = body(7);
    let changes = small_change(&db, &work, ROWS + 9, &duplicate);
    let mut session = db.host_writer(&work).expect("writer");
    let decided = session
        .decide_all(std::slice::from_ref(&changes))
        .expect("decide");
    let [Judged::Rejected(violations)] = decided.as_slice() else {
        panic!("a key conflict was not rejected: {decided:?}");
    };
    assert_eq!(violations.len(), 1, "exactly the text key is violated");
    assert!(!violations.examples_truncated(0), "two rows, budget four");
    let mut ids: Vec<Value> = violations
        .cited_facts(0)
        .iter()
        .map(|fact| {
            assert_eq!(
                fact.values()[1],
                Value::String(duplicate.clone().into_boxed_str())
            );
            fact.values()[0].clone()
        })
        .collect();
    ids.sort_by_key(|value| match value {
        Value::U64(id) => *id,
        other => panic!("u64 ids only, got {other:?}"),
    });
    assert_eq!(ids, vec![Value::U64(7), Value::U64(ROWS + 9)]);
    drop(session);
    assert_eq!(db.generation(common::work()).expect("generation"), before);
}

/// A cancelled decision refuses with cancellation and leaves the store as
/// it was; a fresh context decides and commits the same change.
#[test]
fn cancellation_leaves_no_partial_state() {
    let dir = common::TempDir::new("gate-bounded-cancel");
    let db = build_store(dir.path());
    let before = db.generation(common::work()).expect("generation");

    let work = WorkContext::new();
    let changes = small_change(&db, &work, ROWS + 1, &body(ROWS + 1));
    let mut session = db.host_writer(&work).expect("writer");
    work.cancel();
    let error = session
        .decide_all(std::slice::from_ref(&changes))
        .expect_err("cancelled work refuses");
    assert!(error.is_cancelled(), "typed cancellation, got {error:?}");
    drop(session);
    assert_eq!(db.generation(common::work()).expect("generation"), before);

    let fresh = WorkContext::new();
    let changes = small_change(&db, &fresh, ROWS + 1, &body(ROWS + 1));
    let mut session = db.host_writer(&fresh).expect("writer");
    let decided = session
        .decide_all(std::slice::from_ref(&changes))
        .expect("decide");
    assert!(matches!(decided.as_slice(), [Judged::Accepted(_)]));
    drop(session);
    commit(&db, &fresh, &changes);
    assert_ne!(db.generation(common::work()).expect("generation"), before);
}
