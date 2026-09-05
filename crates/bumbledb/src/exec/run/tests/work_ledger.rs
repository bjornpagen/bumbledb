//! The warm executor's per-execution work ledger (audit-core #2): the
//! join recursion polls cancellation/deadline at the bounded quantum on
//! binding EXPLORATION — not per emitted row — and COLT pool growth is
//! charged to working bytes, so the bounded-restart trigger can fire from
//! join growth. Gate anchors: QRY-002/003, Q-BUDGET, chapter 12 §7.
use super::*;
use crate::work::{ExecutionPolicy, Resource, WorkContext, WorkError};

fn bounded(working_bytes: u64) -> WorkContext {
    ExecutionPolicy {
        input_bytes: u64::MAX,
        working_bytes,
        scratch_bytes: u64::MAX,
        result_bytes: u64::MAX,
        rows: u64::MAX,
        work_units: u64::MAX,
        timeout: std::time::Duration::from_secs(3600),
    }
    .start()
    .expect("bounded ledger")
}

/// A counters seam that cancels the shared operation after a few explored
/// batches — the cooperative-stop shape a host issues mid-join.
struct CancelAfterBatches {
    work: WorkContext,
    batches: usize,
    cancel_at: usize,
}

impl Counters for CancelAfterBatches {
    fn node_entry(&mut self, _: usize) {}
    fn batch(&mut self, _: usize, _: usize) {
        self.batches += 1;
        if self.batches == self.cancel_at {
            self.work.cancel();
        }
    }
    fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
    fn probe_hash(&mut self, _: usize, _: usize) {}
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    fn residual(&mut self, _: usize, _: bool) {}
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {}
    fn skip(&mut self, _: usize) {}
}

fn is_work_refusal(error: &crate::error::Error, expected: &WorkError) -> bool {
    matches!(
        error,
        crate::error::Error::Store(store) if matches!(
            &**store,
            crate::storage::store::StoreError::Work(work) if work == expected
        )
    )
}

/// Cancellation fires INSIDE a selective join that emits nothing: every
/// explored binding pair survives its probes and dies at the residual
/// filter, so no row ever reaches the sink — the executor's own bounded
/// quantum poll on exploration must stop the join with the typed
/// cancellation (no per-emitted-row poll exists to catch it).
#[test]
fn cancellation_fires_inside_a_selective_join_that_emits_nothing() {
    let schema = schema(2);
    // Probes always hit (shared x0), the residual x1 < x2 never holds:
    // R0's b values are all large, R1's all small.
    let a: Vec<(u64, u64)> = (0..8192).map(|i| (i, 1_000_000 + i)).collect();
    let b: Vec<(u64, u64)> = (0..8192).map(|i| (i, i)).collect();
    let views = views_of(&schema, &[a, b]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(VarId(1)),
            right: OperandAddr::from(VarId(2)),
            op: crate::ir::WordCmp::Lt,
        }],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);

    let work = bounded(u64::MAX);
    let mut counters = CancelAfterBatches {
        work: work.clone(),
        batches: 0,
        cancel_at: 4,
    };
    let mut executor = Executor::new(&plan);
    executor.begin_work(&work);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let result = executor.execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);
    let error = result.expect_err("a cancelled operation refuses");
    assert!(
        is_work_refusal(&error, &WorkError::Cancelled),
        "typed cancellation from inside the join, got {error:?}"
    );
    assert!(sink.rows.is_empty(), "nothing was emitted");
    assert!(
        counters.batches >= 4,
        "the cancel fired mid-exploration ({} batches)",
        counters.batches
    );
}

