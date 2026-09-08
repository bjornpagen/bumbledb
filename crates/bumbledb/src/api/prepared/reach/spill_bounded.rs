//! Derived-stage ownership and borrowed drains across representations.
//! These operate on completed producer state, not streaming query execution.
use super::*;
use crate::exec::run::{Bindings, Sink};
use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, ProjectionSink};
use crate::image::CacheGeneration;
use crate::work::{GenerationHandle, GenerationState, WorkContext, WorkError};
use bumbledb_theory::schema::ValueType;

fn work() -> crate::work::WorkContext {
    WorkContext::new()
}

fn churn_text(generation: &GenerationHandle, prefix: &str) {
    let work = work();
    let interner = InternerHandle::new(generation, &work);
    for i in 0..1024 {
        drop(interner.intern(&format!("{prefix}-{i}")).unwrap());
    }
}

#[test]
fn sealed_scratch_projection_and_aggregate_own_text_after_producer_drop() {
    for aggregate in [false, true] {
        let work = work();
        let generation = generation();
        let interner = InternerHandle::new(&generation, &work);
        let owner = interner.intern("live-stage-text").unwrap();
        let token = owner.word;
        let mut bindings = Bindings::new(2);
        bindings.set(0, 7);
        bindings.set(1, token);
        let mut derived = DerivedImages::default();
        derived.begin(1);
        if aggregate {
            let mut sink = AggregateSink::new(
                [
                    FindSpec::Agg(AggSpec::Count),
                    FindSpec::Var { slot: 1, width: 1 },
                ],
                2,
            );
            sink.begin(Some(work.clone()));
            sink.emit(&bindings);
            sink.force_spill().unwrap();
            derived
                .stash_aggregate(
                    0,
                    &[ValueType::U64, ValueType::String],
                    &mut sink,
                    &mut Vec::new(),
                    &work,
                    &generation,
                )
                .unwrap();
        } else {
            let mut sink = ProjectionSink::new(vec![0, 1]);
            sink.begin(Some(work.clone()));
            sink.emit(&bindings);
            sink.force_spill().unwrap();
            derived
                .stash_finished(
                    0,
                    &[ValueType::U64, ValueType::String],
                    &mut sink,
                    &work,
                    &generation,
                )
                .unwrap();
        }
        drop(owner);
        churn_text(&generation, "with-stage");
        let stage = derived.published.last_mut().unwrap();
        stage.check_generation(&generation).unwrap();
        assert!(
            stage
                .check_generation(&crate::image::test_generation())
                .is_err()
        );
        let SealedStage::Scratch(stage) = stage else {
            panic!("explicit scratch stage")
        };
        assert_eq!(stage.texts.len(), 1);
        SealedStage::for_each_scratch_row(stage, &work, |row| {
            assert_eq!(row, &[if aggregate { 1 } else { 7 }, token]);
            assert_eq!(
                generation
                    .resolver()
                    .with_text(row[1], str::to_owned)
                    .as_deref(),
                Some("live-stage-text")
            );
            Ok(true)
        })
        .unwrap();
        drop(derived);
        churn_text(&generation, "after-stage");
        assert_eq!(generation.resolver().lookup("live-stage-text"), None);
    }
}

#[test]
fn recursive_accumulator_owns_text_after_frontier_refill_in_both_representations() {
    for disk in [false, true] {
        let work = work();
        let generation = generation();
        let interner = InternerHandle::new(&generation, &work);
        let mut driver = ReachDriver {
            base: Vec::new(),
            rec: Vec::new(),
            field_types: vec![ValueType::String],
            sink: ProjectionSink::new(vec![0]),
            units: 0,
            frontier: TransientImage::default(),
        };
        driver.sink.begin(Some(work.clone()));
        if disk {
            driver.sink.force_spill().unwrap();
        }
        let mut owners = crate::image::TextOwners::default();
        let mut bindings = Bindings::new(1);
        for (index, text) in ["first-round", "second-round"].into_iter().enumerate() {
            let owner = interner.intern(text).unwrap();
            bindings.set(0, owner.word);
            driver.sink.emit(&bindings);
            let stage = next_frontier(
                &mut driver,
                &work,
                &generation,
                index,
                index + 1,
                &mut owners,
            )
            .unwrap();
            assert_eq!(stage.row_count(), 1);
            assert_eq!(stage.is_resident(), !disk);
            drop(owner);
            drop(stage);
        }
        churn_text(&generation, "after-frontier-refill");
        assert_eq!(owners.len(), 2);
        assert!(generation.resolver().lookup("first-round").is_some());
        let mut derived = DerivedImages::default();
        derived.begin(1);
        assert_eq!(
            derived
                .stash_finished(0, &driver.field_types, &mut driver.sink, &work, &generation)
                .unwrap(),
            2
        );
        drop(driver);
        drop(owners);
        churn_text(&generation, "after-accumulator");
        assert!(generation.resolver().lookup("first-round").is_some());
        drop(derived);
        churn_text(&generation, "after-result");
        assert_eq!(generation.resolver().lookup("first-round"), None);
        assert_eq!(generation.resolver().lookup("second-round"), None);
    }
}

