//! Borrowed sink drains, explicit scratch transitions, and cancellation.
//! Completed sink rows are materialized results, not streaming execution.

use crate::error::Error;
use crate::exec::run::{Bindings, Sink as _};
use crate::exec::scratch::ScratchRelation;
use crate::exec::sink::{
    AggSpec, AggregateSink, FindSpec, ProjectionSink, STEP_QUANTUM, SinkProgress, SpillSet,
};
use crate::work::WorkContext;

fn work() -> crate::work::WorkContext {
    WorkContext::new()
}

fn assert_hashed_kernel_state(actual: &SpillSet, dynamic: &SpillSet) {
    assert!(actual.unique_rows.is_none() && dynamic.unique_rows.is_none());
    assert_eq!(actual.len(), dynamic.len());
    assert_eq!(actual.spilled(), dynamic.spilled());
    assert_eq!(actual.pending_steps, dynamic.pending_steps);
    assert_eq!(actual.progress(), dynamic.progress());
    assert_eq!(
        actual.error.as_ref().map(ToString::to_string),
        dynamic.error.as_ref().map(ToString::to_string)
    );
}

#[test]
fn direct_hashed_kernel_preserves_duplicates_and_order_across_explicit_transition() {
    for arity in [0, 1, 2, 3] {
        for transition in [None, Some(0), Some(3)] {
            let mut direct = SpillSet::with_capacity_hint(arity, 0, true);
            let mut dynamic = SpillSet::with_capacity_hint(arity, 0, true);
            for _ in 0..2 {
                for set in [&mut direct, &mut dynamic] {
                    set.clear();
                    set.begin(Some(work()));
                }
                let mut expected = Vec::new();
                for emit in 0..20u64 {
                    if transition == Some(emit) {
                        direct.spill().unwrap();
                        dynamic.spill().unwrap();
                    }
                    let row: Vec<_> = (0..arity).map(|word| emit / 2 + word as u64).collect();
                    let new = !expected.contains(&row);
                    if new {
                        expected.push(row.clone());
                    }
                    assert_eq!(direct.insert_inner::<false>(&row), new);
                    assert_eq!(dynamic.insert(&row), new);
                    assert_hashed_kernel_state(&direct, &dynamic);
                }
                for set in [&mut direct, &mut dynamic] {
                    let mut actual = Vec::new();
                    set.for_each_since(0, &mut |row| {
                        actual.push(row.to_vec());
                        Ok(true)
                    })
                    .unwrap();
                    assert_eq!(actual, expected);
                }
            }
        }
    }
}

#[test]
fn direct_hashed_kernel_preserves_cancellation_stickiness_and_reuse() {
    let mut direct = SpillSet::with_capacity_hint(2, 0, true);
    let mut dynamic = SpillSet::with_capacity_hint(2, 0, true);
    for set in [&mut direct, &mut dynamic] {
        let context = work();
        context.cancel();
        set.begin(Some(context));
    }
    for emit in 0..STEP_QUANTUM {
        assert_eq!(
            direct.insert_inner::<false>(&[1, 7]),
            dynamic.insert(&[1, 7])
        );
        assert_hashed_kernel_state(&direct, &dynamic);
        assert_eq!(
            direct.progress(),
            if emit + 1 < STEP_QUANTUM {
                SinkProgress::Continue
            } else {
                SinkProgress::Stop
            }
        );
    }
    assert_eq!(direct.len(), 1);
    assert!(!direct.insert_inner::<false>(&[999, 7]));
    assert!(!dynamic.insert(&[999, 7]));
    assert_hashed_kernel_state(&direct, &dynamic);
    for set in [&mut direct, &mut dynamic] {
        set.clear();
        set.begin(None);
    }
    assert!(direct.insert_inner::<false>(&[99, 8]));
    assert!(dynamic.insert(&[99, 8]));
    assert_hashed_kernel_state(&direct, &dynamic);
    assert_eq!(direct.progress(), SinkProgress::Continue);
    assert_eq!(direct.len(), 1);
}

