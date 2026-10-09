//! `SQLite` measured under exactly the engine's protocol: prepared statements
//! reused, every value decoded, and the fairness pragmas checked first.
use bumbledb::Value;
use bumbledb::schema::ValueType;

use crate::oracle::sqlite::sqlmap;
use crate::oracle::sqlite::translate::ParamSlot;

mod cap;
mod cold_containment_walk;
mod cold_containment_walk_delete;
mod commits;
mod durable;
mod fairness_check;
mod insert_stream;
mod new;
mod open_for_bench;
mod sample;
#[cfg(test)]
mod tests;

pub use cap::{CapMs, CapOutcome, DEFAULT_CAP, with_cap};
pub use cold_containment_walk::cold_containment_walk;
pub use cold_containment_walk_delete::cold_containment_walk_delete;
pub use commits::{commit_batch, commit_single};
pub use durable::{DURABILITY, SQLITE_SYNC, assert_durable_parity, configure_durable};
pub use insert_stream::insert_stream;
pub use open_for_bench::{mmap_whole_file, open_for_bench};
pub use sample::{sample, sample_args, sample_capped};

pub struct PreparedFamily<'c> {
    stmt: rusqlite::Statement<'c>,
    param_order: Vec<ParamSlot>,

    signature: Vec<ValueType>,
}

#[must_use]
pub fn bind_params(order: &[ParamSlot], params: &[Value]) -> Vec<rusqlite::types::Value> {
    order
        .iter()
        .map(|slot| match slot {
            ParamSlot::Whole(p) => sqlmap::to_sql_value(&params[usize::from(p.0)]),
            ParamSlot::Start(p) => sqlmap::interval_halves(&params[usize::from(p.0)]).0,
            ParamSlot::End(p) => sqlmap::interval_halves(&params[usize::from(p.0)]).1,
        })
        .collect()
}

/// # Panics
/// On a set arg in a placeholder slot (a translator invariant).
#[must_use]
pub fn bind_args(
    order: &[ParamSlot],
    draw: &[crate::oracle::naive::ParamValue],
) -> Vec<rusqlite::types::Value> {
    use crate::oracle::naive::ParamValue;
    let scalar = |p: &bumbledb::ParamId| match &draw[usize::from(p.0)] {
        ParamValue::Scalar(value) => value,
        ParamValue::Set(_) => panic!("a set param has no placeholder slot"),
    };
    order
        .iter()
        .map(|slot| match slot {
            ParamSlot::Whole(p) => sqlmap::to_sql_value(scalar(p)),
            ParamSlot::Start(p) => sqlmap::interval_halves(scalar(p)).0,
            ParamSlot::End(p) => sqlmap::interval_halves(scalar(p)).1,
        })
        .collect()
}

/// The fairness contract as code, checked before measuring.
pub struct FairnessCheck;

pub(crate) const POSTING_INSERT: &str = "INSERT INTO \"Posting\" VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
