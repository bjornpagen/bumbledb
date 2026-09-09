use super::WordMap;
impl<V: Copy> WordMap<V> {
    pub(crate) fn q1_control_report(&self, label: &str, role: &str) {
        let owners = [
            (self.ctrl.len(), self.ctrl.capacity(), 1),
            (self.keys.len(), self.keys.capacity(), 8),
            (
                self.values.len(),
                self.values.capacity(),
                std::mem::size_of::<V>(),
            ),
            (self.stamps.len(), self.stamps.capacity(), 1),
            (self.dense.len(), self.dense.capacity(), 4),
        ];
        let bytes: usize = owners.iter().map(|(_, cap, size)| cap * size).sum();
        println!(
            "MAP {label} role={role} rows={} arity={} owners={owners:?} bytes={bytes}",
            self.len, self.arity
        );
    }
}
