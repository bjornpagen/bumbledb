//! Engine-backed db-bridge tests (C09/C05 db verbs / RUN / FFI / API).
//! Authored in F1, NEVER run here; F3 executes them. They drive the real
//! machinery below the N-API layer: real runtime registry, real engine,
//! real sealed results — kind/generation/foreign refusals, cancellation
//! joins, bounded chunks.

use super::*;
use crate::runtime::owners::{DirectoryOwner, ManagedDb};
use crate::runtime::{CloseReport, Options};

bumbledb::schema! {
    pub Mini;
    relation Item { a: u64, b: u64 }
    Item(a) -> Item;
}

fn options() -> Options {
    Options {
        workers: 2,
        queue_capacity: 8,
        cleanup_capacity: 8,
        owner_capacity: 4,
        native_handle_capacity: 16,
        aggregate_bytes: [64 << 20; 4],
        chunk_bytes: 1 << 20,
        cleanup_timeout: Duration::from_millis(500),
    }
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 16 << 20,
        working_bytes: 16 << 20,
        scratch_bytes: 16 << 20,
        result_bytes: 16 << 20,
        rows: 1 << 20,
        work_units: 1 << 30,
        timeout: Duration::from_secs(10),
    }
}

fn unique_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "bumbledb-p06-db-{tag}-{}-{seq}",
        std::process::id()
    ))
}

fn acquire(runtime: &Arc<Runtime>, path: &std::path::Path) -> DirectoryOwner {
    let (tx, rx) = std::sync::mpsc::channel();
    let operation = runtime
        .acquire_directory(
            path.to_string_lossy().into_owned(),
            policy(),
            Box::new(move || {
                tx.send(()).unwrap();
            }),
        )
        .expect("acquire submits");
    rx.recv_timeout(Duration::from_secs(10))
        .expect("acquire notify");
    match runtime.take(&operation) {
        Ok(Output::Directory(owner)) => owner,
        _ => panic!("expected a directory owner"),
    }
}

fn attach(owner: &DirectoryOwner, descriptor: &bumbledb::SchemaDescriptor) -> ManagedDb {
    let path = owner.child_path("db").expect("child path");
    let Ok(bumbledb::Admission::Accepted(db)) = crate::Engine::create(&path, descriptor.clone())
    else {
        panic!("engine create accepts a fresh store")
    };
    owner
        .attach_db(crate::assemble_inner(db, descriptor.clone(), Vec::new()))
        .expect("attach db")
}

fn insert_rows(db: &ManagedDb, rows: &[[u64; 2]]) {
    let lease = db.access().expect("lease");
    let admitted = lease
        .db()
        .write(|tx| {
            let descriptor = Mini.descriptor();
            let fields = descriptor.relations[0].fields.clone();
            let owned: Vec<[Value; 2]> = rows
                .iter()
                .map(|row| [Value::U64(row[0]), Value::U64(row[1])])
                .collect();
            let collection =
                bumbledb::AcceptedCollection::from_value_rows(RelationId(0), &fields, owned)
                    .expect("shape-proved rows");
            tx.insert_accepted(&collection).map(|_| ())
        })
        .expect("write commits");
    assert!(matches!(admitted, bumbledb::Admission::Accepted(_)));
}

