//! Pack and exact float banks against independent oracles: Pack against
//! [`crate::interval::sweep::sweep`], not the sink's own `emit_pack_group`.

use crate::error::Error;
use crate::exec::run::{Bindings, Sink as _};
use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, SinkProgress};
use crate::interval::sweep::{Continuation, sweep};
use crate::work::WorkContext;
use bumbledb_theory::F64;
use std::collections::BTreeMap;

/// First word count that forces wide (token) Pack keys.
const WIDE_WORDS: usize = 49;

fn work() -> crate::work::WorkContext {
    WorkContext::new()
}

fn independent_pack(claims: &[(Vec<u64>, u64, u64)]) -> Vec<Vec<u64>> {
    struct Collect<'a> {
        group: &'a [u64],
        rows: &'a mut Vec<Vec<u64>>,
    }
    impl Continuation<u64, ()> for Collect<'_> {
        type Error = ();
        fn segment(&mut self, (): ()) -> std::result::Result<(), ()> {
            Ok(())
        }
        fn maximal(&mut self, start: u64, frontier: u64) -> std::result::Result<(), ()> {
            let mut row = self.group.to_vec();
            row.push(start);
            row.push(frontier);
            self.rows.push(row);
            Ok(())
        }
    }

    let mut by_group: BTreeMap<Vec<u64>, Vec<(u64, u64)>> = BTreeMap::new();
    for (group, start, end) in claims {
        by_group
            .entry(group.clone())
            .or_default()
            .push((*start, *end));
    }
    let mut rows = Vec::new();
    for (group, mut segments) in by_group {
        segments.sort_unstable();
        let items = segments
            .iter()
            .copied()
            .map(|(start, end)| Ok((start, end, ())));
        sweep(
            items,
            None,
            &mut Collect {
                group: &group,
                rows: &mut rows,
            },
        )
        .expect("oracle sweep");
    }
    rows.sort();
    rows
}

fn feed_pack(sink: &mut AggregateSink, slots: usize, claims: &[(Vec<u64>, u64, u64)]) {
    let mut bindings = Bindings::new(slots);
    for (i, (group, start, end)) in claims.iter().enumerate() {
        bindings.reset();
        for (word, value) in group.iter().enumerate() {
            bindings.set(word, *value);
        }
        let pack = group.len();
        bindings.set(pack, *start);
        bindings.set(pack + 1, *end);
        bindings.set(pack + 2, i as u64);
        sink.emit(&bindings);
    }
}

fn resident_pack(
    finds: Vec<FindSpec>,
    slots: usize,
    claims: &[(Vec<u64>, u64, u64)],
) -> Vec<Vec<u64>> {
    let mut sink = AggregateSink::new(finds, slots);
    sink.begin(Some(work()));
    feed_pack(&mut sink, slots, claims);
    let mut got = sink.into_answers().unwrap();
    got.sort();
    got
}

#[test]
fn reverse_start_overlap_unions_to_one_segment() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let claims = vec![(vec![7], 10, 20), (vec![7], 0, 15)];
    let expected = independent_pack(&claims);
    assert_eq!(expected, vec![vec![7, 0, 20]]);
    assert_eq!(resident_pack(finds, 4, &claims), expected);
}

#[test]
fn interleaved_adjacent_gapped_and_duplicate_claims() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let claims = vec![
        (vec![1], 10, 20),
        (vec![2], 100, 110),
        (vec![1], 0, 15),
        (vec![2], 110, 125),
        (vec![1], 30, 40),
        (vec![1], 40, 50),
        (vec![3], 7, 8),
        (vec![3], 20, 30),
        (vec![1], 10, 20),
        (vec![2], 100, 110),
    ];
    let expected = independent_pack(&claims);
    assert_eq!(
        expected,
        vec![
            vec![1, 0, 20],
            vec![1, 30, 50],
            vec![2, 100, 125],
            vec![3, 7, 8],
            vec![3, 20, 30],
        ]
    );
    assert_eq!(resident_pack(finds, 4, &claims), expected);
}