#[test]
fn bulk_hashed_insert_preserves_transition_order_and_early_stop() {
    for arity in [1, 2, 9] {
        for transition in [None, Some(0), Some(3)] {
            let mut bulk = SpillSet::with_capacity_hint(arity, 0, true);
            let mut control = SpillSet::with_capacity_hint(arity, 0, true);
            for set in [&mut bulk, &mut control] {
                set.begin(Some(work()));
            }
            let rows: Vec<u64> = (0..20u64)
                .flat_map(|id| (0..arity).map(move |word| id / 2 + word as u64))
                .collect();
            for (start, end) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 10), (10, 20)] {
                if transition == Some(start) {
                    bulk.spill().unwrap();
                    control.spill().unwrap();
                }
                let words = &rows[start * arity..end * arity];
                for row in words.chunks_exact(arity) {
                    control.insert_inner::<false>(row);
                }
                bulk.insert_hashed_rows(words);
                assert_hashed_kernel_state(&bulk, &control);
            }
            let expected: Vec<Vec<u64>> = (0..10u64)
                .map(|id| (0..arity).map(|word| id + word as u64).collect())
                .collect();
            for cap in [2, 10] {
                for set in [&mut bulk, &mut control] {
                    let mut got = Vec::new();
                    set.for_each_since(0, &mut |row| {
                        got.push(row.to_vec());
                        Ok(got.len() < cap)
                    })
                    .unwrap();
                    assert_eq!(got, expected[..cap]);
                }
                assert_hashed_kernel_state(&bulk, &control);
            }
        }
    }
}

#[test]
fn bulk_hashed_insert_matches_cancellation_boundaries_stickiness_and_reuse() {
    for cancel_at in [None, Some(0), Some(255), Some(256), Some(257)] {
        for duplicates in [false, true] {
            let mut bulk = SpillSet::with_capacity_hint(2, 0, true);
            let mut control = SpillSet::with_capacity_hint(2, 0, true);
            for set in [&mut bulk, &mut control] {
                set.begin(Some(work()));
            }
            for (start, end) in [
                (0, 1),
                (1, 250),
                (250, 255),
                (255, 256),
                (256, 257),
                (257, 520),
            ] {
                if cancel_at == Some(start) {
                    bulk.work.as_ref().unwrap().cancel();
                    control.work.as_ref().unwrap().cancel();
                }
                let words: Vec<_> = (start..end)
                    .flat_map(|id| [if duplicates { id / 2 } else { id }, 7])
                    .collect();
                for row in words.as_chunks::<2>().0 {
                    control.insert_inner::<false>(row);
                }
                bulk.insert_hashed_rows(&words);
                assert_hashed_kernel_state(&bulk, &control);
                assert!(!bulk.spilled());
                assert_eq!(
                    bulk.ram_iter_since(0).collect::<Vec<_>>(),
                    control.ram_iter_since(0).collect::<Vec<_>>()
                );
            }
            assert_eq!(
                bulk.progress(),
                if cancel_at.is_some() {
                    SinkProgress::Stop
                } else {
                    SinkProgress::Continue
                }
            );
            for set in [&mut bulk, &mut control] {
                set.clear();
                set.begin(None);
            }
            bulk.insert_hashed_rows(&[99, 8, 99, 8, 100, 9]);
            for row in [[99, 8], [99, 8], [100, 9]] {
                control.insert_inner::<false>(&row);
            }
            assert_hashed_kernel_state(&bulk, &control);
            assert_eq!(bulk.progress(), SinkProgress::Continue);
            assert_eq!(
                bulk.ram_iter_since(0).collect::<Vec<_>>(),
                vec![&[99, 8][..], &[100, 9][..]]
            );
        }
    }
}

