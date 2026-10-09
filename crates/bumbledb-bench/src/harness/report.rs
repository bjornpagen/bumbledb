//! The thing a human reads before making (or refusing)
use crate::harness::Stats;

#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    pub crate_version: String,
    pub git_rev: String,
    /// The compiling `rustc -vV`: release line, host and LLVM version.
    pub toolchain: &'static str,

    pub timestamp: String,
    pub host: String,

    pub shared: Option<SharedMachine>,
    /// Maximum independent lane workers, not query-engine threads.
    pub parallel_jobs: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedMachine {
    pub boost: &'static str,
    pub load_start: [f64; 3],
    pub load_end: [f64; 3],
}

impl SharedMachine {
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "boost {} — load 1/5/15 {:.2} {:.2} {:.2} (start) → {:.2} {:.2} {:.2} (end)",
            self.boost,
            self.load_start[0],
            self.load_start[1],
            self.load_start[2],
            self.load_end[0],
            self.load_end[1],
            self.load_end[2],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Win,
    Loss,
    ReportOnly,
}

pub const P99_BUDGET_NS: u64 = 10_000_000;

#[derive(Debug, Clone, PartialEq)]
pub struct ReadFamilyReport {
    pub name: String,
    /// Operations per timed sample, shared by both engines. Percentiles for
    /// batches above one describe per-operation batch averages, not call tails.
    pub batch: u32,
    pub ours: Stats,
    pub theirs: Stats,
    pub ratio_p50: f64,
    pub verdict: Verdict,
    pub p99_within_budget: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WriteFamilyReport {
    pub name: String,
    pub ours: Stats,
    pub theirs: Option<Stats>,
    pub facts_per_sec: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreNumbers {
    pub db_bytes: u64,
    pub sqlite_bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunReport {
    pub provenance: Provenance,
    pub config: RunConfig,
    pub corpus_digest: String,
    pub verify_stamp: String,

    pub budget_gates: bool,

    pub partial: bool,
    pub reads: Vec<ReadFamilyReport>,
    pub writes: Vec<WriteFamilyReport>,
    pub store: StoreNumbers,
}

mod budget;
mod json_out;
mod markdown;
mod provenance;
mod run_report;
#[cfg(test)]
mod tests;
mod verdict;
mod write_artifacts;

pub use budget::within_budget;
pub(crate) use json_out::push_provenance;
pub use json_out::to_json;
pub use markdown::to_markdown;
pub use provenance::{git_rev, host_description, provenance, timestamp_iso8601};
pub use verdict::verdict;
pub use write_artifacts::write_artifacts;

#[cfg(test)]
use crate::worlds::families::{self, Kind};
#[cfg(test)]
use provenance::civil;
