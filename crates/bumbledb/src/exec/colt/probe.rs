#[cfg(test)]
use super::hash_words;
use super::{Colt, Cursor, Map, ctrl_tag, eq_byte_mask, unpack_child, zero_byte_mask};

/// A successful key lookup, before the optional child-lane load. This never
/// escapes a probe call: forcing another node may relocate the bucket pool.
enum KeyHit {
    Row(u32),
    Bucket(usize),
}

enum ProbeTarget<'a> {
    Row(u32),
    Map(&'a Map),
}

/// A resolved cursor borrowed for a probe batch. Construction performs any
/// fallible force; subsequent lookups cannot mutate or relocate COLT pools.
pub(crate) struct Probe<'a> {
    colt: &'a Colt,
    level: usize,
    target: ProbeTarget<'a>,
}

impl Probe<'_> {
    /// The batch selects its key width once. `K == 0` dispatches the runtime
    /// width, including empty keys; fixed widths use the same probe walk.
    #[inline(always)]
    pub(crate) fn get_prehashed_width<const K: usize>(
        &self,
        key: &[u64],
        hash: u64,
    ) -> Option<Cursor> {
        self.key_hit::<K>(key, hash).map(|hit| match hit {
            KeyHit::Row(position) => Cursor::Row(position),
            KeyHit::Bucket(index) => unpack_child(self.colt.buckets[index]),
        })
    }

    #[inline(always)]
    pub(crate) fn contains_prehashed_width<const K: usize>(&self, key: &[u64], hash: u64) -> bool {
        self.key_hit::<K>(key, hash).is_some()
    }

    #[inline(always)]
    fn key_hit<const K: usize>(&self, key: &[u64], hash: u64) -> Option<KeyHit> {
        debug_assert_eq!(key.len(), self.colt.arity_at(self.level));
        debug_assert!(K == 0 || key.len() == K);
        match self.target {
            ProbeTarget::Row(position) => self
                .colt
                .position_matches(self.level, position, key)
                .then_some(KeyHit::Row(position)),
            ProbeTarget::Map(m) => {
                let (found, idx) = if K == 0 {
                    self.colt.probe_hashed(m, key, hash)
                } else {
                    self.colt.probe_walk::<K>(m, key, hash)
                };
                found.then(|| KeyHit::Bucket(m.child_at(idx)))
            }
        }
    }

    pub(crate) fn prefetch_batch(&self, hashes: &[u64], children: bool) {
        let ProbeTarget::Map(m) = self.target else {
            return;
        };
        if children {
            for &hash in hashes {
                self.colt.prefetch_map::<true>(m, hash);
            }
        } else {
            for &hash in hashes {
                self.colt.prefetch_map::<false>(m, hash);
            }
        }
    }

    pub(crate) fn any_position_matches(
        &self,
        child: Cursor,
        checks: &[(usize, usize, u64)],
    ) -> bool {
        self.colt.any_position_matches(child, checks)
    }
}

impl Colt {
    #[cfg(test)]
    pub fn get(&mut self, cursor: Cursor, level: usize, key: &[u64]) -> Option<Cursor> {
        self.get_prehashed(cursor, level, key, hash_words(key))
            .expect("test COLT has no refusing work ledger")
    }

    #[cfg(test)]
    pub fn get_prehashed(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Result<Option<Cursor>, crate::work::WorkError> {
        Ok(self
            .prepare_probe(cursor, level)?
            .get_prehashed_width::<0>(key, hash))
    }

    #[inline(always)]
    pub(super) fn probe_child_at(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Result<Option<Cursor>, crate::work::WorkError> {
        Ok(self
            .prepare_probe_at(cursor, level)?
            .get_prehashed_width::<0>(key, hash))
    }

    /// Resolve one cursor after fallible force, then keep its map borrowed for
    /// any number of read-only probes. A pinned row needs no force or hash.
    #[inline(always)]
    pub(crate) fn prepare_probe(
        &mut self,
        cursor: Cursor,
        level: usize,
    ) -> Result<Probe<'_>, crate::work::WorkError> {
        self.prepare_probe_at(cursor, self.join_index(level))
    }

    #[inline(always)]
    fn prepare_probe_at(
        &mut self,
        cursor: Cursor,
        level: usize,
    ) -> Result<Probe<'_>, crate::work::WorkError> {
        let target = match cursor {
            Cursor::Row(position) => ProbeTarget::Row(position),
            Cursor::Node(node) => {
                let map = self.force(node, level)?;
                ProbeTarget::Map(&self.maps[map as usize])
            }
        };
        Ok(Probe {
            colt: self,
            level,
            target,
        })
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