fn output_key_witness() -> crate::plan::fj::ProjectionDistinctWitness {
    use crate::ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId};
    use crate::schema::{
        FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValidateDescriptor as _, ValueType,
    };
    use bumbledb_theory::schema::{FieldId, RelationId, StatementDescriptor};
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "entry".into(),
            fields: ["id", "value"]
                .into_iter()
                .map(|name| FieldDescriptor {
                    name: name.into(),
                    value_type: ValueType::U64,
                })
                .collect(),
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        }],
    }
    .validate()
    .unwrap();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let validated = crate::ir::validate::validate(&schema, &query).unwrap();
    let normalized = crate::ir::normalize::normalize_rules(&schema, &[], validated.rules());
    crate::plan::fj::provably_distinct_projection(&normalized[0], &schema, &query.rules()[0].finds)
        .unwrap()
}

#[test]
fn proved_output_append_matches_ordered_set_across_ram_spill_and_reuse() {
    let witness = output_key_witness();
    for transition in [None, Some(0), Some(2)] {
        let mut hashed = SpillSet::with_capacity_hint(2, 0, true);
        let mut append = SpillSet::with_capacity_hint(2, 0, true);
        append.elide_output_hashing(witness);
        for round in 0..2 {
            hashed.clear();
            append.clear();
            hashed.begin(Some(work()));
            append.begin(Some(work()));
            for (emitted, i) in (0..259u64).rev().enumerate() {
                if transition == Some(emitted) {
                    hashed.spill().unwrap();
                    append.spill().unwrap();
                }
                let row = [i + round * 1000, i % 7];
                assert!(hashed.insert(&row));
                assert!(append.insert(&row));
            }
            assert_eq!(
                append.ram.len(),
                0,
                "proof tier must never populate WordMap"
            );
            assert_eq!(append.len(), hashed.len());
            assert_eq!(append.spilled(), hashed.spilled());
            if !append.spilled() {
                for watermark in [0, 1, 257, 259, 260, usize::MAX] {
                    assert_eq!(
                        append.ram_iter_since(watermark).collect::<Vec<_>>(),
                        hashed.ram_iter_since(watermark).collect::<Vec<_>>()
                    );
                }
            }
            let mut expected = Vec::new();
            let mut actual = Vec::new();
            hashed
                .for_each_since(0, &mut |row| {
                    expected.push(row.to_vec());
                    Ok(true)
                })
                .unwrap();
            append
                .for_each_since(0, &mut |row| {
                    actual.push(row.to_vec());
                    Ok(true)
                })
                .unwrap();
            assert_eq!(actual, expected);
            let mut tail = Vec::new();
            append
                .for_each_since(257, &mut |row| {
                    tail.push(row.to_vec());
                    Ok(true)
                })
                .unwrap();
            assert_eq!(tail, expected[257..]);
            let mut visited = 0;
            append
                .for_each_since(0, &mut |_| {
                    visited += 1;
                    Ok(false)
                })
                .unwrap();
            assert_eq!(visited, 1);
        }
    }
}

#[test]
fn proved_output_append_keeps_work_quantum_failure_sticky_and_reuses_capacity() {
    let ledger = work();
    let mut append = SpillSet::with_capacity_hint(2, 0, true);
    append.elide_output_hashing(output_key_witness());
    append.begin(Some(ledger.clone()));
    for i in 0..STEP_QUANTUM - 1 {
        assert!(append.insert(&[u64::from(i), 0]));
    }
    ledger.cancel();
    assert_eq!(append.pending_steps, STEP_QUANTUM - 1);
    assert!(!append.insert(&[u64::from(STEP_QUANTUM), 0]));
    assert_eq!(append.progress(), SinkProgress::Stop);
    assert_eq!(append.len(), (STEP_QUANTUM - 1) as usize);
    assert!(!append.insert(&[9999, 0]));
    let capacity = append.unique_rows.as_ref().unwrap().words.capacity();
    append.clear();
    append.begin(Some(work()));
    assert_eq!(
        append.unique_rows.as_ref().unwrap().words.capacity(),
        capacity
    );
    assert!(append.insert(&[99, 7]));
    assert_eq!(append.progress(), SinkProgress::Continue);
    assert_eq!(
        append.ram_iter_since(0).collect::<Vec<_>>(),
        vec![&[99, 7][..]]
    );
}

