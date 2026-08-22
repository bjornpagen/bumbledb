use bumbledb::obs::{Category, TraceEvent};

pub(super) struct Sweep<'a> {

    pub(super) spans: Vec<&'a TraceEvent>,

    pub(super) parent: Vec<Option<usize>>,

    pub(super) child_ns: Vec<u64>,
}

/// Phase accumulators are synthetic point events (their `a0` is a duration
/// total, not a timestamped span) — containment math must not see them — and
/// point events have nothing to charge or enclose. Equal-tick nests (a sub-tick
/// child inside a sub-tick parent shares both endpoints on the 41.67 ns
/// counter) are ordered by the recorder's drop order: spans record at drop, so
/// the CHILD lands in the buffer first — reversing record order among equal
/// keys puts the parent first, where the stack walk needs it.
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
