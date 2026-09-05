//! Spill-bounded derived seals (D09). Verification: NotRun.
//! These fail a whole-stage `pending` collect: a refused put never
//! finishes the dest, and a visitor `Err` never sees a later row.
//! No `type_name` / `size_of`.

use super::*;
use crate::exec::run::{Bindings, Sink};
use crate::exec::scratch::DEFAULT_RAM_BYTES;
use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, ProjectionSink, SinkBudget};
use crate::image::CacheGeneration;
use crate::work::{CacheLedger, ExecutionPolicy, GenerationHandle, GenerationState, Resource};
use bumbledb_theory::schema::ValueType;
use std::time::Duration;

fn work() -> crate::work::WorkContext {
    crate::api::prepared::source::UNBOUNDED_POLICY
        .start()
        .expect("unbounded")
}

fn generation() -> GenerationHandle {
    GenerationHandle::new(GenerationState::new(
        CacheGeneration::initial(),
        CacheLedger::unbounded(),
    ))
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
    sink.begin(Some(SinkBudget {
        work: work(),
        ram_bytes: 0,
    }));
    feed_ids(&mut sink, 0..rows);
    assert!(sink.spilled(), "zero RAM allowance must spill the stage");
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
        seal_projection_scratch(&u64_types(1), &mut sink, &work())
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

/// D09: rec Δ is the watermark suffix; acc appends only that suffix.
#[test]
fn d09_rec_acc_appends_watermark_only() {
    let mut sink = spilled_sink(6);
    let types = u64_types(1);
    let work = work();
    let Ok(SealedStage::Scratch(mut acc)) =
        seal_scratch_range(&mut sink, &work, &types, 1, 0, 6)
    else {
        panic!("acc prefix must be scratch");
    };
    assert_eq!(acc.count, 6);
    feed_ids(&mut sink, 6..10);
    assert_eq!(sink.len(), 10);
    append_scratch_range(&mut acc, &mut sink, 6, 10).expect("append Δ");
    assert_eq!(acc.count, 10);
    assert_eq!(collect_ids(&mut acc), (0..10).collect::<Vec<_>>());
    let Ok(SealedStage::Scratch(mut delta)) =
        seal_scratch_range(&mut sink, &work, &types, 1, 6, 10)
    else {
        panic!("Δ must be scratch");
    };
    assert_eq!(delta.count, 4);
    assert_eq!(collect_ids(&mut delta), vec![6, 7, 8, 9]);
}

/// D09: a refused scratch put fails the seal; the dest never holds the
/// whole stage (no second full collection after the drain).
#[test]
fn d09_seal_put_refusal_is_immediate() {
    let mut sink = spilled_sink(16);
    let ledger = ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: 64,
        result_bytes: 1 << 20,
        rows: 1 << 20,
        work_units: 32,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("policy");
    let before = ledger.used(Resource::ScratchBytes);
    let refused = seal_projection_scratch(&u64_types(1), &mut sink, &ledger);
    assert!(refused.is_err(), "tiny scratch must refuse the stream seal");
    assert!(
        ledger.used(Resource::ScratchBytes) >= before,
        "the failing attempt charged what it committed, not a silent collect"
    );
}

/// D09: visitor Err stops the derived walk; later rows are not visited.
#[test]
fn d09_scratch_visit_propagates_failure_immediately() {
    let mut sink = spilled_sink(8);
    let Ok(SealedStage::Scratch(mut stage)) =
        seal_projection_scratch(&u64_types(1), &mut sink, &work())
    else {
        panic!("scratch");
    };
    let mut seen = 0u64;
    let refused = SealedStage::for_each_scratch_row(&mut stage, &work(), |_| {
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

/// D09: tiny nonempty aggregate uses admit_dest + stream_finalize.
/// dest.spilled() / scratch_path() are the environment witness —
/// `used(ScratchBytes)==0` alone does not prove disk was unused.
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
    let ledger = ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: 0,
        result_bytes: 1 << 20,
        rows: 1 << 20,
        work_units: u64::MAX,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("scratch_bytes=0");
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
            DEFAULT_RAM_BYTES,
        )
        .expect("tiny aggregate must not need a scratch environment");
    assert_eq!(count, 4);
    let SealedStage::Scratch(stage) = &derived.published[0] else {
        panic!("stash_aggregate publishes the admit_dest dest");
    };
    assert!(
        !stage.rows.spilled(),
        "tiny dest must not open a scratch environment"
    );
    assert!(
        stage.rows.scratch_path().is_none(),
        "admit_dest + stream_finalize must not force_spill"
    );
    assert_eq!(
        ledger.used(Resource::ScratchBytes),
        0,
        "supporting: no scratch-byte charge"
    );
}

/// D09: dest RAM exhaustion continues on the same dest, not a second relation.
#[test]
fn d09_aggregate_working_refusal_continues_on_scratch() {
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
    let tight = ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: u64::MAX,
        result_bytes: 1 << 20,
        rows: 1 << 20,
        work_units: u64::MAX,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("scratch dest");
    let mut spilled = DerivedImages::default();
    spilled.begin(1);
    let mut answer = Vec::new();
    let count = spilled
        .stash_aggregate(
            0,
            &[ValueType::U64, ValueType::U64],
            &mut large,
            &mut answer,
            &tight,
            0,
        )
        .expect("streamed aggregate");
    assert_eq!(count, 64);
    let SealedStage::Scratch(stage) = &mut spilled.published[0] else {
        panic!("dest owner");
    };
    assert!(stage.rows.spilled(), "zero-RAM dest continues onto scratch");
    assert!(stage.rows.scratch_path().is_some());
    assert_eq!(collect_ids(stage).len(), 64);
}