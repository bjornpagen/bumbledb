#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/p3-flat-kernels-v2.rs"]
#[allow(dead_code)]
mod previous;
pub use previous::branchy;

#[inline(never)]
pub fn block8(
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
    let mut base = 0;
    for ends in ends.chunks_exact(8) {
        if starts[base + 7] >= hi {
            break;
        }
        for (i, &end) in ends.iter().enumerate() {
            if end > lo {
                out.push(positions[base + i]);
            }
        }
        base += 8;
    }
    for j in base..starts.len() {
        if starts[j] >= hi {
            break;
        }
        if ends[j] > lo {
            out.push(positions[j]);
        }
    }
}

#[inline(never)]
pub fn mask8(
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
    let mut base = 0;
    for ends in ends.chunks_exact(8) {
        if starts[base + 7] >= hi {
            break;
        }
        let mut mask = 0u8;
        for (i, &end) in ends.iter().enumerate() {
            mask |= u8::from(end > lo) << i;
        }
        while mask != 0 {
            let i = mask.trailing_zeros() as usize;
            out.push(positions[base + i]);
            mask &= mask - 1;
        }
        base += 8;
    }
    for j in base..starts.len() {
        if starts[j] >= hi {
            break;
        }
        if ends[j] > lo {
            out.push(positions[j]);
        }
    }
}