#[test]
fn failed_proved_output_transition_preserves_rows_and_can_retry() {
    let context = work();
    let mut append = SpillSet::with_capacity_hint(2, 0, true);
    append.elide_output_hashing(output_key_witness());
    append.begin(Some(context.clone()));
    assert!(append.insert(&[1, 2]));
    assert!(append.insert(&[3, 4]));
    context.cancel();
    assert!(append.spill().is_err());
    assert!(!append.spilled());
    assert_eq!(append.len(), 2);
    assert_eq!(
        append.ram_iter_since(0).collect::<Vec<_>>(),
        vec![&[1, 2][..], &[3, 4][..]]
    );
    append.begin(Some(work()));
    append.spill().unwrap();
    assert!(append.spilled());
    assert!(append.insert(&[5, 6]));
    let mut rows = Vec::new();
    append
        .for_each_since(0, &mut |row| {
            rows.push(row.to_vec());
            Ok(true)
        })
        .unwrap();
    assert_eq!(rows, vec![vec![1, 2], vec![3, 4], vec![5, 6]]);
}

fn bulk_unique_set() -> SpillSet {
    let mut set = SpillSet::with_capacity_hint(2, 0, true);
    set.elide_output_hashing(output_key_witness());
    let rows = set.unique_rows.as_mut().unwrap();
    rows.words = vec![0; 3].into_boxed_slice().into_vec();
    rows.words.clear();
    assert_eq!(
        rows.words.capacity(),
        3,
        "capacity deliberately ends mid-row"
    );
    set.begin(Some(work()));
    set
}

impl SpillSet {
    #[expect(
        unsafe_code,
        reason = "The test writer initializes every word from its input"
    )]
    fn insert_unique_rows(&mut self, words: &[u64]) {
        let arity = self.ram.arity();
        assert_eq!(words.len() % arity, 0);
        let mut scratch = vec![0; arity];
        // SAFETY: every target word is initialized from the corresponding
        // source word, including allocation/poll/spill boundary rows.
        unsafe {
            self.insert_unique_with(words.len() / arity, &mut scratch, |offset, out| {
                for (target, &word) in out.iter_mut().zip(&words[offset * arity..]) {
                    target.write(word);
                }
            });
        }
    }
}

fn assert_bulk_unique_state(actual: &SpillSet, expected: &SpillSet) {
    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual.spilled(), expected.spilled());
    assert_eq!(actual.pending_steps, expected.pending_steps);
    assert_eq!(actual.progress(), expected.progress());
    assert_eq!(
        actual.error.as_ref().map(ToString::to_string),
        expected.error.as_ref().map(ToString::to_string)
    );
    let actual_rows = actual.unique_rows.as_ref().unwrap();
    let expected_rows = expected.unique_rows.as_ref().unwrap();
    assert_eq!(actual_rows.len, expected_rows.len);
    assert_eq!(actual_rows.words, expected_rows.words);
    assert_eq!(actual_rows.words.capacity(), expected_rows.words.capacity());
}

