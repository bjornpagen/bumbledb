impl super::Executor {
    pub(crate) fn saved_overlap_snapshot(
        &self,
    ) -> crate::interval::overlap::p3_cache_observer::CacheSnapshot {
        let mut snapshot = self.overlap.saved_structure_snapshot();
        snapshot.owners.extend([
            (
                "hits",
                self.overlap_hits.len(),
                self.overlap_hits.capacity(),
                size_of::<u32>(),
            ),
            (
                "lookup_key",
                self.overlap_key.len(),
                self.overlap_key.capacity(),
                size_of::<u64>(),
            ),
        ]);
        snapshot
    }
}
