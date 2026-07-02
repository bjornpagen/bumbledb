//! An open-addressed map over inline u64 word tuples (PRD 20): the sink
//! machinery's seen-sets and group maps — the same open-addressing/
//! linear-probing/pow2 pattern as COLT's forced maps (PRD 18), grown by
//! rehash because sink state scales with output, which is unknown upfront.

/// Fixed-arity word-tuple keys mapping to `V`. No tombstones (insert-only).
#[derive(Debug)]
pub struct WordMap<V> {
    arity: usize,
    /// `capacity * arity` key words.
    keys: Vec<u64>,
    values: Vec<Option<V>>,
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
            len: 0,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
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

    /// Iterates `(key words, value)` in storage order (unordered).
    pub fn iter(&self) -> impl Iterator<Item = (&[u64], &V)> {
        self.values.iter().enumerate().filter_map(|(idx, v)| {
            v.as_ref()
                .map(|value| (&self.keys[idx * self.arity..(idx + 1) * self.arity], value))
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
        let old_keys = std::mem::replace(&mut self.keys, vec![0; new_capacity * self.arity]);
        let old_values = std::mem::replace(
            &mut self.values,
            std::iter::repeat_with(|| None).take(new_capacity).collect(),
        );
        for (idx, value) in old_values.into_iter().enumerate() {
            if let Some(value) = value {
                let key = &old_keys[idx * self.arity..(idx + 1) * self.arity];
                let new_idx = self.probe(key);
                self.keys[new_idx * self.arity..(new_idx + 1) * self.arity].copy_from_slice(key);
                self.values[new_idx] = Some(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
