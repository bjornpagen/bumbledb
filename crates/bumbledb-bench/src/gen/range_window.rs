use crate::gen::{Sizes, AT_BASE, AT_STEP};

/// The range family's `[start, end)` window over posting timestamps —
/// ≈2% of the corpus by construction.
///
/// # Panics
///
/// Only on a programmer-invariant violation: a posting count whose span
/// exceeds i64 (the scale table tops out at 10⁷).
#[must_use]
pub fn range_window(sizes: &Sizes) -> (i64, i64) {
    let span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let start = AT_BASE + span / 4;
    (start, start + span / 50)
}
