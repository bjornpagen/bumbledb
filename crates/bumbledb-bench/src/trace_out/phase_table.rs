use bumbledb::obs::{Category, TraceEvent};

#[must_use]
pub fn render_phase_table(events: &[TraceEvent]) -> Option<String> {
    use std::fmt::Write as _;

    let mut cells: Vec<(usize, usize, u64, u64)> = Vec::new();
    for event in events.iter().filter(|e| e.cat() == Category::Phase) {
        let Some((phase, node)) = parse_phase(event.point()) else {
            continue;
        };
        cells.push((node, phase, event.a0(), event.a1()));
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

    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    let us = |ns: u64| ns as f64 / 1000.0;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<16} {:>10} {:>12} {:>10} {:>12}",
        "phase", "calls", "total_us", "avg_ns", "excl_us"
    );
    for &(node, phase, ns, calls) in &cells {
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

fn parse_phase(point: bumbledb::obs::TracePoint) -> Option<(usize, usize)> {
    match point {
        bumbledb::obs::TracePoint::JoinPhase { phase, node } => {
            Some((usize::from(phase), usize::from(node)))
        }
        _ => None,
    }
}
