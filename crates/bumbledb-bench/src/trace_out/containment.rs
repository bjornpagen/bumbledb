use bumbledb::obs::{Category, TraceEvent};

/// One capture's span forest, swept once: the single containment
/// derivation [`fold_stacks`](super::fold_stacks) and
/// [`FlameSummary::compute`](super::FlameSummary::compute) both charge
/// by (filter, sort, stack walk, direct-child charging — previously
/// spelled twice, drift-prone).
pub(super) struct Sweep<'a> {
    /// The real spans (`TraceEvent::Span`, `Category::Phase` excluded),
    /// sorted parent-before-child.
    pub(super) spans: Vec<&'a TraceEvent>,
    /// Each span's direct parent, as an index into `spans`; `None` at
    /// the roots.
    pub(super) parent: Vec<Option<usize>>,
    /// The summed durations of each span's DIRECT children — a span's
    /// self time is `dur_ns - child_ns`.
    pub(super) child_ns: Vec<u64>,
}

/// The one containment sweep. Phase accumulators are synthetic point
/// events (their `a0` is a duration total, not a timestamped span) —
/// containment math must not see them — and point events have
/// nothing to charge or enclose.
///
/// Containment is positional: spans re-sorted by `(start, -end)` and
/// walked with a stack. Equal-tick nests (a sub-tick child inside a
/// sub-tick parent shares both endpoints on the 41.67 ns counter) are
/// ordered by the recorder's drop order: spans record at drop, so the
/// CHILD lands in the buffer first — reversing record order among equal
/// keys puts the parent first, where the stack walk needs it. (A stable
/// sort alone preserved the buffer's child-first order and inverted the
/// pair.)
pub(super) fn sweep(events: &[TraceEvent]) -> Sweep<'_> {
    let mut spans: Vec<(usize, &TraceEvent)> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, TraceEvent::Span { .. } if e.cat() != Category::Phase))
        .collect();
    spans.sort_by_key(|&(recorded, e)| {
        (
            e.start_ns(),
            std::cmp::Reverse(e.start_ns() + e.dur_ns()),
            std::cmp::Reverse(recorded),
        )
    });
    let spans: Vec<&TraceEvent> = spans.into_iter().map(|(_, e)| e).collect();

    let mut parent: Vec<Option<usize>> = vec![None; spans.len()];
    let mut child_ns = vec![0u64; spans.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (index, event) in spans.iter().enumerate() {
        while let Some(&top) = stack.last() {
            if spans[top].start_ns() + spans[top].dur_ns() <= event.start_ns() {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some(&enclosing) = stack.last() {
            parent[index] = Some(enclosing);
            child_ns[enclosing] += event.dur_ns();
        }
        stack.push(index);
    }
    Sweep {
        spans,
        parent,
        child_ns,
    }
}