#[test]
fn bulk_unique_append_matches_each_row_at_capacity_and_poll_boundaries() {
    for cancel_at in [None, Some(0), Some(255), Some(256), Some(257)] {
        let mut control = bulk_unique_set();
        let mut bulk = bulk_unique_set();
        for (start, end) in [
            (0, 1),
            (1, 250),
            (250, 255),
            (255, 256),
            (256, 257),
            (257, 520),
        ] {
            if cancel_at == Some(start) {
                control.work.as_ref().unwrap().cancel();
                bulk.work.as_ref().unwrap().cancel();
            }
            let rows: Vec<u64> = (start..end).flat_map(|id| [id, id % 7]).collect();
            for row in rows.as_chunks::<2>().0 {
                control.insert(row);
            }
            bulk.insert_unique_rows(&rows);
            assert_bulk_unique_state(&bulk, &control);
        }
        let expected_prefix = match cancel_at {
            Some(0 | 255) => 255,
            Some(_) => 511,
            None => 520,
        };
        assert_eq!(bulk.len(), expected_prefix);
        for set in [&mut control, &mut bulk] {
            set.clear();
            set.begin(Some(work()));
        }
        let rows: Vec<u64> = (1000..1300).flat_map(|id| [id, 3]).collect();
        for row in rows.as_chunks::<2>().0 {
            control.insert(row);
        }
        bulk.insert_unique_rows(&rows);
        assert_bulk_unique_state(&bulk, &control);
        assert_eq!(bulk.len(), 300);
    }
}

#[test]
fn bulk_unique_append_preserves_order_across_explicit_transition_boundaries() {
    for transition in [0, 2, 255, 256] {
        let mut control = bulk_unique_set();
        let mut bulk = bulk_unique_set();
        let rows: Vec<u64> = (0..259u64).rev().flat_map(|id| [id, id % 7]).collect();
        for (start, end) in [(0, transition), (transition, 259)] {
            if start == transition && !bulk.spilled() {
                control.spill().unwrap();
                bulk.spill().unwrap();
            }
            let words = &rows[start * 2..end * 2];
            for row in words.as_chunks::<2>().0 {
                control.insert(row);
            }
            bulk.insert_unique_rows(words);
        }
        assert_bulk_unique_state(&bulk, &control);
        assert!(bulk.spilled());
        for set in [&mut bulk, &mut control] {
            let mut drained = Vec::new();
            set.for_each_since(0, &mut |row| {
                drained.extend_from_slice(row);
                Ok(true)
            })
            .unwrap();
            assert_eq!(drained, rows);
        }
    }
}

#[test]
fn bulk_unique_append_observes_cancellation_at_the_same_poll_row() {
    let mut control = bulk_unique_set();
    let mut bulk = bulk_unique_set();
    let prefix: Vec<u64> = (0..250).flat_map(|id| [id, 0]).collect();
    for row in prefix.as_chunks::<2>().0 {
        control.insert(row);
    }
    bulk.insert_unique_rows(&prefix);
    for set in [&control, &bulk] {
        set.work.as_ref().unwrap().cancel();
    }
    let tail: Vec<u64> = (250..270).flat_map(|id| [id, 0]).collect();
    for row in tail.as_chunks::<2>().0 {
        control.insert(row);
    }
    bulk.insert_unique_rows(&tail);
    assert_bulk_unique_state(&bulk, &control);
    assert_eq!(bulk.len(), 255);
    assert_eq!(bulk.pending_steps, 0);
    assert_eq!(bulk.progress(), SinkProgress::Stop);
}

