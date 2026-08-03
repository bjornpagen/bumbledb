//! Trace export (docs/architecture/60-validation.md): every captured run becomes a
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

mod containment;
mod flame_summary;
mod fold;
mod phase_table;
mod split_harness;
#[cfg(test)]
mod tests;
mod write_chrome;

pub use fold::fold_stacks;
pub use phase_table::render_phase_table;
pub use split_harness::split_harness;
pub use write_chrome::{write_chrome, write_trace_file, write_trace_pair};

#[cfg(test)]
use bumbledb::obs::Category;
use bumbledb::obs::TraceEvent;

/// The one traced-artifact fold every `--trace`-bearing lane shares:
/// splits a capture into engine and harness streams, writes the
/// `<dir>/<stem>.{json,folded}` pair ([`write_trace_pair`]), and
/// renders the engine flame top-10 (plus the phase table when the run
/// produced one) — the embed the reports carry.
///
/// # Errors
///
/// Trace I/O, as a message.
pub fn emit_pair(
    dir: &std::path::Path,
    stem: &str,
    events: Vec<TraceEvent>,
) -> Result<String, String> {
    let (engine, harness_events) = split_harness(events);
    write_trace_pair(dir, stem, &engine, &harness_events).map_err(|e| format!("trace: {e}"))?;
    let mut table = FlameSummary::compute(&engine).render_top(10);
    if let Some(phases) = render_phase_table(&engine) {
        table.push('\n');
        table.push_str(&phases);
    }
    Ok(table)
}

/// The traced twin sample's protocol: zero warmups, the one sample —
/// the capture prices a single steady-state invocation, never a window.
pub const TRACED_ONE: crate::harness::Protocol = crate::harness::Protocol {
    warmups: 0,
    samples: 1,
};

/// The traced solo sample (the `measure.rs` discipline as a stage):
/// AFTER a family's timed window, ONE more engine invocation runs
/// inside a capture (under the harness `sample` span) — the timed
/// windows stay untraced. Writes the `.json`/`.folded` pair under
/// `dir` and returns the flame embed; a `None` dir is the untraced
/// run, where the closure is never called. The engine-only lanes'
/// stage; mirrored worlds use [`traced_twin`].
///
/// # Errors
///
/// The runner's error and trace I/O, as messages.
pub fn traced_solo(
    dir: Option<&std::path::Path>,
    family: &str,
    ours: &mut dyn FnMut(crate::harness::Protocol) -> Result<crate::harness::Measurement, String>,
) -> Result<Option<String>, String> {
    let Some(dir) = dir else { return Ok(None) };
    let (_, events) = crate::harness::traced_sample(&mut || ours(TRACED_ONE).map(|m| m.work))?;
    emit_pair(dir, family, events).map(Some)
}

/// The traced twin sample ([`traced_solo`] lifted to a world with a
/// mirror): the engine invocation runs inside the capture and the
/// mirror runs the SAME extra op untraced — the twins stay in
/// lockstep, so a post-state fold never sees a one-sided commit. A
/// `None` dir is the untraced run, where neither closure is ever
/// called.
///
/// # Errors
///
/// Either runner's error and trace I/O, as messages.
pub fn traced_twin(
    dir: Option<&std::path::Path>,
    family: &str,
    ours: &mut dyn FnMut(crate::harness::Protocol) -> Result<crate::harness::Measurement, String>,
    theirs: &mut dyn FnMut(crate::harness::Protocol) -> Result<crate::harness::Measurement, String>,
) -> Result<Option<String>, String> {
    let Some(dir) = dir else { return Ok(None) };
    let table = traced_solo(Some(dir), family, ours)?;
    theirs(TRACED_ONE)?;
    Ok(table)
}
