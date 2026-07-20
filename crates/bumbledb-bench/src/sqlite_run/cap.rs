use std::time::{Duration, Instant};

/// The DNF cap (docs/architecture/60-validation.md § the scenario worlds):
/// a per-sample wall-clock bound on a `SQLite` lane, enforced by `SQLite`'s
/// progress handler. The oracle GATE never runs under a cap — correctness
/// is sacred; the cap bounds timed/pre-flight samples only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapMs(pub u64);

/// The scenario default for adversarial lanes.
pub const DEFAULT_CAP: CapMs = CapMs(1_000);

/// One capped run's outcome: the closure finished under the deadline, or
/// the progress handler interrupted it — the honest DNF constructor (a
/// tripped run carries no result, so no censored number can leak out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapOutcome<T> {
    Done(T),
    Tripped,
}

/// The number of `SQLite` VM ops between progress-handler invocations:
/// 50k ops is coarse enough that the handler's `Instant::now()` read is
/// negligible against any statement worth capping, and fine enough that
/// an overrun is caught within a fraction of the cap.
const CAP_GRANULARITY_OPS: std::ffi::c_int = 50_000;

/// Runs `run` under a per-sample wall-clock cap: installs the progress
/// handler with the deadline, ALWAYS clears it before returning, and
/// folds the interrupt into [`CapOutcome::Tripped`].
///
/// # Errors
///
/// Any `SQLite` error other than the cap's own
/// [`rusqlite::ErrorCode::OperationInterrupted`], stringified.
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
