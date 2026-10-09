//! The non-ledger worlds (joins, graph, olap, points, rings, temporal). Every
//! scenario runs under the ledger's protocol (`SQLite` file-backed, WAL,
//! `synchronous=FULL`, fully indexed, `ANALYZE`, prepared statements reused,
//! medians of samples), and every query is checked against the mirror for each
//! parameter set before it is timed.
pub mod graph;
pub mod joins;
pub mod olap;
pub mod points;
pub mod rings;
pub mod temporal;

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
use bumbledb::{Db, Query, RelationId, StatementId, Value};
use rusqlite::Connection;

use crate::harness;

pub use all::all;
pub use geomean::{dnf_count, geomean};
pub use json_out::to_json;
pub use mix::mix;
pub use render::render;
pub(crate) use run::profile;
pub use run::{gate_scenario, run};

pub use crate::harness::sqlite_run::{CapMs, DEFAULT_CAP};

#[derive(Debug, Clone, Copy)]
pub enum Twin {
    Canonical,

    Tuned(fn() -> crate::oracle::sqlite::translate::Translated),

    Hand(fn() -> crate::oracle::sqlite::translate::Translated),
}

pub enum Surface {
    Query(fn() -> Query),

    KeyedGet {
        relation: RelationId,

        key: fn(&Schema) -> StatementId,
    },
}

pub struct ScenarioQuery {
    pub name: &'static str,
    pub surface: Surface,

    pub params: fn(u64) -> Vec<Vec<Value>>,

    pub about: &'static str,

    pub twin: Twin,

    pub cap: Option<CapMs>,
}

pub struct Scenario {
    pub name: &'static str,
    pub about: &'static str,

    pub schema: fn() -> &'static Schema,

    pub descriptor: fn() -> SchemaDescriptor,

    #[expect(
        clippy::type_complexity,
        reason = "the tuple shape directly represents parallel protocol streams"
    )]
    pub rows: fn(u64) -> Vec<(RelationId, Box<dyn Iterator<Item = Vec<Value>>>)>,

    pub extra_indexes: &'static [&'static str],
    pub queries: fn() -> Vec<ScenarioQuery>,
}

pub struct QueryReport {
    pub scenario: &'static str,
    pub name: &'static str,
    pub about: &'static str,

    pub answers: u64,
    pub ours: harness::Stats,

    pub lanes: Vec<LaneReport>,
}

impl QueryReport {
    #[must_use]
    pub fn primary_ratio(&self) -> Option<f64> {
        match self.lanes.first()?.outcome {
            LaneOutcome::Timed { ratio_p50, .. } => Some(ratio_p50),
            LaneOutcome::ExceededCap { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneReport {
    pub lane: &'static str,
    pub outcome: LaneOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LaneOutcome {
    Timed {
        stats: harness::Stats,
        ratio_p50: f64,
    },

    ExceededCap {
        cap: CapMs,
    },
}

struct Stores {
    db: Db<SchemaDescriptor>,
    conn: Connection,
}
