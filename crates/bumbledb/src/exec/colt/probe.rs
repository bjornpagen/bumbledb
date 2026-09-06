#[cfg(test)]
use super::hash_words;
use super::{Colt, Cursor, Map, ctrl_tag, eq_byte_mask, unpack_child, zero_byte_mask};

impl Colt {
    #[cfg(test)]
    pub fn get(&mut self, cursor: Cursor, level: usize, key: &[u64]) -> Option<Cursor> {
        self.get_prehashed(cursor, level, key, hash_words(key))
            .expect("test COLT has no refusing work ledger")
    }

    /// # Errors
    /// Returns the force/growth refusal. A miss is `Ok(None)`, never an error.
    #[inline(always)]
    pub fn get_prehashed(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Result<Option<Cursor>, crate::work::WorkError> {
        self.probe_child_at(cursor, self.join_index(level), key, hash)
    }

    /// The sibling batch has already selected its fixed key width.
    /// `K == 0` retains runtime dispatch, including actual empty keys.
    ///
    /// # Errors
    /// Returns the same force/growth refusal as `get_prehashed`.
    #[inline(always)]
    pub(crate) fn get_prehashed_width<const K: usize>(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Result<Option<Cursor>, crate::work::WorkError> {
        self.probe_child_at_width::<K>(cursor, self.join_index(level), key, hash)
    }

    #[inline(always)]
    pub(super) fn probe_child_at(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Result<Option<Cursor>, crate::work::WorkError> {
        self.probe_child_at_width::<0>(cursor, level, key, hash)
    }

    #[inline(always)]
    fn probe_child_at_width<const K: usize>(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Result<Option<Cursor>, crate::work::WorkError> {
        debug_assert_eq!(key.len(), self.arity_at(level));
        debug_assert!(K == 0 || key.len() == K);
        match cursor {
            Cursor::Row(position) => Ok(self
                .position_matches(level, position, key)
                .then_some(Cursor::Row(position))),
            Cursor::Node(node) => {
                let map = self.force(node, level)?;
                let m = &self.maps[map as usize];
                let (found, idx) = if K == 0 {
                    self.probe_hashed(m, key, hash)
                } else {
                    self.probe_walk::<K>(m, key, hash)
                };
                if !found {
                    return Ok(None);
                }
                Ok(Some(unpack_child(self.buckets[m.child_at(idx)])))
            }
        }
    }

    /// # Errors
    /// Returns the force/growth refusal before the node is marked forced.
    pub fn ensure_forced(
        &mut self,
        cursor: Cursor,
        level: usize,
    ) -> Result<(), crate::work::WorkError> {
        if let Cursor::Node(node) = cursor {
            self.force(node, self.join_index(level))?;
        }
        Ok(())
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

    /// Tag-gated scalar probing: a tag miss never loads the key block.
    /// The unconditional NEON sweep won in isolation but lost under the
    /// actual executor's cache displacement (bumblebench's in-situ record).
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
