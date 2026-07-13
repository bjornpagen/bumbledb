#[cfg(test)]
use super::hash_words;
use super::{ctrl_tag, eq_byte_mask, unpack_child, zero_byte_mask, Colt, Cursor, Map, Slot};

impl Colt {
    /// Probes for `key` at `cursor`'s level, forcing the node if needed.
    /// Returns the child cursor on a hit. (The executor probes through
    /// [`Colt::get_prehashed`]; this convenience form serves the tests.)
    #[cfg(test)]
    pub fn get(&mut self, cursor: Cursor, level: usize, key: &[u64]) -> Option<Cursor> {
        self.get_prehashed(cursor, level, key, hash_words(key))
    }

    /// Probe with a precomputed hash (phase 2 of the two-phase batched
    /// probe): the load chain starts here; the hash was phase-1 ALU work.
    /// `level` is a join level.
    ///
    /// Inlined into the executor's probe loops (measured): an
    /// L2-resident probe stream's surviving cost class is instructions
    /// retired per probe — call ceremony here was first on the bill.
    #[inline(always)]
    pub fn get_prehashed(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Option<Cursor> {
        self.probe_child_at(cursor, self.selection_levels + level, key, hash)
    }

    /// [`Colt::get_prehashed`] over an internal (selection-inclusive)
    /// level — the shared body selection probes also walk.
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
            // A pinned row: the probe is a field-equality check, and the
            // child stays pinned to the same position.
            Cursor::Row(position) => self
                .position_matches(level, position, key)
                .then_some(Cursor::Row(position)),
            Cursor::Node(node) => {
                let map = self.force(node, level);
                // By reference (measured): `Map` is a 48-byte
                // Copy struct — a by-value bind here was one stack copy
                // per probe, a first-class suspect in the emulation that
                // reproduced the 55–60 ns plateau.
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

    /// Forces a node cursor ahead of a probe batch (no-op for pinned rows
    /// and already-forced nodes): phase 2's loads then hit a ready map.
    pub fn ensure_forced(&mut self, cursor: Cursor, level: usize) {
        if let Cursor::Node(node) = cursor {
            self.force(node, self.selection_levels + level);
        }
    }

    /// Bucket probe with a precomputed hash: the
    /// home bucket's 8 ctrl bytes load as one aligned SWAR word and the
    /// tag mask gates the key reads — a tag-missed slot never touches
    /// the key columns (the shape measured as the
    /// in-situ winner over full-key sweeping). Arity-monomorphic
    /// (measured): the dispatch happens once per probe, and each
    /// candidate's key compare is straight-line word compares — a
    /// runtime-length slice equality here compiled to a `bcmp` call per
    /// tag match.
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

    /// The monomorphic bucket walk: one aligned load
    /// of the home bucket's 8 ctrl bytes, SWAR tag/empty masks, key
    /// compares unrolled to `A` strided word compares per candidate.
    /// Scalar — the measured in-situ winner over a NEON sweep. A miss
    /// resolves at the bucket's first empty slot (inserts land there:
    /// buckets fill left to right, so occupied slots are a prefix and a
    /// match can never sit past an empty); a FULL bucket overflows to
    /// the next (bucket-linear probing, negligible below 0.4 load).
    #[inline(always)]
    fn probe_walk<const A: usize>(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        debug_assert_eq!(key.len(), A);
        debug_assert_eq!(m.arity, A);
        let nbm = m.nbuckets - 1;
        let wanted = ctrl_tag(hash);
        let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;
        loop {
            let group = m.ctrl_start + b * 8;
            let cw =
                u64::from_le_bytes(self.ctrl[group..group + 8].try_into().expect("ctrl group"));
            let mut matches = eq_byte_mask(cw, wanted);
            while matches != 0 {
                let slot = (matches.trailing_zeros() as usize) >> 3;
                let base = m.bucket_start + b * (8 * A + 8);
                let mut eq = true;
                #[expect(
                    clippy::needless_range_loop,
                    reason = "the explicit constant range is the intended unroll shape"
                )] // 0..A is the unroll
                // guarantee: A is const — iterating the runtime slice
                // would bound the loop by its len and block it
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

    /// The rare wide-key fallback (arity > 4 — beyond every bench plan).
    fn probe_walk_general(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        let nbm = m.nbuckets - 1;
        let wanted = ctrl_tag(hash);
        let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;
        loop {
            let group = m.ctrl_start + b * 8;
            let cw =
                u64::from_le_bytes(self.ctrl[group..group + 8].try_into().expect("ctrl group"));
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
