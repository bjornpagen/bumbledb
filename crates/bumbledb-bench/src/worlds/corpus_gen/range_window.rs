use crate::worlds::corpus_gen::{AT_BASE, AT_STEP, Sizes};

#[must_use]
pub fn range_window(sizes: &Sizes) -> (i64, i64) {
    let span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let start = AT_BASE + span / 4;
    (start, start + span / 50)
}
