use std::path::PathBuf;

use bumbledb::Db;
use rusqlite::Connection;

use crate::corpus_gen::GenConfig;
use crate::harness::Protocol;
use crate::report;
use crate::schema::Ledger;

mod bench;
mod churn_cmd;
mod corpus;
mod corpus_gen;
mod crud;
mod lawful;
mod merge;
mod read_family;
mod scenarios;
mod sweep_commit;
#[cfg(test)]
mod tests;
mod trace;
mod verify_store;

pub(crate) mod write_families;

pub use bench::cmd_bench;
pub(crate) use bench::obs_missing;
pub use churn_cmd::cmd_churn;
pub use corpus::{corpus_paths, ensure_corpus, ensure_corpus_with};
pub use corpus_gen::{cmd_gen, cmd_verify};
pub use crud::cmd_crud;
pub use lawful::cmd_lawful;
pub use merge::cmd_merge;
pub use scenarios::cmd_scenarios;
pub use sweep_commit::cmd_sweep_commit;
pub use trace::cmd_trace;
pub use verify_store::cmd_verify_store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusPaths {

    pub root: PathBuf,
    pub db: PathBuf,
    pub oracle: PathBuf,
    pub cal_db: PathBuf,
    pub cal_oracle: PathBuf,
    pub stamp: PathBuf,
}

const CASES_FILE: &str = "verify.cases";

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)] 
struct BenchRun<'a> {
    cfg: GenConfig,
    proto: Protocol,
    alloc: bool,
    trace: bool,
    proxy_per_rep: bool,

    first_family_warmed: bool,
    trace_dir: PathBuf,
    db: &'a Db<Ledger>,
    conn: &'a Connection,
    cal_db: &'a Db<crate::calendar::Scheduling>,
    cal_conn: &'a Connection,
    flames: Vec<report::FlameEmbed>,
}
