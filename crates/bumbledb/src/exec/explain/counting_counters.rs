use super::CountingCounters;
use crate::plan::fj::ValidatedPlan;

impl CountingCounters {
    #[must_use]
    pub fn new(plan: &ValidatedPlan) -> Self {
        let nodes = plan.nodes().len();
        let stride = plan
            .nodes()
            .iter()
            .map(|n| n.subatoms.len())
            .max()
            .unwrap_or(0);
        Self {
            stride,
            node_entries: vec![0; nodes],
            cover_choices: vec![[0; 2]; nodes * stride],
            probes: vec![[0; 2]; nodes * stride],
            hashes: vec![0; nodes * stride],
            residuals: vec![[0; 2]; nodes],
            skips: vec![0; nodes],
            batches: vec![[0; 2]; nodes],
            emits: 0,
        }
    }

    /// `(batches drawn, entries yielded)` for one node — the "batching
    /// engaged" observable (docs/architecture/50-validation.md).
    #[cfg(test)]
    #[must_use]
    pub fn batches(&self, node: usize) -> (u64, u64) {
        let [b, e] = self.batches[node];
        (b, e)
    }

    /// Bindings emitted to the sink (the measured cardinality after the
    /// last node).
    #[cfg(test)]
    #[must_use]
    pub fn emits(&self) -> u64 {
        self.emits
    }

    /// The measured cardinality *after* node `k`: how many complete
    /// extensions survived it — entries of the next node, or sink emits
    /// for the last.
    #[must_use]
    pub fn actual_after(&self, node: usize) -> u64 {
        self.node_entries
            .get(node + 1)
            .copied()
            .unwrap_or(self.emits)
    }

    /// The `[Exact, Estimate]` cover-choice histogram cell.
    #[must_use]
    pub fn cover_histogram(&self, node: usize, subatom: usize) -> [u64; 2] {
        self.cover_choices[node * self.stride + subatom]
    }
}
