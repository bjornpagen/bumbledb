use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use super::*;
use crate::runtime::registry::Capability;
use crate::runtime::session::PayloadWork;
use crate::runtime::{CloseReport, Operation, Options};
use bumbledb::{RelationId, Theory, Value, WorkError};

bumbledb::schema! {
    pub Items;
    relation Item { id: u64, text: str }
    Item(id) -> Item;
}

fn runtime(workers: usize) -> Arc<Runtime> {
    Runtime::start(Options {
        workers,
        queue_capacity: 8,
        cleanup_capacity: 8,
        owner_capacity: 4,
        native_handle_capacity: 16,
        cleanup_timeout: Duration::from_secs(2),
    })
    .unwrap()
}

fn payload(rows: u64, text: &str) -> Payload {
    use bumbledb::schema::ValidateDescriptor as _;
    let schema = Arc::new(Items.descriptor().validate().unwrap());
    let mut draft = ChangeSet::builder(&schema, WorkContext::new());
    for id in 0..rows {
        draft
            .insert(RelationId(0), &[Value::U64(id), Value::String(text.into())])
            .unwrap();
    }
    let changes = draft.finish().unwrap();
    Payload::Changes {
        fingerprint: crate::hex_fingerprint(&changes.schema().0),
        changes,
        schema,
    }
}

fn admit(runtime: &Arc<Runtime>, payload: Payload, kind: NativeKind) -> Capability {
    RegistryAdmission::admit(Arc::clone(runtime), kind, payload)
        .unwrap()
        .cap()
}

fn submit(
    runtime: &Arc<Runtime>,
    cap: Capability,
    work: PayloadWork,
) -> (Arc<Operation>, Receiver<()>) {
    let (notify, complete) = channel();
    let operation = runtime
        .submit_payload(
            cap,
            WorkContext::new(),
            Box::new(move || {
                notify.send(()).unwrap();
            }),
            |_| Ok(work),
        )
        .unwrap();
    (operation, complete)
}

fn take(
    runtime: &Arc<Runtime>,
    operation: &Arc<Operation>,
    complete: &Receiver<()>,
) -> Result<Output, RuntimeError> {
    complete.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime.take(operation)
}

fn finish(runtime: &Arc<Runtime>) {
    let (notify, complete) = channel();
    runtime.drain(
        None,
        Box::new(move |report| {
            notify.send(report).unwrap();
        }),
    );
    assert_eq!(
        complete.recv_timeout(Duration::from_secs(5)).unwrap(),
        CloseReport::Closed
    );
    assert_eq!(runtime.inspect().natives, 0);
}

fn page(
    runtime: &Arc<Runtime>,
    cap: Capability,
) -> Result<Option<Vec<ChangeRecordWire>>, RuntimeError> {
    let (op, done) = submit(runtime, cap, Box::new(publish_page));
    match take(runtime, &op, &done)? {
        Output::ChangePage(page) => Ok(page),
        _ => panic!("expected change page"),
    }
}

fn first_id(page: &[ChangeRecordWire]) -> u64 {
    match page[0].values[0] {
        ValueOut::U64(id) => id,
        _ => panic!("id"),
    }
}

