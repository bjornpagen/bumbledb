//! Hand-rolled command-line parsing (no clap — the quarantine allows
//! rusqlite only). One flat token walk per subcommand; every error names
//! the offending token.

use std::path::PathBuf;

use crate::corpus_gen::Scale;
use crate::lanes::writes::DurabilityLane;

mod help;
mod parse;
#[cfg(test)]
mod tests;

pub use help::help;
pub use parse::parse;

/// The corpus identity + location every store-touching command shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusArgs {
    pub scale: Scale,
    pub seed: u64,
    /// The digest-keyed cache root (`<dir>/<digest-prefix>/…`).
    pub dir: PathBuf,
}

impl Default for CorpusArgs {
    fn default() -> Self {
        Self {
            scale: Scale::S,
            seed: 1,
            dir: PathBuf::from("bench-data"),
        }
    }
}

/// `bench`'s knobs.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)]
// a 1:1 mirror of independent CLI
// flags; folding them into state enums would misrepresent the surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchArgs {
    pub corpus: CorpusArgs,
    /// Selected family names; `None` = the full suite.
    pub families: Option<Vec<String>>,
    /// Measured-sample override for the read protocol.
    pub samples: Option<u32>,
    pub trace: bool,
    pub alloc: bool,
    /// Time against `Db::ephemeral` stores (the in-memory
    /// characterization lane) instead of the durable constructors.
    pub ephemeral: bool,
    /// Per-rep proxy stamps + normalized p50 — the
    /// confirm-run mode for suspicious findings.
    pub proxy_per_rep: bool,
    pub out: Option<PathBuf>,
    /// Skip the verify-stamp gate; the report is branded UNVERIFIED.
    pub i_am_lying: bool,
}

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// Print usage and exit 0.
    Help,
    /// Print the versioned query list to stdout.
    Queries,
    /// Generate + load both stores into the digest-keyed directory.
    Gen(CorpusArgs),
    /// The oracle: compare both engines, stamp on success.
    Verify { corpus: CorpusArgs, cases: u32 },
    /// The offline sweeper: `Db::verify_store` over the corpus store.
    VerifyStore(CorpusArgs),
    /// The timing run (refuses without a fresh stamp).
    Bench(BenchArgs),
    /// One traced warm+cold pair for one family.
    Trace { corpus: CorpusArgs, family: String },
    /// The scenario suites: non-ledger worlds, oracle-gated then timed.
    Scenarios(ScenarioArgs),
    /// The crud home-turf world (report-class): OLTP round-trips under
    /// matched durability pairs — `SQLite`'s strong regime, benched to
    /// lose honestly.
    Crud(ScenarioArgs),
    /// The lawful home-turf world (report-class): judged-law admission
    /// vs SQL constraint enforcement — `SQLite`'s strong regime,
    /// benched to lose honestly.
    Lawful(ScenarioArgs),
    /// The T8 commit-size sweep: judgment spans by touched-parent count
    /// over ephemeral windowed twins, delta-order vs key-sorted probes.
    SweepCommit(SweepArgs),
    /// Merge N run directories' `report.json` into a min-of-runs table.
    Merge { dirs: Vec<PathBuf> },
    /// The storage metric lane: on-disk bytes per corpus scale, both
    /// engines (report-class; no timing).
    Storage(StorageArgs),
    /// The writes metric lane: write/commit/delete throughput ladder
    /// across durability lanes (report-class).
    Writes(WritesArgs),
    /// The curves metric lane: scale-curve runner + the
    /// cold/warm/memoized panel (report-class).
    Curves(CurvesArgs),
}

/// `sweep-commit`'s knobs. No scale flag: the sweep owns its ambient
/// mass (a fixed tree; the swept parameter is the commit size).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepArgs {
    /// Touched-parent counts; `None` = the default ladder
    /// ([`crate::sweep::DEFAULT_SIZES`]).
    pub sizes: Option<Vec<u64>>,
    /// Sample commits per (size, order) cell; `None` = the lane default.
    pub samples: Option<u32>,
    pub seed: u64,
    /// Scratch root for the ephemeral twin stores.
    pub dir: PathBuf,
}

impl Default for SweepArgs {
    fn default() -> Self {
        Self {
            sizes: None,
            samples: None,
            seed: 1,
            dir: PathBuf::from("bench-data"),
        }
    }
}

/// `scenarios`' knobs. Scenarios own their sizes (no scale flag): the
/// corpus identity is (scenario, seed). The `crud` and `lawful` worlds
/// share this shape — one flag vocabulary across the world commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioArgs {
    pub seed: u64,
    pub dir: PathBuf,
    /// Selected scenario names; `None` = the full registry.
    pub only: Option<Vec<String>>,
    /// Measured samples per query per engine.
    pub samples: Option<u32>,
    pub out: Option<PathBuf>,
}

impl Default for ScenarioArgs {
    fn default() -> Self {
        Self {
            seed: 1,
            dir: PathBuf::from("bench-data"),
            only: None,
            samples: None,
            out: None,
        }
    }
}

/// `storage`'s knobs ([`crate::lanes::storage`]). `Scale::Tiny` stays a
/// test-injection point through this struct, never a CLI token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageArgs {
    /// Corpus scales, in run order.
    pub scales: Vec<Scale>,
    pub seed: u64,
    pub dir: PathBuf,
    /// Scratch root for the churn ladder; `None` = skip churn.
    pub churn_dir: Option<PathBuf>,
    pub out: Option<PathBuf>,
}

impl Default for StorageArgs {
    fn default() -> Self {
        Self {
            scales: vec![Scale::S],
            seed: 1,
            dir: PathBuf::from("bench-data"),
            churn_dir: None,
            out: None,
        }
    }
}

/// `writes`' knobs ([`crate::lanes::writes`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritesArgs {
    pub scale: Scale,
    pub seed: u64,
    pub dir: PathBuf,
    /// Durability lanes, in run order. `NoSync` first by default: the
    /// durable lane's fsync shadow must land after every nosync sample
    /// (the write-order pin, `driver/write_families.rs`).
    pub lanes: Vec<DurabilityLane>,
    /// Rows-per-commit ladder; zero is rejected at parse time.
    pub batches: Vec<u32>,
    /// Measured samples per cell; `None` = the lane default.
    pub samples: Option<u32>,
    pub out: Option<PathBuf>,
}

impl Default for WritesArgs {
    fn default() -> Self {
        Self {
            scale: Scale::S,
            seed: 1,
            dir: PathBuf::from("bench-data"),
            lanes: vec![DurabilityLane::NoSync, DurabilityLane::Durable],
            batches: vec![1, 10, 100, 1000],
            samples: None,
            out: None,
        }
    }
}

/// `curves`' knobs ([`crate::lanes::curves`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurvesArgs {
    /// Corpus scales, in run order.
    pub scales: Vec<Scale>,
    /// Selected family names; `None` = the full lane roster.
    pub families: Option<Vec<String>>,
    pub seed: u64,
    pub dir: PathBuf,
    /// Measured samples per point; `None` = the lane default.
    pub samples: Option<u32>,
    /// The DNF cap: per-sample `SQLite` wall-clock bound, milliseconds.
    pub cap_ms: u64,
    /// Add the cold/warm/memoized panel.
    pub warmth: bool,
    pub out: Option<PathBuf>,
}

impl Default for CurvesArgs {
    fn default() -> Self {
        Self {
            scales: vec![Scale::S],
            families: None,
            seed: 1,
            dir: PathBuf::from("bench-data"),
            samples: None,
            cap_ms: 30_000,
            warmth: false,
            out: None,
        }
    }
}
