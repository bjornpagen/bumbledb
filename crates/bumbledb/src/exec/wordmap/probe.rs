use super::{WordMap, WINDOW};

/// The 7-bit hash tag a ctrl byte carries (bit 7 marks occupancy).
pub(super) fn tag(hash: u64) -> u8 {
    0x80 | u8::try_from(hash >> 57).expect("7 bits")
}

/// SWAR zero-byte mask: bit 7 of each zero byte in `w` sets.
#[inline(always)]
fn zero_byte_mask(w: u64) -> u64 {
    w.wrapping_sub(0x0101_0101_0101_0101) & !w & 0x8080_8080_8080_8080
}

/// SWAR byte-equality mask against a broadcast needle.
#[inline(always)]
fn eq_byte_mask(w: u64, needle: u8) -> u64 {
    zero_byte_mask(w ^ (u64::from(needle) * 0x0101_0101_0101_0101))
}

impl<V: Copy> WordMap<V> {
    /// Whether the stored key at `slot` equals `key` — a manual word
    /// loop: a runtime-length slice compare here compiles to a `bcmp`
    /// call inside the probe loop (the measured law, same as colt).
    #[inline(always)]
    fn key_at_matches(&self, slot: usize, key: &[u64]) -> bool {
        let stored = &self.keys[slot * self.arity..slot * self.arity + key.len()];
        let mut matches = true;
        for i in 0..key.len() {
            matches &= stored[i] == key[i];
        }
        matches
    }

    /// [`WordMap::key_at_matches`] with the width fixed at K: the loop
    /// unrolls to K straight-line compares, `slot * K` strength-reduces.
    #[inline(always)]
    fn key_at_matches_core<const K: usize>(&self, slot: usize, key: &[u64]) -> bool {
        let stored = &self.keys[slot * K..slot * K + K];
        let mut matches = true;
        for i in 0..K {
            matches &= stored[i] == key[i];
        }
        matches
    }

    /// Slot index for `key`: the match, or the empty slot to fill.
    /// Branchless window scan: eight ctrl bytes load as
    /// one word; SWAR masks mark empties and tag matches; candidates
    /// resolve in slot order. One well-predicted exit branch per window
    /// replaces one branch per slot — the measured 4.6× at hit-rate 0,
    /// which is the seen-set's steady state.
    pub(super) fn probe(&self, key: &[u64], hash: u64) -> (bool, usize) {
        self.probe_with(hash, |slot| self.key_at_matches(slot, key))
    }

    /// [`WordMap::probe`] with the key width fixed at K — the compare is
    /// K straight-line words.
    #[inline(always)]
    pub(super) fn probe_core<const K: usize>(&self, key: &[u64], hash: u64) -> (bool, usize) {
        self.probe_with(hash, |slot| self.key_at_matches_core::<K>(slot, key))
    }

    /// The window-scan body, generic over the key compare so the const-
    /// arity and runtime-arity probes share one probe shape.
    #[inline(always)]
    fn probe_with(&self, hash: u64, key_at: impl Fn(usize) -> bool) -> (bool, usize) {
        debug_assert!(!self.values.is_empty());
        let capacity = self.capacity();
        let mask = capacity - 1;
        let wanted = tag(hash);
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {
            // The mirror tail makes an 8-byte read at any idx < capacity
            // in-bounds and wrap-correct.
            let window = u64::from_le_bytes(
                self.ctrl[idx..idx + WINDOW]
                    .try_into()
                    .expect("window read"),
            );
            let empties = zero_byte_mask(window);
            let matches = eq_byte_mask(window, wanted);
            let mut candidates = empties | matches;
            while candidates != 0 {
                let bit = candidates & candidates.wrapping_neg();
                let offset = (bit.trailing_zeros() as usize) >> 3;
                let slot = (idx + offset) & mask;
                if empties & bit != 0 {
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
