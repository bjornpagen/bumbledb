//! Addon delivery, draft, codec and snapshot ownership tests.
//!
//! Sensitivity (D25): a post-register checkpoint that drops `QueuedOutput`
//! loses the consumed page. Resource abort must retry the same row;
//! adopt-and-abort must leave nothing a fresh ticket can commit;
//! oversized first row refuses unchanged; terminal store failure is never
//! lawful EOF.

use std::sync::Arc;
use std::time::Duration;

use bumbledb::work::{WorkContext, WorkError};
use bumbledb::{DeliveryTicket, RelationId, Value};

use super::delivery::{PullOutcome, is_terminal_backing, publish_from_payload, pull_from_payload};
use super::*;
use crate::marshal::ValueOut;
use crate::runtime::owners::{DirectoryOwner, ManagedDb};
use crate::runtime::registry::registry_draft::DraftPayload;
use crate::runtime::registry::{Capability, NativeKind, Payload, RegistryAdmission, ResultState};
use crate::runtime::{CloseReport, Options, Output, Runtime, RuntimeError};

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
        cleanup_timeout: Duration::from_millis(500),
    }
}

fn policy() -> WorkContext {
    WorkContext::new()
}

fn unique_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "bumbledb-l13-db-{tag}-{}-{seq}",
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
    let Ok(bumbledb::Admission::Accepted(db)) =
        crate::Engine::create(&path, descriptor.clone(), work())
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
        .write(work(), |tx| {
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

fn work() -> WorkContext {
    policy()
}

// ---- D25 / D12 consumer counterexamples on the native pull -----------------

fn cursor_payload(runtime: &Arc<Runtime>, db: &ManagedDb) -> Payload {
    let (mut payload, _) = sealed_result(runtime, db);
    let ctx = work();
    let Output::ResultCursor(cursor) = transfer_from_payload(&mut payload, &ctx).expect("transfer")
    else {
        panic!("expected a cursor")
    };
    Payload::Cursor {
        cursor,
        drained: false,
    }
}

fn first_key(queued: &crate::runtime::QueuedOutput) -> u64 {
    match queued.rows.first().and_then(|row| row.first()) {
        Some(ValueOut::U64(key)) => *key,
        _ => panic!("expected a u64 key cell"),
    }
}

fn submit_publish(runtime: &Arc<Runtime>, cap: Capability) -> Result<Output, RuntimeError> {
    let (tx, rx) = std::sync::mpsc::channel();
    let operation = runtime
        .submit_payload(
            cap,
            policy(),
            Box::new(move || {
                tx.send(()).unwrap();
            }),
            |_| {
                Ok(Box::new(move |context, payload, publication| {
                    publish_from_payload(payload, context, publication)
                }))
            },
        )
        .expect("publish submits");
    rx.recv_timeout(Duration::from_secs(10)).expect("notify");
    runtime.take(&operation)
}

#[test]
fn d12_cancelled_pull_refuses_and_retry_delivers_same_row() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("oversized-retry");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);
    let mut payload = cursor_payload(&runtime, &db);
    let ctx = work();

    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        pull_from_payload(&mut payload, &cancelled),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));

    // Cancellation left next_row unmoved: a fresh delivery yields a
    // multirow page starting at the same first row.
    match pull_from_payload(&mut payload, &ctx).expect("retry") {
        outcome @ PullOutcome::Page { .. } => {
            let PullOutcome::Page { queued, terminal } = &outcome else {
                unreachable!()
            };
            assert_eq!(queued.rows.len(), 3);
            assert_eq!(first_key(queued), 1);
            assert!(*terminal);
            let Output::Page(Some(handoff)) = outcome.committed_output().expect("handoff") else {
                panic!("L12 must receive the committed QueuedOutput")
            };
            assert_eq!(handoff.rows.len(), 3);
        }
        PullOutcome::Eof => panic!("retry after abort must not skip to EOF"),
        PullOutcome::Terminal(_) => panic!("retry after abort is not backing failure"),
    }

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d25_abort_retry_same_row_then_commit_keeps_queued_owner() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("abort-retry");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[7, 70], [8, 80]]);
    let mut payload = cursor_payload(&runtime, &db);
    let ctx = work();

    let PullOutcome::Page { queued, .. } = pull_from_payload(&mut payload, &ctx).unwrap() else {
        panic!("healthy preview")
    };
    assert_eq!(first_key(&queued), 7);
    drop(queued);

    match pull_from_payload(&mut payload, &ctx).expect("same rows") {
        PullOutcome::Page { queued, terminal } => {
            assert_eq!(queued.rows.len(), 2);
            assert_eq!(first_key(&queued), 7);
            assert!(terminal);
        }
        PullOutcome::Eof => panic!("aborted pull must retry row 7, not EOF"),
        PullOutcome::Terminal(_) => panic!("resource abort is not a closed cursor"),
    }

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d25_delivery_batches_include_multiple_rows() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("multirow");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);
    let ctx = work();
    let mut payload = cursor_payload(&runtime, &db);

    match pull_from_payload(&mut payload, &ctx).expect("batch") {
        PullOutcome::Page { queued, terminal } => {
            assert_eq!(
                queued.rows.len(),
                3,
                "delivery should include multiple rows, not force one-row batches"
            );
            assert_eq!(first_key(&queued), 1);
            assert!(terminal);
        }
        PullOutcome::Eof => panic!("allowance must yield a multirow page, not EOF"),
        PullOutcome::Terminal(_) => panic!("healthy backing is not terminal"),
    }

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d12_arm_cancel_after_page_retries_same_first_row() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("arm-cancel");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);
    let payload = cursor_payload(&runtime, &db);
    let admission = RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Cursor, payload)
        .expect("admit cursor");

    runtime.arm_publication_cancel();
    assert!(
        matches!(
            submit_publish(&runtime, admission.cap()),
            Err(RuntimeError::Work(bumbledb::work::WorkError::Cancelled))
        ),
        "armed cancel drops the local page; next_row must not advance"
    );

    match submit_publish(&runtime, admission.cap()) {
        Ok(Output::Page(Some(queued))) => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 3);
        }
        Ok(Output::Page(None)) => panic!("retry after arm-cancel skipped to EOF"),
        other => panic!(
            "retry must deliver the same first row, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }

    let _ = admission.request_close();
    drop(db);
    drop(owner);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d12_reject_keeps_row_accept_advances() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("accept-reject");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20]]);
    let mut payload = cursor_payload(&runtime, &db);
    let ctx = work();

    match pull_from_payload(&mut payload, &ctx).expect("first") {
        PullOutcome::Page { queued, .. } => assert_eq!(first_key(&queued), 1),
        PullOutcome::Eof | PullOutcome::Terminal(_) => panic!("expected a page"),
    }
    match pull_from_payload(&mut payload, &ctx).expect("after abort") {
        PullOutcome::Page { queued, .. } => assert_eq!(first_key(&queued), 1),
        PullOutcome::Eof => panic!("abort must not skip to EOF"),
        PullOutcome::Terminal(_) => panic!("abort is not backing failure"),
    }

    let admission =
        RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Cursor, payload).expect("admit");
    match submit_publish(&runtime, admission.cap()) {
        Ok(Output::Page(Some(queued))) => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 2);
        }
        other => panic!(
            "live accept must advance after abort-retry, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }
    match submit_publish(&runtime, admission.cap()) {
        Ok(Output::Page(None)) => {}
        Ok(Output::Page(Some(_))) => panic!("accept must not republish the same page"),
        other => panic!(
            "second pull after accept must be EOF, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }
    let _ = admission.request_close();

    drop(db);
    drop(owner);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d12_publication_boundary_cannot_skip_or_duplicate() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("pub-boundary");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);
    let mut payload = cursor_payload(&runtime, &db);
    let ctx = work();

    match pull_from_payload(&mut payload, &ctx).expect("page") {
        PullOutcome::Page { queued, .. } => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 3);
        }
        PullOutcome::Eof | PullOutcome::Terminal(_) => panic!("expected a page"),
    }
    match pull_from_payload(&mut payload, &ctx).expect("retry abort") {
        PullOutcome::Page { queued, .. } => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 3);
        }
        PullOutcome::Eof => panic!("aborted pull must not skip rows"),
        PullOutcome::Terminal(_) => panic!("abort is not backing failure"),
    }

    let payload = cursor_payload(&runtime, &db);
    let admission =
        RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Cursor, payload).expect("admit");
    match submit_publish(&runtime, admission.cap()) {
        Ok(Output::Page(Some(queued))) => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 3);
        }
        other => panic!(
            "live accept must publish one page, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }
    match submit_publish(&runtime, admission.cap()) {
        Ok(Output::Page(None)) => {}
        Ok(Output::Page(Some(_))) => panic!("success must advance once; no duplicate page"),
        other => panic!(
            "second pull after accept must be EOF, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }
    let _ = admission.request_close();

    drop(db);
    drop(owner);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d12_native_conversion_refusal_retries_same_first_row() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("overlap-reserve");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20]]);
    let mut payload = cursor_payload(&runtime, &db);
    let ctx = work();

    {
        let Payload::Cursor { cursor, drained } = &mut payload else {
            panic!("expected a cursor")
        };
        assert!(!*drained);
        let mut ticket = DeliveryTicket::open(cursor);
        let cancelled = work();
        let mut queued = crate::marshal::result_rows(&cancelled, 0).unwrap();
        cancelled.cancel();
        let refused = ticket.visit_page(&ctx, |row| {
            crate::marshal::push_result_row(&cancelled, &mut queued, &row)
        });
        assert!(matches!(refused, Err(bumbledb::Error::Store(_))));
        assert!(queued.rows.is_empty());
        assert_eq!(ticket.previewed_rows(), 0);
        ticket.abort();
    }

    match pull_from_payload(&mut payload, &ctx).expect("retry same cursor") {
        PullOutcome::Page { queued, .. } => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 2);
        }
        PullOutcome::Eof => panic!("conversion cancellation must not commit or skip to EOF"),
        PullOutcome::Terminal(_) => panic!("conversion cancellation must not poison the cursor"),
    }

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d12_adopt_and_abort_cannot_be_committed_by_a_fresh_ticket() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("adopt-abort");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);
    let mut payload = cursor_payload(&runtime, &db);
    let ctx = work();

    {
        let Payload::Cursor { cursor, .. } = &mut payload else {
            panic!("expected a cursor")
        };
        let mut ticket = DeliveryTicket::open(cursor);
        assert!(ticket.preview_page(&ctx).expect("preview").is_some());
        assert!(ticket.adopt().is_some());
        ticket.abort();
    }
    {
        let Payload::Cursor { cursor, .. } = &mut payload else {
            panic!("expected a cursor")
        };
        DeliveryTicket::open(cursor).commit();
    }

    match pull_from_payload(&mut payload, &ctx).expect("unpreviewed commit is a no-op") {
        PullOutcome::Page { queued, .. } => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 3);
        }
        PullOutcome::Eof => panic!("fresh ticket must not commit an aborted preview"),
        PullOutcome::Terminal(_) => panic!("adopt-and-abort is not backing failure"),
    }

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d12_backing_failure_stays_terminal() {
    let store = RuntimeError::Engine {
        kind: crate::tags::error_family::STORE,
        message: "scratch page unreadable".into(),
    };
    let corruption = RuntimeError::Engine {
        kind: crate::tags::error_family::CORRUPTION,
        message: "page unreadable".into(),
    };
    assert!(is_terminal_backing(&store));
    assert!(is_terminal_backing(&corruption));
    assert!(
        PullOutcome::Terminal(store.clone())
            .committed_output()
            .is_err()
    );

    let cancel = RuntimeError::Work(WorkError::Cancelled);
    assert!(!is_terminal_backing(&cancel));
    let allocation = RuntimeError::Work(WorkError::Allocation);
    assert!(!is_terminal_backing(&allocation));
}

