//! The `SQLite` runner and the fairness contract (docs/architecture/60-validation.md):
//! `SQLite` measured under exactly the engine's protocol, with the
//! fairness rules encoded as assertions — a benchmark nobody can dismiss
//! as a strawman.
//!
//! Symmetry argument for the timed path: bumbledb materializes every row
//! into a `ResultBuffer`; the `SQLite` side does typed `get_ref` reads on
//! every column of every row (a full drain — no lazy-cursor discounts).
//! Both engines touch every value; decoding into `compare::Owned` is
//! verify's job, never the timed path's.

use bumbledb::Value;
use bumbledb::schema::ValueType;

use crate::sqlmap;
use crate::translate::ParamSlot;

mod bulk;
mod cold_containment_walk;
mod commits;
mod fairness_check;
mod new;
mod open_for_bench;
mod sample;
#[cfg(test)]
mod tests;

pub use bulk::bulk;
pub use cold_containment_walk::cold_containment_walk;
pub use commits::{commit_batch, commit_single};
pub use open_for_bench::open_for_bench;
pub use sample::{sample, sample_args};

/// A family's statement, prepared exactly once and reused across every
/// warmup and sample (mirroring `PreparedQuery`). This is the **only**
/// construction site for timed `SQLite` statements — statement reuse is
/// asserted by type: no re-prepare path exists.
pub struct PreparedFamily<'c> {
    stmt: rusqlite::Statement<'c>,
    param_order: Vec<ParamSlot>,
    result_types: Vec<ValueType>,
}

/// The positional bindings of one execution: each placeholder slot takes
/// its param's whole value or one endpoint of an interval-typed param,
/// through the normative mapping (`crate::sqlmap`).
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

/// [`bind_params`] over a family draw: scalar positions bind through the
/// slot order; set positions never appear there (their element lists are
/// SQL literals in the re-rendered statement).
///
/// # Panics
///
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
/// misconfigured oracle fails the run instead of flattering the engine.
pub struct FairnessCheck;

/// The `SQLite` posting insert, mirroring the corpus loader's shape.
const POSTING_INSERT: &str = "INSERT INTO \"Posting\" VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
