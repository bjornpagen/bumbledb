// Equal length guards in each non-inlined mechanism keep the baseline's
// three independent slice arguments from adding per-position bounds checks.
#[inline(never)]
pub fn branchy(
    starts: &[u64],
    ends: &[u64],
    positions: &[u32],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) {
    assert_eq!(starts.len(), ends.len());
    assert_eq!(starts.len(), positions.len());
    out.clear();
    for j in 0..starts.len() {
        if starts[j] >= hi {
            break;
        }
        if ends[j] > lo {
            out.push(positions[j]);
        }
    }
}

#[inline(never)]
pub fn prefix(
    starts: &[u64],
    ends: &[u64],
    positions: &[u32],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) {
    assert_eq!(starts.len(), ends.len());
    assert_eq!(starts.len(), positions.len());
    let eligible = starts.partition_point(|&start| start < hi);
    out.clear();
    let positions = &positions[..eligible];
    for (j, &end) in ends[..eligible].iter().enumerate() {
        if end > lo {
            out.push(positions[j]);
        }
    }
}

#[inline(never)]
pub fn compact(
    starts: &[u64],
    ends: &[u64],
    positions: &[u32],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) {
    assert_eq!(starts.len(), ends.len());
    assert_eq!(starts.len(), positions.len());
    let eligible = starts.partition_point(|&start| start < hi);
    out.clear();
    out.extend_from_slice(&positions[..eligible]);
    let mut written = 0;
    for j in 0..eligible {
        out[written] = positions[j];
        written += usize::from(ends[j] > lo);
    }
    out.truncate(written);
}
