use super::{WINDOW, WordMap, ctrl_tag, eq_byte_mask, zero_byte_mask};

impl<V: Copy> WordMap<V> {

    #[inline(always)]
    fn key_at_matches(&self, slot: usize, key: &[u64]) -> bool {
        let stored = &self.keys[slot * self.arity..slot * self.arity + key.len()];
        let mut matches = true;
        for i in 0..key.len() {
            matches &= stored[i] == key[i];
        }
        matches
    }

    #[inline(always)]
    fn key_at_matches_core<const K: usize>(&self, slot: usize, key: &[u64]) -> bool {
        let stored = &self.keys[slot * K..slot * K + K];
        let mut matches = true;
        for i in 0..K {
            matches &= stored[i] == key[i];
        }
        matches
    }

    pub(super) fn probe(&self, key: &[u64], hash: u64) -> (bool, usize) {
        self.probe_with(hash, |slot| self.key_at_matches(slot, key))
    }

    #[inline(always)]
    pub(super) fn probe_core<const K: usize>(&self, key: &[u64], hash: u64) -> (bool, usize) {
        self.probe_with(hash, |slot| self.key_at_matches_core::<K>(slot, key))
    }

    #[inline(always)]
    fn probe_with(&self, hash: u64, key_at: impl Fn(usize) -> bool) -> (bool, usize) {
        debug_assert!(!self.values.is_empty());
        let capacity = self.capacity();
        let mask = capacity - 1;
        let wanted = ctrl_tag(hash);
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {

            // invariant the slice type cannot carry, because windows

            let window = u64::from_le_bytes(
                *self.ctrl[idx..]
                    .first_chunk::<WINDOW>()
                    .expect("mirror tail keeps windows in-bounds"),
            );
            let empties = zero_byte_mask(window);
            let matches = eq_byte_mask(window, wanted);
            let mut candidates = empties | matches;
            while candidates != 0 {
                let bit = candidates.isolate_lowest_one();
                let offset = (bit.trailing_zeros() as usize) >> 3;
                let slot = (idx + offset) & mask;
                if empties & bit != 0 {
                    return (false, slot);
                }

                // ahead of it — every slot before it was live-occupied or

                if self.stamps[slot] != self.generation {
                    return (false, slot);
                }
                if key_at(slot) {
                    return (true, slot);
                }
                candidates &= !bit;
            }
            idx = (idx + WINDOW) & mask;
        }
    }
}
