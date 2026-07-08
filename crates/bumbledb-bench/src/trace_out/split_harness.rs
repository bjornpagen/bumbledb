use bumbledb::obs::{Category, TraceEvent};

/// Splits one capture into (engine, harness) event streams — the
/// harness's own spans export under a separate tid so tool overhead is
/// honestly separated.
#[must_use]
pub fn split_harness(events: Vec<TraceEvent>) -> (Vec<TraceEvent>, Vec<TraceEvent>) {
    events
        .into_iter()
        .partition(|event| event.cat != Category::Harness)
}
