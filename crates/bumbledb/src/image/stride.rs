//! The stride-padding placement mechanism for [`StridePadder`]
//! (measured).
use super::{LINE, PAD_MIN_STRIDE, PAD_TOLERANCE, SET_STRIDE, StridePadder};

impl StridePadder {
    pub(super) fn new() -> Self {
        Self::with_tolerance(PAD_TOLERANCE)
    }

    pub(super) const fn with_tolerance(tolerance: usize) -> Self {
        Self {
            tolerance,
            prev_start_by_width: [None; 2],
        }
    }

    pub(super) fn place(&mut self, base_addr: usize, elem_size: usize, cursor: usize) -> usize {
        let mut idx = cursor;

        let misalign = (base_addr + idx * elem_size) % LINE;
        if misalign != 0 {
            idx += (LINE - misalign) / elem_size;
        }
        let slab = usize::from(elem_size != 8);
        if let Some(prev) = self.prev_start_by_width[slab] {
            let stride = (idx - prev) * elem_size;
            let residue = stride % SET_STRIDE;

            let in_band = (residue > 0 && residue <= self.tolerance)
                || residue >= SET_STRIDE - self.tolerance;
            if stride >= PAD_MIN_STRIDE && in_band {
                idx += (SET_STRIDE - residue) / elem_size;
            }
        }
        self.prev_start_by_width[slab] = Some(idx);
        idx
    }
}
