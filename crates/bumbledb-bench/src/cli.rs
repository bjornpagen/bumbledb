use std::path::PathBuf;

use crate::churn::ops::{
    DEFAULT_ANALYZE_EVERY, DEFAULT_CYCLES, DEFAULT_SAMPLE_EVERY, DEFAULT_VACUUM_EVERY,
};
use crate::corpus_gen::Scale;
use crate::duralane::DurabilityLane;

mod help;
mod parse;
#[cfg(test)]
mod tests;

pub use help::help;
pub use parse::parse;

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

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchArgs {
    pub corpus: CorpusArgs,

    pub families: Option<Vec<String>>,
    /// Measured-sample override for the read protocol.
    pub samples: Option<u32>,
    pub trace: bool,
    pub alloc: bool,

    pub ephemeral: bool,

    pub proxy_per_rep: bool,
    pub out: Option<PathBuf>,

    pub i_am_lying: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {

    Help,

    Queries,

    Gen(CorpusArgs),

    Verify { corpus: CorpusArgs, cases: u32 },

    VerifyStore(CorpusArgs),

    Bench(BenchArgs),

    Trace { corpus: CorpusArgs, family: String },

    Scenarios(ScenarioArgs),

    Crud(ScenarioArgs),

    Lawful(ScenarioArgs),

    SweepCommit(SweepArgs),

    Merge { dirs: Vec<PathBuf> },

    Storage(StorageArgs),

    Writes(WritesArgs),

    Curves(CurvesArgs),

    Churn(ChurnArgs),
    /// The heap-arm ladder: frozen-vs-LMDB point reads and admission

    Heap(HeapArgs),

    Primerlane(PrimerlaneArgs),
}

impl Cmd {

    #[must_use]
    pub fn runs_measurements(&self) -> bool {
        match self {
            Self::Bench(_)
            | Self::Trace { .. }
            | Self::Scenarios(_)
            | Self::Crud(_)
            | Self::Lawful(_)
            | Self::SweepCommit(_)
            | Self::Storage(_)
            | Self::Writes(_)
            | Self::Curves(_)
            | Self::Churn(_)
            | Self::Heap(_)
            | Self::Primerlane(_) => true,
            Self::Help
            | Self::Queries
            | Self::Gen(_)
            | Self::Verify { .. }
            | Self::VerifyStore(_)
            | Self::Merge { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepArgs {

    pub sizes: Option<Vec<u64>>,

    pub samples: Option<u32>,
    pub seed: u64,

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioArgs {
    pub seed: u64,
    pub dir: PathBuf,

    pub only: Option<Vec<String>>,

    pub samples: Option<u32>,

    pub trace: bool,

    pub alloc: bool,
    pub out: Option<PathBuf>,
}

impl Default for ScenarioArgs {
    fn default() -> Self {
        Self {
            seed: 1,
            dir: PathBuf::from("bench-data"),
            only: None,
            samples: None,
            trace: false,
            alloc: false,
            out: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageArgs {

    pub scales: Vec<Scale>,
    pub seed: u64,
    pub dir: PathBuf,

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritesArgs {
    pub scale: Scale,
    pub seed: u64,
    pub dir: PathBuf,

    /// durable lane's fsync shadow must land after every nosync sample

    pub lanes: Vec<DurabilityLane>,

    pub batches: Vec<u32>,

    pub samples: Option<u32>,

    pub trace: bool,
    pub out: Option<PathBuf>,
}

impl Default for WritesArgs {
    fn default() -> Self {
        Self {
            scale: Scale::S,
            seed: 1,
            dir: PathBuf::from("bench-data"),
            lanes: vec![DurabilityLane::Nosync, DurabilityLane::Durable],
            batches: vec![1, 10, 100, 1000],
            samples: None,
            trace: false,
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
pub struct ChurnArgs {
    pub corpus: CorpusArgs,

    pub cycles: u64,

    pub sample_every: u64,

    pub vacuum_every: u64,

    pub analyze_every: u64,

    pub runs: Option<Vec<String>>,
    pub out: Option<PathBuf>,
}

impl Default for ChurnArgs {
    fn default() -> Self {
        Self {
            corpus: CorpusArgs::default(),
            cycles: DEFAULT_CYCLES,
            sample_every: DEFAULT_SAMPLE_EVERY,
            vacuum_every: DEFAULT_VACUUM_EVERY,
            analyze_every: DEFAULT_ANALYZE_EVERY,
            runs: None,
            out: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimerlaneArgs {

    pub facts: u64,

    pub relations: u32,
    pub seed: u64,

    pub dir: PathBuf,

    pub trace: bool,

    pub alloc: bool,
    pub out: Option<PathBuf>,
}

impl Default for PrimerlaneArgs {
    fn default() -> Self {
        Self {
            facts: 200_000,
            relations: 12,
            seed: 1,
            dir: PathBuf::from("bench-data"),
            trace: false,
            alloc: false,
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
