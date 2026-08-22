#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameRow {
    pub name: &'static str,
    pub calls: u64,
    pub total_ns: u64,

    pub self_ns: u64,
    pub p50_ns: u64,
    pub max_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameSummary {
    pub rows: Vec<FlameRow>,

    pub wall_ns: u64,
}

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

use bumbledb::obs::TraceEvent;

/// # Errors
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
pub const TRACED_ONE: crate::harness::Protocol = crate::harness::Protocol {
    warmups: 0,
    samples: 1,
};

/// AFTER a family's timed window, ONE more engine invocation runs
/// # Errors
pub fn traced_solo(
    dir: Option<&std::path::Path>,
    family: &str,
    ours: &mut dyn FnMut(crate::harness::Protocol) -> Result<crate::harness::Measurement, String>,
) -> Result<Option<String>, String> {
    let Some(dir) = dir else { return Ok(None) };
    let (_, events) = crate::harness::traced_sample(&mut || ours(TRACED_ONE).map(|m| m.work))?;
    emit_pair(dir, family, events).map(Some)
}

/// # Errors
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

/// # Errors
pub fn traced_cold_solo(
    dir: Option<&std::path::Path>,
    family: &str,
    touch: &mut dyn FnMut() -> Result<(), String>,
    query: &mut dyn FnMut() -> Result<u64, String>,
) -> Result<Option<String>, String> {
    let Some(dir) = dir else { return Ok(None) };
    let (_, events) = crate::harness::traced_cold_sample(touch, query)?;
    emit_pair(dir, family, events).map(Some)
}
