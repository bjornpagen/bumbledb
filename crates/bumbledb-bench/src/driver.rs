//! Command orchestration (docs/architecture/60-validation.md): the digest-keyed corpus
//! cache, verify-before-time enforcement, and the bench run that turns
//! measurements into report artifacts. Every failure message names the
//! next action.

use std::path::PathBuf;

use bumbledb::Db;
use rusqlite::Connection;

use crate::gen::GenConfig;
use crate::harness::Protocol;
use crate::report;

mod bench;
mod corpus;
mod gen;
mod merge;
mod read_family;
mod scenarios;
#[cfg(test)]
mod tests;
mod trace;
mod write_families;

pub use bench::cmd_bench;
pub use corpus::{corpus_paths, ensure_corpus, ensure_corpus_with};
pub use gen::{cmd_gen, cmd_verify};
pub use merge::cmd_merge;
pub use scenarios::cmd_scenarios;
pub use trace::cmd_trace;

/// The digest-keyed corpus locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusPaths {
    /// `<dir>/<digest-prefix>/` — everything lives inside.
    pub root: PathBuf,
    pub db: PathBuf,
    pub oracle: PathBuf,
    pub stamp: PathBuf,
}

/// The sidecar recording which case count the stamp was earned with —
/// bench reconstructs the full `VerifyConfig` from it.
const CASES_FILE: &str = "verify.cases";

/// The per-run context the bench families share.
#[allow(clippy::struct_excessive_bools)] // mirrors BenchArgs' flag surface.
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
    db: &'a Db<'a>,
    conn: &'a Connection,
    flames: Vec<report::FlameEmbed>,
}
