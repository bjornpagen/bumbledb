//! An open-addressed map over inline u64 word tuples (docs/architecture/30-execution.md): the sink
//! machinery's seen-sets and group maps — the same open-addressing/
//! linear-probing/pow2 pattern as COLT's forced maps (docs/architecture/30-execution.md), grown by
//! rehash because sink state scales with output, which is unknown upfront.

/// Fixed-arity word-tuple keys mapping to `V`. No tombstones (insert-only).
#[derive(Debug)]
pub struct WordMap<V> {
    arity: usize,
    /// `capacity * arity` key words.
    keys: Vec<u64>,
    values: Vec<Option<V>>,
    /// Occupied slot indices in insertion order — docs/architecture/30-execution.md dense
    /// rule, extended to the sink maps: iteration *and clearing* walk
    /// O(len), never O(capacity), so one hot execution's high-water
    /// cannot tax every later execution's finalize and reset (the
    /// traced 49.9 µs `fk_walk` finalize over 109 rows).
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

impl<V> WordMap<V> {
    /// An empty map for keys of `arity` words (zero arity is legal: every
    /// key is the empty tuple — the global-aggregate group).
    #[must_use]
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            keys: Vec::new(),
            values: Vec::new(),
            dense: Vec::new(),
            len: 0,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Empties the map, retaining capacity (the zero-alloc reuse path).
    /// O(occupied): only the dense-listed slots are touched.
    pub fn clear(&mut self) {
        for &idx in &self.dense {
            self.values[idx as usize] = None;
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
        let idx = self.probe(key);
        let inserted = self.values[idx].is_none();
        if inserted {
            self.keys[idx * self.arity..(idx + 1) * self.arity].copy_from_slice(key);
            self.values[idx] = Some(make());
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            self.len += 1;
        }
        (
            self.values[idx].as_mut().expect("occupied or just filled"),
            inserted,
        )
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
            (
                &self.keys[idx * self.arity..(idx + 1) * self.arity],
                self.values[idx]
                    .as_ref()
                    .expect("dense entries are occupied"),
            )
        })
    }

    /// Slot index for `key`: the match, or the empty slot to fill.
    fn probe(&self, key: &[u64]) -> usize {
        debug_assert!(!self.values.is_empty());
        let mask = self.values.len() - 1;
        let mut idx = usize::try_from(hash_words(key)).expect("64-bit usize") & mask;
        loop {
            if self.values[idx].is_none()
                || &self.keys[idx * self.arity..(idx + 1) * self.arity] == key
            {
                return idx;
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
        let mut old_values = std::mem::replace(
            &mut self.values,
            std::iter::repeat_with(|| None).take(new_capacity).collect(),
        );
        // Re-probe in dense (insertion) order so iteration order — and
        // with it every downstream determinism property — survives
        // growth. All keys are distinct; the probe lands on empties. The
        // dense list is rewritten in place: a rehash never changes the
        // entry count, so the buffer it has is the buffer it needs — no
        // fresh allocation, ever.
        for i in 0..self.dense.len() {
            let old_idx = self.dense[i] as usize;
            let key = &old_keys[old_idx * self.arity..(old_idx + 1) * self.arity];
            let new_idx = self.probe(key);
            self.keys[new_idx * self.arity..(new_idx + 1) * self.arity].copy_from_slice(key);
            self.values[new_idx] = old_values[old_idx].take();
            self.dense[i] = u32::try_from(new_idx).expect("slot index fits u32");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
