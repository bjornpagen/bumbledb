//! Command orchestration (docs/architecture/60-validation.md): the digest-keyed corpus
//! cache, verify-before-time enforcement, and the bench run that turns
//! measurements into report artifacts. Every failure message names the
//! next action.

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
// pub(crate): the device-honesty lock test drives the write families
// against a live ram disk from `crate::devhonesty::tests`.
pub(crate) mod write_families;

pub use bench::cmd_bench;
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

/// The digest-keyed corpus locations: the ledger store pair, the
/// calendar store pair (the second theory shares the digest directory —
/// one identity, one stamp), and the stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusPaths {
    /// `<dir>/<digest-prefix>/` — everything lives inside.
    pub root: PathBuf,
    pub db: PathBuf,
    pub oracle: PathBuf,
    pub cal_db: PathBuf,
    pub cal_oracle: PathBuf,
    pub stamp: PathBuf,
}

/// The sidecar recording which case count the stamp was earned with —
/// bench reconstructs the full `VerifyConfig` from it.
const CASES_FILE: &str = "verify.cases";

/// The per-run context the bench families share.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)] // mirrors BenchArgs' flag surface.
struct BenchRun<'a> {
    cfg: GenConfig,
    proto: Protocol,
    alloc: bool,
    trace: bool,
    proxy_per_rep: bool,
    /// Whether the process-start warm discipline ran: the FIRST
    /// measured family additionally absorbs the 1.45–1.97 GHz
    /// process-start band with extra discarded iterations.
    first_family_warmed: bool,
    trace_dir: PathBuf,
    db: &'a Db<Ledger>,
    conn: &'a Connection,
    cal_db: &'a Db<crate::calendar::Scheduling>,
    cal_conn: &'a Connection,
    flames: Vec<report::FlameEmbed>,
}
