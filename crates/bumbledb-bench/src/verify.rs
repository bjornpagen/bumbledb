//! `verify` — the oracle command and the stamp (docs/architecture/60-validation.md): the
//! command that earns the right to time anything. Every family query and
//! N randomized queries must produce value-identical result multisets on
//! bumbledb and `SQLite`, or the run fails loudly with arbitration
//! bundles.
//!
//! Arbitration procedure (normative): an engine-vs-`SQLite` mismatch on a
//! *family* ⇒ compare the translator's output against the hand-written
//! golden (docs/architecture/60-validation.md). Golden ≠ translator ⇒ translator bug;
//! golden == translator ⇒ a human reads the semantics docs and rules
//! which engine is wrong. Randomized mismatches: minimize by re-running
//! the case's shape at smaller scales (manual; the bundle carries
//! everything needed).

use std::path::PathBuf;

use bumbledb::Db;

use crate::gen::GenConfig;

mod binary_fingerprint;
mod check;
mod display;
mod run;
mod run_algebra;
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

/// The verify run's identity: corpus config, randomized-case count, and
/// the tool-owned scratch directory (delete-and-recreated — never point
/// it at user data).
#[derive(Debug, Clone)]
pub struct VerifyConfig {
    pub gen: GenConfig,
    pub random_cases: u32,
    pub out_dir: PathBuf,
}

/// The default randomized-case count.
pub const DEFAULT_RANDOM_CASES: u32 = 500;

/// A successful verify: how much evidence was collected, and the stamp
/// that now gates timing runs.
#[derive(Debug, Clone)]
pub struct VerifyReport {
    /// Query × param-set executions compared.
    pub cases: u64,
    /// The stamp hex, also written to `out_dir/verify.stamp`.
    pub stamp: String,
}

/// A failed verify: the arbitration bundle directories (up to 8).
#[derive(Debug, Clone)]
pub struct VerifyFailure {
    pub bundles: Vec<PathBuf>,
}

/// One case's identity, for bundles and progress.
struct Case<'a> {
    label: String,
    query: &'a bumbledb::Query,
    sql: &'a str,
    /// The family's hand-written golden, when the case is a family.
    golden_sql: Option<&'static str>,
}

/// Everything a run accumulates. Generic over the store's schema
/// definition: the family lane runs against the ledger store, the
/// randomized lane against the generator-target store.
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

/// The randomized slice of the empty-store pass.
const EMPTY_STORE_RANDOM_CASES: u32 = 100;
