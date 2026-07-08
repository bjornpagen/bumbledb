//! The leaf fast-path dispatcher and the pinned-row arm (docs/perf/ PRD 05).

use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, LeafBatch, Sink, Source,
    ValidatedPlan,
};

impl Executor {
    /// The leaf fast paths (docs/perf/ PRD 05). `None` = declined —
    /// multi-position forced nodes the sink cannot scan, sinks without
    /// scan support, byte-column folds — and the generic batch path runs
    /// instead (conservative by construction: correctness never depends
    /// on a fast path firing).
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

    /// The pinned-row arm: a batch of exactly one, with every batch
    /// scaffold skipped — gather, residuals, emit.
    #[allow(clippy::too_many_arguments)] // the run_node context, unpacked
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
        {
            let key_slots = &self.slot_map[node_idx][0];
            let arity = key_slots.len();
            counters.node_entry(node_idx);
            counters.cover_choice(node_idx, 0, false);
            counters.batch(node_idx, 1);
            counters.phase_start(node_idx, JoinPhase::Descend);
            colts[occ].gather_row(level, position, &mut self.leaf_row[..arity.max(1)]);
            for (idx, (lhs_src, rhs_src)) in self.leaf_residual_sources.iter().enumerate() {
                let value = |src: &Source| match *src {
                    Source::Batch(word) => self.leaf_row[word],
                    Source::Slot(slot) => bindings.get(slot),
                };
                let op = self.residual_slots[node_idx][idx].0.op;
                let pass = op.compare(&value(lhs_src), &value(rhs_src));
                counters.residual(node_idx, pass);
                if !pass {
                    counters.phase_end(node_idx, JoinPhase::Descend);
                    return Flow::Continue;
                }
            }
            let batch = LeafBatch {
                keys: &self.leaf_row,
                arity,
                survivors: &[0],
                key_slots,
                bindings,
            };
            let stop_on_skip = !node.sink_relevant && sink.may_skip();
            let flow = sink.emit_batch(&batch, stop_on_skip);
            counters.emit();
            counters.phase_end(node_idx, JoinPhase::Descend);
            if flow == Flow::SkipSuffix {
                counters.skip(node_idx);
                return Flow::SkipSuffix;
            }
            Flow::Continue
        }
    }
}
