use bumbledb::obs::{Category, TraceEvent};

#[must_use]
pub fn split_harness(events: Vec<TraceEvent>) -> (Vec<TraceEvent>, Vec<TraceEvent>) {
    events
        .into_iter()
        .partition(|event| event.cat() != Category::Harness)
}
