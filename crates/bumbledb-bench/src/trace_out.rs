//! Trace export (docs/architecture/50-validation.md): every captured run becomes a
//! Chrome Trace Format artifact (Perfetto / `chrome://tracing`) plus a
//! terminal flame summary — where-the-time-goes without leaving the
//! repo. Hand-rolled JSON, per the dependency quarantine.

/// One aggregated span name in the flame summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameRow {
    pub name: &'static str,
    pub calls: u64,
    pub total_ns: u64,
    /// Total minus the durations of *directly* nested children.
    pub self_ns: u64,
    pub p50_ns: u64,
    pub max_ns: u64,
}

/// The terminal where-the-time-goes table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameSummary {
    /// Sorted by self time, descending.
    pub rows: Vec<FlameRow>,
    /// `max(end) - min(start)` over every event.
    pub wall_ns: u64,
}

/// How many rows the render keeps.
const RENDER_ROWS: usize = 24;

mod flame_summary;
mod phase_table;
mod split_harness;
mod write_chrome;
#[cfg(test)]
mod tests;

pub use phase_table::render_phase_table;
pub use split_harness::split_harness;
pub use write_chrome::{write_chrome, write_trace_file};

#[cfg(test)]
use bumbledb::obs::{Category, TraceEvent};
