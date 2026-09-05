use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use super::*;

fn options() -> Options {
    Options {
        workers: 1,
        queue_capacity: 2,
        cleanup_capacity: 4,
        owner_capacity: 4,
        native_handle_capacity: 8,
        aggregate_bytes: [100; 4],
        chunk_bytes: 100,
        cleanup_timeout: Duration::from_millis(20),
    }
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 10,
        working_bytes: 10,
        scratch_bytes: 10,
        result_bytes: 10,
        rows: 10,
        work_units: 10,
        timeout: Duration::from_secs(5),
    }
}

fn submit(runtime: &Runtime, work: Work) -> (Arc<Operation>, Receiver<()>) {
    let (send, receive) = mpsc::channel();
    let operation = runtime
        .submit(
            policy(),
            Box::new(move || {
                send.send(()).unwrap();
            }),
            |_| Ok(work),
        )
        .unwrap();
    (operation, receive)
}

fn close(runtime: &Runtime) -> CloseReport {
    let (send, receive) = mpsc::channel();
    runtime.drain(
        None,
        Box::new(move |report| {
            send.send(report).unwrap();
        }),
    );
    receive.recv_timeout(Duration::from_secs(2)).unwrap()
}

#[test]
fn retained_completion_keeps_reservations_until_taken() {
    let runtime = Runtime::start(options()).unwrap();
    let (operation, done) = submit(
        &runtime,
        Box::new(|context| {
            context.step(3)?;
            Ok(Output::Ready)
        }),
    );
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(runtime.inspect().reserved, [10; 4]);
    assert!(matches!(runtime.take(&operation), Ok(Output::Ready)));
    assert_eq!(runtime.inspect().reserved, [0; 4]);
    // PINNED (P12's runtimeTake double-take note, decided wave-E): a second
    // take is the TYPED SpentHandle refusal, never a null/None payload —
    // runtime_take and every *Take verb ride this one path.
    assert!(matches!(
        runtime.take(&operation),
        Err(RuntimeError::SpentHandle)
    ));
    assert_eq!(close(&runtime), CloseReport::Closed);
    assert_eq!(close(&runtime), CloseReport::Closed);
}

