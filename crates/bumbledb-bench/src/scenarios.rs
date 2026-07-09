//! The scenario suites (docs/architecture/50-validation.md, extended):
//! additional schema+corpus+query worlds beyond the ledger, each
//! stressing a different regime — join-order pressure, graph fan-out,
//! OLAP rollups, point-lookup overhead. Every scenario runs under the
//! ledger benchmark's exact protocol (`SQLite` file-backed, WAL,
//! `synchronous=FULL`, fully indexed, prepared statements reused,
//! `ANALYZE`, DISTINCT in the timed SQL, median-of-samples), and every
//! query is **oracle-gated before it is timed**: each query × param set
//! must produce value-identical multisets on both engines or the run
//! fails loudly — no timing without agreement.
//!
//! Scenarios are `Kind::Report`-class by design: they exist to *measure*
//! regimes, not to gate the suite. The ledger's ten families remain the
//! gate.

pub mod graph;
pub mod joins;
pub mod olap;
pub mod points;

mod all;
mod geomean;
mod load;
mod mix;
mod render;
mod run;
mod run_query;

#[cfg(test)]
mod tests;

use bumbledb::schema::Schema;
use bumbledb::{Db, Query, RelationId, Value};
use rusqlite::Connection;

use crate::harness;

pub use all::all;
pub use geomean::geomean;
pub use mix::mix;
pub use render::render;
pub use run::run;

/// One scenario query: IR + seeded param sets + a one-line regime note.
pub struct ScenarioQuery {
    pub name: &'static str,
    pub query: fn() -> Query,
    /// Seeded param sets; rotation order is the measurement order.
    pub params: fn(u64) -> Vec<Vec<Value>>,
    /// What regime this query stresses (rendered in the report).
    pub about: &'static str,
}

/// One scenario: a schema, a deterministic corpus, extra `SQLite`
/// indexes for its predicate columns (key/containment indexes come from the
/// schema statements via [`sqlmap::expected_indexes`]), and a query
/// list.
pub struct Scenario {
    pub name: &'static str,
    pub about: &'static str,
    pub schema: fn() -> &'static Schema,
    /// Relations in containment order with their row iterators.
    #[allow(clippy::type_complexity)]
    pub rows: fn(u64) -> Vec<(RelationId, Box<dyn Iterator<Item = Vec<Value>>>)>,
    /// `CREATE INDEX` statements for predicate columns the statement
    /// registry does not already cover.
    pub extra_indexes: &'static [&'static str],
    pub queries: fn() -> Vec<ScenarioQuery>,
}

/// One measured query row of the scenario report.
pub struct QueryReport {
    pub scenario: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// Median result rows across the rotation (the work sanity check).
    pub rows: u64,
    pub ours: harness::Stats,
    pub theirs: harness::Stats,
    pub ratio_p50: f64,
}

/// A loaded scenario store pair.
struct Stores {
    db: Db<'static>,
    conn: Connection,
}
