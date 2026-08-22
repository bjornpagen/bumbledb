use super::{LOAD_DEN, WordMap, ctrl_tag, hash_core, hash_words};

impl<V: Copy> WordMap<V> {
    /// # Panics
    /// Only on a programmer-invariant violation: `key.len() != arity`.
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
            self.stale -= usize::from(self.ctrl[idx] != 0);
            self.set_ctrl(idx, ctrl_tag(hash));
            self.keys[idx * K..idx * K + K].copy_from_slice(&key[..K]);
            self.values[idx].write(make());
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            self.len += 1;
        }
        // SAFETY: the slot's ctrl byte is set (matched or just written),

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
            self.stale -= usize::from(self.ctrl[idx] != 0);
            self.set_ctrl(idx, ctrl_tag(hash));
            self.keys[idx * self.arity..(idx + 1) * self.arity].copy_from_slice(key);
            self.values[idx].write(make());
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            self.len += 1;
        }
        // SAFETY: the slot's ctrl byte is set (matched or just written),

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
