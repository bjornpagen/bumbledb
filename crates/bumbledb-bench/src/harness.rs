//! Ordinary benchmark timing, parameter rotation, and allocation diagnostics.
use bumbledb::Value;

mod cold;
mod measure;
mod rotation;
mod stats;
#[cfg(test)]
mod tests;
mod work;

pub use cold::{measure_cold, org_touch};
pub use measure::{measure, measure_batched, measure_interleaved};
pub use stats::{normalized_p50, stats};
pub use work::bench_work;

/// Warmup and measured sample counts. Each family selects its protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protocol {
    pub warmups: u32,
    pub samples: u32,
}

impl Protocol {
    pub const WARM: Self = Self {
        warmups: 32,
        samples: 256,
    };

    pub const COLD: Self = Self {
        warmups: 2,
        samples: 16,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub min: u64,
    pub p50: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
    pub mean_ns: u64,
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub stats: Stats,
    pub work: u64,

    pub p50_norm: Option<u64>,

    pub alloc: Option<bumbledb::alloc_counter::AllocSnapshot>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Modes {
    pub alloc_window: bool,
    /// Record an effective-GHz proxy reading after EVERY sample:
    pub proxy_per_rep: bool,
}

pub const QUANTUM_FLOOR_NS: u64 = 500;

/// Existing automatic batching ceiling; overrides do not amplify the
/// protocol beyond its current maximum operations per timed sample.
pub const MAX_READ_BATCH: u32 = 16;

#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
pub fn facts_per_sec(m: &Measurement, samples: u32) -> f64 {
    let total_secs = (m.stats.mean_ns * u64::from(samples)) as f64 / 1e9;
    m.work as f64 / total_secs.max(f64::EPSILON)
}

#[derive(Debug, Clone)]
pub struct Rotation<T = Vec<Value>> {
    sets: Vec<T>,
    cursor: usize,
}
