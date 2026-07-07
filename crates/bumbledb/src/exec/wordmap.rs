//! An open-addressed map over inline u64 word tuples (docs/architecture/30-execution.md): the sink
//! machinery's seen-sets and group maps. Rebuilt by docs/perf/ PRD 06 as
//! a tag-byte-controlled single-probe-line map: a control byte per slot
//! (0 = empty, else `0x80 | top-7-hash-bits`) means a probe step
//! usually touches ONE ctrl line, key words load only on a tag match
//! (~1/128 of collisions falsely), and values are uninitialized until
//! occupied — no `Option` in the slot array.
//!
//! Geometry and probe shape follow the measured law (docs/silicon/03,
//! bumblebench exps 01/02): these maps are MISS-heavy by construction —
//! a seen-set's first sight of every distinct key is a miss — and misses
//! cost more than hits in open addressing (walk length plus a
//! mispredicted exit branch). Two consequences, built in:
//!
//! - **33% max load** (was 50%): dropping load factor shortens the walks
//!   that misses pay for (measured miss cost fell 9.2 → 2.8 ns between
//!   38% and 5% load); the {50, 33, 25}% ledger sweep picked 33% —
//!   most of the walk win at 1.5× the memory.
//! - **Branchless window probing**: the ctrl bytes are scanned eight at
//!   a time with SWAR masks — one well-predicted exit branch per window
//!   instead of one branch per slot (measured 4.6× at hit-rate 0). The
//!   ctrl slab carries a `WINDOW-1`-byte mirror of its first bytes so a
//!   window read never wraps.
//!
//! Growth stays rehash-double with insertion order preserved (the dense
//! rule: iteration *and clearing* walk `O(len)`, never `O(capacity)`).
//!
#![allow(unsafe_code)] // 00-product unsafe policy: this module is allowlisted
#![allow(clippy::inline_always)] // docs/silicon/03/04: the probe path's
// inlining is load-bearing (per-element call ceremony was measured cost)
// and machine-checked by scripts/check-asm.sh, not trusted to attributes.
//! `unsafe` per the 00-product policy (this module is allowlisted): the
//! `MaybeUninit` reads are gated by ctrl-byte occupancy, and the probe
//! indices are masked to the power-of-two capacity — both invariants
//! stated at the sites. `V: Copy` keeps the uninitialized-slot story
//! drop-free (both users store `()` and `usize`).

use std::mem::MaybeUninit;

/// Ctrl bytes scanned per probe step (one SWAR word).
const WINDOW: usize = 8;

/// Fixed-arity word-tuple keys mapping to `V`. No tombstones (insert-only).
#[derive(Debug)]
pub struct WordMap<V> {
    arity: usize,
    /// One control byte per slot (0 = empty, else `0x80 | tag7(hash)`),
    /// plus a `WINDOW - 1`-byte mirror of the first bytes at the tail so
    /// window loads never wrap (`ctrl.len() == capacity + WINDOW - 1`).
    ctrl: Vec<u8>,
    /// `capacity * arity` key words.
    keys: Vec<u64>,
    /// One value per slot, initialized exactly when its ctrl byte is set.
    values: Vec<MaybeUninit<V>>,
    /// Occupied slot indices in insertion order — docs/architecture/30-execution.md dense
    /// rule, extended to the sink maps: iteration *and clearing* walk
    /// O(len), never O(capacity), so one hot execution's high-water
    /// cannot tax every later execution's finalize and reset.
    dense: Vec<u32>,
    len: usize,
}

