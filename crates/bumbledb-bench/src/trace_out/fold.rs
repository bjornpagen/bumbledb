use bumbledb::obs::{Category, TraceEvent};

/// Folds a capture's span tree into Brendan-Gregg collapsed-stack format
/// — one `frameA;frameB;frameC <self_ns>` line per distinct enclosure
/// path, self time in nanoseconds — feeding straight into `flamegraph.pl`
/// / inferno beside the Chrome `.json`. Enclosure is the same containment
/// sweep the flame summary charges self time by (spans re-sorted by
/// `(start, -end)`, a stack walk): a span's stack is its chain of
/// enclosing ancestors, its charge is its duration minus its DIRECT
/// children's — identical siblings collapse onto one line (the folded
/// contract). Point events and `Category::Phase` accumulators carry no
/// duration to charge and are excluded, exactly as [`super::FlameSummary`]
/// excludes them. Lines are name-sorted for a byte-stable artifact.
#[must_use]
pub fn fold_stacks(events: &[TraceEvent]) -> String {
    use std::fmt::Write as _;
    let mut spans: Vec<&TraceEvent> = events
        .iter()
        .filter(|e| e.dur_ns > 0 && e.cat != Category::Phase)
        .collect();
    spans.sort_by_key(|e| (e.start_ns, std::cmp::Reverse(e.start_ns + e.dur_ns)));
    let mut child_ns = vec![0u64; spans.len()];
    // The full `a;b;c` enclosure path ending at each span, built as the
    // stack sweep discovers each span's parent.
    let mut path_of: Vec<String> = vec![String::new(); spans.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (index, event) in spans.iter().enumerate() {
        while let Some(&top) = stack.last() {
            if spans[top].start_ns + spans[top].dur_ns <= event.start_ns {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some(&parent) = stack.last() {
            child_ns[parent] += event.dur_ns;
            path_of[index] = format!("{};{}", path_of[parent], event.name);
        } else {
            path_of[index].push_str(event.name);
        }
        stack.push(index);
    }

    let mut folded: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for (index, event) in spans.iter().enumerate() {
        *folded
            .entry(std::mem::take(&mut path_of[index]))
            .or_default() += event.dur_ns - child_ns[index];
    }

    let mut out = String::new();
    for (stack, self_ns) in folded {
        let _ = writeln!(out, "{stack} {self_ns}");
    }
    out
}
