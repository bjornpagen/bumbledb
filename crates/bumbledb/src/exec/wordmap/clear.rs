use super::{WordMap, WINDOW};

impl<V: Copy> WordMap<V> {
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
}