#[test]
fn small_aggregate_keeps_free_join_image_alive_after_producer_drop() {
    let work = work();
    let generation = generation();
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink = AggregateSink::new(&finds, 2);
    let mut bindings = Bindings::new(2);
    for (id, group) in [(1, 7), (2, 7), (3, 8)] {
        bindings.set(0, group);
        bindings.set(1, id);
        sink.emit(&bindings);
    }
    let mut derived = DerivedImages::default();
    derived.begin(1);
    assert_eq!(
        derived
            .stash_aggregate(
                0,
                &u64_types(2),
                &mut sink,
                &mut Vec::new(),
                &work,
                &generation,
            )
            .unwrap(),
        2
    );
    let image = Arc::clone(
        derived.published[0]
            .resident()
            .expect("small aggregate uses Free Join"),
    );
    let mut rows: Vec<_> = (0..image.row_count())
        .map(|row| (image.column_words(0)[row], image.column_words(1)[row]))
        .collect();
    rows.sort_unstable();
    assert_eq!(rows, [(7, 2), (8, 1)]);
    let weak = Arc::downgrade(&image);
    drop(derived);
    assert_eq!(image.column_words(0).len(), 2);
    assert_eq!(
        weak.strong_count(),
        1,
        "only the consumer now owns the image"
    );
    let bytes = image.byte_size();
    let before = crate::alloc_counter::snapshot().window;
    drop(image);
    let after = crate::alloc_counter::snapshot().window;
    assert!(weak.upgrade().is_none());
    #[cfg(feature = "alloc-counter")]
    assert!(after.dealloc_bytes - before.dealloc_bytes >= bytes as u64);
    let _ = (bytes, before, after);
}

#[test]
fn bounded_image_publishes_emitted_rows_and_rebinds_its_generation() {
    let work = work();
    let first = generation();
    let second = generation();
    let mut slot = TransientImage::default();
    let image = slot
        .refill_bounded(&work, &u64_types(2), 3, &first, |_, write| {
            write(&[2, 9]);
            Ok(())
        })
        .unwrap();
    assert_eq!(image.row_count(), 1, "an upper bound is not a cardinality");
    let bytes = image.byte_size();
    let address = image.column_words(0).as_ptr();
    drop(image);
    let image = slot
        .refill_bounded(&work, &u64_types(2), 3, &second, |_, write| {
            write(&[4, 11]);
            Ok(())
        })
        .unwrap();
    assert!(image.generation().ptr_eq(&second));
    assert_eq!(image.column_words(0), &[4]);
    assert_eq!(image.byte_size(), bytes);
    assert_eq!(
        image.column_words(0).as_ptr(),
        address,
        "reuse the same slab"
    );
}

fn generation() -> GenerationHandle {
    GenerationHandle::new(GenerationState::new(CacheGeneration::initial()))
}

fn u64_types(n: usize) -> Vec<ValueType> {
    vec![ValueType::U64; n]
}

fn feed_ids(sink: &mut ProjectionSink, values: impl IntoIterator<Item = u64>) {
    let mut bindings = Bindings::new(1);
    for value in values {
        bindings.set(0, value);
        sink.emit(&bindings);
    }
}

fn spilled_sink(rows: u64) -> ProjectionSink {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    sink.begin(Some(work()));
    feed_ids(&mut sink, 0..rows);
    sink.force_spill().unwrap();
    assert!(sink.spilled(), "explicit transition uses disk");
    sink
}

fn collect_ids(stage: &mut ScratchStage) -> Vec<u64> {
    let mut out = Vec::new();
    SealedStage::for_each_scratch_row(stage, &work(), |row| {
        out.push(row[0]);
        Ok(true)
    })
    .expect("charged visit");
    out.sort_unstable();
    out
}

/// D09: a spilled projection seals as scratch and the L03 visitor yields
/// every row — not a resident rematerialization of the stage.
#[test]
fn d09_spilled_projection_seal_is_scratch_and_complete() {
    let mut sink = spilled_sink(24);
    let Ok(SealedStage::Scratch(mut stage)) =
        seal_scratch_range(&mut sink, &work(), &u64_types(1), &generation(), 0)
    else {
        panic!("spilled seal must stay scratch");
    };
    assert_eq!(stage.count, 24);
    assert_eq!(collect_ids(&mut stage), (0..24).collect::<Vec<_>>());
}

/// D09: a tiny unspilled projection stays resident.
#[test]
fn d09_small_projection_stays_resident() {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    feed_ids(&mut sink, 0..3);
    assert!(!sink.spilled());
    let mut derived = DerivedImages::default();
    derived.begin(1);
    let count = derived
        .stash_finished(0, &u64_types(1), &mut sink, &work(), &generation())
        .expect("seal");
    assert_eq!(count, 3);
    assert!(
        derived.published[0].is_resident(),
        "small stages stay resident"
    );
}