#[test]
fn saturated_workers_still_report_incomplete_then_reclaim_late_success() {
    let runtime = Runtime::start(options()).unwrap();
    let (release, blocked) = mpsc::channel();
    let (entered, running) = mpsc::channel();
    let (_operation, done) = submit(
        &runtime,
        Box::new(move |_| {
            entered.send(()).unwrap();
            blocked.recv().unwrap(); // Models an OS call which cannot be preempted.
            Ok(Output::Ready)
        }),
    );
    running.recv_timeout(Duration::from_secs(2)).unwrap();
    let (queued, queued_done) = submit(
        &runtime,
        Box::new(|_| panic!("cancelled queue must not execute")),
    );
    assert!(matches!(close(&runtime), CloseReport::Incomplete(_)));
    let inspection = runtime.inspect();
    assert_eq!(inspection.phase, Phase::Closing);
    assert_eq!(inspection.reserved, [20; 4]);
    assert!(matches!(
        runtime.submit(policy(), Box::new(|| {}), |_| Ok(Box::new(|_| Ok(
            Output::Ready
        )))),
        Err(RuntimeError::ClosedHandle)
    ));
    release.send(()).unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    queued_done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(close(&runtime), CloseReport::Closed);
    assert_eq!(runtime.inspect().reserved, [0; 4]);
    assert!(matches!(
        runtime.take(&queued),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
}

#[test]
fn queue_wait_uses_original_deadline_and_cancel_has_reserved_capacity() {
    let runtime = Runtime::start(options()).unwrap();
    let (release, blocked) = mpsc::channel();
    let (entered, running) = mpsc::channel();
    let (first, first_done) = submit(
        &runtime,
        Box::new(move |_| {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
            Ok(Output::Ready)
        }),
    );
    running.recv_timeout(Duration::from_secs(2)).unwrap();
    let (notify, done) = mpsc::channel();
    let mut short = policy();
    short.timeout = Duration::from_millis(1);
    let expired = runtime
        .submit(
            short,
            Box::new(move || {
                notify.send(()).unwrap();
            }),
            |_| Ok(Box::new(|_| panic!("expired queue must not execute"))),
        )
        .unwrap();
    let (third, third_done) = submit(&runtime, Box::new(|_| Ok(Output::Ready)));
    assert!(matches!(
        runtime.submit(policy(), Box::new(|| {}), |_| Ok(Box::new(|_| Ok(
            Output::Ready
        )))),
        Err(RuntimeError::QueueFull)
    ));
    let (notify, cancelled) = mpsc::channel();
    runtime.drain(
        Some(&third),
        Box::new(move |report| {
            notify.send(report).unwrap();
        }),
    );
    // A finite incomplete report, not a timing sleep, proves the original clock expired.
    assert!(matches!(
        cancelled.recv_timeout(Duration::from_secs(2)).unwrap(),
        CloseReport::Incomplete(_)
    ));
    release.send(()).unwrap();
    first_done.recv_timeout(Duration::from_secs(2)).unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    third_done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(matches!(runtime.take(&first), Ok(Output::Ready)));
    assert!(matches!(
        runtime.take(&expired),
        Err(RuntimeError::Work(WorkError::DeadlineExceeded))
    ));
    assert_eq!(close(&runtime), CloseReport::Closed);
}

#[test]
fn aggregate_refusal_happens_before_input_prepare_and_refunds_failures() {
    let mut small = options();
    small.aggregate_bytes = [9; 4];
    let runtime = Runtime::start(small).unwrap();
    assert!(matches!(
        runtime.submit(policy(), Box::new(|| {}), |_| panic!(
            "must reserve before copy"
        )),
        Err(RuntimeError::ResourceLimit { .. })
    ));
    assert_eq!(runtime.inspect().reserved, [0; 4]);
    assert_eq!(close(&runtime), CloseReport::Closed);
    let runtime = Runtime::start(options()).unwrap();
    assert!(matches!(
        runtime.submit(policy(), Box::new(|| {}), |_| Err(
            RuntimeError::InvalidArgument
        )),
        Err(RuntimeError::InvalidArgument)
    ));
    assert_eq!(runtime.inspect().reserved, [0; 4]);
    assert_eq!(close(&runtime), CloseReport::Closed);
}

#[test]
fn panicked_worker_faults_runtime_and_releases_other_operations() {
    let runtime = Runtime::start(options()).unwrap();
    let (operation, done) = submit(&runtime, Box::new(|_| panic!("contained native failure")));
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_ne!(runtime.inspect().phase, Phase::Open);
    assert!(matches!(
        runtime.take(&operation),
        Err(RuntimeError::Internal)
    ));
    assert_eq!(close(&runtime), CloseReport::Closed);
    assert_eq!(runtime.inspect().reserved, [0; 4]);
}

#[test]
fn preparation_panic_faults_and_reclaims_registered_lease() {
    let runtime = Runtime::start(options()).unwrap();
    assert!(matches!(
        runtime.submit(policy(), Box::new(|| {}), |_| panic!("prepare failed")),
        Err(RuntimeError::Internal)
    ));
    assert_eq!(close(&runtime), CloseReport::Closed);
    assert_eq!(runtime.inspect().retained, 0);
}

// --- Managed directory owner (C09 / RUN-05 / REP-009) ---------------------
//
// The transitional Legacy/Managed owner split is gone: every native DB now
// lives in the one runtime registry, acquired behind a kernel-held
// directory lock. These tests drive that owner registry directly (no engine
// or N-API), proving acquisition, path-traversal/reserved-namespace refusal,
// the one close authority reporting Closed, and idempotent close join.

fn owner_options() -> Options {
    Options {
        workers: 2,
        queue_capacity: 4,
        cleanup_capacity: 8,
        owner_capacity: 4,
        native_handle_capacity: 8,
        aggregate_bytes: [1 << 20; 4],
        chunk_bytes: 1 << 16,
        cleanup_timeout: Duration::from_millis(200),
    }
}

fn owner_policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1 << 16,
        working_bytes: 1 << 16,
        scratch_bytes: 1 << 16,
        result_bytes: 1 << 16,
        rows: 1 << 16,
        work_units: 1 << 16,
        timeout: Duration::from_secs(5),
    }
}

