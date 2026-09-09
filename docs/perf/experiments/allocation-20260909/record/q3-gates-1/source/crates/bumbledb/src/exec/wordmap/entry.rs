use super::{LOAD_DEN, WordMap, ctrl_tag, hash_core, hash_words};

impl<V: Copy> WordMap<V> {
    #[must_use]
    pub(crate) fn contains_key(&self, key: &[u64]) -> bool {
        assert_eq!(key.len(), self.arity);
        self.len() != 0 && self.probe(key, hash_words(key)).0
    }

    /// Insert complete, nonempty-width rows in their original order.
    /// Zero-width keys require explicit row counts and use `insert` instead.
    ///
    /// # Panics
    /// If the map has zero arity, `words` contains an incomplete row,
    /// capacity is exhausted, or `V::default` panics.
    #[inline(never)]
    pub(crate) fn insert_rows(&mut self, words: &[u64])
    where
        V: Default,
    {
        assert_ne!(self.arity, 0, "bulk insertion requires nonzero arity");
        assert_eq!(words.len() % self.arity, 0, "incomplete bulk row");
        match self.arity {
            1 => self.insert_rows_core::<1>(words),
            2 => self.insert_rows_core::<2>(words),
            3 => self.insert_rows_core::<3>(words),
            4 => self.insert_rows_core::<4>(words),
            5 => self.insert_rows_core::<5>(words),
            6 => self.insert_rows_core::<6>(words),
            7 => self.insert_rows_core::<7>(words),
            8 => self.insert_rows_core::<8>(words),
            _ => {
                for row in words.chunks_exact(self.arity) {
                    self.entry_dyn(row, hash_words(row), V::default);
                }
            }
        }
    }

    #[inline]
    fn insert_rows_core<const K: usize>(&mut self, words: &[u64])
    where
        V: Default,
    {
        for row in words.as_chunks::<K>().0 {
            // Scalar and bulk insertion share lookup, growth and publication.
            self.entry_core::<K>(row, V::default);
        }
    }

    /// # Panics
    /// If `key.len() != arity`, capacity is exhausted, or `make` panics.
    #[inline(always)]
    pub fn get_or_insert_with(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        let (row, inserted) = self.entry(key, make);
        (&mut self.values[row], inserted)
    }

    // Set consumers need an ordinal, not a discarded mutable value reference.
    // Keep lookup, growth and publication shared with real-value consumers.
    #[inline(always)]
    fn entry(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (usize, bool) {
        assert_eq!(key.len(), self.arity);

        match self.arity {
            0 => self.entry_core::<0>(key, make),
            1 => self.entry_core::<1>(key, make),
            2 => self.entry_core::<2>(key, make),
            3 => self.entry_core::<3>(key, make),
            4 => self.entry_core::<4>(key, make),
            5 => self.entry_core::<5>(key, make),
            6 => self.entry_core::<6>(key, make),
            7 => self.entry_core::<7>(key, make),
            8 => self.entry_core::<8>(key, make),
            _ => self.entry_dyn_hashing(key, make),
        }
    }

    #[cold]
    #[inline(never)]
    fn entry_dyn_hashing(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (usize, bool) {
        self.entry_dyn(key, hash_words(key), make)
    }

    #[inline(always)]
    fn entry_core<const K: usize>(
        &mut self,
        key: &[u64],
        make: impl FnOnce() -> V,
    ) -> (usize, bool) {
        let hash = hash_core::<K>(key);
        self.entry_hashed_core::<K>(key, hash, make)
    }

    #[inline(always)]
    fn entry_hashed_core<const K: usize>(
        &mut self,
        key: &[u64],
        hash: u64,
        make: impl FnOnce() -> V,
    ) -> (usize, bool) {
        debug_assert_eq!(key.len(), K);
        if (self.len() + 1) * LOAD_DEN > self.capacity() {
            // Duplicates at the boundary do not grow either index or payload.
            if self.len() != 0 {
                let (found, slot) = self.probe_core::<K>(key, hash);
                if found {
                    return (self.slots[slot] as usize, false);
                }
            }
            self.grow();
        }
        let (found, slot) = self.probe_core::<K>(key, hash);
        if found {
            (self.slots[slot] as usize, false)
        } else {
            (self.insert_vacant(&key[..K], slot, hash, make), true)
        }
    }

    pub(super) fn entry_dyn(
        &mut self,
        key: &[u64],
        hash: u64,
        make: impl FnOnce() -> V,
    ) -> (usize, bool) {
        debug_assert_eq!(key.len(), self.arity);
        if (self.len() + 1) * LOAD_DEN > self.capacity() {
            if self.len() != 0 {
                let (found, slot) = self.probe(key, hash);
                if found {
                    return (self.slots[slot] as usize, false);
                }
            }
            self.grow();
        }
        let (found, slot) = self.probe(key, hash);
        if found {
            (self.slots[slot] as usize, false)
        } else {
            (self.insert_vacant(key, slot, hash, make), true)
        }
    }

    #[inline(always)]
    fn insert_vacant(
        &mut self,
        key: &[u64],
        slot: usize,
        hash: u64,
        make: impl FnOnce() -> V,
    ) -> usize {
        let row = self.len();
        let ordinal = u32::try_from(row).expect("row ordinal fits u32");
        let stale = self.stale - usize::from(self.ctrl[slot] != 0);
        let value = make();
        // Reserve values before appending Copy keys. A failed key reservation
        // leaves its length unchanged; after it succeeds, the reserved value
        // push cannot allocate or invoke user code. Publish the index last.
        self.values.reserve(1);
        self.keys.extend_from_slice(key);
        self.values.push(value);
        self.slots[slot] = ordinal;
        self.stale = stale;
        self.set_ctrl(slot, ctrl_tag(hash));
        row
    }

    #[inline(always)]
    pub fn insert(&mut self, key: &[u64]) -> bool
    where
        V: Default,
    {
        self.entry(key, V::default).1
    }
}
