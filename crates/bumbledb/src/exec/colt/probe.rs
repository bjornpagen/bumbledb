#[cfg(test)]
use super::hash_words;
use super::{Colt, Cursor, Map, Slot, ctrl_tag, eq_byte_mask, unpack_child, zero_byte_mask};

impl Colt {
    #[cfg(test)]
    pub fn get(&mut self, cursor: Cursor, level: usize, key: &[u64]) -> Option<Cursor> {
        self.get_prehashed(cursor, level, key, hash_words(key))
    }

    #[inline(always)]
    pub fn get_prehashed(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Option<Cursor> {
        self.probe_child_at(cursor, self.join_index(level), key, hash)
    }

    #[inline(always)]
    pub(super) fn probe_child_at(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Option<Cursor> {
        debug_assert_eq!(key.len(), self.arity_at(level));
        match cursor {
            Cursor::Row(position) => self
                .position_matches(level, position, key)
                .then_some(Cursor::Row(position)),
            Cursor::Node(node) => {
                let map = self.force(node, level);

                let m = &self.maps[map as usize];
                let (found, idx) = self.probe_hashed(m, key, hash);
                if !found {
                    return None;
                }
                match unpack_child(self.buckets[m.child_at(idx)]) {
                    Slot::Single(position) => Some(Cursor::Row(position)),
                    Slot::Node(child) => Some(Cursor::Node(child)),
                }
            }
        }
    }

    pub fn ensure_forced(&mut self, cursor: Cursor, level: usize) {
        if let Cursor::Node(node) = cursor {
            self.force(node, self.join_index(level));
        }
    }

    #[inline(always)]
    pub(super) fn probe_hashed(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        match key.len() {
            1 => self.probe_walk::<1>(m, key, hash),
            2 => self.probe_walk::<2>(m, key, hash),
            3 => self.probe_walk::<3>(m, key, hash),
            4 => self.probe_walk::<4>(m, key, hash),
            _ => self.probe_walk_general(m, key, hash),
        }
    }

    /// Scalar — the measured in-situ winner over a NEON sweep. A miss

    #[inline(always)]
    fn probe_walk<const A: usize>(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        debug_assert_eq!(key.len(), A);
        debug_assert_eq!(m.arity, A);
        let nbm = m.nbuckets - 1;
        let wanted = ctrl_tag(hash);

        let (groups, _) = self.ctrl.as_chunks::<8>();
        let group_base = m.ctrl_start / 8;
        let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;
        loop {
            let cw = u64::from_le_bytes(groups[group_base + b]);
            let mut matches = eq_byte_mask(cw, wanted);
            while matches != 0 {
                let slot = (matches.trailing_zeros() as usize) >> 3;
                let base = m.bucket_start + b * (8 * A + 8);
                let mut eq = true;
                #[expect(
                    clippy::needless_range_loop,
                    reason = "the explicit constant range is the intended unroll shape"
                )]
                for i in 0..A {
                    eq &= self.buckets[base + i * 8 + slot] == key[i];
                }
                if eq {
                    return (true, b * 8 + slot);
                }
                matches &= matches - 1;
            }
            let empties = zero_byte_mask(cw);
            if empties != 0 {
                let slot = (empties.trailing_zeros() as usize) >> 3;
                return (false, b * 8 + slot);
            }
            b = (b + 1) & nbm;
        }
    }

    fn probe_walk_general(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        let nbm = m.nbuckets - 1;
        let wanted = ctrl_tag(hash);

        let (groups, _) = self.ctrl.as_chunks::<8>();
        let group_base = m.ctrl_start / 8;
        let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;
        loop {
            let cw = u64::from_le_bytes(groups[group_base + b]);
            let mut matches = eq_byte_mask(cw, wanted);
            while matches != 0 {
                let slot = (matches.trailing_zeros() as usize) >> 3;
                let idx = b * 8 + slot;
                let mut eq = true;
                for (i, expected) in key.iter().enumerate() {
                    eq &= self.buckets[m.key_word_at(idx, i)] == *expected;
                }
                if eq {
                    return (true, idx);
                }
                matches &= matches - 1;
            }
            let empties = zero_byte_mask(cw);
            if empties != 0 {
                let slot = (empties.trailing_zeros() as usize) >> 3;
                return (false, b * 8 + slot);
            }
            b = (b + 1) & nbm;
        }
    }
}