/// Executes a whole-relation scan query into a sealed [`CompleteResult`]
/// inside the pinned read session, exactly as `runtimeSnapshotExecute` does.
fn sealed_result(runtime: &Arc<Runtime>, db: &ManagedDb) -> (Arc<ResultShared>, u64) {
    let lease = db.access().expect("lease");
    let Output::Session(opened) = runtime.spawn_read_session_for(db, lease).expect("session")
    else {
        panic!("expected a session")
    };
    let query = bumbledb::Query {
        interiors: Vec::new(),
        head: vec![bumbledb::HeadTerm::Var, bumbledb::HeadTerm::Var],
        rules: vec![bumbledb::Rule {
            finds: vec![
                bumbledb::FindTerm::Var(bumbledb::VarId(0)),
                bumbledb::FindTerm::Var(bumbledb::VarId(1)),
            ],
            atoms: vec![bumbledb::Atom {
                source: bumbledb::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (
                        bumbledb::FieldId(0),
                        bumbledb::Term::Var(bumbledb::VarId(0)),
                    ),
                    (
                        bumbledb::FieldId(1),
                        bumbledb::Term::Var(bumbledb::VarId(1)),
                    ),
                ],
            }],
            negated: Vec::new(),
            conditions: Vec::new(),
        }],
        rec: None,
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let operation = opened
        .session
        .submit(
            policy(),
            Box::new(move || {
                tx.send(()).unwrap();
            }),
            move |_| Ok(execute_complete_work(query, Vec::new())),
        )
        .expect("execute submits");
    rx.recv_timeout(Duration::from_secs(10))
        .expect("execute notify");
    let Output::CompleteResult(result) = runtime.take(&operation).expect("execute output") else {
        panic!("expected a sealed result")
    };
    let rows = result.len();
    let retained = runtime.retain_native(result.byte_len()).expect("retained");
    let shared = Arc::new(ResultShared {
        runtime: Arc::clone(runtime),
        slot: Mutex::new(ResultSlot {
            entry: Some(ResultEntry {
                result,
                _retained: retained,
            }),
            spent: false,
        }),
    });
    // The result is OWNED and independent: closing the source session must
    // not disturb it.
    let (tx, rx) = std::sync::mpsc::channel();
    opened.session.drain(Box::new(move |report| {
        tx.send(report).unwrap();
    }));
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(10))
            .expect("session drain"),
        CloseReport::Closed
    );
    (shared, rows)
}

