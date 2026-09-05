use super::*;

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 10,
        working_bytes: 10,
        scratch_bytes: 10,
        result_bytes: 10,
        rows: 10,
        work_units: 10,
        timeout: Duration::from_secs(60),
    }
}

#[test]
fn zero_never_means_unlimited_and_overflow_never_refunds() {
    let ctx = ExecutionPolicy {
        input_bytes: 0,
        work_units: u64::MAX,
        ..policy()
    }
    .start()
    .unwrap();
    assert_eq!(
        ctx.input(1),
        Err(WorkError::Exhausted {
            resource: Resource::InputBytes,
            used: 0,
            requested: 1,
            limit: 0
        })
    );
    ctx.step(u64::MAX).unwrap();
    assert!(matches!(
        ctx.step(1),
        Err(WorkError::Exhausted { used: u64::MAX, .. })
    ));
    assert_eq!(ctx.used(Resource::WorkUnits), u64::MAX);
}

#[test]
fn clones_share_cumulative_work_and_linear_live_bytes() {
    let ctx = policy().start().unwrap();
    let worker = ctx.clone();
    ctx.input(6).unwrap();
    assert!(worker.input(5).is_err());
    worker.input(4).unwrap();
    let first = ctx.reserve(ByteKind::Working, 6).unwrap();
    assert!(worker.reserve(ByteKind::Working, 5).is_err());
    let second = worker.reserve(ByteKind::Working, 4).unwrap();
    drop(first);
    assert_eq!(ctx.used(Resource::WorkingBytes), 4);
    drop(worker);
    assert_eq!(ctx.used(Resource::WorkingBytes), 4);
    drop(second);
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    assert_eq!(ctx.used(Resource::InputBytes), 10);
}

#[test]
fn joining_reservations_releases_the_donor_ledger_reference() {
    let ctx = policy().start().unwrap();
    let mut owner = ctx.reserve(ByteKind::Working, 1).unwrap();
    for _ in 0..9 {
        owner.join(ctx.reserve(ByteKind::Working, 1).unwrap());
        assert_eq!(
            Arc::strong_count(&ctx.0),
            2,
            "one context and one live owner"
        );
    }
    assert_eq!(owner.bytes(), 10);
    assert_eq!(ctx.used(Resource::WorkingBytes), 10);
    drop(owner);
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    assert_eq!(Arc::strong_count(&ctx.0), 1);
}

#[test]
fn concurrent_admission_cannot_oversubscribe_the_same_allowance() {
    let ctx = policy().start().unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(16));
    std::thread::scope(|scope| {
        let jobs: Vec<_> = (0..16)
            .map(|_| {
                let (ctx, barrier) = (ctx.clone(), Arc::clone(&barrier));
                scope.spawn(move || {
                    barrier.wait();
                    ctx.rows(1).is_ok()
                })
            })
            .collect();
        let admitted = jobs
            .into_iter()
            .map(|job| usize::from(job.join().unwrap()))
            .sum::<usize>();
        assert_eq!(admitted, 10);
    });
    assert_eq!(ctx.used(Resource::Rows), 10);
}

#[test]
fn cancellation_deadline_and_unwind_do_not_leak_reservations() {
    let ctx = policy().start().unwrap();
    let result = std::panic::catch_unwind(|| {
        let _bytes = ctx.reserve(ByteKind::Result, 10).unwrap();
        panic!("owner unwinds");
    });
    assert!(result.is_err());
    assert_eq!(ctx.used(Resource::ResultBytes), 0);
    let queued = ctx.clone();
    ctx.cancel();
    assert_eq!(queued.step(0), Err(WorkError::Cancelled));
    assert!(matches!(
        queued.reserve(ByteKind::Working, 1),
        Err(WorkError::Cancelled)
    ));
    let expired = ExecutionPolicy {
        timeout: Duration::ZERO,
        ..policy()
    }
    .start()
    .unwrap();
    assert_eq!(expired.checkpoint(), Err(WorkError::DeadlineExceeded));
}
