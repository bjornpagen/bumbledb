//! Finite operation allowance for bench constructors and scorecard cells.
//!
//! L07's create/open/read/write take an explicit [`WorkContext`]. Timing
//! cells mint a fresh allowance per operation so work counters stay the
//! cell's admitted-work denominator (REVIEW-001). This is not a default-build
//! per-tuple atomic.

use bumbledb::{ExecutionPolicy, WorkContext};

/// Generous bench allowance — not a product cap and not an RSS identity.
#[must_use]
pub fn bench_policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1 << 40,
        working_bytes: 1 << 40,
        scratch_bytes: 1 << 40,
        result_bytes: 1 << 40,
        rows: 1 << 40,
        work_units: 1 << 40,
        timeout: std::time::Duration::from_secs(3600),
    }
}

/// # Errors
/// Invalid timeout — the bench policy is representable on a 64-bit clock.
pub fn bench_work() -> Result<WorkContext, String> {
    bench_policy()
        .start()
        .map_err(|error| format!("bench work: {error}"))
}

/// Tight work-unit cap for D08-shaped semantic checks (no timing).
/// # Errors
pub fn capped_work_units(units: u64) -> Result<WorkContext, String> {
    ExecutionPolicy {
        work_units: units,
        ..bench_policy()
    }
    .start()
    .map_err(|error| format!("capped work: {error}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn capped_work_refuses_after_the_admitted_budget() {
        let work = super::capped_work_units(2).expect("work");
        work.step(2).expect("budget admits exactly two units");
        assert!(
            work.step(1).is_err(),
            "D08: work without further output still stops"
        );
    }
}