fn draft_payload() -> DraftPayload {
    use bumbledb::schema::ValidateDescriptor as _;
    let schema = Mini.descriptor().validate().expect("valid schema");
    DraftPayload {
        schema: Arc::new(schema),
        pending: Vec::new(),
        terminal: false,
    }
}

#[test]
fn draft_chunks_accumulate_without_quotas_and_cancellation_releases_the_prefix() {
    let ctx = work();
    let mut payload = Payload::Draft(draft_payload());
    let rows = vec![vec![Value::U64(1), Value::U64(10)]];
    for _ in 0..2 {
        assert!(matches!(
            ingest_from_payload(&mut payload, &ctx, 0, true, rows.clone()),
            Ok(Output::Mutation { submitted: 1, .. })
        ));
    }
    let Payload::Draft(entry) = &payload else {
        panic!("draft")
    };
    assert_eq!(entry.pending.len(), 2);
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        ingest_from_payload(&mut payload, &cancelled, 0, true, rows.clone()),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    let Payload::Draft(entry) = &payload else {
        panic!("draft")
    };
    assert!(entry.terminal);
    assert_eq!(
        entry.pending.capacity(),
        0,
        "failure releases previously staged rows"
    );
    assert!(matches!(
        ingest_from_payload(&mut payload, &ctx, 0, true, rows),
        Err(RuntimeError::SpentHandle)
    ));
    assert!(matches!(
        finish_from_payload(&mut payload, &ctx),
        Err(RuntimeError::SpentHandle)
    ));
}

