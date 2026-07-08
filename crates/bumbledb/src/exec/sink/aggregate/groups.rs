use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{Acc, AggregateSink, FindSpec};
use crate::ir::AggOp;

impl AggregateSink {
    /// Refreshes the leaf-shape cache (outer slots + group constancy) —
    /// pointer-keyed on `key_slots`, so pinned batch-of-one leaves pay
    /// nothing after the first batch (PRD 05).
    pub(super) fn refresh_shape_cache(&mut self, batch: &LeafBatch<'_>) {
        self.cached_outer_slots.clear();
        for slot in 0..self.binding_scratch.len() {
            if matches!(batch.source_of(slot), LeafSource::Outer) {
                self.cached_outer_slots.push(slot);
            }
        }
        self.cached_constant_group = self
            .group_slots
            .iter()
            .all(|slot| matches!(batch.source_of(*slot), LeafSource::Outer));
    }

    /// Probes the group map with the key currently in `key_scratch`,
    /// seeding a fresh accumulator row on first sight. The one place a
    /// group probe happens — the batch path memoizes around it.
    pub(super) fn probe_group(&mut self) -> usize {
        #[cfg(test)]
        {
            self.group_probes += 1;
        }
        let next = self.groups.len();
        let (idx, inserted) = self.groups.get_or_insert_with(&self.key_scratch, || next);
        let group_idx = *idx;
        if inserted {
            // Fresh accumulator row, seeded per op.
            for find in &self.finds {
                if let FindSpec::Agg { op, signed, .. } = find {
                    self.accs.push(match (op, signed) {
                        (AggOp::Sum, true) => Acc::SumSigned(0),
                        (AggOp::Sum, false) => Acc::SumUnsigned(0),
                        (AggOp::Min, _) => Acc::Min(u64::MAX),
                        (AggOp::Max, _) => Acc::Max(u64::MIN),
                        (AggOp::Count, _) => Acc::Count(0),
                    });
                }
            }
        }
        group_idx
    }
}
