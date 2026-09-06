//! Spill-bounded stage drain (D09). Independent of reach.rs seals: those
//! must call [`ProjectionSink::stream_into_scratch`] /
//! [`AggregateSink::stream_finalize`] on an admitted dest. Visitor `Err`
//! stops immediately. Tiny dests never open an environment; `ScratchBytes`
//! and `WorkingBytes` are not that witness. No `type_name`/`size_of`.

use crate::error::Error;
use crate::exec::run::{Bindings, Sink as _};
use crate::exec::scratch::{DEFAULT_RAM_BYTES, ScratchRelation};
use crate::exec::sink::{
    AggSpec, AggregateSink, FindSpec, ProjectionSink, STEP_QUANTUM, SinkBudget, SinkProgress,
    SpillSet,
};
use crate::work::{ExecutionPolicy, Resource};
use std::time::Duration;

fn work() -> crate::work::WorkContext {
    crate::api::prepared::source::UNBOUNDED_POLICY
        .start()
        .expect("unbounded ledger")
}

fn tight_work(scratch: u64, units: u64) -> crate::work::WorkContext {
    ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: scratch,
        result_bytes: 1 << 20,
        rows: 1 << 20,
        work_units: units,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("valid policy")
}

fn assert_hashed_kernel_state(actual: &SpillSet, dynamic: &SpillSet) {
    assert!(actual.unique_rows.is_none() && dynamic.unique_rows.is_none());
    assert_eq!(actual.len(), dynamic.len());
    assert_eq!(actual.spilled(), dynamic.spilled());
    assert_eq!(actual.pending_steps, dynamic.pending_steps);
    assert_eq!(actual.progress(), dynamic.progress());
    assert_eq!(
        actual.error.as_ref().map(ToString::to_string),
        dynamic.error.as_ref().map(ToString::to_string),
    );
    if let (Some(actual), Some(dynamic)) = (&actual.budget, &dynamic.budget) {
        for resource in [
            Resource::InputBytes,
            Resource::WorkingBytes,
            Resource::ScratchBytes,
            Resource::ResultBytes,
            Resource::Rows,
            Resource::WorkUnits,
        ] {
            assert_eq!(
                actual.work.used(resource),
                dynamic.work.used(resource),
                "{resource:?}"
            );
        }
    }
}

#[test]
fn direct_hashed_kernel_preserves_duplicate_results_order_and_spill_boundary() {
    for (arity, allowance) in [(0, usize::MAX), (1, usize::MAX), (2, 64), (3, 0)] {
        let mut direct = SpillSet::with_capacity_hint(arity, 0, true);
        let mut dynamic = SpillSet::with_capacity_hint(arity, 0, true);
        for budgeted in [true, false] {
            for set in [&mut direct, &mut dynamic] {
                set.clear();
                set.begin(budgeted.then(|| SinkBudget {
                    work: work(),
                    ram_bytes: allowance,
                }));
            }
            let mut expected = Vec::new();
            for emit in 0..20u64 {
                let row: Vec<_> = (0..arity)
                    .map(|word| emit / 2 + u64::try_from(word).unwrap())
                    .collect();
                let new = !expected.contains(&row);
                if new {
                    expected.push(row.clone());
                }
                assert_eq!(direct.insert_inner::<false>(&row), new);
                assert_eq!(dynamic.insert(&row), new);
                assert_hashed_kernel_state(&direct, &dynamic);
                if budgeted && arity == 2 && emit == 2 {
                    assert!(!direct.spilled(), "two rows exactly fit the allowance");
                }
                if budgeted && arity == 2 && emit == 3 {
                    assert!(
                        direct.spilled(),
                        "even a duplicate triggers the conservative pre-probe threshold"
                    );
                }
            }
            for set in [&mut direct, &mut dynamic] {
                let mut actual = Vec::new();
                set.for_each_since(0, &mut |row| {
                    actual.push(row.to_vec());
                    Ok(true)
                })
                .unwrap();
                assert_eq!(
                    actual, expected,
                    "first-emission order survives scratch and reset"
                );
            }
        }
    }
}