fn unique_base(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("bumbledb-p06-{tag}-{}-{seq}", std::process::id()))
}

fn drain_owner(owner: &super::owners::DirectoryOwner) -> CloseReport {
    let (send, receive) = mpsc::channel();
    owner.drain(Box::new(move |report| {
        send.send(report).unwrap();
    }));
    receive.recv_timeout(Duration::from_secs(2)).unwrap()
}

#[test]
fn directory_owner_lifecycle_and_path_safety() {
    let runtime = Runtime::start(owner_options()).unwrap();
    let base = unique_base("lifecycle");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");

    let (send, done) = mpsc::channel();
    let operation = runtime
        .acquire_directory(
            dir.to_string_lossy().into_owned(),
            owner_policy(),
            Box::new(move || {
                send.send(()).unwrap();
            }),
        )
        .unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();

    let Ok(Output::Directory(owner)) = runtime.take(&operation) else {
        panic!("expected a directory owner output")
    };

    // A normal child resolves; traversal, reserved namespaces and the
    // ownership sibling are refused before any path is handed out.
    assert!(owner.child_path("data").is_ok());
    assert!(matches!(
        owner.child_path(".."),
        Err(RuntimeError::InvalidPath)
    ));
    assert!(matches!(
        owner.child_path("."),
        Err(RuntimeError::InvalidPath)
    ));
    assert!(matches!(
        owner.child_path("~lease"),
        Err(RuntimeError::InvalidPath)
    ));
    assert!(matches!(
        owner.child_path("a/b"),
        Err(RuntimeError::InvalidPath)
    ));

    // One close authority: the drain reports the real terminal outcome, and
    // a second close on the same inert owner joins it (idempotent).
    assert_eq!(drain_owner(&owner), CloseReport::Closed);
    assert_eq!(drain_owner(&owner), CloseReport::Closed);
    assert_eq!(runtime.inspect().owners, 0);

    assert_eq!(close(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn suspended_owner_fences_a_second_acquire() {
    // RUN-05/REP-009: while one owner holds the kernel lock, a second
    // acquire of the same path refuses with DirectoryBusy and mutates
    // nothing. Releasing the first lets the next owner in.
    let runtime = Runtime::start(owner_options()).unwrap();
    let base = unique_base("fence");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let path = dir.to_string_lossy().into_owned();

    let (send, done) = mpsc::channel();
    let first_op = runtime
        .acquire_directory(
            path.clone(),
            owner_policy(),
            Box::new(move || {
                send.send(()).unwrap();
            }),
        )
        .unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    let Ok(Output::Directory(first)) = runtime.take(&first_op) else {
        panic!("first acquire must own the directory")
    };

    let (send, done) = mpsc::channel();
    let second_op = runtime
        .acquire_directory(
            path.clone(),
            owner_policy(),
            Box::new(move || {
                send.send(()).unwrap();
            }),
        )
        .unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(
        matches!(runtime.take(&second_op), Err(RuntimeError::DirectoryBusy)),
        "a live owner fences the same path"
    );

    assert_eq!(drain_owner(&first), CloseReport::Closed);

    // With the first owner released, the next acquire succeeds.
    let (send, done) = mpsc::channel();
    let third_op = runtime
        .acquire_directory(
            path,
            owner_policy(),
            Box::new(move || {
                send.send(()).unwrap();
            }),
        )
        .unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    let Ok(Output::Directory(third)) = runtime.take(&third_op) else {
        panic!("acquire after release must succeed")
    };
    assert_eq!(drain_owner(&third), CloseReport::Closed);

    assert_eq!(close(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

// --- C7: control drain and bounded affine lanes (authored; NotRun) --------

#[test]
fn control_lane_teardown_runs_when_ordinary_queue_is_full() {
    let runtime = Arc::new(Runtime::start(options()).unwrap());
    let (release, blocked) = mpsc::channel();
    let (entered, running) = mpsc::channel();
    let (_operation, done) = submit(
        &runtime,
        Box::new(move |_| {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
            Ok(Output::Ready)
        }),
    );
    running.recv_timeout(Duration::from_secs(2)).unwrap();
    let _second = runtime
        .submit(
            policy(),
            Box::new(|| {}),
            |_| Ok(Box::new(|_| Ok(Output::Ready))),
        )
        .unwrap();
    assert!(matches!(
        runtime.submit(
            policy(),
            Box::new(|| {}),
            |_| Ok(Box::new(|_| Ok(Output::Ready)))
        ),
        Err(RuntimeError::QueueFull)
    ));
    let (tx, rx) = mpsc::channel();
    runtime
        .submit_control(
            Box::new(move || {
                tx.send(()).unwrap();
            }),
            Some(Box::new(move |report| {
                rx.send(report).unwrap();
            })),
        )
        .expect("control admits while ordinary queue is saturated");
    release.send(()).unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        CloseReport::Closed
    );
    assert_eq!(close(&runtime), CloseReport::Closed);
}

#[test]
fn d18_idle_shutdown_wakes_sleeping_pool_without_reentering_state() {
    // D18: drain an idle pool. lane_send must not re-lock runtime.state
    // from begin_close/drain (std Mutex deadlock / hang).
    let runtime = Runtime::start(options()).unwrap();
    assert_eq!(runtime.inspect().active, 0);
    assert_eq!(runtime.inspect().phase, Phase::Open);
    assert_eq!(close(&runtime), CloseReport::Closed);
    assert_eq!(runtime.inspect().phase, Phase::Closed);
}

#[test]
fn d29_worker_inbox_wakeup_reaches_a_sleeping_pool() {
    // D24/D29 sensitivity: a sleeping worker must observe an admitted inbox
    // item (lane_send holds the bookkeeping lock across notify; the wait
    // path try_recv's before Condvar::wait). One ready job after idle
    // proves wakeup. Failed admission must not leave a route.
    let runtime = Runtime::start(options()).unwrap();
    assert_eq!(runtime.inspect().active, 0);
    let (operation, done) = submit(&runtime, Box::new(|_| Ok(Output::Ready)));
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(matches!(runtime.take(&operation), Ok(Output::Ready)));
    assert_eq!(close(&runtime), CloseReport::Closed);
}

#[test]
fn d29_repository_lock_kind_is_stamped_on_the_capability() {
    let runtime = Runtime::start(options()).unwrap();
    let cap = runtime
        .reserve_native_route(super::registry::NativeKind::RepositoryLock, 0)
        .expect("lock route");
    assert_eq!(cap.kind, super::registry::NativeKind::RepositoryLock);
    runtime.rollback_native_route(cap);
    assert_eq!(runtime.registry.route_count(), 0);
    assert_eq!(close(&runtime), CloseReport::Closed);
}

#[test]
fn d29_failed_native_admission_does_not_leave_a_route() {
    let runtime = Runtime::start(options()).unwrap();
    assert!(matches!(
        runtime.reserve_native_route(super::registry::NativeKind::Result, u64::MAX),
        Err(RuntimeError::ResourceLimit { .. })
    ));
    assert_eq!(runtime.inspect().natives, 0);
    assert_eq!(runtime.registry.route_count(), 0);
    assert_eq!(runtime.inspect().reserved[3], 0);
    assert_eq!(close(&runtime), CloseReport::Closed);
}

#[test]
fn retained_native_guard_refunds_registry_charge_on_drop() {
    let runtime = Runtime::start(options()).unwrap();
    let guard = runtime.retain_native(7).expect("admit native");
    assert_eq!(runtime.inspect().natives, 1);
    assert_eq!(runtime.inspect().reserved[3], 7);
    drop(guard);
    assert_eq!(runtime.inspect().natives, 0);
    assert_eq!(runtime.inspect().reserved[3], 0);
    assert_eq!(close(&runtime), CloseReport::Closed);
}
