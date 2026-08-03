use bumbledb::obs::TraceEvent;

use super::containment;

/// Folds a capture's span tree into Brendan-Gregg collapsed-stack format
/// — one `frameA;frameB;frameC <self_ns>` line per distinct enclosure
/// path, self time in nanoseconds — feeding straight into `flamegraph.pl`
/// / inferno beside the Chrome `.json`. Enclosure is the one containment
/// sweep the flame summary charges self time by
/// ([`containment::sweep`]): a span's stack is its chain of enclosing
/// ancestors, its charge is its duration minus its DIRECT children's —
/// identical siblings collapse onto one line (the folded contract).
/// Point events and `Category::Phase` accumulators carry no duration to
/// charge and are excluded, exactly as [`super::FlameSummary`] excludes
/// them. Lines are name-sorted for a byte-stable artifact.
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
            Some(parent) => format!("{};{}", path_of[parent], event.name),
            None => event.name.to_owned(),
        };
    }

    let mut folded: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for (index, event) in sweep.spans.iter().enumerate() {
        *folded
            .entry(std::mem::take(&mut path_of[index]))
            .or_default() += event.dur_ns - sweep.child_ns[index];
    }

    let mut out = String::new();
    for (stack, self_ns) in folded {
        let _ = writeln!(out, "{stack} {self_ns}");
    }
    out
}
