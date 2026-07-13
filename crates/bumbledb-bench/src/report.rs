//! The report (docs/architecture/60-validation.md): one run → one self-contained,
//! versionable artifact — comparison tables, gate verdicts, budget
//! checks, allocation and execution statistics, flame summaries, and
//! full provenance. The thing a human reads before making (or refusing)
//! the claim. Renders never write outside `out_dir`; the human copies
//! artifacts into the repo when publishing.

use crate::harness::Stats;

/// Where the numbers came from. The engine git rev is read at *runtime*
/// (`git rev-parse HEAD` from the repo dir, "unknown" outside one) —
/// a build script would freeze the rev at compile time and lie after a
/// rebase; runtime resolution names the tree the binary actually ran in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub crate_version: String,
    pub git_rev: String,
    /// ISO-8601 UTC, hand-formatted.
    pub timestamp: String,
    pub host: String,
}

/// The run's configuration, as printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
}

/// Family gate verdicts. `Win` ⇔ ours p50 strictly < theirs p50 (a tie
/// is a loss — the claim is "faster", not "not slower").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Win,
    Loss,
    ReportOnly,
}

/// The warm p99 budget (`00-product.md`): 10 ms, inclusive.
pub const P99_BUDGET_NS: u64 = 10_000_000;

/// Allocation window numbers, feature-independent plain data (the CLI
/// converts from `AllocSnapshot` when the obs build ran one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocReport {
    pub allocs: u64,
    pub deallocs: u64,
    pub alloc_bytes: u64,
    pub dealloc_bytes: u64,
}

/// The execution digest: the planner-honesty numbers a human scans.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecDigest {
    /// The worst per-node estimate-vs-actual factor.
    pub worst_estimate_factor: f64,
    /// Condensed cover histogram (e.g. `n0:t0x256 n1:t1x255/t2x1`).
    pub covers: String,
    /// Bindings emitted to the shared sink across all rules.
    pub emitted: u64,
    /// Emitted bindings rejected by the sink's active seen-set.
    pub absorbed: u64,
}

/// The clock-proxy bracket around one family's measurement block:
/// effective GHz before and
/// after, whether the block was re-measured once, and whether the final
/// bracket still read contaminated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhzReport {
    pub pre: f64,
    pub post: f64,
    pub retried: bool,
    pub contaminated: bool,
}

/// One read family's comparison row.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadFamilyReport {
    pub name: String,
    pub ours: Stats,
    pub theirs: Stats,
    pub ratio_p50: f64,
    pub verdict: Verdict,
    pub alloc: Option<AllocReport>,
    pub exec: Option<ExecDigest>,
    pub p99_within_budget: bool,
    pub ghz: Option<GhzReport>,
    /// Per-rep-normalized p50, when `--proxy-per-rep`
    /// ran: samples rescaled to the cohort's best clock before the
    /// percentile — the confirm-run column that unmasks contamination
    /// hiding inside a block.
    pub p50_norm: Option<u64>,
}

/// One write/cold family's row (`theirs` absent for cold — no `SQLite`
/// mirror exists).
#[derive(Debug, Clone, PartialEq)]
pub struct WriteFamilyReport {
    pub name: String,
    pub ours: Stats,
    pub theirs: Option<Stats>,
    pub facts_per_sec: Option<f64>,
    pub ghz: Option<GhzReport>,
}

/// Store-level numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreNumbers {
    pub db_bytes: u64,
    pub sqlite_bytes: u64,
    pub cache_images: u64,
    pub cache_bytes: u64,
}

/// One traced family's rendered flame table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameEmbed {
    pub name: String,
    pub table: String,
}

/// The whole run, plain data — everything the renderers print.
#[derive(Debug, Clone, PartialEq)]
pub struct RunReport {
    pub provenance: Provenance,
    pub config: RunConfig,
    pub corpus_digest: String,
    pub verify_stamp: String,
    /// The budget gates at scale L; at S/M it prints as informational.
    pub budget_gates: bool,
    /// A `--families`-filtered run: the overall verdict is PARTIAL —
    /// never ALL-WIN, whatever the filtered families did.
    pub partial: bool,
    pub reads: Vec<ReadFamilyReport>,
    pub writes: Vec<WriteFamilyReport>,
    pub store: StoreNumbers,
    pub flames: Vec<FlameEmbed>,
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
