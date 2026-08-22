use bumbledb::obs::{Category, TraceEvent, names};

use super::containment;

/// Point events carry no duration to charge and are excluded, exactly nearest
/// `join` that ENDED before its flush stamp (`PhaseTimers` flushes once per
/// traced execution, after the rule loop) — its `a0`
#[must_use]
pub fn fold_stacks(events: &[TraceEvent]) -> String {
    use std::fmt::Write as _;
    let sweep = containment::sweep(events);

    // before their children, so each parent's path exists when its

    let mut path_of: Vec<String> = vec![String::new(); sweep.spans.len()];
    for (index, event) in sweep.spans.iter().enumerate() {
        path_of[index] = match sweep.parent[index] {
            Some(parent) => format!("{};{}", path_of[parent], event.name()),
            None => event.name().to_owned(),
        };
    }

    let mut folded: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut phase_ns = vec![0u64; sweep.spans.len()];
    for event in events.iter().filter(|e| e.cat() == Category::Phase) {
        let stamp = event.start_ns();
        let host = sweep
            .spans
            .iter()
            .rposition(|s| s.point() == names::JOIN && s.start_ns() + s.dur_ns() <= stamp)
            .or_else(|| {
                sweep
                    .spans
                    .iter()
                    .rposition(|s| s.start_ns() <= stamp && stamp < s.start_ns() + s.dur_ns())
            });
        let path = match host {
            Some(host) => {
                phase_ns[host] += event.a0();
                format!("{};{}", path_of[host], event.name())
            }
            None => event.name().to_owned(),
        };
        *folded.entry(path).or_default() += event.a0();
    }

    for (index, event) in sweep.spans.iter().enumerate() {
        *folded
            .entry(std::mem::take(&mut path_of[index]))
            .or_default() +=
            (event.dur_ns() - sweep.child_ns[index]).saturating_sub(phase_ns[index]);
    }

    let mut out = String::new();
    for (stack, self_ns) in folded {
        let _ = writeln!(out, "{stack} {self_ns}");
    }
    out
}