#[test]
fn d07_draft_finish_normalizes_add_wins_and_spends() {
    let ctx = work();
    let mut payload = Payload::Draft(draft_payload());
    let row = vec![vec![Value::U64(7), Value::U64(70)]];
    ingest_from_payload(&mut payload, &ctx, 0, false, row.clone()).expect("delete");
    ingest_from_payload(&mut payload, &ctx, 0, true, row).expect("insert");
    let Output::Changes(changes) = finish_from_payload(&mut payload, &ctx).expect("finish") else {
        panic!("expected a sealed change set")
    };
    assert_eq!(changes.changes.len(), 1);
    assert!(matches!(
        finish_from_payload(&mut payload, &ctx),
        Err(RuntimeError::SpentHandle)
    ));
}

// ---- D01 / D18 collect + result lifetime -----------------------------------

fn sealed_result(runtime: &Arc<Runtime>, db: &ManagedDb) -> (Payload, u64) {
    let (opened_tx, opened_rx) = std::sync::mpsc::channel();
    let opening = runtime
        .open_session(
            db,
            policy(),
            Box::new(move || {
                opened_tx.send(()).unwrap();
            }),
        )
        .expect("session submits");
    opened_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("session notify");
    let Output::Session(opened) = runtime.take(&opening).expect("session") else {
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
    let (tx, rx) = std::sync::mpsc::channel();
    opened.session.drain(Box::new(move |report| {
        tx.send(report).unwrap();
    }));
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(10))
            .expect("session drain"),
        CloseReport::Closed
    );
    (
        Payload::Result {
            result: Some(result),
            state: ResultState::Live,
        },
        rows,
    )
}

