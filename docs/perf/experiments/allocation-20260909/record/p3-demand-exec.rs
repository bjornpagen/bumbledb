impl super::Executor {
    pub(crate) fn overlap_hit_capacity(&self) -> usize {
        self.overlap_hits.capacity()
    }
    pub(crate) fn audit_overlap_demand(
        &self,
        label: &str,
        samples: &[crate::interval::overlap::p3_demand::Sample],
        initial_capacity: usize,
        output: Option<&std::path::Path>,
    ) {
        self.overlap
            .audit_demand(label, samples, initial_capacity, output);
    }
}
