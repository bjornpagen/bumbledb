//! The magnitude-first cover rule.
use super::KeyCount;

pub(super) fn better_cover(candidate: KeyCount, incumbent: KeyCount) -> bool {
    let (n, b) = (candidate.magnitude(), incumbent.magnitude());
    n < b
        || (n == b
            && matches!(candidate, KeyCount::Exact(_))
            && matches!(incumbent, KeyCount::Estimate(_)))
}
