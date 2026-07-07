//! An open-addressed map over inline u64 word tuples (docs/architecture/30-execution.md): the sink
//! machinery's seen-sets and group maps. Rebuilt by docs/perf/ PRD 06 as
//! a tag-byte-controlled single-probe-line map: a control byte per slot
//! (0 = empty, else `0x80 | top-7-hash-bits`) means a linear-probe step
//! usually touches ONE ctrl line, key words load only on a tag match
//! (~1/128 of collisions falsely), and values are uninitialized until
//! occupied — no `Option` in the slot array. Growth stays rehash-double
//! at 50% load with insertion order preserved (the dense rule: iteration
//! *and clearing* walk `O(len)`, never `O(capacity)`).
//!
#![allow(unsafe_code)] // 00-product unsafe policy: this module is allowlisted
//! `unsafe` per the 00-product policy (this module is allowlisted): the
//! `MaybeUninit` reads are gated by ctrl-byte occupancy, and the probe
//! indices are masked to the power-of-two capacity — both invariants
//! stated at the sites. `V: Copy` keeps the uninitialized-slot story
//! drop-free (both users store `()` and `usize`).

use std::mem::MaybeUninit;

/// Fixed-arity word-tuple keys mapping to `V`. No tombstones (insert-only).
#[derive(Debug)]
pub struct WordMap<V> {
    arity: usize,
    /// One control byte per slot: 0 = empty, else `0x80 | tag7(hash)`.
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

/// The 7-bit hash tag a ctrl byte carries (bit 7 marks occupancy).
fn tag(hash: u64) -> u8 {
    0x80 | u8::try_from(hash >> 57).expect("7 bits")
}

/// The presizing clamp (docs/perf/ PRD 06): hints are planner estimates —
/// trusted enough to kill rehash storms, capped so a wild estimate cannot
/// balloon a sink.
const HINT_CAP: usize = 1 << 21;

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
    #[must_use]
    pub fn with_capacity_hint(arity: usize, hint: usize) -> Self {
        let mut map = Self::new(arity);
        let capacity = (hint.clamp(8, HINT_CAP) * 2).next_power_of_two();
        map.allocate(capacity);
        map
    }

    fn allocate(&mut self, capacity: usize) {
        self.ctrl = vec![0; capacity];
        self.keys = vec![0; capacity * self.arity];
        self.values = std::iter::repeat_with(MaybeUninit::uninit)
            .take(capacity)
            .collect();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Empties the map, retaining capacity (the zero-alloc reuse path).
    /// O(occupied): only the dense-listed slots are touched. `V: Copy`
    /// makes dropped values a non-event.
    pub fn clear(&mut self) {
        for &idx in &self.dense {
            self.ctrl[idx as usize] = 0;
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
    pub fn get_or_insert_with(&mut self, key: &[u64], make: impl FnOnce() -> V) -> (&mut V, bool) {
        assert_eq!(key.len(), self.arity);
        if (self.len + 1) * 2 > self.values.len() {
            self.grow();
        }
        let hash = hash_words(key);
        let (found, idx) = self.probe(key, hash);
        if !found {
            self.ctrl[idx] = tag(hash);
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
    pub fn insert(&mut self, key: &[u64]) -> bool
    where
        V: Default,
    {
        self.get_or_insert_with(key, V::default).1
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

    /// Slot index for `key`: the match, or the empty slot to fill. The
    /// ctrl byte gates the key compare — a mismatched tag steps without
    /// touching the key slab (~127/128 of collisions).
    fn probe(&self, key: &[u64], hash: u64) -> (bool, usize) {
        debug_assert!(!self.values.is_empty());
        let mask = self.values.len() - 1;
        let wanted = tag(hash);
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {
            let c = self.ctrl[idx];
            if c == 0 {
                return (false, idx);
            }
            if c == wanted && &self.keys[idx * self.arity..(idx + 1) * self.arity] == key {
                return (true, idx);
            }
            idx = (idx + 1) & mask;
        }
    }

    fn grow(&mut self) {
        let new_capacity = (self.values.len() * 2).max(8);
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
        self.ctrl = vec![0; new_capacity];
        // Re-probe in dense (insertion) order so iteration order — and
        // with it every downstream determinism property — survives
        // growth. All keys are distinct; the probe lands on empties. The
        // dense list is rewritten in place: a rehash never changes the
        // entry count, so the buffer it has is the buffer it needs — no
        // fresh allocation, ever.
        for i in 0..self.dense.len() {
            let old_idx = self.dense[i] as usize;
            let key = &old_keys[old_idx * self.arity..(old_idx + 1) * self.arity];
            let hash = hash_words(key);
            let (found, new_idx) = self.probe(key, hash);
            debug_assert!(!found, "rehashed keys are distinct");
            self.ctrl[new_idx] = tag(hash);
            self.keys[new_idx * self.arity..(new_idx + 1) * self.arity].copy_from_slice(key);
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

    /// PRD 06 (docs/perf/): the tag-byte map is behavior-identical to a
    /// reference model (`HashMap` + insertion-order list) across randomized
    /// operation sequences — inserted flags, values, iteration order,
    /// lengths — including growth boundaries, adversarial equal-low-bits
    /// keys, clear cycles, and every arity in use.
    #[test]
    fn differential_against_the_reference_model() {
        let mut rng = 0x2468_ACE0_1357_9BDFu64;
        let mut next = move || {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            rng >> 33
        };
        for arity in [1usize, 2, 4] {
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

    /// PRD 06's presizing gate: a hint covering the workload means ZERO
    /// growth — the map allocated once and never rehashed.
    #[test]
    fn a_covering_hint_never_grows() {
        let mut map: WordMap<()> = WordMap::with_capacity_hint(2, 100_000);
        let capacity = map.values.len();
        for i in 0..100_000u64 {
            map.insert(&[i, i ^ 0x5555]);
        }
        assert_eq!(map.len(), 100_000);
        assert_eq!(map.values.len(), capacity, "no rehash under the hint");
    }

    /// Probe-step evidence for the Result section: average probe steps on
    /// a 50%-loaded map stay near 1 (the ctrl byte absorbs collisions).
    #[test]
    fn probe_steps_stay_near_one_at_half_load() {
        let mut map: WordMap<()> = WordMap::with_capacity_hint(2, 32_768);
        let mut rng = 7u64;
        let mut next = move || {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            rng >> 16
        };
        for _ in 0..32_768 {
            map.insert(&[next(), next()]);
        }
        // Measure probes for hits over every key.
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
        println!("avg probe steps at ~50% load: {avg:.3}");
        assert!(avg < 1.6, "near-one probe steps, got {avg}");
    }
}
