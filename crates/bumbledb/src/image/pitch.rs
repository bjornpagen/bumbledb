//! The pitch-padding placement mechanism for [`PitchPadder`]
//! (docs/silicon/11, bumblebench exp 10).

use super::{PitchPadder, LINE, PAD_MIN_PITCH, PAD_TOLERANCE, SET_STRIDE};

impl PitchPadder {
    pub(super) fn new() -> Self {
        Self {
            prev_start_by_width: [None; 2],
        }
    }

    /// Advances `cursor` (an element index into a backing store whose base
    /// address is `base_addr`, elements of `elem_size` bytes) to the next
    /// 128-byte-aligned position, then applies the pitch rule against the
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
            let pitch = (idx - prev) * elem_size;
            let residue = pitch % SET_STRIDE;
            // The measured band (exp 10's discriminators): EXACT 16 KiB
            // multiples are the fast configuration (stagger 16,384 ran
            // clean); the poison is a small NONZERO offset from one
            // (stagger 8/32 mild, 64/128 severe). Cure by rounding the
            // pitch UP to the next exact multiple.
            let in_band = (residue > 0 && residue <= PAD_TOLERANCE)
                || residue >= SET_STRIDE - PAD_TOLERANCE;
            if pitch >= PAD_MIN_PITCH && in_band {
                // Aligned starts make the residue a multiple of LINE,
                // so the delta divides evenly by either element size.
                idx += (SET_STRIDE - residue) / elem_size;
            }
        }
        self.prev_start_by_width[slab] = Some(idx);
        idx
    }
}
