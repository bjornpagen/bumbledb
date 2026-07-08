use super::P99_BUDGET_NS;

/// Budget check — `≤` passes at the boundary exactly.
#[must_use]
pub fn within_budget(p99_ns: u64) -> bool {
    p99_ns <= P99_BUDGET_NS
}