#[test]
fn direct_hashed_kernel_preserves_refusal_polling_stickiness_and_reuse() {
    for (allowance, scratch, units, cancelled) in [
        (64, 0, u64::MAX, false),
        (usize::MAX, 1 << 20, u64::from(STEP_QUANTUM - 1), false),
        (usize::MAX, 1 << 20, u64::MAX, true),
    ] {
        let mut direct = SpillSet::with_capacity_hint(2, 0, true);
        let mut dynamic = SpillSet::with_capacity_hint(2, 0, true);
        for set in [&mut direct, &mut dynamic] {
            let ledger = tight_work(scratch, units);
            if cancelled {
                ledger.cancel();
            }
            set.begin(Some(SinkBudget {
                work: ledger,
                ram_bytes: allowance,
            }));
        }
        let emits = if scratch == 0 { 3 } else { STEP_QUANTUM };
        for emit in 0..emits {
            let row = if scratch == 0 {
                [u64::from(emit), 7]
            } else {
                [1, 7]
            };
            let result = direct.insert_inner::<false>(&row);
            assert_eq!(result, dynamic.insert(&row));
            assert_hashed_kernel_state(&direct, &dynamic);
            if emit + 1 < emits {
                assert_eq!(direct.progress(), SinkProgress::Continue);
            } else {
                assert!(!result);
                assert_eq!(direct.progress(), SinkProgress::Stop);
            }
        }
        assert_eq!(direct.len(), if scratch == 0 { 2 } else { 1 });
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
}

#[test]
fn bulk_hashed_insert_preserves_duplicate_spill_boundaries_and_ordered_early_stop() {
    for arity in [1, 2, 9] {
        for allowance in [None, Some(usize::MAX), Some(2 * (arity * 8 + 16)), Some(0)] {
            let mut bulk = SpillSet::with_capacity_hint(arity, 0, true);
            let mut control = SpillSet::with_capacity_hint(arity, 0, true);
            for set in [&mut bulk, &mut control] {
                set.begin(allowance.map(|ram_bytes| SinkBudget {
                    work: work(),
                    ram_bytes,
                }));
            }
            let rows: Vec<u64> = (0..20u64)
                .flat_map(|id| (0..arity).map(move |word| id / 2 + u64::try_from(word).unwrap()))
                .collect();
            for (start, end) in [(0, 0), (0, 1), (1, 2), (2, 3), (3, 4), (4, 10), (10, 20)] {
                let words = &rows[start * arity..end * arity];
                for row in words.chunks_exact(arity) {
                    control.insert_inner::<false>(row);
                }
                bulk.insert_hashed_rows(words);
                assert_hashed_kernel_state(&bulk, &control);
                if !bulk.spilled() {
                    assert_eq!(
                        bulk.ram_iter_since(0).collect::<Vec<_>>(),
                        control.ram_iter_since(0).collect::<Vec<_>>()
                    );
                }
                if allowance == Some(2 * (arity * 8 + 16)) && end == 3 {
                    assert!(!bulk.spilled(), "two distinct rows exactly fit");
                }
                if allowance == Some(2 * (arity * 8 + 16)) && end == 4 {
                    assert!(bulk.spilled(), "the next duplicate still triggers spill");
                }
            }
            let expected: Vec<Vec<u64>> = (0..10u64)
                .map(|id| {
                    (0..arity)
                        .map(|word| id + u64::try_from(word).unwrap())
                        .collect()
                })
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
fn bulk_hashed_insert_matches_poll_refusal_stickiness_and_reuse() {
    for (allowance, scratch, units, cancelled) in [
        (usize::MAX, 1 << 20, 0, false),
        (usize::MAX, 1 << 20, 255, false),
        (usize::MAX, 1 << 20, 256, false),
        (usize::MAX, 1 << 20, 511, false),
        (usize::MAX, 1 << 20, 512, false),
        (usize::MAX, 1 << 20, u64::MAX, true),
        (64, 0, u64::MAX, false),
    ] {
        for duplicates in [false, true] {
            let mut bulk = SpillSet::with_capacity_hint(2, 0, true);
            let mut control = SpillSet::with_capacity_hint(2, 0, true);
            for set in [&mut bulk, &mut control] {
                let ledger = tight_work(scratch, units);
                if cancelled {
                    ledger.cancel();
                }
                set.begin(Some(SinkBudget {
                    work: ledger,
                    ram_bytes: allowance,
                }));
            }
            for (start, end) in [
                (0, 0),
                (0, 1),
                (1, 250),
                (250, 255),
                (255, 256),
                (256, 257),
                (257, 520),
            ] {
                let words: Vec<_> = (start..end)
                    .flat_map(|id| [if duplicates { id / 2 } else { id }, 7])
                    .collect();
                for row in words.as_chunks::<2>().0 {
                    control.insert_inner::<false>(row);
                }
                bulk.insert_hashed_rows(&words);
                assert_hashed_kernel_state(&bulk, &control);
                assert!(!bulk.spilled(), "either resident or failed before spilling");
                assert_eq!(
                    bulk.ram_iter_since(0).collect::<Vec<_>>(),
                    control.ram_iter_since(0).collect::<Vec<_>>()
                );
            }
            // A fresh execution must discard sticky refusal/pending state
            // without changing the retained map's ordinary growth behavior.
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
    for allowance in [0, 64, usize::MAX] {
        let mut hashed = SpillSet::with_capacity_hint(2, 0, true);
        let mut append = SpillSet::with_capacity_hint(2, 0, true);
        append.elide_output_hashing(witness);
        for round in 0..2 {
            hashed.clear();
            append.clear();
            hashed.begin(Some(SinkBudget {
                work: work(),
                ram_bytes: allowance,
            }));
            append.begin(Some(SinkBudget {
                work: work(),
                ram_bytes: allowance,
            }));
            for i in (0..259u64).rev() {
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
    let ledger = tight_work(1 << 20, u64::from(STEP_QUANTUM - 1));
    let mut append = SpillSet::with_capacity_hint(2, 0, true);
    append.elide_output_hashing(output_key_witness());
    append.begin(Some(SinkBudget {
        work: ledger.clone(),
        ram_bytes: usize::MAX,
    }));
    for i in 0..STEP_QUANTUM - 1 {
        assert!(append.insert(&[u64::from(i), 0]));
    }
    assert_eq!(ledger.used(Resource::WorkUnits), 0);
    assert_eq!(append.pending_steps, STEP_QUANTUM - 1);
    assert!(!append.insert(&[u64::from(STEP_QUANTUM), 0]));
    assert_eq!(append.progress(), SinkProgress::Stop);
    assert_eq!(append.len(), (STEP_QUANTUM - 1) as usize);
    assert!(!append.insert(&[9999, 0]));
    let capacity = append.unique_rows.as_ref().unwrap().words.capacity();
    append.clear();
    append.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: usize::MAX,
    }));
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
fn failed_proved_output_spill_retains_the_previous_ordered_rows() {
    let mut append = SpillSet::with_capacity_hint(2, 0, true);
    append.elide_output_hashing(output_key_witness());
    append.begin(Some(SinkBudget {
        work: tight_work(0, 1 << 20),
        ram_bytes: 64,
    }));
    assert!(append.insert(&[1, 2]));
    assert!(append.insert(&[3, 4]));
    assert!(!append.insert(&[5, 6]));
    assert!(!append.spilled());
    assert!(append.error.is_some());
    assert_eq!(append.len(), 2);
    assert!(!append.insert(&[7, 8]));
    assert_eq!(
        append.ram_iter_since(0).collect::<Vec<_>>(),
        vec![&[1, 2][..], &[3, 4][..]]
    );
}

fn bulk_unique_set(allowance: usize, scratch: u64, units: u64) -> SpillSet {
    let mut set = SpillSet::with_capacity_hint(2, 0, true);
    set.elide_output_hashing(output_key_witness());
    // Exactly three words: the two-word rows leave a one-word tail.
    let rows = set.unique_rows.as_mut().unwrap();
    rows.words = vec![0; 3].into_boxed_slice().into_vec();
    rows.words.clear();
    assert_eq!(rows.words.capacity(), 3);
    set.begin(Some(SinkBudget {
        work: tight_work(scratch, units),
        ram_bytes: allowance,
    }));
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
    for resource in [
        Resource::InputBytes,
        Resource::WorkingBytes,
        Resource::ScratchBytes,
        Resource::ResultBytes,
        Resource::Rows,
        Resource::WorkUnits,
    ] {
        assert_eq!(
            actual.budget.as_ref().unwrap().work.used(resource),
            expected.budget.as_ref().unwrap().work.used(resource),
            "{resource:?}"
        );
    }
}

#[test]
fn bulk_unique_append_matches_each_row_at_capacity_and_poll_boundaries() {
    for units in [0, 255, 256, 511, 512, 1 << 20] {
        let mut control = bulk_unique_set(usize::MAX, 1 << 20, units);
        let mut bulk = bulk_unique_set(usize::MAX, 1 << 20, units);
        // Separate calls exercise empty input, partial pending quanta, the
        // boundary row itself, and growth with non-row-aligned capacity.
        for (start, end) in [
            (0, 0),
            (0, 1),
            (1, 250),
            (250, 255),
            (255, 256),
            (256, 257),
            (257, 520),
        ] {
            let rows: Vec<u64> = (start..end).flat_map(|id| [id, id % 7]).collect();
            for row in rows.as_chunks::<2>().0 {
                control.insert(row);
            }
            bulk.insert_unique_rows(&rows);
            assert_bulk_unique_state(&bulk, &control);
        }
        let expected_prefix = if units < 256 {
            255
        } else if units < 512 {
            511
        } else {
            520
        };
        assert_eq!(bulk.len(), expected_prefix);

        // A successful new execution reuses precisely the retained capacity
        // and no old sticky failure or partial quantum.
        for set in [&mut control, &mut bulk] {
            set.clear();
            set.begin(Some(SinkBudget {
                work: work(),
                ram_bytes: usize::MAX,
            }));
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
fn bulk_unique_append_matches_spill_threshold_order_and_failure_prefix() {
    // Spill before any row, between rows, immediately before a poll,
    // or immediately after a poll. One-byte slack must not admit another row.
    for allowance in [0, 65, 255 * 32, 256 * 32] {
        let mut control = bulk_unique_set(allowance, 1 << 20, 1 << 20);
        let mut bulk = bulk_unique_set(allowance, 1 << 20, 1 << 20);
        let rows: Vec<u64> = (0..259u64).rev().flat_map(|id| [id, id % 7]).collect();
        for row in rows.as_chunks::<2>().0 {
            control.insert(row);
        }
        bulk.insert_unique_rows(&rows);
        assert_bulk_unique_state(&bulk, &control);
        assert!(bulk.spilled());
        let mut drained = Vec::new();
        bulk.for_each_since(0, &mut |row| {
            drained.extend_from_slice(row);
            Ok(true)
        })
        .unwrap();
        assert_eq!(drained, rows);
        let mut control_drained = Vec::new();
        control
            .for_each_since(0, &mut |row| {
                control_drained.extend_from_slice(row);
                Ok(true)
            })
            .unwrap();
        assert_eq!(drained, control_drained);
        assert_bulk_unique_state(&bulk, &control);
    }

    for (allowance, prefix) in [(64, 2), (255 * 32, 255)] {
        let mut control = bulk_unique_set(allowance, 0, 1 << 20);
        let mut bulk = bulk_unique_set(allowance, 0, 1 << 20);
        let rows: Vec<u64> = (0..259).flat_map(|id| [id, 0]).collect();
        for row in rows.as_chunks::<2>().0 {
            control.insert(row);
        }
        bulk.insert_unique_rows(&rows);
        assert_bulk_unique_state(&bulk, &control);
        assert!(bulk.error.is_some());
        assert!(!bulk.spilled());
        assert_eq!(bulk.len(), prefix);
        control.insert(&[9999, 0]);
        bulk.insert_unique_rows(&[9999, 0]);
        assert_bulk_unique_state(&bulk, &control);
        assert_eq!(bulk.len(), prefix);
    }
}

#[test]
fn bulk_unique_append_observes_cancellation_at_the_same_poll_row() {
    let mut control = bulk_unique_set(usize::MAX, 1 << 20, 1 << 20);
    let mut bulk = bulk_unique_set(usize::MAX, 1 << 20, 1 << 20);
    let prefix: Vec<u64> = (0..250).flat_map(|id| [id, 0]).collect();
    for row in prefix.as_chunks::<2>().0 {
        control.insert(row);
    }
    bulk.insert_unique_rows(&prefix);
    for set in [&control, &bulk] {
        set.budget.as_ref().unwrap().work.cancel();
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
    let mut set = bulk_unique_set(usize::MAX, 1 << 20, 1 << 20);
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
    let ledger = tight_work(1 << 20, u64::from(STEP_QUANTUM - 1));
    let mut seen = SpillSet::with_capacity_hint(1, 0, false);
    seen.begin(Some(SinkBudget {
        work: ledger.clone(),
        ram_bytes: usize::MAX,
    }));
    assert!(seen.insert(&[7]));
    for _ in 1..STEP_QUANTUM - 1 {
        assert!(!seen.insert(&[7]));
    }
    assert_eq!(seen.pending_steps, STEP_QUANTUM - 1);
    assert_eq!(ledger.used(Resource::WorkUnits), 0);
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
fn exact_spill_threshold_preserves_duplicates_and_insertion_order_across_tiers() {
    let mut seen = SpillSet::with_capacity_hint(2, 0, true);
    seen.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 64, // two conservatively charged two-word keys
    }));
    assert!(seen.insert(&[1, 2]));
    assert!(seen.insert(&[3, 4]));
    assert!(
        !seen.spilled(),
        "equality with the threshold remains resident"
    );
    assert!(!seen.insert(&[1, 2]));
    assert!(
        seen.spilled(),
        "the pre-probe threshold also applies to duplicates"
    );
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
fn failed_spill_keeps_the_resident_set_and_refuses_all_later_insertions() {
    let mut seen = SpillSet::with_capacity_hint(2, 0, true);
    seen.begin(Some(SinkBudget {
        work: tight_work(0, 1 << 20),
        ram_bytes: 64,
    }));
    assert!(seen.insert(&[1, 2]));
    assert!(seen.insert(&[3, 4]));
    assert!(!seen.insert(&[5, 6]));
    assert!(!seen.spilled());
    assert!(seen.error.is_some());
    assert!(!seen.insert(&[7, 8]));
    assert_eq!(seen.len(), 2);
    let rows: Vec<_> = seen.ram_iter_since(0).map(<[u64]>::to_vec).collect();
    assert_eq!(rows, vec![vec![1, 2], vec![3, 4]]);
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
    spilled.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 0,
    }));
    feed(&mut spilled);
    assert!(spilled.spilled());
    let mut dest = ScratchRelation::new(&work(), 0);
    dest.force_spill().expect("dest");
    let written = spilled
        .stream_into_scratch(&mut dest, 0, 0)
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
            return Err(Error::DerivedBudgetExceeded {
                rounds: 0,
                tuples: 3,
            });
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
    sink.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 0,
    }));
    let mut bindings = Bindings::new(2);
    for i in 0..40u64 {
        bindings.reset();
        bindings.set(0, i % 8);
        bindings.set(1, i);
        sink.emit(&bindings);
    }
    assert!(sink.group_state_spilled());
    let mut dest = ScratchRelation::new(&work(), 0);
    dest.force_spill().expect("dest");
    let mut answer = Vec::new();
    let written = sink
        .stream_finalize_into_scratch(&mut dest, &mut answer)
        .expect("stream finalize");
    assert_eq!(written, 8);
    assert_eq!(dest.len(), 8);
    assert_eq!(sink.progress(), SinkProgress::Finish);
}

/// D09: a refused dest put leaves only the rows that committed.
#[test]
fn d09_stream_put_refusal_does_not_buffer_the_rest() {
    let finds = [FindSpec::Var { slot: 0, width: 1 }];
    let mut sink = ProjectionSink::with_capacity_hint(&finds, 1, 0);
    sink.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 0,
    }));
    let mut bindings = Bindings::new(1);
    for i in 0..16u64 {
        bindings.set(0, i);
        sink.emit(&bindings);
    }
    let ledger = tight_work(64, 32);
    let baseline = ledger.used(Resource::ScratchBytes);
    let mut dest = ScratchRelation::new(&ledger, 0);
    let refused = dest
        .force_spill()
        .and_then(|()| sink.stream_into_scratch(&mut dest, 0, 0));
    assert!(refused.is_err(), "tiny scratch must refuse a 16-row stream");
    assert!(
        dest.len() < 16,
        "failure must not finish a second full collection"
    );
    let _ = baseline;
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
    let mut dest = AggregateSink::admit_dest(&work(), DEFAULT_RAM_BYTES);
    assert!(!dest.spilled());
    assert!(dest.scratch_path().is_none());
    let mut answer = Vec::new();
    let written = sink
        .stream_finalize(&mut dest, &mut answer)
        .expect("tiny finalize");
    assert_eq!(written, 3);
    assert!(!dest.spilled(), "tiny dest must not open an environment");
    assert!(
        dest.scratch_path().is_none(),
        "admit_dest + stream_finalize must not force_spill"
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
    let mut dest = ProjectionSink::admit_dest(&work(), DEFAULT_RAM_BYTES);
    let written = sink
        .stream_into_scratch(&mut dest, 0, 0)
        .expect("tiny stream");
    assert_eq!(written, 3);
    assert!(!dest.spilled());
    assert!(dest.scratch_path().is_none());
    assert_eq!(dest.len(), 3);
}

/// D09: a dest admitted with no RAM allowance continues onto scratch
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
    let mut dest = AggregateSink::admit_dest(&work(), 0);
    let mut answer = Vec::new();
    let written = sink
        .stream_finalize(&mut dest, &mut answer)
        .expect("bounded spill dest");
    assert_eq!(written, 8);
    assert!(dest.spilled(), "zero-RAM dest must continue onto scratch");
    assert!(dest.scratch_path().is_some());
    assert_eq!(dest.len(), 8);
    assert_eq!(sink.progress(), SinkProgress::Finish);
}

/// D09: a producer finalize error surfaces before dest opens an environment.
#[test]
fn d09_finalize_preserves_producer_error_without_opening_dest() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink = AggregateSink::new(&finds, 2);
    sink.begin(Some(SinkBudget {
        work: tight_work(0, 1),
        ram_bytes: 0,
    }));
    let mut bindings = Bindings::new(2);
    bindings.set(0, 1);
    bindings.set(1, 1);
    sink.emit(&bindings);
    let mut dest = AggregateSink::admit_dest(&work(), DEFAULT_RAM_BYTES);
    let mut answer = Vec::new();
    let refused = sink.stream_finalize(&mut dest, &mut answer);
    assert!(refused.is_err(), "sticky producer error must surface");
    assert!(!dest.spilled());
    assert!(dest.scratch_path().is_none());
}
