use crate::corpus_gen::{AT_BASE, AT_STEP, Sizes};

/// # Panics
/// Only on a programmer-invariant violation: a posting count whose span
#[must_use]
pub fn range_window(sizes: &Sizes) -> (i64, i64) {
    let span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let start = AT_BASE + span / 4;
    (start, start + span / 50)
}
