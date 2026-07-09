use bumbledb::Interval;

use crate::gen::{mix, Rng, Sizes, AT_BASE, AT_STEP};
use crate::schema::ids;

/// Segments per account — every account carries exactly this many, so
/// mandate row `r` is segment `r % MANDATE_SEGMENTS` of account
/// `r / MANDATE_SEGMENTS` (random access without prefix sums).
pub const MANDATE_SEGMENTS: u64 = 4;

/// One mandate segment: the granting org and the half-open active
/// window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub org: u64,
    pub start: i64,
    pub end: i64,
}

/// One account's full mandate history, **valid under the pointwise key
/// by construction**: four sequential segments,
///
/// - segment 0 → 1 **abutting** (`end0 == start1` — the neighbor-probe
///   boundary as a data case, not just a unit test),
/// - segment 1 → 2 and 2 → 3 **gapped** (a strictly positive gap, so a
///   gap instant exists for the membership-miss draw),
/// - segment 3 ending at the **sentinel** `Interval::<i64>::MAX_END`
///   (the "currently active" convention) on every even account —
///   structurally guaranteed, never left to chance.
///
/// Orgs draw independently per segment, so one account's history spans
/// several orgs (the overlap family joins across accounts through a
/// shared org).
///
/// # Panics
///
/// Never in practice: window arithmetic stays far below `i64::MAX` at
/// every scale (the size table tops out at 10⁷ postings).
#[must_use]
pub fn mandate_segments(seed: u64, sizes: &Sizes, account: u64) -> [Segment; 4] {
    let mut rng = Rng::new(mix(seed, ids::MANDATE, account));
    // The posting-at span: segments tile it so at-instant probes over
    // posting timestamps mostly land inside a window.
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
