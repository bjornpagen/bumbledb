//! Addon delivery, draft, codec and snapshot ownership tests.
//!
//! Sensitivity (D25): a post-register checkpoint that drops `QueuedOutput`
//! loses the consumed page. Resource abort must retry the same row;
//! adopt-and-abort must leave nothing a fresh ticket can commit;
//! oversized first row refuses unchanged; terminal store failure is never
//! lawful EOF.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bumbledb::work::{ExecutionPolicy, WorkContext, WorkError};
use bumbledb::{Answers, DeliveryTicket, RelationId, Value};

use super::delivery::{
    PullOutcome, is_terminal_backing, page_row_cap, publish_from_payload, pull_from_payload,
    register_page,
};
use super::*;
use crate::marshal::ValueOut;
use crate::runtime::owners::{DirectoryOwner, ManagedDb};
use crate::runtime::registry::registry_draft::DraftPayload;
use crate::runtime::registry::{Capability, NativeKind, Payload, RegistryAdmission, ResultState};
use crate::runtime::{CloseReport, DraftLedger, Options, Output, Runtime, RuntimeError};

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
    policy().start().unwrap()
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
                    publish_from_payload(payload, context, 1 << 20, publication)
                }))
            },
        )
        .expect("publish submits");
    rx.recv_timeout(Duration::from_secs(10)).expect("notify");
    runtime.take(&operation)
}

#[test]
fn d12_oversized_first_row_refuses_and_retry_delivers_same_row() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("oversized-retry");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);
    let mut payload = cursor_payload(&runtime, &db);
    let ctx = work();

    match pull_from_payload(&mut payload, &ctx, 0) {
        Err(RuntimeError::ResourceLimit { dimension, .. }) => {
            assert_eq!(dimension, "resultBytes");
        }
        Ok(PullOutcome::Eof) => panic!("oversized first row must not be EOF"),
        Ok(PullOutcome::Page { .. }) => panic!("oversized first row must not deliver"),
        Ok(PullOutcome::Terminal(_)) => panic!("oversized first row is not backing failure"),
        Err(other) => panic!("expected resultBytes refusal, got {other:?}"),
    }

    // Abort left next_row unmoved: retry under allowance delivers a
    // multirow page starting at the same first row.
    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("retry") {
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
            let _owner = handoff.charge;
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

    assert!(matches!(
        pull_from_payload(&mut payload, &ctx, 0),
        Err(RuntimeError::ResourceLimit {
            dimension: "resultBytes",
            ..
        })
    ));

    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("same rows") {
        PullOutcome::Page { queued, terminal } => {
            assert_eq!(queued.rows.len(), 2);
            assert_eq!(first_key(&queued), 7);
            assert!(terminal);
            let _owner = queued.charge;
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
fn d25_multirow_page_under_allowance() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("multirow");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);
    let ctx = work();
    assert_eq!(page_row_cap(&ctx, 3), 3);
    let mut payload = cursor_payload(&runtime, &db);

    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("batch") {
        PullOutcome::Page { queued, terminal } => {
            assert_eq!(
                queued.rows.len(),
                3,
                "into_cursor must use the work/remaining row cap, not 1"
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
    let admission = RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Cursor, 64, payload)
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

    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("first") {
        PullOutcome::Page { queued, .. } => assert_eq!(first_key(&queued), 1),
        PullOutcome::Eof | PullOutcome::Terminal(_) => panic!("expected a page"),
    }
    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("after abort") {
        PullOutcome::Page { queued, .. } => assert_eq!(first_key(&queued), 1),
        PullOutcome::Eof => panic!("abort must not skip to EOF"),
        PullOutcome::Terminal(_) => panic!("abort is not backing failure"),
    }

    let admission = RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Cursor, 64, payload)
        .expect("admit");
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

    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("page") {
        PullOutcome::Page { queued, .. } => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 3);
        }
        PullOutcome::Eof | PullOutcome::Terminal(_) => panic!("expected a page"),
    }
    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("retry abort") {
        PullOutcome::Page { queued, .. } => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 3);
        }
        PullOutcome::Eof => panic!("aborted pull must not skip rows"),
        PullOutcome::Terminal(_) => panic!("abort is not backing failure"),
    }

    let payload = cursor_payload(&runtime, &db);
    let admission = RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Cursor, 64, payload)
        .expect("admit");
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
fn d12_overlap_reserve_refusal_retries_same_first_row() {
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
        cursor.rebind_work(&ctx);
        let mut ticket = DeliveryTicket::open(cursor);
        assert!(
            ticket
                .preview_page(&ctx, 1 << 20)
                .expect("preview")
                .is_some()
        );
        let answers = ticket.adopt().expect("adopt");
        let starved = ExecutionPolicy {
            input_bytes: 16 << 20,
            working_bytes: 16 << 20,
            scratch_bytes: 16 << 20,
            result_bytes: 0,
            rows: 1 << 20,
            work_units: 1 << 30,
            timeout: Duration::from_secs(10),
        }
        .start()
        .unwrap();
        match register_page(&starved, &answers) {
            Err(
                RuntimeError::Work(WorkError::Exhausted {
                    resource: bumbledb::work::Resource::ResultBytes,
                    ..
                })
                | RuntimeError::ResourceLimit {
                    dimension: "resultBytes",
                    ..
                },
            ) => {}
            other => panic!("overlap reserve must refuse, got {other:?}"),
        }
        ticket.abort();
    }

    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("retry same cursor") {
        PullOutcome::Page { queued, .. } => {
            assert_eq!(first_key(&queued), 1);
            assert_eq!(queued.rows.len(), 2);
        }
        PullOutcome::Eof => panic!("budget refusal must not commit or skip to EOF"),
        PullOutcome::Terminal(_) => panic!("budget refusal must not poison the cursor"),
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
        cursor.rebind_work(&ctx);
        let mut ticket = DeliveryTicket::open(cursor);
        assert!(
            ticket
                .preview_page(&ctx, 1 << 20)
                .expect("preview")
                .is_some()
        );
        assert!(ticket.adopt().is_some());
        ticket.abort();
    }
    {
        let Payload::Cursor { cursor, .. } = &mut payload else {
            panic!("expected a cursor")
        };
        DeliveryTicket::open(cursor).commit();
    }

    match pull_from_payload(&mut payload, &ctx, 1 << 20).expect("unpreviewed commit is a no-op") {
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
    let budget = RuntimeError::ResourceLimit {
        dimension: "resultBytes",
        used: 0,
        requested: 32,
        limit: 0,
    };
    assert!(!is_terminal_backing(&budget));
}

