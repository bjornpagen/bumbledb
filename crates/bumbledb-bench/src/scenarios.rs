//! The scenario suites (docs/architecture/60-validation.md, extended):
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
//!
//! The `SQLite` side of a query is data ([`Twin`]): the canonical
//! translation, an optional hand-tuned twin lane, or — only where the
//! translator refuses — a hand-written best shot; every lane is gated.
//! Adversarial lanes carry a per-sample wall-clock cap ([`CapMs`]): a
//! tripped lane reports [`LaneOutcome::ExceededCap`] with no
//! percentiles, excluded from geomeans and counted.

pub mod graph;
pub mod joins;
pub mod olap;
pub mod points;

mod all;
mod geomean;
pub(crate) mod json_out;
mod load;
mod mix;
mod render;
mod run;
mod run_query;

#[cfg(test)]
mod tests;

use bumbledb::schema::{Schema, SchemaDescriptor};
use bumbledb::{Db, Query, RelationId, Value};
use rusqlite::Connection;

use crate::harness;

pub use all::all;
pub use geomean::{dnf_count, geomean};
pub use json_out::to_json;
pub use mix::mix;
pub use render::render;
pub use run::{gate_scenario, run};

pub use crate::sqlite_run::{CapMs, DEFAULT_CAP};

/// The `SQLite` twin lane(s) of one scenario query.
#[derive(Debug, Clone, Copy)]
pub enum Twin {
    /// The canonical translation is the one lane (lane name "sqlite").
    Canonical,
    /// Canonical PLUS a hand-tuned rendering (lane "sqlite-tuned") — both
    /// gated, both timed, both reported (the never-flatter-ourselves law:
    /// where the canonical rendering inflates SQL — Allen basics OR-chains —
    /// `SQLite` also gets its best shot).
    Tuned(fn() -> crate::translate::Translated),
    /// The translator refuses the query (`Pack`): the lane ("sqlite-hand") is
    /// a hand-written best-shot SQL, gated identically — the `free_busy`
    /// precedent (calendar/families.rs). Legal ONLY where translate errs
    /// (asserted by test).
    Hand(fn() -> crate::translate::Translated),
}

/// One scenario query: IR + seeded param sets + a one-line regime note.
pub struct ScenarioQuery {
    pub name: &'static str,
    pub query: fn() -> Query,
    /// Seeded param sets; rotation order is the measurement order.
    pub params: fn(u64) -> Vec<Vec<Value>>,
    /// What regime this query stresses (rendered in the report).
    pub about: &'static str,
    /// Which `SQLite` renderings exist — data on the query, never a
    /// silently skipped (or silently invented) lane.
    pub twin: Twin,
    /// The per-sample DNF cap for adversarial lanes; `None` = uncapped,
    /// the progress handler is never installed (existing lanes are
    /// untouched by construction).
    pub cap: Option<CapMs>,
}

/// One scenario: a schema, a deterministic corpus, extra `SQLite`
/// indexes for its predicate columns (key/containment indexes come from the
/// schema statements via [`sqlmap::expected_indexes`]), and a query
/// list.
pub struct Scenario {
    pub name: &'static str,
    pub about: &'static str,
    /// The validated schema, for the inspection surfaces (DDL, typing).
    pub schema: fn() -> &'static Schema,
    /// The declared schema, for store creation — the scenario table is
    /// data, so its stores share the dynamic `Db<SchemaDescriptor>`
    /// state (loads and queries are all dynamic-surface).
    pub descriptor: fn() -> SchemaDescriptor,
    /// Relations in containment order with their row iterators.
    #[expect(
        clippy::type_complexity,
        reason = "the tuple shape directly represents parallel protocol streams"
    )]
    pub rows: fn(u64) -> Vec<(RelationId, Box<dyn Iterator<Item = Vec<Value>>>)>,
    /// `CREATE INDEX` statements for predicate columns the statement
    /// registry does not already cover.
    pub extra_indexes: &'static [&'static str],
    pub queries: fn() -> Vec<ScenarioQuery>,
}

/// One measured query entry of the scenario report.
pub struct QueryReport {
    pub scenario: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// Median answers across the rotation (the work sanity check).
    pub answers: u64,
    pub ours: harness::Stats,
    /// The `SQLite` lane(s), one entry per [`Twin`] rendering.
    pub lanes: Vec<LaneReport>,
}

impl QueryReport {
    /// First lane's `Timed` ratio — the geomean's input. `None` when the
    /// primary lane exceeded its cap (a DNF contributes no ratio).
    #[must_use]
    pub fn primary_ratio(&self) -> Option<f64> {
        match self.lanes.first()?.outcome {
            LaneOutcome::Timed { ratio_p50, .. } => Some(ratio_p50),
            LaneOutcome::ExceededCap { .. } => None,
        }
    }
}

/// One `SQLite` lane's result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneReport {
    pub lane: &'static str,
    pub outcome: LaneOutcome,
}

/// What one `SQLite` lane produced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LaneOutcome {
    Timed {
        stats: harness::Stats,
        ratio_p50: f64,
    },
    /// The honest DNF: a sample tripped the per-sample wall-clock cap —
    /// the lane carries NO percentiles (a censored p50 is not a p50);
    /// excluded from geomeans and counted by the renderers.
    ExceededCap { cap: CapMs },
}

/// A loaded scenario store pair.
struct Stores {
    db: Db<SchemaDescriptor>,
    conn: Connection,
}
