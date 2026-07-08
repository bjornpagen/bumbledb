//! The magnitude-first cover rule (docs/architecture/30-execution.md).

use super::KeyCount;

/// The magnitude-first cover rule (docs/architecture/30-execution.md): iterating a cover
/// costs O(its keys) plus a probe into every other subatom per key, and
/// both labels are admissible bounds on that cost — an `Estimate`
/// (unforced position count) is exact iteration cost pre-force and an
/// upper bound on post-force keys. So the smaller magnitude wins
/// regardless of label; on a tie, `Exact` wins (it cannot shrink); a
/// full tie keeps the incumbent (lowest subatom index — deterministic).
/// The old rule — "an Exact always displaces an Estimate" — iterated a
/// 500-key forced map while a 7-row param-filtered view sat unforced
/// beside it: the measured wrong-cover in the balance family.
pub(super) fn better_cover(candidate: KeyCount, incumbent: KeyCount) -> bool {
    let (n, b) = (candidate.magnitude(), incumbent.magnitude());
    n < b
        || (n == b
            && matches!(candidate, KeyCount::Exact(_))
            && matches!(incumbent, KeyCount::Estimate(_)))
}
