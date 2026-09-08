//! Cancellation is polled on explored bindings, including joins that emit
//! nothing. Rebinding changes cancellation, not ownership of reusable pools.
use super::*;
use crate::work::{WorkContext, WorkError};

#[test]
fn sibling_width_batch_preserves_first_refusal_and_successful_prefix() {
    #[derive(Default)]
    struct Probes(Vec<bool>);
    impl Counters for Probes {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, hit: bool) {
            self.0.push(hit);
        }
        fn residual(&mut self, _: usize, _: bool) {}
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    let schema = schema(1);
    let normalized = normalized(vec![occurrence(0, 0, &[(0, 0)])], vec![]);
    let plan = planned(&normalized, &schema, &[0]);
    let images = views_of(&schema, &[vec![(7, 10), (8, 20)]]);
    let first_key = images[0].column_words(0)[0];
    for (fixed, children) in [(false, false), (false, true), (true, false), (true, true)] {
        {
            let work = WorkContext::new();
            work.cancel();
            let mut colt = colts_for(&plan, &images).pop().unwrap();
            colt.bind(Some(&work));
            let mut executor = Executor::new(&plan);
            let sentinel = Cursor::Row(u32::MAX);
            let mut scratch = NodeScratch {
                // Survivors are deliberately not element-ordered.
                survivors: vec![2, 0, 1, 3],
                parents: vec![0, 1, 2, 3],
                pending_cursors: vec![Cursor::Row(0), Colt::root(), Cursor::Row(0), Cursor::Row(0)],
                probe_keys: vec![first_key, u64::MAX, first_key, first_key],
                hashes: [first_key, u64::MAX, first_key, first_key]
                    .map(|key| crate::exec::colt::hash_key(&[key]))
                    .to_vec(),
                children: vec![vec![sentinel; if children { 4 } else { 0 }]],
                mask: vec![9; 4],
                ..NodeScratch::default()
            };
            let mut counters = Probes::default();
            if fixed {
                executor.probe_sibling_batch::<1, _>(
                    &mut scratch,
                    &mut colt,
                    0,
                    0,
                    0,
                    Some(0),
                    1,
                    Colt::root(),
                    1,
                    &mut counters,
                );
            } else {
                executor.probe_sibling_batch::<0, _>(
                    &mut scratch,
                    &mut colt,
                    0,
                    0,
                    0,
                    Some(0),
                    1,
                    Colt::root(),
                    1,
                    &mut counters,
                );
            }
            assert_eq!(counters.0, vec![true, false]);
            assert_eq!(scratch.mask, vec![1, 0, 9, 9]);
            assert_eq!(
                scratch.children[0],
                if children {
                    vec![sentinel, sentinel, Cursor::Row(0), sentinel]
                } else {
                    vec![]
                },
                "misses and presence-only probes never write child output"
            );
            assert_eq!(scratch.survivors, vec![2, 0, 1, 3]);
            assert!(colt.forced_capacity(Colt::root()).is_none());
            let DriveState::Poisoned(Poison::Work(error)) = executor.drive_state else {
                panic!("force refusal must poison the executor");
            };
            assert_eq!(error, WorkError::Cancelled);
        }
    }
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

fn is_work_refusal(error: &crate::error::Error, expected: WorkError) -> bool {
    matches!(
        error,
        crate::error::Error::Store(store) if matches!(
            &**store,
            crate::storage::store::StoreError::Work(work) if *work == expected
        )
    )
}

#[test]
fn cancelled_leaf_and_pipeline_retain_scratch_and_resume_with_fresh_work() {
    for relations in 1..=3u16 {
        let schema = schema(usize::from(relations));
        let normalized = normalized(
            (0..relations)
                .map(|r| occurrence(r, u32::from(r), &[(0, r), (1, r + 1)]))
                .collect(),
            vec![],
        );
        let order: Vec<_> = (0..relations).collect();
        let plan = planned_with_sinks(&normalized, &schema, &order, &all_vars(&normalized));
        let views = views_of(
            &schema,
            &(0..relations)
                .map(|r| {
                    (0..512)
                        .map(|i| (i + u64::from(r), i + u64::from(r) + 1))
                        .collect()
                })
                .collect::<Vec<_>>(),
        );
        let expected: BTreeSet<Vec<u64>> = (0..512)
            .map(|i| {
                let mut row = vec![0; plan.slot_count()];
                for var in 0..=relations {
                    row[plan.slot_of(VarId(var))] = i + u64::from(var);
                }
                row
            })
            .collect();
        let mut executor = Executor::with_batch_size(&plan, 16);
        let roster = executor.scratch.as_ptr();
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut sink = CollectSink::default();

        for cancel in [false, true, false] {
            let work = WorkContext::new();
            executor.begin_work(&work, &mut colts);
            sink.rows.clear();
            let mut counters = CancelAfterBatches {
                work,
                batches: 0,
                cancel_at: if cancel { 3 } else { usize::MAX },
            };
            let result =
                executor.execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);
            assert_eq!(
                executor.scratch.as_ptr(),
                roster,
                "retain the buffer roster"
            );
            assert_eq!(executor.scratch.len(), plan.nodes().len());
            assert!(
                executor.ledger.is_none(),
                "always release the execution ledger"
            );
            if cancel {
                assert!(is_work_refusal(&result.unwrap_err(), WorkError::Cancelled));
                assert!(
                    counters.batches >= 3,
                    "cancellation happens during execution"
                );
            } else {
                result.unwrap();
                assert_eq!(sink.rows, expected, "relations {relations}");
            }
        }
    }
}

