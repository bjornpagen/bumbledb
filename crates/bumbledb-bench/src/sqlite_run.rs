//! Both engines touch every value; decoding into `compare::Owned` is `SQLite`
//! measured under exactly the engine's protocol, with the
use bumbledb::Value;
use bumbledb::schema::ValueType;

use crate::sqlmap;
use crate::translate::ParamSlot;

mod cap;
mod cold_containment_walk;
mod cold_containment_walk_delete;
mod commits;
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
    draw: &[crate::naive::ParamValue],
) -> Vec<rusqlite::types::Value> {
    use crate::naive::ParamValue;
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

/// The fairness contract as code — run before measuring, so a
pub struct FairnessCheck;

pub(crate) const POSTING_INSERT: &str = "INSERT INTO \"Posting\" VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
