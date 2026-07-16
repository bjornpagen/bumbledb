//! The stride-padding placement mechanism for [`StridePadder`]
//! (measured).

use super::{LINE, PAD_MIN_STRIDE, PAD_TOLERANCE, SET_STRIDE, StridePadder};

impl StridePadder {
    pub(super) fn new() -> Self {
        Self::with_tolerance(PAD_TOLERANCE)
    }

    /// The production rule with an explicit band half-width — the
    /// falsifier's hook: the shipped tolerance and its widened twin lay
    /// out side by side in one process for the interleaved A/B.
    pub(super) const fn with_tolerance(tolerance: usize) -> Self {
        Self {
            tolerance,
            prev_start_by_width: [None; 2],
        }
    }

    /// Advances `cursor` (an element index into a backing store whose base
    /// address is `base_addr`, elements of `elem_size` bytes) to the next
    /// 128-byte-aligned position, then applies the stride rule against the
    /// previous column in the same slab.
    pub(super) fn place(&mut self, base_addr: usize, elem_size: usize, cursor: usize) -> usize {
        let mut idx = cursor;
        // Align the absolute address to the line size.
        let misalign = (base_addr + idx * elem_size) % LINE;
        if misalign != 0 {
            idx += (LINE - misalign) / elem_size;
        }
        let slab = usize::from(elem_size != 8);
        if let Some(prev) = self.prev_start_by_width[slab] {
            let stride = (idx - prev) * elem_size;
            let residue = stride % SET_STRIDE;
            // The measured band: EXACT 16 KiB
            // multiples are the fast configuration (stagger 16,384 ran
            // clean); the poison is a small NONZERO offset from one
            // (stagger 8/32 mild, 64/128 severe; at image-scale
            // pitches the band ends by ~1.5 KiB — [`PAD_TOLERANCE`]).
            // Cure by rounding the stride UP to the next exact
            // multiple.
            let in_band = (residue > 0 && residue <= self.tolerance)
                || residue >= SET_STRIDE - self.tolerance;
            if stride >= PAD_MIN_STRIDE && in_band {
                // Aligned starts make the residue a multiple of LINE,
                // so the delta divides evenly by either element size.
                idx += (SET_STRIDE - residue) / elem_size;
            }
        }
        self.prev_start_by_width[slab] = Some(idx);
        idx
    }
}