fn drain_runtime(runtime: &Arc<Runtime>) -> CloseReport {
    let (tx, rx) = std::sync::mpsc::channel();
    runtime.drain(
        None,
        Box::new(move |report| {
            tx.send(report).unwrap();
        }),
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("runtime drain")
}

#[test]
fn sealed_results_outlive_their_session_and_collect_leaves_the_backing() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("collect");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);

    let (shared, rows) = sealed_result(&runtime, &db);
    assert_eq!(rows, 3, "the sealed result carries the complete answer set");
    let work = policy().start().unwrap();

    // A byte cap below the sealed size refuses BEFORE materializing and
    // leaves the sealed backing available.
    match collect_result(&shared, &work, 0) {
        Err(RuntimeError::ResourceLimit { dimension, .. }) => {
            assert_eq!(dimension, "resultBytes");
        }
        _ => panic!("a zero-byte collect cap must refuse typed"),
    }
    match collect_result(&shared, &work, 1 << 20).expect("bounded collect") {
        Output::Rows(collected) => assert_eq!(collected.len(), 3),
        _ => panic!("expected rows"),
    }
    // Collect leaves the backing: a second collect still answers.
    match collect_result(&shared, &work, 1 << 20).expect("second collect") {
        Output::Rows(collected) => assert_eq!(collected.len(), 3),
        _ => panic!("expected rows"),
    }

    drop(db);
    drop(owner);
    drop(shared);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn one_shot_transfer_spends_the_result_and_pages_are_byte_bounded() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("pages");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30], [4, 40]]);

    let (shared, _) = sealed_result(&runtime, &db);
    let work = policy().start().unwrap();
    let Output::ResultCursor(cursor) = transfer_result(&shared, &work).expect("transfer") else {
        panic!("expected a cursor")
    };
    // The spend is atomic and one-shot: a second transfer AND a collect
    // both refuse SpentHandle before touching the backing.
    assert!(matches!(
        transfer_result(&shared, &work),
        Err(RuntimeError::SpentHandle)
    ));
    assert!(matches!(
        collect_result(&shared, &work, 1 << 20),
        Err(RuntimeError::SpentHandle)
    ));

    let retained = runtime.retain_native(cursor.byte_len()).expect("retained");
    let cursor_shared = Arc::new(CursorShared {
        runtime: Arc::clone(&runtime),
        slot: Mutex::new(Some(CursorEntry {
            cursor,
            pending: None,
            drained: false,
            _retained: retained,
        })),
    });
    // A 1-byte page cap still delivers AT LEAST one row per page; the
    // overflow row is buffered, never dropped and never double-delivered.
    let mut delivered = 0usize;
    let mut pulls = 0usize;
    loop {
        pulls += 1;
        assert!(pulls < 64, "paging terminates");
        match cursor_pull(&cursor_shared, &work, 1).expect("pull") {
            Output::Page(Some(rows)) => {
                assert_eq!(rows.len(), 1, "a tiny byte cap bounds each page to one row");
                delivered += rows.len();
            }
            Output::Page(None) => break,
            _ => panic!("expected a page"),
        }
    }
    assert_eq!(delivered, 4, "every row is delivered exactly once");
    // EOF is terminal: further pulls stay EOF.
    assert!(matches!(
        cursor_pull(&cursor_shared, &work, 1 << 20).expect("post-EOF pull"),
        Output::Page(None)
    ));
    // Close is join-idempotent; a closed cursor refuses typed.
    {
        let mut slot = cursor_shared.slot.lock().unwrap();
        drop(slot.take());
    }
    assert!(matches!(
        cursor_pull(&cursor_shared, &work, 1),
        Err(RuntimeError::ClosedHandle)
    ));

    drop(db);
    drop(owner);
    drop(cursor_shared);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn drafts_charge_a_cumulative_budget_and_finish_normalizes_add_wins() {
    let runtime = Runtime::start(options()).unwrap();
    let work = policy().start().unwrap();
    let descriptor = Mini.descriptor();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.clone().validate().expect("valid schema")
    };
    let retained = runtime.retain_native(0).expect("retained");
    let shared = Arc::new(DraftShared {
        runtime: Arc::clone(&runtime),
        slot: Mutex::new(DraftSlot {
            entry: Some(DraftEntry {
                schema: Arc::new(schema),
                sealed: Arc::new(crate::seal(descriptor, Vec::new())),
                pending: Vec::new(),
                used_input: 0,
                allowance_input: 100,
                retained,
            }),
        }),
    });

    // Chunks accumulate against ONE cumulative input budget — the second
    // chunk sees the first chunk's spend, and exhaustion SPENDS the draft.
    let rows = vec![vec![Value::U64(1), Value::U64(10)]];
    match draft_ingest(&shared, &work, 0, true, rows.clone(), 60) {
        Ok(Output::Mutation { submitted, .. }) => assert_eq!(submitted, 1),
        _ => panic!("first chunk admits"),
    }
    match draft_ingest(&shared, &work, 0, true, rows.clone(), 60) {
        Err(RuntimeError::ResourceLimit {
            dimension, used, ..
        }) => {
            assert_eq!(dimension, "inputBytes");
            assert_eq!(used, 60, "the budget is cumulative, never reset per chunk");
        }
        _ => panic!("the cumulative budget must refuse the second chunk"),
    }
    // The failed draft is SPENT: later ingestion and finish refuse.
    assert!(matches!(
        draft_ingest(&shared, &work, 0, true, rows, 1),
        Err(RuntimeError::SpentHandle)
    ));
    assert!(matches!(
        draft_finish(&shared, &work),
        Err(RuntimeError::SpentHandle)
    ));

    // A fresh draft: same-command add-wins over delete of the identical
    // fact (the engine's one-command normalization).
    let descriptor = Mini.descriptor();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.clone().validate().expect("valid schema")
    };
    let retained = runtime.retain_native(0).expect("retained");
    let fresh = Arc::new(DraftShared {
        runtime: Arc::clone(&runtime),
        slot: Mutex::new(DraftSlot {
            entry: Some(DraftEntry {
                schema: Arc::new(schema),
                sealed: Arc::new(crate::seal(descriptor, Vec::new())),
                pending: Vec::new(),
                used_input: 0,
                allowance_input: 1 << 20,
                retained,
            }),
        }),
    });
    let row = vec![vec![Value::U64(7), Value::U64(70)]];
    draft_ingest(&fresh, &work, 0, false, row.clone(), 16).expect("delete admits");
    draft_ingest(&fresh, &work, 0, true, row, 16).expect("insert admits");
    let Output::Changes(changes) = draft_finish(&fresh, &work).expect("finish") else {
        panic!("expected a sealed change set")
    };
    assert_eq!(
        changes.changes.len(),
        1,
        "one normalized effect: add wins over delete of the identical fact"
    );
    // Finish consumed the draft.
    assert!(matches!(
        draft_finish(&fresh, &work),
        Err(RuntimeError::SpentHandle)
    ));

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
}

