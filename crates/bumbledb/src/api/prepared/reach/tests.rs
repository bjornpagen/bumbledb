use super::*;
use crate::exec::run::{Bindings, Sink};
use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, ProjectionSink};
use crate::image::CacheGeneration;
use crate::image::intern::InternerHandle;
use crate::work::{GenerationHandle, GenerationState, WorkContext};
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
fn recursive_accumulator_owns_text_after_frontier_refill() {
    {
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
        let mut owners = crate::image::TextOwners::default();
        let mut bindings = Bindings::new(1);
        for (index, text) in ["first-round", "second-round"].into_iter().enumerate() {
            let owner = interner.intern(text).unwrap();
            bindings.set(0, owner.word);
            driver.sink.emit(&bindings);
            let image = next_frontier(
                &mut driver,
                &work,
                &generation,
                index,
                index + 1,
                &mut owners,
            )
            .unwrap();
            assert_eq!(image.row_count(), 1);
            drop(owner);
            drop(image);
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
    let image = Arc::clone(&derived.published[0]);
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

#[test]
fn finished_projection_stage_seals_resident() {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    feed_ids(&mut sink, 0..3);
    let mut derived = DerivedImages::default();
    derived.begin(1);
    let count = derived
        .stash_finished(0, &u64_types(1), &mut sink, &work(), &generation())
        .expect("seal");
    assert_eq!(count, 3);
    assert_eq!(derived.published[0].row_count(), 3);
}
