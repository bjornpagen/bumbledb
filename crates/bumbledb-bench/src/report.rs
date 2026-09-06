//! The thing a human reads before making (or refusing)
use crate::harness::Stats;

#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    pub crate_version: String,
    pub git_rev: String,

    pub timestamp: String,
    pub host: String,

    pub shared: Option<SharedMachine>,
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

    pub store: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Win,
    Loss,
    ReportOnly,
}

pub const P99_BUDGET_NS: u64 = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocReport {
    pub allocs: u64,
    pub deallocs: u64,
    pub alloc_bytes: u64,
    pub dealloc_bytes: u64,
}

impl From<bumbledb::alloc_counter::AllocSnapshot> for AllocReport {
    fn from(s: bumbledb::alloc_counter::AllocSnapshot) -> Self {
        Self {
            allocs: s.window.allocs,
            deallocs: s.window.deallocs,
            alloc_bytes: s.window.alloc_bytes,
            dealloc_bytes: s.window.dealloc_bytes,
        }
    }
}

impl From<crate::clockproxy::GhzStamp> for GhzReport {
    fn from(stamp: crate::clockproxy::GhzStamp) -> Self {
        Self {
            pre: stamp.pre,
            post: stamp.post,
            retried: stamp.retried,
            contaminated: stamp.contaminated(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhzReport {
    pub pre: f64,
    pub post: f64,
    pub retried: bool,
    pub contaminated: bool,
}

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
    pub alloc: Option<AllocReport>,
    pub p99_within_budget: bool,
    /// Legacy merged stamp; retain it unchanged for existing consumers.
    pub ghz: Option<GhzReport>,
    /// Recorded engine brackets. None is unknown, never inferred from ghz.
    pub ghz_ours: Option<GhzReport>,
    pub ghz_theirs: Option<GhzReport>,

    /// ran: samples rescaled to the cohort's best clock before the
    pub p50_norm: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WriteFamilyReport {
    pub name: String,
    pub ours: Stats,
    pub theirs: Option<Stats>,
    pub facts_per_sec: Option<f64>,
    /// Legacy outer bracket, including its original contamination flag.
    pub ghz: Option<GhzReport>,
    /// Whole engine blocks, including setup and teardown outside samples.
    /// None is unknown or absent, never inferred from the outer bracket.
    pub ghz_ours: Option<GhzReport>,
    pub ghz_theirs: Option<GhzReport>,
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
mod ghz;
mod json_out;
mod markdown;
mod merge;
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
pub use merge::merge_markdown;
pub use provenance::{git_rev, host_description, provenance, timestamp_iso8601};
pub use verdict::verdict;
pub use write_artifacts::write_artifacts;

#[cfg(test)]
use crate::families::{self, Kind};
#[cfg(test)]
use crate::json;
#[cfg(test)]
use provenance::civil;
