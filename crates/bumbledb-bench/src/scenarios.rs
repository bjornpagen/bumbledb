//! Every scenario runs under the `synchronous=FULL`, fully indexed, prepared
//! statements reused, `ANALYZE`, DISTINCT in the timed SQL, median-of-samples),
//! and every ledger benchmark's exact protocol (`SQLite` file-backed, WAL,
//! query is **oracle-gated before it is timed**: each query × param set

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
mod trace;

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
pub use run::{gate_scenario, run};

pub use crate::sqlite_run::{CapMs, DEFAULT_CAP};

#[derive(Debug, Clone, Copy)]
pub enum Twin {

    Canonical,

    Tuned(fn() -> crate::translate::Translated),

    Hand(fn() -> crate::translate::Translated),
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

#[derive(Debug, Clone, Default)]
pub struct QueryModes {

    pub trace_root: Option<std::path::PathBuf>,

    pub alloc: bool,
}

pub struct QueryReport {
    pub scenario: &'static str,
    pub name: &'static str,
    pub about: &'static str,

    pub answers: u64,
    pub ours: harness::Stats,

    pub lanes: Vec<LaneReport>,

    pub flame: Option<String>,

    pub alloc: Option<crate::report::AllocReport>,
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

    ExceededCap { cap: CapMs },
}

struct Stores {
    db: Db<SchemaDescriptor>,
    conn: Connection,
}