#[test]
fn d18_sealed_results_outlive_their_session_and_cancelled_collection() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("collect");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);

    let (mut payload, rows) = sealed_result(&runtime, &db);
    assert_eq!(rows, 3);
    let ctx = work();
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        collect_from_payload(&mut payload, &cancelled),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    match collect_from_payload(&mut payload, &ctx).expect("bounded collect") {
        Output::Rows(queued) => assert_eq!(queued.rows.len(), 3),
        _ => panic!("expected rows"),
    }
    match collect_from_payload(&mut payload, &ctx).expect("second collect") {
        Output::Rows(queued) => assert_eq!(queued.rows.len(), 3),
        _ => panic!("expected rows"),
    }

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[cfg(feature = "alloc-counter")]
#[test]
fn native_collection_allocates_only_the_final_scalar_rows() {
    use bumbledb::alloc_counter;

    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("collect-allocations");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    let input: Vec<_> = (0..512).map(|i| [i, i * 10]).collect();
    insert_rows(&db, &input);
    let (mut payload, count) = sealed_result(&runtime, &db);
    drop(db);
    drop(owner);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    // The completed result is independent. Stop background workers before
    // the process-global counter window; nextest isolates this test process.
    let ctx = work();
    for _ in 0..2 {
        alloc_counter::reset();
        let output = collect_from_payload(&mut payload, &ctx).expect("collect completed rows");
        let allocated = alloc_counter::snapshot().window;
        let Output::Rows(queued) = output else {
            panic!("expected rows");
        };
        eprintln!("native scalar collect: {allocated:?}");
        assert_eq!(queued.rows.len() as u64, count);
        assert!(matches!(
            queued.rows[511].as_slice(),
            [ValueOut::U64(511), ValueOut::U64(5110)]
        ));
        assert_eq!(
            allocated.allocs,
            count + 1,
            "one outer vector and one final vector per row; no intermediate Answers"
        );
        assert_eq!(
            allocated.alloc_bytes,
            count * (size_of::<Vec<ValueOut>>() + 2 * size_of::<ValueOut>()) as u64
        );
        let before_drop = alloc_counter::snapshot().window;
        drop(queued);
        let after_drop = alloc_counter::snapshot().window;
        assert_eq!(
            after_drop.dealloc_bytes - before_drop.dealloc_bytes,
            allocated.alloc_bytes
        );
        assert_eq!(after_drop.deallocs - before_drop.deallocs, allocated.allocs);
    }
    drop(payload);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn native_text_collection_owns_one_copy_for_small_and_large_results() {
    bumbledb::schema! {
        pub TextRows;
        relation Item { a: u64, b: str }
    }
    // The larger fixture exceeds 8 MiB without changing result representation.
    // These are delivery allocations, not execution or RSS measurements.
    for (count, text_bytes) in [(128u64, 1024usize), (129, 65536)] {
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("collect-text");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = TextRows.descriptor();
        let db = attach(&owner, &descriptor);
        let text = "\u{1f41d}".repeat(text_bytes / 4);
        {
            let lease = db.access().unwrap();
            for id in 0..count {
                let admitted = lease
                    .db()
                    .write(work(), |tx| {
                        let rows = bumbledb::AcceptedCollection::from_value_rows(
                            RelationId(0),
                            &descriptor.relations[0].fields,
                            [[Value::U64(id), Value::String(text.clone().into())]],
                        )
                        .unwrap();
                        tx.insert_accepted(&rows).map(|_| ())
                    })
                    .unwrap();
                assert!(matches!(admitted, bumbledb::Admission::Accepted(_)));
            }
        }
        let (mut payload, actual_count) = sealed_result(&runtime, &db);
        assert_eq!(actual_count, count);
        drop(db);
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);

        #[cfg(feature = "alloc-counter")]
        let expected_bytes =
            count * (size_of::<Vec<ValueOut>>() + 2 * size_of::<ValueOut>() + text_bytes) as u64;
        let refused = work();
        refused.cancel();
        assert!(matches!(
            collect_from_payload(&mut payload, &refused),
            Err(RuntimeError::Work(WorkError::Cancelled))
        ));
        let mut last = None;
        for _ in 0..2 {
            let delivery = work();
            #[cfg(feature = "alloc-counter")]
            bumbledb::alloc_counter::reset();
            let output = collect_from_payload(&mut payload, &delivery).unwrap();
            #[cfg(feature = "alloc-counter")]
            let allocated = bumbledb::alloc_counter::snapshot().window;
            let Output::Rows(queued) = output else {
                panic!("expected rows");
            };

            assert_eq!(queued.rows.len() as u64, count);
            #[cfg(feature = "alloc-counter")]
            {
                eprintln!("native text collect ({count} x {text_bytes}): {allocated:?}");
                assert_eq!(allocated.allocs, count * 2 + 1);
                assert_eq!(allocated.alloc_bytes, expected_bytes);
            }
            last = Some(queued);
        }
        drop(payload);
        // All engine/session/result owners are gone. The queued output owns
        // complete UTF-8 and exact integers, ready for JavaScript transfer.
        let queued = last.unwrap();
        for (index, row) in queued.rows.iter().enumerate() {
            assert!(
                matches!(row.as_slice(), [ValueOut::U64(id), ValueOut::Text(value)] if *id == index as u64 && value == &text)
            );
        }
        drop(queued);
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[test]
fn d12_one_shot_transfer_spends_and_second_use_refuses() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("transfer");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30], [4, 40]]);

    let (mut payload, _) = sealed_result(&runtime, &db);
    let ctx = work();
    let Output::ResultCursor(_) = transfer_from_payload(&mut payload, &ctx).expect("transfer")
    else {
        panic!("expected a cursor")
    };
    assert!(matches!(
        transfer_from_payload(&mut payload, &ctx),
        Err(RuntimeError::SpentHandle)
    ));
    assert!(matches!(
        collect_from_payload(&mut payload, &ctx),
        Err(RuntimeError::SpentHandle)
    ));

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d18_queued_output_close_drains_without_wrapper_authority() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("queued-close");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20]]);
    let (payload, _) = sealed_result(&runtime, &db);
    let admission = RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Result, payload)
        .expect("admit result");
    let cap = admission.cap();
    let (tx, rx) = std::sync::mpsc::channel();
    close_admitted(
        &runtime,
        cap,
        Box::new(move |report| {
            let _ = tx.send(report);
        }),
    );
    let report = rx.recv_timeout(Duration::from_secs(10)).expect("close");
    assert!(matches!(
        report,
        CloseReport::Closed | CloseReport::Incomplete(_)
    ));

    drop(db);
    drop(owner);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

