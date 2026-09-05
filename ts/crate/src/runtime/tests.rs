use std::sync::mpsc::{self, Receiver};

use super::*;

fn options() -> Options {
    Options {
        workers: 1,
        queue_capacity: 2,
        cleanup_capacity: 4,
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
