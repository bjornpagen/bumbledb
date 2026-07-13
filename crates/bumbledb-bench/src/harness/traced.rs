use bumbledb::obs::TraceEvent;

/// One traced sample: the closure runs inside `obs::start_capture` /
/// `finish_capture` (empty without the engine's `trace` feature), under
/// a harness `sample` span so tool overhead is visible in the trace.
///
/// # Errors
///
/// The closure's error (the capture is drained either way).
pub fn traced_sample<F>(f: &mut F) -> Result<(u64, Vec<TraceEvent>), String>
where
    F: FnMut() -> Result<u64, String>,
{
    use bumbledb::obs::{Category, names};
    bumbledb::obs::start_capture();
    let span = bumbledb::obs::span(names::SAMPLE, Category::Harness);
    let result = f();
    span.end();
    let events = bumbledb::obs::finish_capture();
    Ok((result?, events))
}

/// One traced *cold* sample: one capture holding the harness `touch`
/// span (the eviction commit) followed by the `sample` span around the
/// timed execution — the rebuild spike, visible end to end.
///
/// # Errors
///
/// Either closure's error (the capture is drained either way).
pub fn traced_cold_sample<T, F>(touch: &mut T, f: &mut F) -> Result<(u64, Vec<TraceEvent>), String>
where
    T: FnMut() -> Result<(), String>,
    F: FnMut() -> Result<u64, String>,
{
    use bumbledb::obs::{Category, names};
    bumbledb::obs::start_capture();
    let span = bumbledb::obs::span(names::TOUCH, Category::Harness);
    let touched = touch();
    span.end();
    let result = touched.and_then(|()| {
        let span = bumbledb::obs::span(names::SAMPLE, Category::Harness);
        let result = f();
        span.end();
        result
    });
    let events = bumbledb::obs::finish_capture();
    Ok((result?, events))
}
