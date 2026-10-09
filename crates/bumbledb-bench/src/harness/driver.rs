use std::path::PathBuf;

use bumbledb::Db;
use rusqlite::Connection;

use crate::harness::Protocol;
use crate::worlds::corpus_gen::GenConfig;
use crate::worlds::ledger::Ledger;

mod bench;
mod corpus;
mod corpus_gen;
mod crud;
mod lawful;
pub(crate) mod profile;
mod read_family;
mod scenarios;
#[cfg(test)]
mod tests;
mod verify_store;

pub(crate) mod write_families;

pub use bench::cmd_bench;
pub use corpus::{corpus_paths, ensure_corpus, ensure_corpus_with};
pub use corpus_gen::{cmd_gen, cmd_verify};
pub use crud::cmd_crud;
pub use lawful::cmd_lawful;
pub use profile::cmd_profile;
pub use scenarios::cmd_scenarios;
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

struct BenchRun<'a> {
    cfg: GenConfig,
    proto: Protocol,
    read_batch: Option<std::num::NonZeroU32>,

    first_family_warmed: bool,
    db: &'a Db<Ledger>,
    conn: &'a Connection,
    cal_db: &'a Db<crate::worlds::calendar::Scheduling>,
    cal_conn: &'a Connection,
}
