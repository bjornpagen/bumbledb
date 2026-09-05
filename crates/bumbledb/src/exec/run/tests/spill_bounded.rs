//! Spill-bounded derived consumption (D09). Verification: NotRun.
//! A scratch-backed stage walks through L03's charged visitor: `Err`
//! stops immediately and `Ok(false)` is a clean early stop. Peak decode
//! storage is one row. No `type_name` / `size_of`.

use super::*;
use crate::api::prepared::derived::{ScratchStage, SealedStage};
use crate::error::Error;
use crate::exec::scratch::ScratchRelation;
use crate::work::ExecutionPolicy;
use bumbledb_theory::schema::ValueType;
use std::time::Duration;

fn work() -> crate::work::WorkContext {
    ExecutionPolicy {
        input_bytes: u64::MAX,
        working_bytes: u64::MAX,
        scratch_bytes: u64::MAX,
        result_bytes: u64::MAX,
        rows: u64::MAX,
        work_units: u64::MAX,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("ledger")
}

fn encode_row(words: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(words.len() * 8);
    for word in words {
        out.extend_from_slice(&word.to_be_bytes());
    }
    out
}

fn scratch_stage(rows: &[[u64; 2]]) -> ScratchStage {
    let work = work();
    let mut dest = ScratchRelation::new(&work, 0);
    dest.force_spill().expect("spill");
    for (index, row) in rows.iter().enumerate() {
        dest.put(&(index as u64).to_be_bytes(), &encode_row(row))
            .expect("put");
    }
    ScratchStage {
        rows: dest,
        field_types: vec![ValueType::U64, ValueType::U64],
        row_words: 2,
        count: rows.len() as u64,
    }
}

/// D09: join/negation consumption of a scratch stage is one charged
/// visitor, not a Vec of every decoded row.
#[test]
fn d09_scratch_stage_visit_is_one_row_and_fallible() {
    let mut stage = scratch_stage(&[[1, 10], [2, 20], [3, 30], [4, 40]]);
    let mut seen = Vec::new();
    SealedStage::for_each_scratch_row(&mut stage, &work(), |row| {
        seen.push(row.to_vec());
        Ok(seen.len() < 2)
    })
    .expect("early stop");
    assert_eq!(seen, vec![vec![1, 10], vec![2, 20]]);

    let mut seen = 0u64;
    let refused = SealedStage::for_each_scratch_row(&mut stage, &work(), |_| {
        seen += 1;
        if seen == 2 {
            return Err(Error::DerivedBudgetExceeded {
                rounds: 0,
                tuples: 2,
            });
        }
        Ok(true)
    });
    assert!(refused.is_err());
    assert_eq!(seen, 2, "no later row after the failing visit");
}