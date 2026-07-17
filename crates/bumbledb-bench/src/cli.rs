//! Hand-rolled command-line parsing (no clap — the quarantine allows
//! rusqlite only). One flat token walk per subcommand; every error names
//! the offending token.

use std::path::PathBuf;

use crate::corpus_gen::Scale;

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
    /// The T8 commit-size sweep: judgment spans by touched-parent count
    /// over ephemeral windowed twins, delta-order vs key-sorted probes.
    SweepCommit(SweepArgs),
    /// Merge N run directories' `report.json` into a min-of-runs table.
    Merge { dirs: Vec<PathBuf> },
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
/// corpus identity is (scenario, seed).
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
