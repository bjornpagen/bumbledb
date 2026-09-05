//! Spill-bounded stage drain (D09). Independent of reach.rs seals: those
//! must call [`ProjectionSink::stream_into_scratch`] /
//! [`AggregateSink::stream_finalize`] on an admitted dest. Visitor `Err`
//! stops immediately. Tiny dests never open an environment; `ScratchBytes`
//! and `WorkingBytes` are not that witness. No `type_name`/`size_of`.

use super::*;
use crate::error::Error;
use crate::exec::run::{Bindings, Sink as _};
use crate::exec::scratch::{DEFAULT_RAM_BYTES, ScratchRelation};
use crate::exec::sink::{
    AggSpec, AggregateSink, FindSpec, ProjectionSink, SinkBudget, SinkProgress,
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

fn decode_stage_rows(dest: &mut ScratchRelation) -> Vec<Vec<u64>> {
    let mut rows = Vec::new();
    dest.visit(&mut |_, value| {
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
    let finds = [FindSpec::Var { slot: 0, width: 1 }, FindSpec::Var {
        slot: 1,
        width: 1,
    }];
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
    let refused = dest.force_spill().and_then(|()| sink.stream_into_scratch(&mut dest, 0, 0));
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