#[test]
fn physical_terminal_force_observes_cancellation_and_releases_its_pools() {
    let schema = schema(1);
    let normalized = normalized(vec![occurrence(0, 0, &[(0, 0)])], vec![]);
    let plan = planned(&normalized, &schema, &[0]);
    let views = views_of(&schema, &[(0..64).map(|id| (id % 4, id)).collect()]);
    for cancelled in [true, false] {
        let work = WorkContext::new();
        if cancelled {
            work.cancel();
        }
        let mut executor = Executor::new(&plan);
        executor.set_physical_distinct(plan.scalar_set_traversal());
        let mut colts = colts_for(&plan, &views);
        executor.begin_work(&work, &mut colts);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut sink = CollectSink::default();
        let result = executor.execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        );
        if cancelled {
            assert!(is_work_refusal(&result.unwrap_err(), WorkError::Cancelled));
            assert!(sink.rows.is_empty());
        } else {
            result.unwrap();
            assert_eq!(
                sink.rows,
                BTreeSet::from([vec![0], vec![1], vec![2], vec![3]])
            );
            assert!(colts[0].forced_capacity(Colt::root()).is_some());
        }
        let retained = colts.iter().map(Colt::retained_bytes).sum::<usize>();
        let before = crate::alloc_counter::snapshot().window;
        drop(colts);
        let after = crate::alloc_counter::snapshot().window;
        #[cfg(feature = "alloc-counter")]
        assert!(after.dealloc_bytes - before.dealloc_bytes >= retained as u64);
        let _ = (retained, before, after);
    }
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

    let work = WorkContext::new();
    let mut counters = CancelAfterBatches {
        work: work.clone(),
        batches: 0,
        cancel_at: 4,
    };
    let mut executor = Executor::new(&plan);
    let mut colts = colts_for(&plan, &views);
    executor.begin_work(&work, &mut colts);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let result = executor.execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);
    let error = result.expect_err("a cancelled operation refuses");
    assert!(
        is_work_refusal(&error, WorkError::Cancelled),
        "typed cancellation from inside the join, got {error:?}"
    );
    assert!(sink.rows.is_empty(), "nothing was emitted");
    assert!(
        counters.batches >= 4,
        "the cancel fired mid-exploration ({} batches)",
        counters.batches
    );
}