#[test]
#[expect(
    unsafe_code,
    reason = "The interrupted writer only initializes, never deinitializes, words"
)]
fn generated_unique_rows_do_not_publish_an_unfinished_span_on_panic() {
    let mut set = bulk_unique_set();
    set.insert_unique_rows(&[1, 2]);
    set.unique_rows.as_mut().unwrap().words.reserve(4);
    let pending = set.pending_steps;
    let mut scratch = [0; 2];
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: the callback only writes initialized words and never
        // returns; the partially filled spare span must remain unpublished.
        unsafe {
            set.insert_unique_with(2, &mut scratch, |_, out| {
                out[0].write(999);
                panic!("interrupted gather");
            });
        }
    }));
    assert!(failed.is_err());
    assert_eq!(set.len(), 1);
    assert_eq!(set.pending_steps, pending);
    assert_eq!(set.unique_rows.as_ref().unwrap().words, [1, 2]);
    set.insert_unique_rows(&[3, 4, 5, 6]);
    assert_eq!(set.unique_rows.as_ref().unwrap().words, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn resident_dedup_polls_duplicates_at_the_exact_work_quantum_and_stops_stickily() {
    let ledger = work();
    let mut seen = SpillSet::with_capacity_hint(1, 0, false);
    seen.begin(Some(ledger.clone()));
    assert!(seen.insert(&[7]));
    for _ in 1..STEP_QUANTUM - 1 {
        assert!(!seen.insert(&[7]));
    }
    assert_eq!(seen.pending_steps, STEP_QUANTUM - 1);
    ledger.cancel();
    assert_eq!(seen.progress(), SinkProgress::Continue);
    assert!(
        !seen.insert(&[8]),
        "poll refuses before the next tuple is inserted"
    );
    assert_eq!(seen.pending_steps, 0);
    assert_eq!(seen.progress(), SinkProgress::Stop);
    assert!(!seen.insert(&[9]), "failure is sticky");
    assert_eq!(seen.len(), 1);
    seen.clear();
    seen.begin(None);
    assert!(seen.insert(&[8]));
    assert_eq!(seen.progress(), SinkProgress::Continue);
}

#[test]
fn explicit_transition_preserves_duplicates_and_insertion_order() {
    let mut seen = SpillSet::with_capacity_hint(2, 0, true);
    seen.begin(Some(work()));
    assert!(seen.insert(&[1, 2]));
    assert!(seen.insert(&[3, 4]));
    assert!(!seen.insert(&[1, 2]));
    assert!(!seen.spilled(), "duplicates do not choose a storage policy");
    seen.spill().unwrap();
    assert!(seen.insert(&[5, 6]));
    assert!(!seen.insert(&[3, 4]));
    let mut rows = Vec::new();
    seen.for_each_since(0, &mut |row| {
        rows.push(row.to_vec());
        Ok(true)
    })
    .unwrap();
    assert_eq!(rows, vec![vec![1, 2], vec![3, 4], vec![5, 6]]);
    assert_eq!(seen.len(), 3);
}

#[test]
fn failed_hashed_transition_keeps_the_resident_set_and_can_retry() {
    let context = work();
    let mut seen = SpillSet::with_capacity_hint(2, 0, true);
    seen.begin(Some(context.clone()));
    assert!(seen.insert(&[1, 2]));
    assert!(seen.insert(&[3, 4]));
    context.cancel();
    assert!(seen.spill().is_err());
    assert!(!seen.spilled());
    assert_eq!(seen.len(), 2);
    assert_eq!(
        seen.ram_iter_since(0)
            .map(<[u64]>::to_vec)
            .collect::<Vec<_>>(),
        vec![vec![1, 2], vec![3, 4]]
    );
    seen.begin(Some(work()));
    seen.spill().unwrap();
    assert!(seen.spilled());
    assert!(!seen.insert(&[1, 2]));
    assert!(seen.insert(&[5, 6]));
    assert_eq!(seen.len(), 3);
}

fn decode_stage_rows(dest: &mut ScratchRelation) -> Vec<Vec<u64>> {
    let mut rows = Vec::new();
    dest.visit(&mut |_: &[u8], value: &[u8]| {
        let mut words = Vec::new();
        for chunk in value.as_chunks::<8>().0 {
            words.push(u64::from_be_bytes(*chunk));
        }
        rows.push(words);
        Ok(true)
    })
    .expect("visit dest");
    rows.sort();
    rows
}

/// D09: spilled projection streams into scratch one put at a time.
#[test]
fn d09_projection_stream_into_scratch_matches_resident() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Var { slot: 1, width: 1 },
    ];
    let feed = |sink: &mut ProjectionSink| {
        let mut bindings = Bindings::new(2);
        for i in 0..32u64 {
            bindings.reset();
            bindings.set(0, i);
            bindings.set(1, i.wrapping_mul(3));
            sink.emit(&bindings);
        }
    };

    let mut resident = ProjectionSink::with_capacity_hint(&finds, 2, 0);
    feed(&mut resident);
    let mut expected = Vec::new();
    resident
        .for_each_answer(&mut |row| {
            expected.push(row.to_vec());
            Ok(())
        })
        .expect("resident drain");
    expected.sort();

    let mut spilled = ProjectionSink::with_capacity_hint(&finds, 2, 0);
    spilled.begin(Some(work()));
    feed(&mut spilled);
    spilled.seen.spill().unwrap();
    assert!(spilled.spilled());
    let mut dest = ScratchRelation::new(&work());
    dest.force_spill().expect("dest");
    let written = spilled
        .stream_into_scratch(&mut dest, 0, 0, |_| Ok(()))
        .expect("stream");
    assert_eq!(written, 32);
    assert_eq!(decode_stage_rows(&mut dest), expected);
    assert_eq!(spilled.progress(), SinkProgress::Continue);
}

