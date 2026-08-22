use bumbledb::obs::TraceEvent;

/// # Errors
pub fn traced_sample<F>(f: &mut F) -> Result<(u64, Vec<TraceEvent>), String>
where
    F: FnMut() -> Result<u64, String>,
{
    use bumbledb::obs::names;
    bumbledb::obs::start_capture();
    let span = bumbledb::obs::span(names::SAMPLE);
    let result = f();
    span.end();
    let events = bumbledb::obs::finish_capture();
    Ok((result?, events))
}

/// # Errors
pub fn traced_cold_sample<T, F>(touch: &mut T, f: &mut F) -> Result<(u64, Vec<TraceEvent>), String>
where
    T: FnMut() -> Result<(), String> + ?Sized,
    F: FnMut() -> Result<u64, String> + ?Sized,
{
    use bumbledb::obs::names;
    bumbledb::obs::start_capture();
    let span = bumbledb::obs::span(names::TOUCH);
    let touched = touch();
    span.end();
    let result = touched.and_then(|()| {
        let span = bumbledb::obs::span(names::SAMPLE);
        let result = f();
        span.end();
        result
    });
    let events = bumbledb::obs::finish_capture();
    Ok((result?, events))
}
