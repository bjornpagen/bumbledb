use bumbledb::Interval;

use crate::corpus_gen::{AT_BASE, AT_STEP, Rng, Sizes, mix};
use crate::schema::ids;

pub const MANDATE_SEGMENTS: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub org: u64,
    pub start: i64,
    pub end: i64,
}

/// # Panics
#[must_use]
pub fn mandate_segments(seed: u64, sizes: &Sizes, account: u64) -> [Segment; 4] {
    let mut rng = Rng::new(mix(seed, ids::MANDATE, account));

    let span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let unit = u64::try_from(span / 8).expect("positive").max(1);
    let length = |rng: &mut Rng| 1 + i64::try_from(rng.range(unit)).expect("fits");
    let gap = |rng: &mut Rng| 1 + i64::try_from(rng.range(unit / 4 + 1)).expect("fits");

    let start0 = AT_BASE + i64::try_from(rng.range(unit)).expect("fits");
    let end0 = start0 + length(&mut rng);
    let end1 = end0 + length(&mut rng);
    let start2 = end1 + gap(&mut rng);
    let end2 = start2 + length(&mut rng);
    let start3 = end2 + gap(&mut rng);
    let end3 = if account.is_multiple_of(2) {
        Interval::<i64>::MAX_END
    } else {
        start3 + length(&mut rng)
    };
    let org = |rng: &mut Rng| rng.range(sizes.orgs.max(1));
    [
        Segment {
            org: org(&mut rng),
            start: start0,
            end: end0,
        },
        Segment {
            org: org(&mut rng),
            start: end0,
            end: end1,
        },
        Segment {
            org: org(&mut rng),
            start: start2,
            end: end2,
        },
        Segment {
            org: org(&mut rng),
            start: start3,
            end: end3,
        },
    ]
}
