use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapMs(pub u64);

pub const DEFAULT_CAP: CapMs = CapMs(1_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapOutcome<T> {
    Done(T),
    Tripped,
}

const CAP_GRANULARITY_OPS: std::ffi::c_int = 50_000;

/// handler with the deadline, ALWAYS clears it before returning, and
/// # Errors
pub fn with_cap<T>(
    conn: &rusqlite::Connection,
    cap: CapMs,
    run: impl FnOnce() -> Result<T, rusqlite::Error>,
) -> Result<CapOutcome<T>, String> {
    let deadline = Instant::now() + Duration::from_millis(cap.0);
    conn.progress_handler(
        CAP_GRANULARITY_OPS,
        Some(move || Instant::now() >= deadline),
    );
    let result = run();
    conn.progress_handler(CAP_GRANULARITY_OPS, None::<fn() -> bool>);
    match result {
        Ok(value) => Ok(CapOutcome::Done(value)),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::OperationInterrupted =>
        {
            Ok(CapOutcome::Tripped)
        }
        Err(e) => Err(format!("capped run: {e}")),
    }
}