/// D09: visitor Err stops before later rows are written.
#[test]
fn d09_drain_since_propagates_failure_immediately() {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    let mut bindings = Bindings::new(1);
    for i in 0..8u64 {
        bindings.set(0, i);
        sink.emit(&bindings);
    }
    let mut seen = 0u64;
    let refused = sink.drain_since(0, &mut |_| {
        seen += 1;
        if seen == 3 {
            return Err(Error::ResultBytesOverflow);
        }
        Ok(true)
    });
    assert!(refused.is_err());
    assert_eq!(seen, 3, "no later row after the failing visit");
}

/// D09: Ok(false) is a clean early stop, not a hidden collect.
#[test]
fn d09_drain_since_early_stop_is_not_an_error() {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    let mut bindings = Bindings::new(1);
    for i in 0..6u64 {
        bindings.set(0, i);
        sink.emit(&bindings);
    }
    let mut seen = 0u64;
    sink.drain_since(0, &mut |_| {
        seen += 1;
        Ok(seen < 2)
    })
    .expect("early stop");
    assert_eq!(seen, 2);
}

/// D09: aggregate finalize streams groups into scratch; peak dest
/// entries equal published groups, not a flat reconstruction of claims.
#[test]
fn d09_aggregate_stream_finalize_stays_one_row_per_group() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink = AggregateSink::new(&finds, 2);
    sink.begin(Some(work()));
    let mut bindings = Bindings::new(2);
    for i in 0..40u64 {
        bindings.reset();
        bindings.set(0, i % 8);
        bindings.set(1, i);
        sink.emit(&bindings);
    }
    sink.spill_groups().unwrap();
    assert!(sink.group_state_spilled());
    let mut dest = ScratchRelation::new(&work());
    dest.force_spill().expect("dest");
    let mut answer = Vec::new();
    let written = sink
        .stream_finalize(&mut dest, &mut answer, |_| Ok(()))
        .expect("stream finalize");
    assert_eq!(written, 8);
    assert_eq!(dest.len(), 8);
    assert_eq!(sink.progress(), SinkProgress::Finish);
}

#[test]
fn cancelled_destination_refuses_stream_without_consuming_source() {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    let mut bindings = Bindings::new(1);
    for i in 0..4096u64 {
        bindings.set(0, i);
        sink.emit(&bindings);
    }
    let context = work();
    let mut dest = ScratchRelation::new(&context);
    context.cancel();
    #[cfg(feature = "alloc-counter")]
    let before = crate::alloc_counter::snapshot().window;
    let refused = sink.stream_into_scratch(&mut dest, 0, 0, |_| Ok(()));
    #[cfg(feature = "alloc-counter")]
    let after = crate::alloc_counter::snapshot().window;
    assert!(refused.is_err());
    assert_eq!(dest.len(), 0);
    assert!(!dest.spilled());
    #[cfg(feature = "alloc-counter")]
    assert!(
        after.alloc_bytes - before.alloc_bytes < 4096,
        "early failure must not build a 4096-row destination copy"
    );
    let mut retry = ScratchRelation::new(&work());
    assert_eq!(
        sink.stream_into_scratch(&mut retry, 0, 0, |_| Ok(()))
            .unwrap(),
        4096
    );
    assert_eq!(retry.len(), 4096);
}