#[test]
fn apply_is_witnessed_judged_and_refuses_a_second_writer() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("apply");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let descriptor = Mini.descriptor();
    let db = attach(&owner, &descriptor);
    let work = policy().start().unwrap();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.clone().validate().expect("valid schema")
    };

    let mut builder = bumbledb::ChangeSet::builder(&schema, work.clone());
    builder
        .insert(RelationId(0), &[Value::U64(1), Value::U64(10)])
        .expect("stages");
    let changes = builder.finish().expect("seals");

    let lease = db.access().expect("lease");
    let store_hex = lease.db().integration_store().identity().store.to_string();

    // A stale expected witness is a DOMAIN outcome (`moved`), never an
    // error; a foreign store witness refuses typed.
    match apply_change_set(
        &lease,
        &changes,
        &ExpectedOwned::Exact {
            store: store_hex.clone(),
            generation: 999,
        },
        &work,
    )
    .expect("moved is a domain outcome")
    {
        Output::Apply(ApplyOutcomeOwned::Moved { witnessed, .. }) => assert_eq!(witnessed, 999),
        _ => panic!("expected moved"),
    }
    assert!(matches!(
        apply_change_set(
            &lease,
            &changes,
            &ExpectedOwned::Exact {
                store: "00".repeat(16),
                generation: 0,
            },
            &work,
        ),
        Err(RuntimeError::Engine { .. })
    ));

    // The accepted apply commits once; re-applying the same set is
    // no-change (set semantics, one normalized final effect).
    let accepted_generation =
        match apply_change_set(&lease, &changes, &ExpectedOwned::Any, &work).expect("applies") {
            Output::Apply(ApplyOutcomeOwned::Accepted { generation, store }) => {
                assert_eq!(store, store_hex);
                generation
            }
            _ => panic!("expected accepted"),
        };
    match apply_change_set(&lease, &changes, &ExpectedOwned::Any, &work).expect("re-applies") {
        Output::Apply(ApplyOutcomeOwned::NoChange { generation, .. }) => {
            assert!(generation >= accepted_generation);
        }
        _ => panic!("expected no-change"),
    }

    // A live write session's admission flag fences apply with WriterBusy —
    // refusal, never a parked worker.
    lease
        .writing
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(matches!(
        apply_change_set(&lease, &changes, &ExpectedOwned::Any, &work),
        Err(RuntimeError::WriterBusy)
    ));
    lease
        .writing
        .store(false, std::sync::atomic::Ordering::Release);

    drop(lease);
    drop(db);
    drop(owner);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_row_codec_is_the_change_set_grammar_and_refuses_foreign_records() {
    let work = policy().start().unwrap();
    let descriptor = Mini.descriptor();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.validate().expect("valid schema")
    };
    let rows = vec![
        vec![Value::U64(2), Value::U64(20)],
        vec![Value::U64(1), Value::U64(10)],
        vec![Value::U64(1), Value::U64(10)],
    ];
    let bytes = encode_rows_bytes(&schema, RelationId(0), &rows, &work).expect("encodes");
    // Set semantics: the duplicate deduplicates; canonical order decides.
    let decoded = decode_rows_values(&schema, RelationId(0), &bytes, &work).expect("decodes");
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0], vec![Value::U64(1), Value::U64(10)]);
    // Foreign-relation payloads refuse typed: the same bytes decoded as
    // another relation are never silently reinterpreted.
    assert!(decode_rows_values(&schema, RelationId(1), &bytes, &work).is_err());
    // Tampered bytes refuse through the strict parser.
    let mut tampered = bytes;
    let last = tampered.len() - 1;
    tampered[last] ^= 0xff;
    assert!(decode_rows_values(&schema, RelationId(0), &tampered, &work).is_err());
}
