//! The measurement engine (docs/architecture/60-validation.md): warmup → measured
//! samples → exact percentiles, with optional allocation windows and a
//! precisely defined cold protocol. Everything the report prints comes
//! from here — the harness owns time, never queries (runners pass
//! closures over their own prepared statements).

use bumbledb::obs::TraceEvent;
use bumbledb::Value;

mod cold;
mod measure;
mod rotation;
mod stats;
#[cfg(test)]
mod tests;
mod traced;

pub use cold::{measure_cold, org_touch};
pub use measure::{measure, measure_batched};
pub use stats::{normalized_p50, stats};
pub use traced::{traced_cold_sample, traced_sample};

/// The warmup/measure protocol. Warm reads use [`Protocol::WARM`]; writes
/// and cold runs use fewer (docs/architecture/60-validation.md, [`Protocol::COLD`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protocol {
    pub warmups: u32,
    pub samples: u32,
}

impl Protocol {
    /// The warm-read default: 32 warmups, 256 measured samples.
    pub const WARM: Self = Self {
        warmups: 32,
        samples: 256,
    };
    /// The cold default: every sample pays the touch, so few are needed.
    pub const COLD: Self = Self {
        warmups: 2,
        samples: 16,
    };
}

/// Exact percentiles of one measured window, in nanoseconds.
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

/// One measured window: percentiles plus the summed per-sample work
/// counts (the anti-dead-code contract — every runner drains its rows
/// and returns the count, which the harness black-boxes and sums).
#[derive(Debug, Clone)]
pub struct Measurement {
    pub stats: Stats,
    pub work: u64,
    /// The per-rep-normalized p50 (docs/silicon2/00), when
    /// [`Modes::proxy_per_rep`] ran: computed here, where the pre-sort
    /// sample/GHz alignment still exists.
    pub p50_norm: Option<u64>,
    /// The allocation window over the measured samples, when
    /// [`Modes::alloc_window`] ran (needs the `obs` feature).
    #[cfg(feature = "obs")]
    pub alloc: Option<bumbledb::alloc_counter::AllocSnapshot>,
    /// One additional post-measurement traced sample, when
    /// [`Modes::trace`] ran — traces never contaminate the measured
    /// samples.
    pub trace: Option<(u64, Vec<TraceEvent>)>,
}

/// Optional harness modes — alloc window and trace capture are
/// mutually exclusive (README rule); the per-rep proxy composes with
/// either.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modes {
    pub alloc_window: bool,
    pub trace: bool,
    /// Record an effective-GHz proxy reading after EVERY sample
    /// (docs/silicon2/00): co-tenant contamination arrives as
    /// seconds-long 2.0–2.4 GHz spans that survive min-of-reps between
    /// clean block-bracket proxies (fleet exp 15's phantom-finding
    /// machinery). Costs ~200 µs/sample — a confirm-run tool, not a
    /// routine gate mode.
    pub proxy_per_rep: bool,
}

/// The quantum floor (docs/silicon/00-baseline-and-harness.md): the
/// 24 MHz counter behind `Instant` quantizes at 41.67 ns, so a gated
/// per-sample time must be at least 12 ticks — below it, the driver
/// batches executes per sample and divides.
pub const QUANTUM_FLOOR_NS: u64 = 500;

/// Round-robin over a fixed param-set vector — the gate-style rotation
/// (misses included exactly where the family's policy says so). Generic
/// over the set representation: scenario worlds rotate plain
/// `Vec<Value>`, the ledger families rotate [`crate::families::Draw`]s.
#[derive(Debug, Clone)]
pub struct Rotation<T = Vec<Value>> {
    sets: Vec<T>,
    cursor: usize,
}
