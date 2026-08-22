use super::WordMap;

impl<V: Copy> WordMap<V> {
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

    pub fn iter(&self) -> impl Iterator<Item = (&[u64], &V)> {
        self.iter_since(0)
    }

    pub fn iter_since(&self, since: usize) -> impl Iterator<Item = (&[u64], &V)> {
        self.dense[since.min(self.dense.len())..]
            .iter()
            .map(move |&idx| {
                let idx = idx as usize;
                debug_assert_ne!(self.ctrl[idx], 0, "dense entries are occupied");
                (
                    &self.keys[idx * self.arity..(idx + 1) * self.arity],
                    // SAFETY: dense lists only occupied slots; occupied slots
                    unsafe { self.values[idx].assume_init_ref() },
                )
            })
    }
}