fn hash_words(words: &[u64]) -> u64 {
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for w in words {
        h ^= *w;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    h
}

/// [`hash_words`] with the word count fixed at compile time — same
/// seed, same fold order, same constants, so the two forms are
/// hash-identical (pinned by test). Under const K, LLVM fully unrolls
/// the fold, hoists prefix hashes of batch-constant words out of the
/// caller's row loop, and fuses the key gather with the hash — the
/// free transformations runtime arity blocks (docs/silicon2/03,
/// exp 15: hand-fused variants measured redundant or worse).
#[inline(always)]
fn hash_core<const K: usize>(words: &[u64]) -> u64 {
    debug_assert_eq!(words.len(), K);
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for i in 0..K {
        h ^= words[i];
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    h
}

/// The probe hash for a key — public so callers can software-pipeline
/// the hash ahead of the probe's branches (docs/silicon/04: the
/// probe-exit mispredict flush kills a speculated next hash chain;
/// computing h(i+1) before probe(i) recovers 60–65% of that exposure).
#[must_use]
#[inline(always)]
#[allow(dead_code)] // the prehashed seam (docs/silicon2/02: kept for 03's
// const-arity internals; only test callers remain — PRD 10's audit
// ledger rules on it)
pub fn hash_of(key: &[u64]) -> u64 {
    hash_words(key)
}

/// The 7-bit hash tag a ctrl byte carries (bit 7 marks occupancy).
fn tag(hash: u64) -> u8 {
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

/// The presizing clamp (docs/perf/ PRD 06): hints are planner estimates —
/// trusted enough to kill rehash storms, capped so a wild estimate cannot
/// balloon a sink.
const HINT_CAP: usize = 1 << 21;

/// Max load as `len × LOAD_DEN ≤ capacity` — 3 = 33% (docs/silicon/03,
/// justified by the {50, 33, 25}% family-ledger sweep recorded in that
/// PRD's Result: 50% loses badly on spread (+28%), 25% costs triangle
/// +7%; 33% is best-or-near-best everywhere. Misses pay for walks, and
/// these maps are miss-heavy).
const LOAD_DEN: usize = 3;

impl<V: Copy> WordMap<V> {
    /// An empty map for keys of `arity` words (zero arity is legal: every
    /// key is the empty tuple — the global-aggregate group).
    #[must_use]
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            ctrl: Vec::new(),
            keys: Vec::new(),
            values: Vec::new(),
            dense: Vec::new(),
            len: 0,
        }
    }

    /// An empty map presized for ~`hint` entries (docs/perf/ PRD 06): one
    /// allocation up front instead of a rehash ladder inside the first
    /// measured execution. The map still grows if the hint was short.
    /// Sizing covers the hint at the shipped max load (docs/silicon/03).
    #[must_use]
    pub fn with_capacity_hint(arity: usize, hint: usize) -> Self {
        let mut map = Self::new(arity);
        let capacity = (hint.clamp(2, HINT_CAP) * LOAD_DEN).next_power_of_two();
        map.allocate(capacity);
        map
    }

    fn allocate(&mut self, capacity: usize) {
        debug_assert!(capacity.is_power_of_two() && capacity >= WINDOW);
        self.ctrl = vec![0; capacity + WINDOW - 1];
        self.keys = vec![0; capacity * self.arity];
        self.values = std::iter::repeat_with(MaybeUninit::uninit)
            .take(capacity)
            .collect();
    }

    /// The slot capacity (`values.len()`; ctrl carries the mirror tail).
    #[inline(always)]
    fn capacity(&self) -> usize {
        self.values.len()
    }

    /// Writes one ctrl byte, mirroring the head bytes into the tail so
    /// window loads never wrap.
    #[inline(always)]
    fn set_ctrl(&mut self, idx: usize, value: u8) {
        self.ctrl[idx] = value;
        if idx < WINDOW - 1 {
            let capacity = self.capacity();
            self.ctrl[capacity + idx] = value;
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Empties the map, retaining capacity (the zero-alloc reuse path).
    /// O(occupied): only the dense-listed slots are touched. `V: Copy`
    /// makes dropped values a non-event.
    pub fn clear(&mut self) {
        let capacity = self.capacity();
        for i in 0..self.dense.len() {
            let idx = self.dense[i] as usize;
            self.ctrl[idx] = 0;
            if idx < WINDOW - 1 {
                self.ctrl[capacity + idx] = 0;
            }
        }
        self.dense.clear();
        self.len = 0;
    }

    /// Gets the value for `key`, inserting `make()` when absent. Returns
    /// `(value, inserted)`.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: `key.len() != arity`.
    #[inline(always)] // the whole chain inlines into the sink row loops so
    // LLVM can hoist batch-constant prefix hashes out of them (exp 15)
    pub fn get_or_insert_with(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        assert_eq!(key.len(), self.arity);
        // The const-arity dispatch (docs/silicon2/03, exp 15): runtime
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

    /// The runtime-arity fallback's hashing shell, deliberately outlined
    /// (docs/silicon2/03): exotic widths only — a `bl` here is the cold
    /// arm, and keeping `hash_words` inside it keeps the hot sink
    /// symbols free of hash calls (the check-asm gate).
    #[cold]
    #[inline(never)]
    fn entry_dyn_hashing(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        self.entry_dyn(key, hash_words(key), make)
    }

    /// The monomorphic entry: hash and probe with the key width fixed at
    /// K, so the hash unrolls and fuses with the gather (exp 15).
    #[inline(always)]
    fn entry_core<const K: usize>(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        let hash = hash_core::<K>(key);
        self.entry_hashed_core::<K>(key, hash, make)
    }

    /// [`WordMap::get_or_insert_with`] with the hash supplied by the
    /// caller — the hash-ahead seam (docs/silicon/04). The hash MUST be
    /// [`hash_of`] of `key`; a mismatched hash is a logic error that
    /// corrupts nothing but finds nothing.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: `key.len() != arity`.
    #[inline(always)]
    pub fn get_or_insert_prehashed(
        &mut self,
        key: &[u64],
        hash: u64,
        make: impl FnOnce() -> V,
    ) -> (&mut V, bool) {
        assert_eq!(key.len(), self.arity);
        // Same dispatch as `get_or_insert_with`, hash supplied: the probe,
        // compare, and copy still monomorphize (docs/silicon2/03).
        match self.arity {
            1 => self.entry_hashed_core::<1>(key, hash, make),
            2 => self.entry_hashed_core::<2>(key, hash, make),
            3 => self.entry_hashed_core::<3>(key, hash, make),
            4 => self.entry_hashed_core::<4>(key, hash, make),
            6 => self.entry_hashed_core::<6>(key, hash, make),
            8 => self.entry_hashed_core::<8>(key, hash, make),
            _ => self.entry_dyn(key, hash, make),
        }
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
    /// pre-silicon2/03 body, kept for exotic widths (0, 5, 7, > 8).
    fn entry_dyn(&mut self, key: &[u64], hash: u64, make: impl FnOnce() -> V) -> (&mut V, bool) {
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

    /// [`WordMap::insert`] with the hash supplied by the caller (the
    /// hash-ahead seam, docs/silicon/04).
    #[allow(dead_code)] // kept with the seam (docs/silicon2/02); PRD 10 rules
    pub fn insert_prehashed(&mut self, key: &[u64], hash: u64) -> bool
    where
        V: Default,
    {
        self.get_or_insert_prehashed(key, hash, V::default).1
    }

    /// Iterates `(key words, value)` in insertion order — O(len) via the
    /// dense list, whatever the capacity.
    pub fn iter(&self) -> impl Iterator<Item = (&[u64], &V)> {
        self.dense.iter().map(move |&idx| {
            let idx = idx as usize;
            debug_assert_ne!(self.ctrl[idx], 0, "dense entries are occupied");
            (
                &self.keys[idx * self.arity..(idx + 1) * self.arity],
                // SAFETY: dense lists only occupied slots; occupied slots
                // were initialized at insert and survive rehash by copy.
                unsafe { self.values[idx].assume_init_ref() },
            )
        })
    }

    /// Whether the stored key at `slot` equals `key` — a manual word
    /// loop: a runtime-length slice compare here compiles to a `bcmp`
    /// call inside the probe loop (docs/silicon/02's law, same as colt).
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
    /// unrolls to K straight-line compares, `slot * K` strength-reduces
    /// (docs/silicon2/03).
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
    /// Branchless window scan (docs/silicon/03): eight ctrl bytes load as
    /// one word; SWAR masks mark empties and tag matches; candidates
    /// resolve in slot order. One well-predicted exit branch per window
    /// replaces one branch per slot — the measured 4.6× at hit-rate 0,
    /// which is the seen-set's steady state.
    fn probe(&self, key: &[u64], hash: u64) -> (bool, usize) {
        self.probe_with(hash, |slot| self.key_at_matches(slot, key))
    }

    /// [`WordMap::probe`] with the key width fixed at K — the compare is
    /// K straight-line words (docs/silicon2/03).
    #[inline(always)]
    fn probe_core<const K: usize>(&self, key: &[u64], hash: u64) -> (bool, usize) {
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

    fn grow(&mut self) {
        let new_capacity = (self.capacity() * 2).max(WINDOW);
        // Rehash visibility (docs/architecture/50-validation.md): sink-map
        // growth inside a measured execution is exactly the presizing
        // opportunity the trace should surface.
        crate::obs::event(
            crate::obs::names::WORDMAP_GROW,
            crate::obs::Category::Execute,
            new_capacity as u64,
            self.arity as u64,
        );
        let old_keys = std::mem::replace(&mut self.keys, vec![0; new_capacity * self.arity]);
        let old_values = std::mem::replace(
            &mut self.values,
            std::iter::repeat_with(MaybeUninit::uninit)
                .take(new_capacity)
                .collect(),
        );
        self.ctrl = vec![0; new_capacity + WINDOW - 1];
        // The rehash re-probes every key, so it rides the same const-
        // arity dispatch as the entry points (docs/silicon2/03).
        match self.arity {
            1 => self.rehash_core::<1>(&old_keys, &old_values),
            2 => self.rehash_core::<2>(&old_keys, &old_values),
            3 => self.rehash_core::<3>(&old_keys, &old_values),
            4 => self.rehash_core::<4>(&old_keys, &old_values),
            6 => self.rehash_core::<6>(&old_keys, &old_values),
            8 => self.rehash_core::<8>(&old_keys, &old_values),
            _ => self.rehash_dyn(&old_keys, &old_values),
        }
    }

    /// The rehash body with the key width fixed at K. Re-probes in dense
    /// (insertion) order so iteration order — and with it every
    /// downstream determinism property — survives growth. All keys are
    /// distinct; the probe lands on empties. The dense list is rewritten
    /// in place: a rehash never changes the entry count, so the buffer it
    /// has is the buffer it needs — no fresh allocation, ever.
    fn rehash_core<const K: usize>(&mut self, old_keys: &[u64], old_values: &[MaybeUninit<V>]) {
        for i in 0..self.dense.len() {
            let old_idx = self.dense[i] as usize;
            let key = &old_keys[old_idx * K..old_idx * K + K];
            let hash = hash_core::<K>(key);
            let (found, new_idx) = self.probe_core::<K>(key, hash);
            debug_assert!(!found, "rehashed keys are distinct");
            self.set_ctrl(new_idx, tag(hash));
            self.keys[new_idx * K..new_idx * K + K].copy_from_slice(key);
            // SAFETY: old_idx was occupied (dense-listed), so its value
            // was initialized; the copy moves it to the new slot.
            self.values[new_idx].write(unsafe { old_values[old_idx].assume_init_read() });
            self.dense[i] = u32::try_from(new_idx).expect("slot index fits u32");
        }
    }

    /// [`WordMap::rehash_core`], runtime-arity form (the dyn widths).
    fn rehash_dyn(&mut self, old_keys: &[u64], old_values: &[MaybeUninit<V>]) {
        for i in 0..self.dense.len() {
            let old_idx = self.dense[i] as usize;
            let key_range = old_idx * self.arity..(old_idx + 1) * self.arity;
            let hash = hash_words(&old_keys[key_range.clone()]);
            let (found, new_idx) = self.probe(&old_keys[key_range.clone()], hash);
            debug_assert!(!found, "rehashed keys are distinct");
            self.set_ctrl(new_idx, tag(hash));
            self.keys[new_idx * self.arity..(new_idx + 1) * self.arity]
                .copy_from_slice(&old_keys[key_range]);
            // SAFETY: old_idx was occupied (dense-listed), so its value
            // was initialized; the copy moves it to the new slot.
            self.values[new_idx].write(unsafe { old_values[old_idx].assume_init_read() });
            self.dense[i] = u32::try_from(new_idx).expect("slot index fits u32");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The dense rule (docs/architecture/30-execution.md, extended to sink maps): after a hot
    /// execution inflates capacity, iteration and clearing stay O(len) —
    /// pinned structurally by insertion-order iteration over a
    /// high-water map.
    #[test]
    fn iteration_is_dense_and_insertion_ordered_after_high_water() {
        let mut map: WordMap<u64> = WordMap::new(1);
        // The hot execution: 50k entries inflate capacity.
        for i in 0..50_000u64 {
            map.get_or_insert_with(&[i], || i);
        }
        assert_eq!(map.len(), 50_000);
        // The cold execution: clear (O(occupied)) then a handful.
        map.clear();
        assert_eq!(map.len(), 0);
        assert_eq!(map.iter().count(), 0, "cleared maps iterate nothing");
        for i in [7u64, 3, 9] {
            map.get_or_insert_with(&[i], || i * 10);
        }
        let entries: Vec<(u64, u64)> = map.iter().map(|(k, v)| (k[0], *v)).collect();
        assert_eq!(
            entries,
            vec![(7, 70), (3, 30), (9, 90)],
            "exactly the occupied entries, in insertion order"
        );
        // Growth preserves insertion order (re-probed via the dense list).
        let mut grown: WordMap<()> = WordMap::new(1);
        for i in (0..100u64).rev() {
            grown.insert(&[i]);
        }
        let order: Vec<u64> = grown.iter().map(|(k, ())| k[0]).collect();
        assert_eq!(order, (0..100u64).rev().collect::<Vec<_>>());
    }

    #[test]
    fn insert_dedups_and_survives_rehash() {
        let mut map: WordMap<()> = WordMap::new(2);
        for i in 0..100u64 {
            assert!(map.insert(&[i, i * 2]));
            assert!(!map.insert(&[i, i * 2]));
        }
        assert_eq!(map.len(), 100);
        let mut seen: Vec<u64> = map.iter().map(|(k, ())| k[0]).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..100).collect::<Vec<u64>>());
    }

    #[test]
    fn values_accumulate_through_get_or_insert() {
        let mut map: WordMap<u64> = WordMap::new(1);
        for i in 0..30u64 {
            let (value, _) = map.get_or_insert_with(&[i % 3], || 0);
            *value += i;
        }
        let mut totals: Vec<(u64, u64)> = map.iter().map(|(k, v)| (k[0], *v)).collect();
        totals.sort_unstable();
        // Sum of 0..30 split by i % 3.
        assert_eq!(totals, vec![(0, 135), (1, 145), (2, 155)]);
    }

    /// PRD 04 (docs/hardening): a rehash never changes the entry count,
    /// so `grow` rewrites the dense list in place — same buffer, same
    /// capacity, insertion order and values intact.
    #[test]
    fn grow_rewrites_the_dense_list_in_place() {
        let mut map: WordMap<u64> = WordMap::new(1);
        for i in 0..20u64 {
            map.get_or_insert_with(&[i], || i * 3);
        }
        let ptr = map.dense.as_ptr();
        let capacity = map.dense.capacity();
        map.grow();
        assert_eq!(map.dense.as_ptr(), ptr, "grow re-allocated the dense list");
        assert_eq!(map.dense.capacity(), capacity);
        assert_eq!(map.len(), 20);
        let keys: Vec<u64> = map.iter().map(|(k, _)| k[0]).collect();
        assert_eq!(
            keys,
            (0..20).collect::<Vec<u64>>(),
            "insertion order survives"
        );
        for i in 0..20u64 {
            let (value, inserted) = map.get_or_insert_with(&[i], || 0);
            assert!(!inserted);
            assert_eq!(*value, i * 3, "values survive the rehash");
        }
    }

    #[test]
    fn zero_arity_keys_share_one_group() {
        let mut map: WordMap<u64> = WordMap::new(0);
        for _ in 0..5 {
            let (value, _) = map.get_or_insert_with(&[], || 0);
            *value += 1;
        }
        assert_eq!(map.len(), 1);
        assert_eq!(map.iter().next().map(|(k, v)| (k.len(), *v)), Some((0, 5)));
    }

    /// The prehashed seam (docs/silicon/04) is behavior-identical to the
    /// hashing form — same slots, same insertion flags — and a wrong
    /// hash finds nothing (stated contract).
    #[test]
    fn prehashed_inserts_match_the_hashing_form() {
        let mut a: WordMap<u64> = WordMap::new(2);
        let mut b: WordMap<u64> = WordMap::new(2);
        for i in 0..1_000u64 {
            let key = [i % 97, i % 13];
            let (va, ia) = a.get_or_insert_with(&key, || i);
            let (vb, ib) = b.get_or_insert_prehashed(&key, hash_of(&key), || i);
            assert_eq!((*va, ia), (*vb, ib), "key {key:?}");
        }
        assert_eq!(a.len(), b.len());
        let ka: Vec<Vec<u64>> = a.iter().map(|(k, _)| k.to_vec()).collect();
        let kb: Vec<Vec<u64>> = b.iter().map(|(k, _)| k.to_vec()).collect();
        assert_eq!(ka, kb, "identical insertion order");
    }

    /// PRD 06 (docs/perf/): the tag-byte map is behavior-identical to a
    /// reference model (`HashMap` + insertion-order list) across randomized
    /// operation sequences — inserted flags, values, iteration order,
    /// lengths — including growth boundaries, adversarial equal-low-bits
    /// keys, clear cycles, and every arity in use: all six monomorph
    /// widths plus a dyn-arm width (5) per docs/silicon2/03. The window
    /// probe (docs/silicon/03) rides the same differential: the
    /// reference IS the portable implementation of record.
    #[test]
    fn differential_against_the_reference_model() {
        let mut rng = 0x2468_ACE0_1357_9BDFu64;
        let mut next = move || {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            rng >> 33
        };
        for arity in [1usize, 2, 3, 4, 5, 6, 8] {
            for round in 0..3 {
                let mut map: WordMap<u64> = if round == 0 {
                    WordMap::new(arity)
                } else {
                    WordMap::with_capacity_hint(arity, 64 << round)
                };
                let mut model: HashMap<Vec<u64>, u64> = HashMap::new();
                let mut order: Vec<Vec<u64>> = Vec::new();
                for op in 0..2_000u64 {
                    // Adversarial low-entropy keys: many collisions and
                    // duplicate inserts; occasional extreme words.
                    let key: Vec<u64> = (0..arity)
                        .map(|_| match next() % 8 {
                            0 => 0,
                            1 => u64::MAX,
                            2 => next() << 32, // equal low bits
                            _ => next() % 64,
                        })
                        .collect();
                    let (value, inserted) = map.get_or_insert_with(&key, || op);
                    match model.get(&key) {
                        None => {
                            assert!(inserted, "model says new");
                            model.insert(key.clone(), op);
                            order.push(key.clone());
                        }
                        Some(existing) => {
                            assert!(!inserted, "model says present");
                            assert_eq!(value, existing, "value survives");
                        }
                    }
                }
                assert_eq!(map.len(), model.len());
                let got: Vec<(Vec<u64>, u64)> = map.iter().map(|(k, v)| (k.to_vec(), *v)).collect();
                let expected: Vec<(Vec<u64>, u64)> =
                    order.iter().map(|k| (k.clone(), model[k])).collect();
                assert_eq!(got, expected, "insertion-order iteration");
                // Clear cycle: capacity retained, behavior fresh.
                map.clear();
                assert_eq!(map.len(), 0);
                assert!(map.insert(&vec![41u64; arity]));
                assert!(!map.insert(&vec![41u64; arity]));
            }
        }
    }

    /// The mirror invariant: ctrl's tail `WINDOW-1` bytes always equal
    /// its head bytes — through inserts, clears, and growth — so window
    /// loads at high indices see the wrapped slots correctly.
    #[test]
    fn the_ctrl_mirror_tracks_the_head() {
        let mut map: WordMap<()> = WordMap::with_capacity_hint(1, 4);
        for i in 0..200u64 {
            map.insert(&[i.wrapping_mul(0x9E37_79B9_7F4A_7C15)]);
            let capacity = map.values.len();
            assert_eq!(
                &map.ctrl[capacity..capacity + WINDOW - 1],
                &map.ctrl[..WINDOW - 1],
                "mirror out of sync after insert {i}"
            );
        }
        map.clear();
        let capacity = map.values.len();
        assert_eq!(
            &map.ctrl[capacity..capacity + WINDOW - 1],
            &map.ctrl[..WINDOW - 1],
            "mirror out of sync after clear"
        );
        assert!(map.ctrl.iter().all(|&c| c == 0), "clear emptied every byte");
    }

    /// PRD 06's presizing gate: a hint covering the workload means ZERO
    /// growth — the map allocated once and never rehashed (now at the
    /// 25% load target, docs/silicon/03).
    #[test]
    fn a_covering_hint_never_grows() {
        let mut map: WordMap<()> = WordMap::with_capacity_hint(2, 100_000);
        let capacity = map.values.len();
        for i in 0..100_000u64 {
            map.insert(&[i, i ^ 0x5555]);
        }
        assert_eq!(map.len(), 100_000);
        assert_eq!(map.values.len(), capacity, "no rehash under the hint");
        assert!(
            map.len() * LOAD_DEN <= capacity,
            "the covered hint keeps load at the shipped max"
        );
    }

    /// The hash-quality contract (docs/silicon/05, bumblebench exp 02):
    /// **false-tag rate — not probe length — is the sensitive quality
    /// metric** for tagged tables. A single-multiply fold hash passes
    /// probe-length vetting (avg 1.40) while collapsing the 7-bit tag to
    /// 19.4% false compares on strided keys (design point 1/128). This
    /// test gates WHATEVER hash the module ships, by property: across
    /// adversarial key families, the measured false-compare rate per
    /// probe must stay ≤ 2/128. The `#[should_panic]` companion proves
    /// the gate's teeth: a plausible cheaper hash fails it.
    #[test]
    fn false_tag_rate_stays_at_the_design_point_on_adversarial_keys() {
        for (name, rate) in adversarial_false_tag_rates(super::hash_words) {
            println!("false-compare rate [{name}]: {rate:.5}");
            assert!(
                rate <= 2.0 / 128.0,
                "family {name}: false-compare rate {rate:.5} above 2/128"
            );
        }
    }

    /// The red case, visible in review: a single-multiply fold hash —
    /// 2× cheaper, passes probe-length vetting — collapses the tag on
    /// low-entropy keys. If a future "optimization" swaps the hash and
    /// this stops panicking, the swap broke the tag and the gate above
    /// will say so.
    #[test]
    #[should_panic(expected = "above 2/128")]
    fn a_single_multiply_hash_fails_the_false_tag_gate() {
        fn foldmul(words: &[u64]) -> u64 {
            let mut h = 0u64;
            for w in words {
                h = (h ^ w).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            }
            h
        }
        for (name, rate) in adversarial_false_tag_rates(foldmul) {
            assert!(
                rate <= 2.0 / 128.0,
                "family {name}: false-compare rate {rate:.5} above 2/128"
            );
        }
    }

    /// Measures the false-compare rate (tag matched, key mismatched, per
    /// probe) of `hash` across the adversarial key families, by walking
    /// a simulated 25%-loaded table exactly as the probe does. The walk
    /// is a model, not the shipped window probe: the metric is a hash
    /// property, independent of probe mechanics.
    fn adversarial_false_tag_rates(hash: fn(&[u64]) -> u64) -> Vec<(&'static str, f64)> {
        let families: Vec<(&'static str, Vec<Vec<u64>>)> = vec![
            ("sequential", (0..16_384u64).map(|i| vec![i]).collect()),
            ("strided-8", (0..16_384u64).map(|i| vec![i * 8]).collect()),
            (
                "strided-4096",
                (0..16_384u64).map(|i| vec![i * 4096]).collect(),
            ),
            (
                "biased-i64-small",
                (0..16_384u64)
                    .map(|i| vec![(1u64 << 63) ^ i.wrapping_sub(8_192)])
                    .collect(),
            ),
            (
                "serial-pairs",
                (0..16_384u64).map(|i| vec![i, i / 64]).collect(),
            ),
            (
                "random-control",
                {
                    let mut rng = 0xDEAD_BEEF_u64;
                    (0..16_384)
                        .map(|_| {
                            rng = rng
                                .wrapping_mul(6_364_136_223_846_793_005)
                                .wrapping_add(1_442_695_040_888_963_407);
                            vec![rng]
                        })
                        .collect()
                },
            ),
        ];
        families
            .into_iter()
            .map(|(name, keys)| {
                let arity = keys[0].len();
                // A 25%-loaded model table: capacity = 4 × keys, linear
                // probing, tag = top-7 bits — the shipped geometry.
                let capacity = (keys.len() * 4).next_power_of_two();
                let mask = capacity - 1;
                let mut slots: Vec<Option<usize>> = vec![None; capacity];
                let mut tags: Vec<u8> = vec![0; capacity];
                for (ki, key) in keys.iter().enumerate() {
                    let h = hash(key);
                    let mut idx = usize::try_from(h).expect("64-bit") & mask;
                    loop {
                        if slots[idx].is_none() {
                            slots[idx] = Some(ki);
                            tags[idx] = super::tag(h);
                            break;
                        }
                        idx = (idx + 1) & mask;
                    }
                }
                // Probe every key (hits) plus an equal count of misses
                // drawn from the same family shape, counting steps where
                // the tag matched but the key did not.
                let mut probes = 0usize;
                let mut false_compares = 0usize;
                let mut probe = |key: &[u64]| {
                    let h = hash(key);
                    let wanted = super::tag(h);
                    let mut idx = usize::try_from(h).expect("64-bit") & mask;
                    probes += 1;
                    loop {
                        match slots[idx] {
                            None => break,
                            Some(ki) => {
                                if tags[idx] == wanted {
                                    if keys[ki].as_slice() == key {
                                        break;
                                    }
                                    false_compares += 1;
                                }
                            }
                        }
                        idx = (idx + 1) & mask;
                    }
                };
                for key in &keys {
                    probe(key);
                }
                for i in 0..keys.len() as u64 {
                    // Same shape, disjoint values (offset far past the family).
                    let miss: Vec<u64> = keys[usize::try_from(i).expect("small") % keys.len()]
                        .iter()
                        .map(|w| w.wrapping_add(0x0100_0000_0000_0000))
                        .collect();
                    debug_assert_eq!(miss.len(), arity);
                    probe(&miss);
                }
                #[allow(clippy::cast_precision_loss)]
                let rate = false_compares as f64 / probes as f64;
                (name, rate)
            })
            .collect()
    }

    /// The const-arity contract (docs/silicon2/03): `hash_core::<K>` is
    /// hash-IDENTICAL to `hash_words` — same seed, fold order, constants
    /// — so the monomorph and dyn arms land keys in the same slots and
    /// the false-tag gate covers both.
    #[test]
    fn hash_core_is_identical_to_hash_words() {
        let mut rng = 0x0F1E_2D3C_4B5A_6978u64;
        let mut next = move || {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            rng
        };
        fn check<const K: usize>(next: &mut impl FnMut() -> u64) {
            for _ in 0..1_000 {
                let key: Vec<u64> = (0..K).map(|_| next()).collect();
                assert_eq!(hash_core::<K>(&key), hash_words(&key), "K={K}");
            }
        }
        check::<1>(&mut next);
        check::<2>(&mut next);
        check::<3>(&mut next);
        check::<4>(&mut next);
        check::<6>(&mut next);
        check::<8>(&mut next);
    }

    /// The const-arity pin (docs/silicon2/03 gate, threshold corrected
    /// in its Result): the K=4 monomorphic insert beats the dyn arm on a
    /// 16 MB miss-heavy fill. Exp 15's 1.9× was measured against its own
    /// dyn reconstruction, which still carried the general-length
    /// compare ladder; the shipped dyn arm was already dieted by the
    /// campaign (manual word loops, no `bcmp`), so the honest in-tree
    /// margin is 1.16–1.25× (16 MB / 2 MB tiers). The pin guards the
    /// MECHANISM — monomorph strictly beats dyn — at a ≥ 10% floor that
    /// survives tier noise. Both arms probe OPAQUE runtime slices (flat
    /// buffer, black-boxed arity) — the shipped sink shape — so the
    /// compiler cannot const-prop the key width into either arm from the
    /// test itself; the monomorph arm's width knowledge comes only from
    /// the internal dispatch. Ignored: a microbenchmark, run explicitly
    /// for the Result section.
    #[test]
    #[ignore = "microbench pin: run explicitly with --ignored"]
    fn const_arity_k4_insert_beats_the_dyn_arm() {
        // 128k arity-4 keys: capacity (128k×3).next_pow2 = 512k slots,
        // 512k × 32 B keys = 16 MiB — the DRAM-tier miss-heavy fill.
        const N: usize = std::hint::black_box(128) * 1024;
        let arity = std::hint::black_box(4usize);
        let flat: Vec<u64> = {
            let mut rng = 0x5DEE_CE66_D42F_1A2Bu64;
            (0..N * arity)
                .map(|_| {
                    rng = rng
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    rng
                })
                .collect()
        };
        let fill_core = |flat: &[u64]| {
            let mut map: WordMap<()> = WordMap::with_capacity_hint(arity, N);
            let start = std::time::Instant::now();
            for i in 0..N {
                map.insert(&flat[i * arity..(i + 1) * arity]);
            }
            let elapsed = start.elapsed();
            assert_eq!(map.len(), N);
            elapsed
        };
        let fill_dyn = |flat: &[u64]| {
            let mut map: WordMap<()> = WordMap::with_capacity_hint(arity, N);
            let start = std::time::Instant::now();
            for i in 0..N {
                let key = &flat[i * arity..(i + 1) * arity];
                let _ = map.entry_dyn(key, hash_words(key), || ());
            }
            let elapsed = start.elapsed();
            assert_eq!(map.len(), N);
            elapsed
        };
        // Interleaved min-of-5 (docs/silicon2/00 doctrine: min-of-N
        // absorbs DVFS and residency noise without a proxy dependency).
        let mut core_best = std::time::Duration::MAX;
        let mut dyn_best = std::time::Duration::MAX;
        for _ in 0..5 {
            core_best = core_best.min(fill_core(&flat));
            dyn_best = dyn_best.min(fill_dyn(&flat));
        }
        let core_ns = u64::try_from(core_best.as_nanos()).expect("fits");
        let dyn_ns = u64::try_from(dyn_best.as_nanos()).expect("fits");
        #[allow(clippy::cast_precision_loss)] // both far below 2^52
        let ratio = dyn_ns as f64 / core_ns as f64;
        println!("const-arity K=4 fill: core {core_ns} ns, dyn {dyn_ns} ns, ratio {ratio:.2}");
        assert!(
            dyn_ns * 10 >= core_ns * 11,
            "K=4 monomorph must beat the dyn arm by ≥ 10%: core {core_ns} ns vs dyn {dyn_ns} ns"
        );
    }

    /// Probe-step evidence for the Result section: average probe steps
    /// at the shipped max load stay near 1 (docs/silicon/03 gate: ≤ 1.2).
    #[test]
    fn probe_steps_stay_near_one_at_max_load() {
        let mut map: WordMap<()> = WordMap::with_capacity_hint(2, 32_768);
        let mut rng = 7u64;
        let mut next = move || {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            rng >> 16
        };
        for _ in 0..32_768 {
            map.insert(&[next(), next()]);
        }
        assert!(
            map.len() * LOAD_DEN <= map.values.len(),
            "the sweep runs at the shipped max load"
        );
        // Measure probes for hits over every key (slot-step model:
        // window loads amortize these steps 8-at-a-time, but the walk
        // length is the portable quality metric).
        let keys: Vec<Vec<u64>> = map.iter().map(|(k, ())| k.to_vec()).collect();
        let mask = map.values.len() - 1;
        let mut steps = 0usize;
        for key in &keys {
            let hash = super::hash_words(key);
            let mut idx = usize::try_from(hash).expect("64-bit") & mask;
            loop {
                steps += 1;
                let c = map.ctrl[idx];
                assert_ne!(c, 0, "key exists");
                if c == super::tag(hash)
                    && &map.keys[idx * map.arity..(idx + 1) * map.arity] == key.as_slice()
                {
                    break;
                }
                idx = (idx + 1) & mask;
            }
        }
        #[allow(clippy::cast_precision_loss)] // both far below 2^52
        let avg = steps as f64 / keys.len() as f64;
        println!("avg probe steps at the shipped max load: {avg:.3}");
        assert!(avg <= 1.2, "near-one probe steps at 25% load, got {avg}");
    }
}
