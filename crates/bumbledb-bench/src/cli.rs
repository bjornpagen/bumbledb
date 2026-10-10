use std::path::PathBuf;

use crate::worlds::corpus_gen::Scale;

mod help;
mod parse;
#[cfg(test)]
mod tests;

pub use help::help;
pub use parse::{parse, parse_invocation};

/// Options that precede the command and apply to the whole run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Globals {
    /// Claim the scheduler boost before a measurement command.
    pub boost: bool,
    /// The concurrent worker count a runner stamps into provenance.
    pub jobs: Option<std::num::NonZeroUsize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusArgs {
    pub scale: Scale,
    pub seed: u64,

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchArgs {
    pub corpus: CorpusArgs,

    pub families: Option<Vec<String>>,
    /// Measured-sample override for the read protocol.
    pub samples: Option<u32>,
    /// Fixed operations per timed read sample; None retains automatic batching.
    pub read_batch: Option<std::num::NonZeroU32>,
    pub out: Option<PathBuf>,

    pub i_am_lying: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileArgs {
    pub corpus: CorpusArgs,
    pub family: String,
    pub seconds: u32,
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Help,

    Queries,

    Gen(CorpusArgs),

    Verify {
        corpus: CorpusArgs,
        cases: u32,
    },

    VerifyStore(CorpusArgs),

    Bench(BenchArgs),

    Profile(ProfileArgs),

    Scenarios(ScenarioArgs),

    Crud(ScenarioArgs),

    Lawful(ScenarioArgs),

    Storage(StorageArgs),

    Writes(WritesArgs),

    Curves(CurvesArgs),

    /// The heap-arm ladder: frozen-vs-LMDB point reads and admission prefixes.
    Heap(HeapArgs),

    /// Kernels at every SIMD level against their scalar twins, and the
    /// `float_stats` families.
    Micro(crate::harness::micro::MicroArgs),

    /// Two `micro` JSON reports side by side, as Markdown.
    MicroCompare {
        old: PathBuf,
        new: PathBuf,
    },

    /// The app-perf regime lane over the ledger corpus (report-class).
    AppPerf(AppPerfArgs),
}

impl Cmd {
    #[must_use]
    pub fn runs_measurements(&self) -> bool {
        match self {
            Self::Bench(_)
            | Self::Profile(_)
            | Self::Scenarios(_)
            | Self::Crud(_)
            | Self::Lawful(_)
            | Self::Storage(_)
            | Self::Writes(_)
            | Self::Curves(_)
            | Self::Heap(_)
            | Self::Micro(_)
            | Self::AppPerf(_) => true,
            Self::Help
            | Self::Queries
            | Self::Gen(_)
            | Self::Verify { .. }
            | Self::VerifyStore(_)
            | Self::MicroCompare { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioArgs {
    pub seed: u64,
    pub dir: PathBuf,

    pub only: Option<Vec<String>>,

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageArgs {
    pub scales: Vec<Scale>,
    pub seed: u64,
    pub dir: PathBuf,

    pub out: Option<PathBuf>,
}

impl Default for StorageArgs {
    fn default() -> Self {
        Self {
            scales: vec![Scale::S],
            seed: 1,
            dir: PathBuf::from("bench-data"),
            out: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritesArgs {
    pub scale: Scale,
    pub seed: u64,
    pub dir: PathBuf,

    pub batches: Vec<u32>,

    pub samples: Option<u32>,
    pub out: Option<PathBuf>,
}

impl Default for WritesArgs {
    fn default() -> Self {
        Self {
            scale: Scale::S,
            seed: 1,
            dir: PathBuf::from("bench-data"),
            batches: vec![1, 10, 100, 1000],
            samples: None,
            out: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurvesArgs {
    pub scales: Vec<Scale>,

    pub families: Option<Vec<String>>,
    pub seed: u64,
    pub dir: PathBuf,

    pub samples: Option<u32>,

    pub cap_ms: u64,

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPerfArgs {
    pub scale: Scale,
    pub seed: u64,

    /// Regime filter; default all.
    pub regimes: Option<Vec<crate::harness::appperf::Regime>>,

    pub samples: Option<u32>,

    /// Tenant count for the churn regime.
    pub tenants: u32,
    pub out: Option<PathBuf>,
}

impl Default for AppPerfArgs {
    fn default() -> Self {
        Self {
            scale: Scale::S,
            seed: 1,
            regimes: None,
            samples: None,
            tenants: 8,
            out: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeapArgs {
    pub scale: Scale,
    pub seed: u64,
    pub dir: PathBuf,
    pub samples: Option<u32>,

    pub prefixes: Vec<u64>,
    pub out: Option<PathBuf>,
}

impl Default for HeapArgs {
    fn default() -> Self {
        Self {
            scale: Scale::S,
            seed: 1,
            dir: PathBuf::from("bench-data"),
            samples: None,
            prefixes: vec![256, 1_024, 4_096, 16_384],
            out: None,
        }
    }
}
