use super::WordMap;

impl<V: Copy> WordMap<V> {
    pub fn clear(&mut self) {
        self.stale += self.len();
        self.keys.clear();
        self.values.clear();
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

    pub fn iter_since(&self, since: usize) -> impl Iterator<Item = (&[u64], &V)> + Clone {
        (since.min(self.len())..self.len()).map(move |row| {
            (
                &self.keys[row * self.arity..(row + 1) * self.arity],
                &self.values[row],
            )
        })
    }
}