#[test]
fn bounded_cursor_retries_cancelled_page_without_advancing() {
    let runtime = runtime(2);
    let original = payload(600, "data");
    let Output::ChangesCursor(cursor) = cursor_from_payload(&original).unwrap() else {
        panic!("cursor")
    };
    drop(original);
    let cap = admit(
        &runtime,
        Payload::ChangesCursor(cursor),
        NativeKind::ChangesCursor,
    );
    runtime.arm_publication_cancel();
    assert!(matches!(
        page(&runtime, cap),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    for (id, size) in [(0, 256), (256, 256), (512, 88)] {
        let page = page(&runtime, cap).unwrap().unwrap();
        assert_eq!(page.len(), size);
        assert_eq!(first_id(&page), id);
    }
    assert!(page(&runtime, cap).unwrap().is_none());
    assert!(page(&runtime, cap).unwrap().is_none());
    finish(&runtime);
}

#[test]
fn oversized_records_travel_alone_and_independent_cursors_restart() {
    let runtime = runtime(1);
    let original = payload(3, &"x".repeat(70_000));
    for _ in 0..2 {
        let Output::ChangesCursor(cursor) = cursor_from_payload(&original).unwrap() else {
            panic!("cursor")
        };
        let cap = admit(
            &runtime,
            Payload::ChangesCursor(cursor),
            NativeKind::ChangesCursor,
        );
        for id in 0..3 {
            let page = page(&runtime, cap).unwrap().unwrap();
            assert_eq!(page.len(), 1);
            assert_eq!(first_id(&page), id);
        }
        assert!(page(&runtime, cap).unwrap().is_none());
    }
    finish(&runtime);
}

#[test]
fn composition_routes_without_blocking_on_one_or_many_workers_and_allows_self() {
    for workers in [1, 2] {
        let runtime = runtime(workers);
        let left = admit(&runtime, payload(2, "a"), NativeKind::Changes);
        let right = admit(&runtime, payload(3, "b"), NativeKind::Changes);
        if workers == 2 {
            assert_ne!(left.worker, right.worker);
        }
        for (a, b, count) in [(left, right, 5), (right, left, 5), (left, left, 2)] {
            let (op, done) = submit(
                &runtime,
                a,
                Box::new(move |work, payload, _| compose_from_payload(payload, b, work)),
            );
            let Output::Changes(composed) = take(&runtime, &op, &done).unwrap() else {
                panic!("changes")
            };
            assert_eq!(composed.changes.len(), count);
        }
        finish(&runtime);
    }
}

#[test]
fn cancellation_between_composition_borrows_releases_the_retained_input() {
    composition_gap(true);
    composition_gap(false);
}

fn composition_gap(cancel: bool) {
    let runtime = runtime(2);
    let left = admit(&runtime, payload(2, "a"), NativeKind::Changes);
    let right = admit(&runtime, payload(3, "b"), NativeKind::Changes);
    assert_ne!(left.worker, right.worker);
    let (entered, waiting) = channel();
    let (release, resume) = channel();
    let (blocker, done_blocker) = submit(
        &runtime,
        right,
        Box::new(move |_, _, _| {
            entered.send(()).unwrap();
            resume.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(Output::Ready)
        }),
    );
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let (retained, did_retain) = channel();
    let work = WorkContext::new();
    let (notify, done) = channel();
    let op = runtime
        .submit_payload(
            left,
            work.clone(),
            Box::new(move || {
                notify.send(()).unwrap();
            }),
            |_| {
                Ok(Box::new(move |work, payload, _| {
                    let next = compose_from_payload(payload, right, work)?;
                    retained.send(()).unwrap();
                    Ok(next)
                }))
            },
        )
        .unwrap();
    did_retain.recv_timeout(Duration::from_secs(5)).unwrap();
    // Closing the first route joins its completed stage. The continuation
    // is now enqueued on the blocked second worker with its own Arc share.
    let (closed, drained) = channel();
    runtime
        .close_resource(
            left,
            Box::new(move |report| {
                closed.send(report).unwrap();
            }),
        )
        .unwrap();
    assert_eq!(
        drained.recv_timeout(Duration::from_secs(5)).unwrap(),
        CloseReport::Closed
    );
    if cancel {
        work.cancel();
    }
    release.send(()).unwrap();
    let output = take(&runtime, &op, &done);
    if cancel {
        assert!(matches!(
            output,
            Err(RuntimeError::Work(WorkError::Cancelled))
        ));
    } else {
        let Output::Changes(changes) = output.unwrap() else {
            panic!("composed after source close")
        };
        assert_eq!(changes.changes.len(), 5);
    }
    assert!(matches!(
        take(&runtime, &blocker, &done_blocker),
        Ok(Output::Ready)
    ));
    // The second source is not consumed by either exit.
    let (op, done) = submit(
        &runtime,
        right,
        Box::new(move |work, payload, _| compose_from_payload(payload, right, work)),
    );
    assert!(matches!(take(&runtime, &op, &done), Ok(Output::Changes(_))));
    finish(&runtime);
}
