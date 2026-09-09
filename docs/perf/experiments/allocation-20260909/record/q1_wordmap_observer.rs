use super::WordMap;

impl<V: Copy> WordMap<V> {
    pub(crate) fn q1_owners(&self) -> [(usize, usize, usize); 5] {
        [
            (self.ctrl.len(), self.ctrl.capacity(), 1),
            (self.keys.len(), self.keys.capacity(), 8),
            (self.values.len(), self.values.capacity(), std::mem::size_of::<V>()),
            (self.stamps.len(), self.stamps.capacity(), 1),
            (self.dense.len(), self.dense.capacity(), 4),
        ]
    }
}
