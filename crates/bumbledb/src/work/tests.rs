use super::*;

#[test]
#[cfg(target_os = "macos")]
fn native_deadline_does_not_enlarge_the_work_ledger() {
    #[expect(
        dead_code,
        reason = "layout-only control for the previous Instant-backed ledger"
    )]
    struct InstantLedger {
        limits: [u64; 6],
        used: [AtomicU64; 6],
        deadline: std::time::Instant,
        cancelled: AtomicBool,
    }
    assert_eq!(
        std::mem::size_of::<Ledger>(),
        std::mem::size_of::<InstantLedger>()
    );
    assert_eq!(
        std::mem::align_of::<Ledger>(),
        std::mem::align_of::<InstantLedger>()
    );
}

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
fn reservation_resize_preserves_old_owner_on_refusal_and_refunds_after_shrink() {
    let ctx = policy().start().unwrap();
    let mut owner = ctx.reserve(ByteKind::Working, 3).unwrap();
    let other = ctx.reserve(ByteKind::Working, 2).unwrap();
    owner.resize(8).unwrap();
    assert_eq!(ctx.used(Resource::WorkingBytes), 10);
    assert!(matches!(
        owner.resize(9),
        Err(WorkError::Exhausted { requested: 1, .. })
    ));
    assert_eq!(owner.bytes(), 8);
    assert_eq!(ctx.used(Resource::WorkingBytes), 10);
    owner.resize(4).unwrap();
    assert_eq!(ctx.used(Resource::WorkingBytes), 6);
    ctx.cancel();
    assert_eq!(owner.resize(5), Err(WorkError::Cancelled));
    owner.resize(0).unwrap();
    assert_eq!(ctx.used(Resource::WorkingBytes), 2);
    drop((owner, other));
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);

    let large = ExecutionPolicy {
        working_bytes: u64::MAX,
        ..policy()
    }
    .start()
    .unwrap();
    let mut owner = large.reserve(ByteKind::Working, 1).unwrap();
    let other = large.reserve(ByteKind::Working, 1).unwrap();
    assert!(
        owner.resize(u64::MAX).is_err(),
        "overflow cannot wrap into admission"
    );
    assert_eq!(owner.bytes(), 1);
    assert_eq!(large.used(Resource::WorkingBytes), 2);
    drop((owner, other));
    assert_eq!(large.used(Resource::WorkingBytes), 0);
}

#[test]
fn growing_reservation_never_refunds_live_bytes() {
    let ctx = policy().start().unwrap();
    let mut owner = ctx.reserve(ByteKind::Result, 3).unwrap();
    owner.grow_to(7).unwrap();
    owner.grow_to(2).unwrap();
    assert_eq!(owner.bytes(), 7);
    assert_eq!(ctx.used(Resource::ResultBytes), 7);
    assert!(matches!(
        owner.grow_to(11),
        Err(WorkError::Exhausted { .. })
    ));
    assert_eq!(owner.bytes(), 7);
    assert_eq!(ctx.used(Resource::ResultBytes), 7);
    ctx.cancel();
    assert_eq!(owner.grow_to(8), Err(WorkError::Cancelled));
    owner.grow_to(0).unwrap();
    assert_eq!(owner.bytes(), 7);
    drop(owner);
    assert_eq!(ctx.used(Resource::ResultBytes), 0);
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
    expired.cancel();
    assert_eq!(
        expired.step(1),
        Err(WorkError::Cancelled),
        "cancellation still wins over deadline and work refusal"
    );
    assert_eq!(expired.used(Resource::WorkUnits), 0);
}
