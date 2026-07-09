use super::hash::{hash_core, hash_words};
use super::probe::tag;
use super::{WordMap, LOAD_DEN};

impl<V: Copy> WordMap<V> {
    /// Gets the value for `key`, inserting `make()` when absent. Returns
    /// `(value, inserted)`.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: `key.len() != arity`.
    #[inline(always)] // the whole chain inlines into the sink row loops so
                      // LLVM can hoist batch-constant prefix hashes out of them
    pub fn get_or_insert_with(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        assert_eq!(key.len(), self.arity);
        // The const-arity dispatch (measured): runtime
        // arity taxed every dedup row +4.3-4.9 ns via the general-length
        // compare/copy ladder, the slot*arity multiplies, and the blocked
        // hash hoisting. One predictable branch here (the same arm every
        // call from a given sink) buys straight-line monomorphs for the
        // widths in use: group keys are 1-4, full bindings 2-6, 8 is
        // headroom. Exotic widths keep the dyn path.
        match self.arity {
            1 => self.entry_core::<1>(key, make),
            2 => self.entry_core::<2>(key, make),
            3 => self.entry_core::<3>(key, make),
            4 => self.entry_core::<4>(key, make),
            6 => self.entry_core::<6>(key, make),
            8 => self.entry_core::<8>(key, make),
            _ => self.entry_dyn_hashing(key, make),
        }
    }

    /// The runtime-arity fallback's hashing shell, deliberately
    /// outlined: exotic widths only — a `bl` here is the cold
    /// arm, and keeping `hash_words` inside it keeps the hot sink
    /// symbols free of hash calls (the check-asm gate).
    #[cold]
    #[inline(never)]
    fn entry_dyn_hashing(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        self.entry_dyn(key, hash_words(key), make)
    }

    /// The monomorphic entry: hash and probe with the key width fixed at
    /// K, so the hash unrolls and fuses with the gather (measured).
    #[inline(always)]
    fn entry_core<const K: usize>(
        &mut self,
        key: &[u64],
        make: impl FnOnce() -> V,
    ) -> (&mut V, bool) {
        let hash = hash_core::<K>(key);
        self.entry_hashed_core::<K>(key, hash, make)
    }

    /// The monomorphic insert core: K straight-line word compares, K
    /// stores, strength-reduced `idx * K` slab indexing.
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
            self.set_ctrl(idx, tag(hash));
            self.keys[idx * K..idx * K + K].copy_from_slice(&key[..K]);
            self.values[idx].write(make());
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            self.len += 1;
        }
        // SAFETY: the slot's ctrl byte is set (matched or just written),
        // so its value was initialized by the write above or a prior one.
        (unsafe { self.values[idx].assume_init_mut() }, !found)
    }

    /// The runtime-arity fallback for widths without a monomorph — the
    /// general-length body, kept for exotic widths (0, 5, 7, > 8).
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
            self.set_ctrl(idx, tag(hash));
            self.keys[idx * self.arity..(idx + 1) * self.arity].copy_from_slice(key);
            self.values[idx].write(make());
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            self.len += 1;
        }
        // SAFETY: the slot's ctrl byte is set (matched or just written),
        // so its value was initialized by the write above or a prior one.
        (unsafe { self.values[idx].assume_init_mut() }, !found)
    }

    /// Whether `key` was newly inserted (a set-flavored helper).
    #[inline(always)]
    pub fn insert(&mut self, key: &[u64]) -> bool
    where
        V: Default,
    {
        self.get_or_insert_with(key, V::default).1
    }
}
