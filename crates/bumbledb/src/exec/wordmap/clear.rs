use super::WordMap;

impl<V: Copy> WordMap<V> {
    /// Empties the map, retaining capacity (the zero-alloc reuse path).
    /// The **generation-stamped slot clear**: one counter bump makes
    /// every occupied slot stale at once — `O(1)`, no ctrl walk — so a
    /// hot execution's multi-million-entry high-water costs the next
    /// execution's reset nothing (the former dense walk was 3.4 ms at
    /// 4.16M entries, every warm execute). Stale slots probe as empties
    /// and inserts reclaim them (`probe.rs`/`entry.rs`), so the warm
    /// same-universe workload re-lands on its old slots and the stale
    /// count drains back toward zero. Two conditions force the one
    /// physical memset instead: the u8 stamp wrap (a stamp value must
    /// never be reused while ctrl bytes from its era survive — the
    /// ghost-key hazard) and stale saturation past half the capacity
    /// (set-but-dead ctrl bytes thin the empties that terminate miss
    /// walks). `V: Copy` makes dropped values a non-event either way.
    pub fn clear(&mut self) {
        self.stale += self.len;
        self.dense.clear();
        self.len = 0;
        if self.stale == 0 {
            return;
        }
        if self.generation == u8::MAX || self.stale * 2 > self.capacity() {
            self.ctrl.fill(0);
            self.generation = 0;
            self.stale = 0;
        } else {
            self.generation += 1;
        }
    }

    /// Iterates `(key words, value)` in insertion order — O(len) via the
    /// dense list, whatever the capacity.
    pub fn iter(&self) -> impl Iterator<Item = (&[u64], &V)> {
        self.iter_since(0)
    }

    /// The dense insertion-order **suffix**: `(key words, value, dense
    /// index)` for every entry inserted at or after `since` — the
    /// frontier watermark's one hook (`docs/architecture/40-execution.md`
    /// § the linear reach driver):
    /// insertion order is preserved across growth (the dense rule), so
    /// round r's frontier is exactly the entries in `[watermark, len)`.
    /// A cold reader — no flag, no branch, no state on the emit path; a
    /// non-recursive program cannot observe it.
    pub fn iter_since(&self, since: usize) -> impl Iterator<Item = (&[u64], &V)> {
        self.dense[since.min(self.dense.len())..]
            .iter()
            .map(move |&idx| {
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