/// A tiny working budget stops COLT growth with the typed refusal — the
/// exact condition that licenses the ONE bounded restart into the cursor
/// fallback (`is_working_exhaustion`).
#[test]
fn a_tiny_working_budget_stops_colt_growth_with_the_typed_refusal() {
    let schema = schema(2);
    // 4096 join keys: forcing the sibling's level map alone owns hundreds
    // of kilobytes of pool capacity — far past the 64 KiB allowance.
    let a: Vec<(u64, u64)> = (0..4096).map(|i| (i, 0)).collect();
    let b: Vec<(u64, u64)> = (0..4096).map(|i| (i, i)).collect();
    let views = views_of(&schema, &[a, b]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);

    // Unbounded ledger: polling and charging must not change the answers.
    let unpolled = run(&plan, &views);
    let work = bounded(u64::MAX);
    let mut executor = Executor::new(&plan);
    executor.begin_work(&work);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    executor
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect("unbounded execute");
    assert_eq!(sink.rows, unpolled, "the ledger never changes answers");
    assert!(
        work.used(Resource::WorkingBytes) > 0,
        "successful execute retains reusable COLT pool charges (D08)"
    );
    assert!(
        work.used(Resource::WorkUnits) >= 4096,
        "explored entries are charged as work"
    );

    // 64 KiB working bytes: the first bounded-quantum poll after the
    // sibling force refuses the growth reservation, typed.
    let tiny = bounded(64 << 10);
    let mut executor = Executor::new(&plan);
    executor.begin_work(&tiny);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let error = executor
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect_err("COLT growth past the allowance refuses");
    assert!(
        matches!(
            &error,
            crate::error::Error::Store(store) if matches!(
                &**store,
                crate::storage::store::StoreError::Work(WorkError::Exhausted {
                    resource: Resource::WorkingBytes,
                    ..
                })
            )
        ),
        "typed working-byte exhaustion from join growth, got {error:?}"
    );
    assert!(
        crate::api::prepared::source::is_working_exhaustion(&error),
        "the refusal is exactly the bounded-restart trigger"
    );
    assert_eq!(
        tiny.used(Resource::WorkingBytes),
        0,
        "the failed execution's growth reservations are refunded before \
         the restart could run"
    );
}

/// Bind the current ledger before force_root. A leftover Exhausted from
/// a cancelled/tiny prior execution must not poison the next bind.
#[test]
fn bind_clears_prior_refusal_before_force() {
    let schema = schema(2);
    let a: Vec<(u64, u64)> = (0..4096).map(|i| (i, 0)).collect();
    let b: Vec<(u64, u64)> = (0..4096).map(|i| (i, i)).collect();
    let views = views_of(&schema, &[a, b]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let mut colts = colts_for(&plan, &views);

    let prior = bounded(64);
    colts[1].bind(Some(&prior));
    let leftover = colts[1].force_root();
    assert!(
        matches!(
            leftover,
            Err(WorkError::Exhausted {
                resource: Resource::WorkingBytes,
                ..
            })
        ),
        "first-map force under 64 working bytes must refuse, got {leftover:?}"
    );

    let current = bounded(u64::MAX);
    colts[1].bind(Some(&current));
    colts[1]
        .force_root()
        .expect("the new execution's force is not poisoned by the old ledger");
}

/// First-map refusal is Err, never a fabricated Ok(None) miss or empty success.
#[test]
fn force_refusal_is_err_not_empty_success() {
    let schema = schema(2);
    let a: Vec<(u64, u64)> = (0..4096).map(|i| (i, 0)).collect();
    let b: Vec<(u64, u64)> = (0..4096).map(|i| (i, i)).collect();
    let views = views_of(&schema, &[a, b]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let mut colts = colts_for(&plan, &views);
    let tiny = bounded(64);
    colts[1].bind(Some(&tiny));
    let refused = colts[1].force_root();
    assert!(
        matches!(
            refused,
            Err(WorkError::Exhausted {
                resource: Resource::WorkingBytes,
                ..
            })
        ),
        "typed refusal, not a fabricated miss, got {refused:?}"
    );
    let probed = colts[1].get_prehashed(
        crate::exec::colt::Colt::root(),
        0,
        &[0],
        crate::exec::colt::hash_key(&[0]),
    );
    assert!(
        probed.is_err(),
        "get_prehashed must not rewrite force refusal as Ok(None), got {probed:?}"
    );
    assert_eq!(
        tiny.used(Resource::WorkingBytes),
        0,
        "failed admit refunds before any dependent clone/select"
    );
}