/// The frontier is only the watermark suffix, even while the source
/// accumulated set continues growing in scratch.
#[test]
fn scratch_frontier_seals_only_the_watermark_suffix() {
    let mut sink = spilled_sink(6);
    let types = u64_types(1);
    let work = work();
    let Ok(SealedStage::Scratch(mut first)) =
        seal_scratch_range(&mut sink, &work, &types, &generation(), 0)
    else {
        panic!("frontier must be scratch");
    };
    assert_eq!(first.count, 6);
    feed_ids(&mut sink, 6..10);
    assert_eq!(sink.len(), 10);
    assert_eq!(collect_ids(&mut first), (0..6).collect::<Vec<_>>());
    let Ok(SealedStage::Scratch(mut delta)) =
        seal_scratch_range(&mut sink, &work, &types, &generation(), 6)
    else {
        panic!("Δ must be scratch");
    };
    assert_eq!(delta.count, 4);
    assert_eq!(collect_ids(&mut delta), vec![6, 7, 8, 9]);
}

/// D09: a refused scratch put fails the seal; the dest never holds the
/// whole stage (no second full collection after the drain).
#[test]
fn cancelled_seal_publishes_nothing_and_keeps_source_readable() {
    let mut sink = spilled_sink(4096);
    let cancelled = work();
    cancelled.cancel();
    let before = crate::alloc_counter::snapshot().window;
    let refused = seal_scratch_range(&mut sink, &cancelled, &u64_types(1), &generation(), 0);
    let after = crate::alloc_counter::snapshot().window;
    assert!(matches!(refused, Err(crate::error::Error::Store(store))
        if matches!(*store, crate::storage::store::StoreError::Work(WorkError::Cancelled))));
    #[cfg(feature = "alloc-counter")]
    assert!(
        after.alloc_bytes - before.alloc_bytes < 4096,
        "no whole-stage collection"
    );
    let _ = (before, after);
    let SealedStage::Scratch(mut stage) =
        seal_scratch_range(&mut sink, &work(), &u64_types(1), &generation(), 0).unwrap()
    else {
        panic!("scratch stage")
    };
    assert_eq!(collect_ids(&mut stage), (0..4096).collect::<Vec<_>>());
}

/// D09: visitor Err stops the derived walk; later rows are not visited.
#[test]
fn d09_scratch_visit_propagates_failure_immediately() {
    let mut sink = spilled_sink(8);
    let Ok(SealedStage::Scratch(mut stage)) =
        seal_scratch_range(&mut sink, &work(), &u64_types(1), &generation(), 0)
    else {
        panic!("scratch");
    };
    let mut seen = 0u64;
    let refused = SealedStage::for_each_scratch_row(&mut stage, &work(), |_| {
        seen += 1;
        if seen == 3 {
            return Err(crate::error::Error::ResultBytesOverflow);
        }
        Ok(true)
    });
    assert!(refused.is_err());
    assert_eq!(seen, 3, "no later row after the failing visit");
}

/// A small aggregate seals directly into an image, with no scratch relation.
#[test]
fn d09_tiny_aggregate_never_opens_scratch() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut small = AggregateSink::new(finds, 2);
    let mut bindings = Bindings::new(2);
    for i in 0..4u64 {
        bindings.reset();
        bindings.set(0, i);
        bindings.set(1, i);
        small.emit(&bindings);
    }
    let ledger = WorkContext::new();
    let mut derived = DerivedImages::default();
    derived.begin(1);
    let mut answer = Vec::new();
    let count = derived
        .stash_aggregate(
            0,
            &[ValueType::U64, ValueType::U64],
            &mut small,
            &mut answer,
            &ledger,
            &generation(),
        )
        .expect("tiny aggregate must not need a scratch environment");
    assert_eq!(count, 4);
    let SealedStage::Resident(image) = &derived.published[0] else {
        panic!("tiny aggregate stays eligible for resident Free Join");
    };
    assert_eq!(image.row_count(), 4);
    let mut groups = image.column_words(0).to_vec();
    groups.sort_unstable();
    assert_eq!(groups, [0, 1, 2, 3]);
    assert_eq!(image.column_words(1), [1, 1, 1, 1]);
}

/// A disk-partitioned producer drains directly into one destination.
#[test]
fn partitioned_aggregate_seals_exactly_without_an_intermediate_answer_set() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut large = AggregateSink::new(finds, 2);
    let mut bindings = Bindings::new(2);
    for i in 0..64u64 {
        bindings.reset();
        bindings.set(0, i);
        bindings.set(1, i);
        large.emit(&bindings);
    }
    large.force_spill().unwrap();
    let work = WorkContext::new();
    let mut spilled = DerivedImages::default();
    spilled.begin(1);
    let mut answer = Vec::new();
    let count = spilled
        .stash_aggregate(
            0,
            &[ValueType::U64, ValueType::U64],
            &mut large,
            &mut answer,
            &work,
            &generation(),
        )
        .expect("streamed aggregate");
    assert_eq!(count, 64);
    let SealedStage::Scratch(stage) = &mut spilled.published[0] else {
        panic!("dest owner");
    };
    assert!(
        !stage.rows.spilled(),
        "destination uses ordinary RAM ownership"
    );
    assert!(stage.rows.scratch_path().is_none());
    assert_eq!(collect_ids(stage), (0..64).collect::<Vec<_>>());
}
