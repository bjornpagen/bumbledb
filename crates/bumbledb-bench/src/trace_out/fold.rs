use bumbledb::obs::{Category, TraceEvent, names};

use super::containment;

/// Folds a capture's span tree into Brendan-Gregg collapsed-stack format
/// — one `frameA;frameB;frameC <self_ns>` line per distinct enclosure
/// path, self time in nanoseconds — feeding straight into `flamegraph.pl`
/// / inferno beside the Chrome `.json`. Enclosure is the one containment
/// sweep the flame summary charges self time by
/// ([`containment::sweep`]): a span's stack is its chain of enclosing
/// ancestors, its charge is its duration minus its DIRECT children's —
/// identical siblings collapse onto one line (the folded contract).
/// Point events carry no duration to charge and are excluded, exactly
/// as [`super::FlameSummary`] excludes them.
///
/// `Category::Phase` accumulators are the exception: each carries the
/// join loop's per-(node, phase) time in `a0` (never a timestamped
/// span — the doctrine's no-per-tuple-spans line), and without them a
/// join-dominated capture folds to one flat `join` bar. Each folds as a
/// synthetic child frame of the `join` span it accounts for — the
/// nearest `join` that ENDED before its flush stamp (`PhaseTimers`
/// flushes once per traced execution, after the rule loop) — its `a0`
/// charged to the frame and subtracted from that join's self time. A
/// capture with no preceding join charges the deepest span containing
/// the stamp (attribution, not identification). Lines are name-sorted
/// for a byte-stable artifact.
#[must_use]
pub fn fold_stacks(events: &[TraceEvent]) -> String {
    use std::fmt::Write as _;
    let sweep = containment::sweep(events);
    // The full `a;b;c` enclosure path ending at each span: parents sort
    // before their children, so each parent's path exists when its
    // children need it.
    let mut path_of: Vec<String> = vec![String::new(); sweep.spans.len()];
    for (index, event) in sweep.spans.iter().enumerate() {
        path_of[index] = match sweep.parent[index] {
            Some(parent) => format!("{};{}", path_of[parent], event.name()),
            None => event.name().to_owned(),
        };
    }

    // Phase accumulators as synthetic frames, their charges collected
    // per host span so the hosts' self time gives them room.
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
