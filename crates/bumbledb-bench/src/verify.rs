use std::path::PathBuf;

use bumbledb::Db;

use crate::corpus_gen::GenConfig;

mod binary_fingerprint;
mod check;
mod display;
mod run;
mod run_algebra;
mod run_calendar;
mod run_converse;
mod run_empty_store;
mod run_naive;
mod stamp_matches;
mod stamp_value;
#[cfg(test)]
mod tests;

pub use binary_fingerprint::binary_fingerprint;
pub use run::{run, run_prepared, run_with_sql_override};
pub use stamp_matches::stamp_matches;
pub use stamp_value::stamp_value;

#[derive(Debug, Clone)]
pub struct VerifyConfig {
    pub corpus_gen: GenConfig,
    pub random_cases: u32,
    pub out_dir: PathBuf,
}

pub const DEFAULT_RANDOM_CASES: u32 = 500;

#[derive(Debug, Clone)]
pub struct VerifyReport {

    pub cases: u64,

    pub stamp: String,
}

#[derive(Debug, Clone)]
pub struct VerifyFailure {
    pub bundles: Vec<PathBuf>,
}

struct Case<'a> {
    label: String,
    query: &'a bumbledb::Query,
    sql: &'a str,

    golden_sql: Option<&'static str>,
}

struct Run<'a, S> {
    db: &'a Db<S>,
    conn: &'a rusqlite::Connection,
    out_dir: PathBuf,
    cases: u64,
    total: u64,
    bundles: Vec<PathBuf>,
}

/// How many mismatch bundles a run collects before giving up.
const MAX_BUNDLES: usize = 8;

const EMPTY_STORE_RANDOM_CASES: u32 = 100;
