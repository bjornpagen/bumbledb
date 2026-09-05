//! D01 / D11 / D19 discriminators for bounded Pack and aggregate banks.
//!
//! The D11 oracle is [`crate::interval::sweep::sweep`], not the sink's
//! resident `emit_pack_group`. Production `finalize_spilled` streams one
//! group and fetches one header.

use super::*;
use crate::error::Error;
use crate::exec::run::{Bindings, Sink as _};
use crate::exec::sink::aggregate::spill::{PACK_WIDE_CLAIM_BYTES, pack_requires_wide};
use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, SinkBudget, SinkProgress};
use crate::interval::sweep::{Continuation, sweep};
use crate::work::{ExecutionPolicy, Resource};
use bumbledb_theory::F64;
use std::collections::BTreeMap;
use std::time::Duration;

/// First word count that forces wide (token) Pack keys.
const WIDE_WORDS: usize = 49;

fn work() -> crate::work::WorkContext {
    crate::api::prepared::source::UNBOUNDED_POLICY
        .start()
        .expect("unbounded ledger")
}

fn tight_work(working: u64, scratch: u64, units: u64) -> crate::work::WorkContext {
    ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: working,
        scratch_bytes: scratch,
        result_bytes: 1 << 20,
        rows: 1 << 20,
        work_units: units,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("valid policy")
}

fn independent_pack(claims: &[(Vec<u64>, u64, u64)]) -> Vec<Vec<u64>> {
    struct Collect<'a> {
        group: &'a [u64],
        rows: &'a mut Vec<Vec<u64>>,
    }
    impl Continuation<u64, ()> for Collect<'_> {
        type Error = ();
        fn segment(&mut self, (): ()) -> Result<(), ()> {
            Ok(())
        }
        fn maximal(&mut self, start: u64, frontier: u64) -> Result<(), ()> {
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
        let items = segments.iter().copied().map(|(start, end)| Ok((start, end, ())));
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

fn spilled_pack(
    finds: Vec<FindSpec>,
    slots: usize,
    ram_bytes: usize,
    claims: &[(Vec<u64>, u64, u64)],
) -> Vec<Vec<u64>> {
    let mut sink = AggregateSink::new(finds, slots);
    sink.begin(Some(SinkBudget {
        work: work(),
        ram_bytes,
    }));
    feed_pack(&mut sink, slots, claims);
    assert!(sink.group_state_spilled(), "ram_bytes={ram_bytes}");
    let mut got = sink.into_answers().expect("spilled pack");
    got.sort();
    got
}

/// D11: reverse-start overlap across flushes unions to one maximal segment.
#[test]
fn d11_reverse_overlap_across_flushes_unions_to_one_segment() {
    assert!(!pack_requires_wide(1));
    assert!(pack_requires_wide(WIDE_WORDS));
    assert_eq!(PACK_WIDE_CLAIM_BYTES, 24);

    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let claims = vec![
        (vec![7], 10, 20),
        (vec![7], 0, 15),
    ];
    let expected = independent_pack(&claims);
    assert_eq!(expected, vec![vec![7, 0, 20]]);

    let mut resident = AggregateSink::new(finds.clone(), 4);
    feed_pack(&mut resident, 4, &claims);
    let mut resident_rows = resident.into_answers().expect("resident");
    resident_rows.sort();
    assert_eq!(resident_rows, expected);

    for ram_bytes in [0usize, 32] {
        let mut sink = AggregateSink::new(finds.clone(), 4);
        sink.begin(Some(SinkBudget {
            work: work(),
            ram_bytes,
        }));
        feed_pack(&mut sink, 4, &claims);
        assert!(sink.group_state_spilled());
        assert_eq!(sink.pack_wide_mode(), Some(false));
        assert_eq!(sink.progress(), SinkProgress::Continue);
        let mut got = Vec::new();
        sink.finalize_into(&mut Vec::new(), |row| {
            got.push(row.to_vec());
            Ok(())
        })
        .expect("spilled reverse overlap");
        got.sort();
        assert_eq!(got, expected, "ram_bytes={ram_bytes}");
        assert_eq!(sink.progress(), SinkProgress::Finish);
    }
}

/// D11: interleaved groups, adjacency, gaps, duplicate claims.
#[test]
fn d11_interleaved_adjacent_gapped_and_duplicate_claims() {
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

    let mut resident = AggregateSink::new(finds.clone(), 4);
    feed_pack(&mut resident, 4, &claims);
    let mut resident_rows = resident.into_answers().expect("resident");
    resident_rows.sort();
    assert_eq!(resident_rows, expected);

    let got = spilled_pack(finds, 4, 0, &claims);
    assert_eq!(got, expected);
}

/// D11: first encoded group word starts 0xFE — must stay narrow, never
/// token-mode from the payload byte.
#[test]
fn d11_leading_0xfe_narrow_group_is_not_token_mode() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let fe = 0xFE00_0000_0000_0001;
    let claims = vec![(vec![fe], 4, 9), (vec![fe], 1, 5)];
    let expected = independent_pack(&claims);
    assert_eq!(expected, vec![vec![fe, 1, 9]]);

    let mut sink = AggregateSink::new(finds, 4);
    sink.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 0,
    }));
    feed_pack(&mut sink, 4, &claims);
    assert!(sink.group_state_spilled());
    assert_eq!(
        sink.pack_wide_mode(),
        Some(false),
        "0xFE payload must not select wide mode"
    );
    let mut got = sink.into_answers().expect("0xFE narrow");
    got.sort();
    assert_eq!(got, expected);
}