fn draft_payload(allowance_input: u64, allowance_rows: u64) -> DraftPayload {
    use bumbledb::schema::ValidateDescriptor as _;
    let schema = Mini.descriptor().validate().expect("valid schema");
    DraftPayload {
        schema: Arc::new(schema),
        pending: Vec::new(),
        used_input: 0,
        used_rows: 0,
        allowance_input,
        allowance_rows,
        ledger: DraftLedger {
            used_work: 0,
            allowance_work: 1 << 30,
            deadline: Instant::now() + Duration::from_secs(10),
            terminal: false,
        },
    }
}

#[test]
fn d07_draft_chunks_share_one_cumulative_budget_and_failure_is_terminal() {
    let ctx = work();
    let mut payload = Payload::Draft(draft_payload(100, 16));
    let rows = vec![vec![Value::U64(1), Value::U64(10)]];
    match ingest_from_payload(&mut payload, &ctx, 0, true, rows.clone(), 60) {
        Ok(Output::Mutation { submitted, .. }) => assert_eq!(submitted, 1),
        other => panic!(
            "first chunk admits, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }
    match ingest_from_payload(&mut payload, &ctx, 0, true, rows.clone(), 60) {
        Err(RuntimeError::ResourceLimit {
            dimension, used, ..
        }) => {
            assert_eq!(dimension, "inputBytes");
            assert_eq!(used, 60);
        }
        other => panic!(
            "cumulative budget must refuse, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }
    assert!(matches!(
        ingest_from_payload(&mut payload, &ctx, 0, true, rows, 1),
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
    let mut payload = Payload::Draft(draft_payload(1 << 20, 16));
    let row = vec![vec![Value::U64(7), Value::U64(70)]];
    ingest_from_payload(&mut payload, &ctx, 0, false, row.clone(), 16).expect("delete");
    ingest_from_payload(&mut payload, &ctx, 0, true, row, 16).expect("insert");
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
fn d18_sealed_results_outlive_their_session_and_collect_is_bounded() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("collect");
    std::fs::create_dir_all(&base).unwrap();
    let owner = acquire(&runtime, &base.join("tenant"));
    let db = attach(&owner, &Mini.descriptor());
    insert_rows(&db, &[[1, 10], [2, 20], [3, 30]]);

    let (mut payload, rows) = sealed_result(&runtime, &db);
    assert_eq!(rows, 3);
    let ctx = work();
    match collect_from_payload(&mut payload, &ctx, 0, 1 << 20) {
        Err(RuntimeError::ResourceLimit { dimension, .. }) => {
            assert_eq!(dimension, "resultBytes");
        }
        other => panic!(
            "zero-byte collect must refuse, got {:?}",
            other.map(|_| "unexpected successful output")
        ),
    }
    match collect_from_payload(&mut payload, &ctx, 1 << 20, 1 << 20).expect("bounded collect") {
        Output::Rows(queued) => assert_eq!(queued.rows.len(), 3),
        _ => panic!("expected rows"),
    }
    match collect_from_payload(&mut payload, &ctx, 1 << 20, 1 << 20).expect("second collect") {
        Output::Rows(queued) => assert_eq!(queued.rows.len(), 3),
        _ => panic!("expected rows"),
    }

    drop(db);
    drop(owner);
    drop(payload);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
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
        collect_from_payload(&mut payload, &ctx, 1 << 20, 1 << 20),
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
    let admission = RegistryAdmission::admit(Arc::clone(&runtime), NativeKind::Result, 64, payload)
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
fn row_codec_payloads_retain_exact_result_capacity_and_refund_on_refusal() {
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::work::Resource;

    bumbledb::schema! {
        pub PayloadRows;
        relation Entry { id: uuid, text: str, bytes: bytes<16> }
    }
    let schema = PayloadRows.descriptor().validate().unwrap();
    let text = "\u{1f41d}".repeat(4097);
    let rows = vec![
        vec![
            Value::Uuid(bumbledb::Uuid::from_u128(1)),
            Value::String(text.clone().into()),
            Value::FixedBytes(Box::new([7; 16])),
        ],
        vec![
            Value::Uuid(bumbledb::Uuid::from_u128(2)),
            Value::String(text.clone().into()),
            Value::FixedBytes(Box::new([8; 16])),
        ],
    ];
    let encode = work();
    let encoded = encode_rows_bytes(&schema, RelationId(0), &rows, &encode).unwrap();
    assert_eq!(encode.used(Resource::WorkingBytes), 0);
    assert_eq!(
        encode.used(Resource::ResultBytes),
        encoded.bytes.capacity() as u64
    );
    assert_eq!(encoded.charge.bytes(), encoded.bytes.capacity() as u64);

    let result_bytes =
        2 * (size_of::<Vec<ValueOut>>() + 3 * size_of::<ValueOut>() + 36 + text.len() + 16) as u64;
    let decode = ExecutionPolicy {
        result_bytes,
        ..policy()
    }
    .start()
    .unwrap();
    let decoded = decode_rows_values(&schema, RelationId(0), &encoded.bytes, &decode).unwrap();
    assert_eq!(decode.used(Resource::WorkingBytes), 0);
    assert_eq!(decode.used(Resource::ResultBytes), result_bytes);
    assert_eq!(decoded.charge.bytes(), result_bytes);
    assert!(
        matches!(&decoded.rows[0][0], ValueOut::Uuid(id) if id == "00000000-0000-0000-0000-000000000001")
    );
    assert!(matches!(&decoded.rows[1][1], ValueOut::Text(value) if value == &text));
    assert!(matches!(&decoded.rows[1][2], ValueOut::Bytes(value) if value.as_slice() == [8; 16]));
    drop(decoded);
    assert_eq!(decode.used(Resource::ResultBytes), 0);

    let refused = ExecutionPolicy {
        result_bytes: result_bytes - 1,
        ..policy()
    }
    .start()
    .unwrap();
    assert!(matches!(
        decode_rows_values(&schema, RelationId(0), &encoded.bytes, &refused),
        Err(RuntimeError::Work(WorkError::Exhausted {
            resource: Resource::ResultBytes,
            ..
        }))
    ));
    assert_eq!(refused.used(Resource::WorkingBytes), 0);
    assert_eq!(refused.used(Resource::ResultBytes), 0);
    let refused = ExecutionPolicy {
        result_bytes: 0,
        ..policy()
    }
    .start()
    .unwrap();
    assert!(matches!(
        encode_rows_bytes(&schema, RelationId(0), &rows, &refused),
        Err(RuntimeError::Work(WorkError::Exhausted {
            resource: Resource::ResultBytes,
            ..
        }))
    ));
    assert_eq!(refused.used(Resource::WorkingBytes), 0);
    assert_eq!(refused.used(Resource::ResultBytes), 0);
    drop(encoded);
    assert_eq!(encode.used(Resource::ResultBytes), 0);
}

#[test]
fn changes_preserve_cancellation_and_resource_errors_through_the_bridge() {
    use bumbledb::schema::ValidateDescriptor as _;
    let schema = Mini.descriptor().validate().unwrap();
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        decode_rows_values(&schema, RelationId(0), &[], &cancelled),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    let refused = ExecutionPolicy {
        working_bytes: 0,
        ..policy()
    }
    .start()
    .unwrap();
    assert!(matches!(
        encode_rows_bytes(
            &schema,
            RelationId(0),
            &[vec![Value::U64(1), Value::U64(2)]],
            &refused
        ),
        Err(RuntimeError::Work(WorkError::Exhausted {
            resource: bumbledb::work::Resource::WorkingBytes,
            ..
        }))
    ));
    assert_eq!(refused.used(bumbledb::work::Resource::WorkingBytes), 0);
    for error in [
        ChangeError::Work(WorkError::Cancelled),
        ChangeError::Row(bumbledb::canonical::RowError::Work(WorkError::Cancelled)),
    ] {
        assert_eq!(
            change_error(&error),
            RuntimeError::Work(WorkError::Cancelled)
        );
    }
}

#[test]
fn d01_answers_out_charges_empty_page_without_escaping() {
    let ctx = ExecutionPolicy {
        input_bytes: 16,
        working_bytes: 16,
        scratch_bytes: 16,
        result_bytes: 8,
        rows: 16,
        work_units: 16,
        timeout: Duration::from_secs(5),
    }
    .start()
    .unwrap();
    let answers = Answers::new();
    let (rows, charge) = crate::marshal::answers_out_charged(&ctx, &answers)
        .expect("empty conversion is a zero-charge owner");
    assert!(rows.is_empty());
    assert_eq!(charge.bytes(), 0);
}

#[test]
fn point_read_output_retains_its_own_capacity_charge_after_decode_drops() {
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
    let output = crate::marshal::row_out_charged(&ctx, &row).expect("admitted output");
    let capacity = (3 * size_of::<ValueOut>() + 7 + 16) as u64;
    assert_eq!(output.charge.bytes(), capacity);
    drop(row);
    drop(encoded);
    assert_eq!(ctx.used(bumbledb::work::Resource::WorkingBytes), 0);
    assert_eq!(ctx.used(bumbledb::work::Resource::ResultBytes), capacity);
    assert!(matches!(&output.values[1], ValueOut::Text(text) if text == "payload"));
    assert!(matches!(&output.values[2], ValueOut::Bytes(bytes) if bytes.as_slice() == [9; 16]));
    drop(output);
    assert_eq!(ctx.used(bumbledb::work::Resource::ResultBytes), 0);

    let refused = ExecutionPolicy {
        result_bytes: capacity - 1,
        ..policy()
    }
    .start()
    .unwrap();
    let encoded = bumbledb::canonical::CanonicalRow::encode(fields, &values, &refused).unwrap();
    let row = bumbledb::canonical::decode(fields, encoded.as_bytes(), &refused).unwrap();
    let retained_source = refused.used(bumbledb::work::Resource::WorkingBytes);
    assert!(matches!(
        crate::marshal::row_out_charged(&refused, &row),
        Err(RuntimeError::Work(WorkError::Exhausted {
            resource: bumbledb::work::Resource::ResultBytes,
            ..
        }))
    ));
    assert_eq!(refused.used(bumbledb::work::Resource::ResultBytes), 0);
    assert_eq!(
        refused.used(bumbledb::work::Resource::WorkingBytes),
        retained_source
    );
}

#[test]
fn a_cancelled_draft_chunk_spends_the_draft_without_fabricating_usage() {
    let mut payload = Payload::Draft(draft_payload(1024, 16));
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        ingest_from_payload(
            &mut payload,
            &cancelled,
            0,
            true,
            vec![vec![Value::U64(1), Value::U64(2)]],
            16
        ),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    let Payload::Draft(entry) = &payload else {
        panic!("draft")
    };
    assert!(entry.ledger.terminal);
    assert_eq!(entry.used_input, 0);
    assert_eq!(entry.used_rows, 0);
    assert!(matches!(
        finish_from_payload(&mut payload, &work()),
        Err(RuntimeError::SpentHandle)
    ));
}
