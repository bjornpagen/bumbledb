//! Leaf dispatch: a pinned row or a fused suffix scan.
use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, LeafBatch, Sink, Source, ValidatedPlan,
};

impl Executor {
    /// Use a pinned row or a directly scannable suffix when the sink permits it.
    pub(super) fn run_leaf_fast<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Option<Flow> {
        let node = &plan.nodes()[node_idx];
        let occ = usize::from(node.subatoms[0].occ.0);
        let (cursor, level) = self.cursors[occ];
        match cursor {
            Cursor::Row(position) => Some(self.run_leaf_pinned(
                plan, node_idx, occ, level, position, colts, bindings, sink, counters,
            )),
            Cursor::Node(_) => self.run_leaf_scan(
                plan, node_idx, occ, level, cursor, colts, bindings, sink, counters,
            ),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )]
    fn run_leaf_pinned<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        occ: usize,
        level: usize,
        position: u32,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        let node = &plan.nodes()[node_idx];
        let super::LeafPrecompute::Fast {
            scan_residuals,
            const_residuals,
            row,
        } = &mut self.leaf
        else {
            unreachable!("fast path is classified Fast");
        };
        let key_slots = &self.slot_map[node_idx][0];
        let arity = key_slots.len();
        counters.node_entry(node_idx);
        counters.cover_choice(node_idx, 0, crate::exec::colt::KeyCount::Estimate(0));
        counters.batch(node_idx, 1);
        colts[occ].gather_row(level, position, &mut row[..arity.max(1)]);
        let mut failed = false;
        for (op, lhs, rhs) in const_residuals.iter() {
            let pass = op.compare(&bindings.get(*lhs), &bindings.get(*rhs));
            counters.residual(node_idx, pass);
            if !pass {
                failed = true;
                break;
            }
        }
        if !failed {
            for (op, lhs_src, rhs_src) in scan_residuals.iter() {
                let value = |src: &Source| match *src {
                    Source::Batch(word) => row[word],
                    Source::Slot(slot) => bindings.get(slot),
                };
                let pass = op.compare(&value(lhs_src), &value(rhs_src));
                counters.residual(node_idx, pass);
                if !pass {
                    failed = true;
                    break;
                }
            }
        }
        if failed {
            return Flow::Continue;
        }
        let batch = LeafBatch {
            keys: row,
            arity,
            survivors: &[0],
            key_slots,
            bindings,
        };
        let flow = super::emit_node_batch(sink, node.suffix_skip, &batch);
        counters.emit();
        if flow.is_terminal() {
            return flow;
        }
        if flow == Flow::SkipSuffix {
            counters.skip(node_idx);
            return Flow::SkipSuffix;
        }
        Flow::Continue
    }
}
