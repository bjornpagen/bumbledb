//! The `SQLite` runner and the fairness contract (docs/architecture/50-validation.md):
//! `SQLite` measured under exactly the engine's protocol, with the
//! fairness rules encoded as assertions — a benchmark nobody can dismiss
//! as a strawman.
//!
//! Symmetry argument for the timed path: bumbledb materializes every row
//! into a `ResultBuffer`; the `SQLite` side does typed `get_ref` reads on
//! every column of every row (a full drain — no lazy-cursor discounts).
//! Both engines touch every value; decoding into `compare::Owned` is
//! verify's job, never the timed path's.

use bumbledb::schema::ValueType;
use bumbledb::ParamId;

mod bulk;
mod cold_fk_walk;
mod commits;
mod fairness_check;
mod new;
mod open_for_bench;
mod sample;
#[cfg(test)]
mod tests;

pub use bulk::bulk;
pub use cold_fk_walk::cold_fk_walk;
pub use commits::{commit_batch, commit_single};
pub use open_for_bench::open_for_bench;
pub use sample::sample;

/// A family's statement, prepared exactly once and reused across every
/// warmup and sample (mirroring `PreparedQuery`). This is the **only**
/// construction site for timed `SQLite` statements — statement reuse is
/// asserted by type: no re-prepare path exists.
pub struct PreparedFamily<'c> {
    stmt: rusqlite::Statement<'c>,
    param_order: Vec<ParamId>,
    result_types: Vec<ValueType>,
}

/// The fairness contract as code — run before measuring, so a
/// misconfigured oracle fails the run instead of flattering the engine.
pub struct FairnessCheck;

/// The `SQLite` posting insert, mirroring the corpus loader's shape.
const POSTING_INSERT: &str = "INSERT INTO \"Posting\" VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
