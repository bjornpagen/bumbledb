use bumbledb::obs::{Category, TraceEvent};

/// Renders the executor phase table from `Category::Phase` accumulator
/// events (docs/architecture/60-validation.md): one row per (node, phase)
/// in node-major, phase-index order, with an `excl_us` column — the
/// phase's time minus everything attributed one node deeper, meaningful
/// for `descend` rows (per-row bookkeeping + leaf emits + the next
/// node's un-phased entry setup). Returns `None` when the capture holds
/// no phase events (non-join plans, pre-upgrade traces).
#[must_use]
pub fn render_phase_table(events: &[TraceEvent]) -> Option<String> {
    use std::fmt::Write as _;

    // (node, phase) -> (total_ns, calls); node 8 is the overflow bucket.
    let mut cells: Vec<(usize, usize, u64, u64)> = Vec::new();
    for event in events.iter().filter(|e| e.cat == Category::Phase) {
        let (phase, node) = parse_phase_name(event.name)?;
        cells.push((node, phase, event.a0, event.a1));
    }
    if cells.is_empty() {
        return None;
    }
    cells.sort_unstable();

    let node_total = |n: usize| -> u64 {
        cells
            .iter()
            .filter(|(node, phase, ..)| *node == n && *phase != 4)
            .map(|(.., ns, _)| ns)
            .sum::<u64>()
            + cells
                .iter()
                .find(|(node, phase, ..)| *node == n && *phase == 4)
                .map_or(0, |(.., ns, _)| *ns)
    };

    #[allow(clippy::cast_precision_loss)]
    let us = |ns: u64| ns as f64 / 1000.0;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<16} {:>10} {:>12} {:>10} {:>12}",
        "phase", "calls", "total_us", "avg_ns", "excl_us"
    );
    for &(node, phase, ns, calls) in &cells {
        // Descend's exclusive time subtracts the entire next node.
        let excl = if phase == 4 {
            ns.saturating_sub(node_total(node + 1))
        } else {
            ns
        };
        let _ = writeln!(
            out,
            "{:<16} {:>10} {:>12.3} {:>10} {:>12.3}",
            bumbledb::obs::names::JOIN_PHASE[phase][node.min(8)],
            calls,
            us(ns),
            ns / calls.max(1),
            us(excl),
        );
    }
    Some(out)
}

/// Recovers `(phase index, node index)` from a registry phase name.
fn parse_phase_name(name: &str) -> Option<(usize, usize)> {
    for (phase, nodes) in bumbledb::obs::names::JOIN_PHASE.iter().enumerate() {
        if let Some(node) = nodes.iter().position(|n| *n == name) {
            return Some((phase, node));
        }
    }
    None
}