/// Wide group heads with equal prefixes stay separate groups.
#[test]
fn wide_group_heads_stay_separate() {
    let finds = vec![
        FindSpec::Var {
            slot: 0,
            width: WIDE_WORDS,
        },
        FindSpec::Pack { slot: WIDE_WORDS },
    ];
    let slots = WIDE_WORDS + 3;
    let mut group_a = vec![0xFF00_0000_0000_0001; WIDE_WORDS];
    let mut group_b = vec![0xFF00_0000_0000_0001; WIDE_WORDS];
    group_a[WIDE_WORDS - 1] = 1;
    group_b[WIDE_WORDS - 1] = 2;
    let claims = vec![
        (group_a.clone(), 10, 20),
        (group_b.clone(), 100, 108),
        (group_a.clone(), 0, 15),
        (group_b.clone(), 108, 120),
        (group_a, 30, 31),
        (group_b, 50, 51),
    ];
    let expected = independent_pack(&claims);
    assert_eq!(resident_pack(finds, slots, &claims), expected);
}

/// Canonical F64 endpoint order; set binding grain keeps a duplicate claim
/// from changing the union.
#[test]
fn float_endpoints_keep_canonical_order() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let a = F64::from(-0.0).to_order_key();
    let b = F64::from(0.0).to_order_key();
    let c = F64::NAN.to_order_key();
    let d = F64::INFINITY.to_order_key();
    let claims = vec![(vec![1], b, d), (vec![1], a, c), (vec![1], b, d)];
    let expected = independent_pack(&claims);
    let got = resident_pack(finds, 4, &claims);
    assert_eq!(got, expected);
    assert_eq!(
        got.len(),
        1,
        "signed-zero through +inf is one run in F64 order"
    );
}

/// Exact sum/count is not rounded until emit; the bits match an
/// independent accumulator, including cancellation.
#[test]
fn exact_sum_count_not_rounded_before_emit() {
    use crate::exec::kernel::numeric::ExactF64Accumulator;

    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Float {
            op: crate::ir::FoldOp::Sum,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Float {
            op: crate::ir::FoldOp::Mean,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Count),
    ];
    let values = [
        F64::from(1e16),
        F64::from(1.0),
        F64::from(-1e16),
        F64::from(-0.0),
        F64::NAN,
        F64::from(2.0),
    ];
    let mut oracle = ExactF64Accumulator::default();
    for value in values {
        oracle.push(value).expect("oracle cardinality");
    }
    let expected = vec![vec![
        3,
        oracle.sum().expect("nonempty").to_order_key(),
        oracle.mean().expect("nonempty").to_order_key(),
        values.len() as u64,
    ]];

    let feed = |sink: &mut AggregateSink| {
        let mut bindings = Bindings::new(2);
        for value in values {
            bindings.reset();
            bindings.set(0, 3);
            bindings.set(1, value.to_order_key());
            sink.emit(&bindings);
        }
    };

    let mut resident = AggregateSink::new(finds, 2);
    feed(&mut resident);
    assert_eq!(resident.into_answers().expect("resident"), expected);
}

#[test]
fn cancelled_pack_stops_at_its_poll_quantum_and_finalizes_no_rows() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let mut sink = AggregateSink::new(finds, 4);
    let context = work();
    sink.begin(Some(context.clone()));
    context.cancel();
    let mut bindings = Bindings::new(4);
    bindings.set(0, 1);
    bindings.set(1, 10);
    bindings.set(2, 20);
    for _ in 0..crate::exec::sink::STEP_QUANTUM {
        sink.emit(&bindings);
    }
    assert_eq!(sink.progress(), SinkProgress::Stop);
    let mut emitted = 0;
    let failure = sink
        .finalize_into(&mut Vec::new(), |_| {
            emitted += 1;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(failure, Error::Store(error) if matches!(
        error.as_ref(), crate::storage::store::StoreError::Work(crate::WorkError::Cancelled)
    )));
    assert_eq!(emitted, 0);
}

/// Finish is recorded only after a successful finalize.
#[test]
fn sink_progress_finish_after_successful_finalize() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let mut sink = AggregateSink::new(finds, 4);
    let mut bindings = Bindings::new(4);
    bindings.set(0, 1);
    bindings.set(1, 2);
    bindings.set(2, 3);
    sink.emit(&bindings);
    assert_eq!(sink.progress(), SinkProgress::Continue);
    sink.finalize_into(&mut Vec::new(), |_| Ok(())).expect("ok");
    assert_eq!(sink.progress(), SinkProgress::Finish);
}
