use super::ProjectionSink;

impl ProjectionSink {
    pub(crate) fn q1_report(&self, label: &str) {
        let dense = self.seen.unique_rows.as_ref()
            .map(|rows| (rows.len, rows.words.len(), rows.words.capacity()));
        let ram = self.seen.ram.q1_owners();
        let ram_capacity: usize = ram.iter().map(|(_, cap, size)| cap * size).sum();
        println!("SINK {label} dense={dense:?} hash_rows={} hash_owners={ram:?} hash_capacity_bytes={ram_capacity} scratch={:?} scan_rows={:?} key_bytes={:?} spilled={}",
            self.seen.ram.len(), (self.scratch.len(), self.scratch.capacity()),
            (self.scan_rows.len(), self.scan_rows.capacity()),
            (self.seen.key_bytes.len(), self.seen.key_bytes.capacity()), self.seen.spilled.is_some());
    }
}