// ---- apply / codec (unchanged public verbs) --------------------------------

#[test]
fn apply_is_witnessed_judged_and_refuses_a_second_writer() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("apply");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let descriptor = Mini.descriptor();
    let db = attach(&owner, &descriptor);
    let ctx = work();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.clone().validate().expect("valid schema")
    };

    let mut builder = bumbledb::ChangeSet::builder(&schema, ctx.clone());
    builder
        .insert(RelationId(0), &[Value::U64(1), Value::U64(10)])
        .expect("stages");
    let changes = builder.finish().expect("seals");

    let lease = db.access().expect("lease");
    let store_hex = lease.db().integration_store().identity().store.to_string();

    match apply_change_set(
        &lease,
        &changes,
        &ExpectedOwned::Exact {
            store: store_hex.clone(),
            generation: 999,
        },
        &ctx,
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
            &ctx,
        ),
        Err(RuntimeError::Engine { .. })
    ));

    let accepted_generation =
        match apply_change_set(&lease, &changes, &ExpectedOwned::Any, &ctx).expect("applies") {
            Output::Apply(ApplyOutcomeOwned::Accepted { generation, store }) => {
                assert_eq!(store, store_hex);
                generation
            }
            _ => panic!("expected accepted"),
        };
    match apply_change_set(&lease, &changes, &ExpectedOwned::Any, &ctx).expect("re-applies") {
        Output::Apply(ApplyOutcomeOwned::NoChange { generation, .. }) => {
            assert!(generation >= accepted_generation);
        }
        _ => panic!("expected no-change"),
    }

    lease
        .writing
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(matches!(
        apply_change_set(&lease, &changes, &ExpectedOwned::Any, &ctx),
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
fn the_row_codec_borrows_decoded_values_and_refuses_foreign_records() {
    let ctx = work();
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
    let bytes = encode_rows_bytes(&schema, RelationId(0), &rows, &ctx).expect("encodes");
    let decoded = decode_rows_values(&schema, RelationId(0), &bytes.bytes, &ctx).expect("decodes");
    assert_eq!(decoded.rows.len(), 2);
    assert!(matches!(
        decoded.rows[0].as_slice(),
        [ValueOut::U64(1), ValueOut::U64(10)]
    ));
    assert!(decode_rows_values(&schema, RelationId(1), &bytes.bytes, &ctx).is_err());
    // A different u64 payload is still a lawful canonical change set;
    // truncation, not arbitrary scalar mutation, is malformed framing.
    let mut tampered = bytes.bytes.clone();
    tampered.pop();
    assert!(decode_rows_values(&schema, RelationId(0), &tampered, &ctx).is_err());
}

#[test]
fn row_codec_outputs_own_payloads_and_release_their_storage() {
    use bumbledb::schema::ValidateDescriptor as _;
    bumbledb::schema! {
        pub PayloadRows;
        relation Entry { id: uuid, text: str, bytes: bytes<16> }
    }
    let schema = PayloadRows.descriptor().validate().unwrap();
    let text = "\u{1f41d}".repeat(4097);
    let rows: Vec<_> = (1..=2)
        .map(|id| {
            vec![
                Value::Uuid(bumbledb::Uuid::from_u128(id)),
                Value::String(text.clone().into()),
                Value::FixedBytes(Box::new([8; 16])),
            ]
        })
        .collect();
    let context = work();
    let encoded = encode_rows_bytes(&schema, RelationId(0), &rows, &context).unwrap();
    let decoded = decode_rows_values(&schema, RelationId(0), &encoded.bytes, &context).unwrap();
    drop(encoded);
    drop(rows);
    assert!(
        matches!(&decoded.rows[0][0], ValueOut::Uuid(id) if id == "00000000-0000-0000-0000-000000000001")
    );
    assert!(matches!(&decoded.rows[1][1], ValueOut::Text(value) if value == &text));
    assert!(matches!(&decoded.rows[1][2], ValueOut::Bytes(value) if value.as_slice() == [8; 16]));
    #[cfg(feature = "alloc-counter")]
    let before_drop = bumbledb::alloc_counter::snapshot().window;
    drop(decoded);
    #[cfg(feature = "alloc-counter")]
    {
        let after_drop = bumbledb::alloc_counter::snapshot().window;
        let owned_bytes =
            2 * (size_of::<Vec<ValueOut>>() + 3 * size_of::<ValueOut>() + 36 + text.len() + 16);
        assert_eq!(
            after_drop.dealloc_bytes - before_drop.dealloc_bytes,
            owned_bytes as u64
        );
        assert_eq!(after_drop.deallocs - before_drop.deallocs, 9);
    }
}

#[test]
fn changes_preserve_cancellation_and_allocation_errors_through_the_bridge() {
    use bumbledb::schema::ValidateDescriptor as _;
    let schema = Mini.descriptor().validate().unwrap();
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        decode_rows_values(&schema, RelationId(0), &[], &cancelled),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    assert!(matches!(
        encode_rows_bytes(
            &schema,
            RelationId(0),
            &[vec![Value::U64(1), Value::U64(2)]],
            &cancelled
        ),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    for reason in [WorkError::Cancelled, WorkError::Allocation] {
        for error in [
            ChangeError::Work(reason),
            ChangeError::Row(bumbledb::canonical::RowError::Work(reason)),
        ] {
            assert_eq!(change_error(&error), RuntimeError::Work(reason));
        }
    }
}

#[test]
fn input_rows_use_checked_capacity_and_preserve_cancellation() {
    let context = work();
    // Deterministic Vec layout overflow, not an enormous OS allocation attempt.
    assert!(matches!(
        super::codec::reserve_input_rows(u64::MAX, &context),
        Err(RuntimeError::Work(WorkError::Allocation) | RuntimeError::InvalidArgument)
    ));
    let mut rows = super::codec::reserve_input_rows(2, &context).unwrap();
    assert!(rows.capacity() >= 2);
    for value in [Value::U64(7), Value::U64(9)] {
        rows.push(vec![value]);
    }
    assert_eq!(rows, vec![vec![Value::U64(7)], vec![Value::U64(9)]]);
    context.cancel();
    assert!(matches!(
        super::codec::reserve_input_rows(0, &context),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
}

#[test]
fn empty_native_output_needs_no_allocation() {
    let ctx = work();
    #[cfg(feature = "alloc-counter")]
    let before = bumbledb::alloc_counter::snapshot().window;
    let output = crate::marshal::result_rows(&ctx, 0).unwrap();
    #[cfg(feature = "alloc-counter")]
    let after = bumbledb::alloc_counter::snapshot().window;
    assert!(output.rows.is_empty());
    assert_eq!(output.rows.capacity(), 0);
    #[cfg(feature = "alloc-counter")]
    assert_eq!(after.allocs, before.allocs);
}

#[test]
fn point_read_output_outlives_its_decoded_row_and_releases_owned_storage() {
    bumbledb::schema! {
        pub OutputRow;
        relation Entry { id: u64, text: str, bytes: bytes<16> }
    }
    let descriptor = OutputRow.descriptor();
    let fields = &descriptor.relations[0].fields;
    let values = [
        Value::U64(7),
        Value::String("payload".into()),
        Value::FixedBytes(Box::new([9; 16])),
    ];
    let ctx = work();
    let encoded = bumbledb::canonical::CanonicalRow::encode(fields, &values, &ctx).unwrap();
    let row = bumbledb::canonical::decode(fields, encoded.as_bytes(), &ctx).unwrap();
    let output = crate::marshal::queued_row(&ctx, &row).unwrap();
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        crate::marshal::queued_row(&cancelled, &row),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    drop(row);
    drop(encoded);
    assert!(matches!(&output.values[1], ValueOut::Text(text) if text == "payload"));
    assert!(matches!(&output.values[2], ValueOut::Bytes(bytes) if bytes.as_slice() == [9; 16]));
    #[cfg(feature = "alloc-counter")]
    let before_drop = bumbledb::alloc_counter::snapshot().window;
    drop(output);
    #[cfg(feature = "alloc-counter")]
    {
        let after_drop = bumbledb::alloc_counter::snapshot().window;
        assert_eq!(
            after_drop.dealloc_bytes - before_drop.dealloc_bytes,
            (3 * size_of::<ValueOut>() + 7 + 16) as u64
        );
        assert_eq!(after_drop.deallocs - before_drop.deallocs, 3);
    }
}

#[test]
fn a_cancelled_draft_chunk_spends_the_draft_without_fabricating_usage() {
    let mut payload = Payload::Draft(draft_payload());
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        ingest_from_payload(
            &mut payload,
            &cancelled,
            0,
            true,
            vec![vec![Value::U64(1), Value::U64(2)]]
        ),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    let Payload::Draft(entry) = &payload else {
        panic!("draft")
    };
    assert!(entry.terminal);
    assert_eq!(entry.pending.capacity(), 0);
    assert!(matches!(
        finish_from_payload(&mut payload, &work()),
        Err(RuntimeError::SpentHandle)
    ));
}