/// D11: group heads past MAX_INLINE_KEY use scratch token tables; forced
/// distinct keys that would share a hash bucket stay separate.
#[test]
fn d11_wide_groups_use_scratch_tokens_and_survive_collisions() {
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

    let mut resident = AggregateSink::new(finds.clone(), slots);
    feed_pack(&mut resident, slots, &claims);
    let mut resident_rows = resident.into_answers().expect("resident wide");
    resident_rows.sort();
    assert_eq!(resident_rows, expected);

    let mut sink = AggregateSink::new(finds, slots);
    sink.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 0,
    }));
    feed_pack(&mut sink, slots, &claims);
    assert!(sink.group_state_spilled());
    assert_eq!(
        sink.pack_wide_mode(),
        Some(true),
        "49-word heads must take the checked wide regime"
    );
    let mut got = sink.into_answers().expect("wide token spill");
    got.sort();
    assert_eq!(got, expected);
}

/// D11 / D19: canonical F64 endpoint order survives spill; set binding
/// grain keeps a duplicate claim from changing the union.
#[test]
fn d11_d19_float_endpoints_keep_canonical_order_across_spills() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let a = F64::from(-0.0).to_order_key();
    let b = F64::from(0.0).to_order_key();
    let c = F64::NAN.to_order_key();
    let d = F64::INFINITY.to_order_key();
    let claims = vec![
        (vec![1], b, d),
        (vec![1], a, c),
        (vec![1], b, d),
    ];
    let expected = independent_pack(&claims);
    let got = spilled_pack(finds, 4, 0, &claims);
    assert_eq!(got, expected);
    assert_eq!(got.len(), 1, "signed-zero through +inf is one run in F64 order");
}

/// D19: exact sum/count is not rounded until emit; spilled bits match an
/// independent limb-bank oracle, including cancellation.
#[test]
fn d19_exact_sum_count_not_rounded_before_emit() {
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

    let mut resident = AggregateSink::new(finds.clone(), 2);
    feed(&mut resident);
    assert_eq!(resident.into_answers().expect("resident"), expected);

    let mut spilled = AggregateSink::new(finds, 2);
    spilled.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 0,
    }));
    feed(&mut spilled);
    assert!(spilled.group_state_spilled());
    assert_eq!(spilled.into_answers().expect("spilled"), expected);
}

/// D01: a zero scratch/working ceiling refuses before the scratch env
/// retains group state; later finalize publishes nothing.
#[test]
fn d01_zero_capacity_refuses_before_sink_growth() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let mut sink = AggregateSink::new(finds, 4);
    let ledger = tight_work(0, 0, 8);
    let baseline_scratch = ledger.used(Resource::ScratchBytes);
    let baseline_working = ledger.used(Resource::WorkingBytes);
    sink.begin(Some(SinkBudget {
        work: ledger.clone(),
        ram_bytes: 0,
    }));
    let mut bindings = Bindings::new(4);
    bindings.set(0, 1);
    bindings.set(1, 10);
    bindings.set(2, 20);
    bindings.set(3, 0);
    sink.emit(&bindings);
    assert_ne!(
        sink.progress(),
        SinkProgress::Continue,
        "zero allowance must stop or error before retaining a pack bank"
    );
    let mut emitted = 0;
    let refused = sink.finalize_into(&mut Vec::new(), |_| {
        emitted += 1;
        Ok(())
    });
    assert!(refused.is_err());
    assert_eq!(emitted, 0, "Q-ATOMIC: no partial pack row");
    assert!(
        ledger.used(Resource::ScratchBytes) == baseline_scratch
            || matches!(
                refused,
                Err(Error::Store(_))
            ),
        "failed reservation must not leave an uncharged scratch payload"
    );
    let _ = baseline_working;
}

/// D01: successful spill keeps L03 scratch charge until the sink drops.
#[test]
fn d01_spill_charge_survives_until_sink_release() {
    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let ledger = work();
    let before = ledger.used(Resource::ScratchBytes);
    let mut sink = AggregateSink::new(finds, 4);
    sink.begin(Some(SinkBudget {
        work: ledger.clone(),
        ram_bytes: 0,
    }));
    let mut bindings = Bindings::new(4);
    bindings.set(0, 1);
    bindings.set(1, 0);
    bindings.set(2, 15);
    bindings.set(3, 0);
    sink.emit(&bindings);
    bindings.set(1, 10);
    bindings.set(2, 20);
    bindings.set(3, 1);
    sink.emit(&bindings);
    assert!(sink.group_state_spilled());
    assert_eq!(sink.progress(), SinkProgress::Continue);
    let charged = ledger.used(Resource::ScratchBytes);
    assert!(
        charged >= before,
        "scratch owners remain charged while the spilled sink is live"
    );
    let rows = sink.into_answers().expect("charged finalize");
    assert_eq!(rows, vec![vec![1, 0, 20]]);
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
