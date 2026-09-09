// Matched standalone mechanisms, not production dispatch or timing authority.
#[inline(never)]
pub fn branchy(
    starts: &[u64],
    ends: &[u64],
    positions: &[u32],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) {
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
pub fn compact(
    starts: &[u64],
    ends: &[u64],
    positions: &[u32],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) {
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