/// D09: a tiny nonempty finalize stays on the admitted dest RAM tier.
/// `dest.spilled()` / `scratch_path()` are the environment witness —
/// post-execute `ScratchBytes` does not prove disk was unused, and
/// `WorkingBytes` does not measure peak memory.
#[test]
fn d09_tiny_finalize_never_opens_scratch_env() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink = AggregateSink::new(&finds, 2);
    let mut bindings = Bindings::new(2);
    for i in 0..3u64 {
        bindings.reset();
        bindings.set(0, i);
        bindings.set(1, i);
        sink.emit(&bindings);
    }
    assert!(!sink.group_state_spilled());
    let mut dest = ScratchRelation::new(&work());
    assert!(!dest.spilled());
    assert!(dest.scratch_path().is_none());
    let mut answer = Vec::new();
    let written = sink
        .stream_finalize(&mut dest, &mut answer, |_| Ok(()))
        .expect("tiny finalize");
    assert_eq!(written, 3);
    assert!(!dest.spilled(), "tiny dest must not open an environment");
    assert!(
        dest.scratch_path().is_none(),
        "stream_finalize must not choose the destination representation"
    );
    assert_eq!(dest.len(), 3);
    assert_eq!(sink.progress(), SinkProgress::Finish);
}

/// D09: tiny projection stream stays on the admitted dest RAM tier.
#[test]
fn d09_tiny_projection_stream_never_opens_scratch_env() {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    let mut bindings = Bindings::new(1);
    for i in 0..3u64 {
        bindings.set(0, i);
        sink.emit(&bindings);
    }
    let mut dest = ScratchRelation::new(&work());
    let written = sink
        .stream_into_scratch(&mut dest, 0, 0, |_| Ok(()))
        .expect("tiny stream");
    assert_eq!(written, 3);
    assert!(!dest.spilled());
    assert!(dest.scratch_path().is_none());
    assert_eq!(dest.len(), 3);
}

/// An explicitly disk-backed destination consumes the finalize stream
/// boundedly — one row per published group, not a reconstructed claim set.
#[test]
fn d09_large_finalize_dest_spills_boundedly() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink = AggregateSink::new(&finds, 2);
    let mut bindings = Bindings::new(2);
    for i in 0..40u64 {
        bindings.reset();
        bindings.set(0, i % 8);
        bindings.set(1, i);
        sink.emit(&bindings);
    }
    let mut dest = ScratchRelation::new(&work());
    dest.force_spill().unwrap();
    let mut answer = Vec::new();
    let written = sink
        .stream_finalize(&mut dest, &mut answer, |_| Ok(()))
        .expect("bounded spill dest");
    assert_eq!(written, 8);
    assert!(dest.spilled(), "the caller selected temporary disk storage");
    assert!(dest.scratch_path().is_some());
    assert_eq!(dest.len(), 8);
    assert_eq!(sink.progress(), SinkProgress::Finish);
}

#[test]
fn finalize_preserves_producer_cancellation_without_opening_destination() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let context = work();
    let mut sink = AggregateSink::new(finds, 2);
    sink.begin(Some(context.clone()));
    let mut bindings = Bindings::new(2);
    bindings.set(0, 1);
    bindings.set(1, 1);
    context.cancel();
    for _ in 0..STEP_QUANTUM {
        sink.emit(&bindings);
    }
    assert_eq!(sink.progress(), SinkProgress::Stop);
    let mut dest = ScratchRelation::new(&work());
    let refused = sink.stream_finalize(&mut dest, &mut Vec::new(), |_| Ok(()));
    assert!(refused.is_err());
    assert_eq!(dest.len(), 0);
    assert!(!dest.spilled());
    assert!(dest.scratch_path().is_none());
}