/// Ordinary execution keeps reusable pools without changing answers.
#[test]
fn ordinary_allocation_preserves_large_join_answers_and_pool_lifetimes() {
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
    let expected = run(&plan, &views);
    assert_eq!(expected.len(), 4096);
    let mut executor = Executor::new(&plan);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut retained = 0;
    for round in 0..3 {
        sink.rows.clear();
        let work = WorkContext::new();
        executor.begin_work(&work, &mut colts);
        executor
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut NoopCounters,
            )
            .unwrap();
        assert_eq!(sink.rows, expected);
        let bytes = colts.iter().map(Colt::retained_bytes).sum::<usize>();
        if round == 0 {
            retained = bytes;
            assert!(retained > 0);
        } else {
            assert_eq!(bytes, retained, "same-shape execution reuses pools");
        }
        work.cancel();
    }
    let before = crate::alloc_counter::snapshot().window;
    drop(colts);
    let after = crate::alloc_counter::snapshot().window;
    #[cfg(feature = "alloc-counter")]
    assert!(after.dealloc_bytes - before.dealloc_bytes >= retained as u64);
    let _ = (before, after);
}

/// Bind the current context before forcing; prior cancellation cannot poison it.
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

    let mut executor = Executor::new(&plan);
    let prior = WorkContext::new();
    prior.cancel();
    executor.begin_work(&prior, &mut colts);
    let leftover = colts[1].force_root();
    assert_eq!(leftover, Err(WorkError::Cancelled));

    let current = WorkContext::new();
    executor.begin_work(&current, &mut colts);
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
    let cancelled = WorkContext::new();
    cancelled.cancel();
    colts[1].bind(Some(&cancelled));
    let refused = colts[1].force_root();
    assert_eq!(refused, Err(WorkError::Cancelled));
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
    assert!(colts[1].forced_capacity(Colt::root()).is_none());
}

/// One entry operation owns binding before force and execution. A cancelled
/// attempt that never executes, and a completed attempt cancelled afterward,
/// must not leak their context into the next operation on the same COLTs.
#[test]
fn begin_work_rebinds_reused_colts_before_force_and_execute() {
    let schema = schema(1);
    let normalized = normalized(vec![occurrence(0, 0, &[(0, 0)])], vec![]);
    let plan = planned(&normalized, &schema, &[0]);
    let views = views_of(&schema, &[(0..32).map(|id| (id % 4, id)).collect()]);
    let mut executor = Executor::new(&plan);
    executor.set_physical_distinct(plan.scalar_set_traversal());
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());

    let cancelled = WorkContext::new();
    cancelled.cancel();
    executor.begin_work(&cancelled, &mut colts);
    assert_eq!(
        colts[0].force_root(),
        Err(WorkError::Cancelled),
        "entry binds before any force, not only at execute"
    );
    assert!(colts[0].forced_capacity(Colt::root()).is_none());

    let first = WorkContext::new();
    executor.begin_work(&first, &mut colts);
    colts[0].force_root().expect("fresh operation can force");
    let retained = colts[0].retained_bytes();
    assert!(retained > 0);
    let mut sink = CollectSink::default();
    executor
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect("direct executor shares the pre-force binding");
    let expected = BTreeSet::from([vec![0], vec![1], vec![2], vec![3]]);
    assert_eq!(sink.rows, expected);
    assert!(executor.ledger.is_none(), "execution releases its ledger");

    first.cancel();
    let next = WorkContext::new();
    executor.begin_work(&next, &mut colts);
    sink.rows.clear();
    executor
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect("completed prior context cannot poison reuse");
    assert_eq!(sink.rows, expected);
    assert_eq!(
        colts[0].retained_bytes(),
        retained,
        "warm pools do not grow"
    );
    assert_eq!(first.checkpoint(), Err(WorkError::Cancelled));
    assert_eq!(next.checkpoint(), Ok(()));
}
