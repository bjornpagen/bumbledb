use super::{LOAD_DEN, WordMap, ctrl_tag, hash_core, hash_words};

impl<V: Copy> WordMap<V> {
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
            // Keep growth before duplicate lookup and preserve each new
            // entry's allocation and publication sequence.
            self.entry_core::<K>(row, V::default);
        }
    }

    /// # Panics
    /// If `key.len() != arity`, capacity is exhausted, or `make` panics.
    #[inline(always)]
    pub fn get_or_insert_with(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
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
    fn entry_dyn_hashing(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        self.entry_dyn(key, hash_words(key), make)
    }

    #[inline(always)]
    fn entry_core<const K: usize>(
        &mut self,
        key: &[u64],
        make: impl FnOnce() -> V,
    ) -> (&mut V, bool) {
        let hash = hash_core::<K>(key);
        self.entry_hashed_core::<K>(key, hash, make)
    }

    #[inline(always)]
    fn entry_hashed_core<const K: usize>(
        &mut self,
        key: &[u64],
        hash: u64,
        make: impl FnOnce() -> V,
    ) -> (&mut V, bool) {
        debug_assert_eq!(key.len(), K);
        if (self.len + 1) * LOAD_DEN > self.capacity() {
            self.grow();
        }
        let (found, idx) = self.probe_core::<K>(key, hash);
        if !found {
            let value = make();
            let dense_idx = u32::try_from(idx).expect("slot index fits u32");
            self.keys[idx * K..idx * K + K].copy_from_slice(&key[..K]);
            self.values[idx].write(value);
            self.dense.push(dense_idx);
            // Publish only after construction and dense growth succeed.
            // Before this point partial slot writes remain empty or stale.
            self.stale -= usize::from(self.ctrl[idx] != 0);
            self.set_ctrl(idx, ctrl_tag(hash));
            self.len += 1;
        }
        // SAFETY: a matched or newly published live slot has an initialized
        // value. Stale slots never match, and V: Copy has no drop state.
        (unsafe { self.values[idx].assume_init_mut() }, !found)
    }

    pub(super) fn entry_dyn(
        &mut self,
        key: &[u64],
        hash: u64,
        make: impl FnOnce() -> V,
    ) -> (&mut V, bool) {
        debug_assert_eq!(key.len(), self.arity);
        if (self.len + 1) * LOAD_DEN > self.capacity() {
            self.grow();
        }
        let (found, idx) = self.probe(key, hash);
        if !found {
            let value = make();
            let dense_idx = u32::try_from(idx).expect("slot index fits u32");
            self.keys[idx * self.arity..(idx + 1) * self.arity].copy_from_slice(key);
            self.values[idx].write(value);
            self.dense.push(dense_idx);
            // Match the constant-arity publication boundary above.
            self.stale -= usize::from(self.ctrl[idx] != 0);
            self.set_ctrl(idx, ctrl_tag(hash));
            self.len += 1;
        }
        // SAFETY: a matched or newly published live slot has an initialized
        // value. Stale slots never match, and V: Copy has no drop state.
        (unsafe { self.values[idx].assume_init_mut() }, !found)
    }

    #[inline(always)]
    pub fn insert(&mut self, key: &[u64]) -> bool
    where
        V: Default,
    {
        self.get_or_insert_with(key, V::default).1
    }
}
