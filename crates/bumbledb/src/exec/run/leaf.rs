//! The leaf fast-path dispatcher and the pinned-row arm.

use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, LeafBatch, Sink, Source,
    ValidatedPlan,
};

/// MEASURE-OR-MERGE TWIN SUPPORT (cleanup-0.5.0 ruling 6,
/// `docs/prds/cleanup-0.5.0/prd-M-measure.md` item 1) — the -off idiom,
/// `cfg(test)` only: no runtime mode ships, and the switch itself dies
/// with the Measure phase's verdict (law or merge).
#[cfg(test)]
impl Executor {
    /// Forces the leaf fast-path classification off, routing every leaf
    /// through the generic batch machinery — the A/B twin's B arm
    /// (correctness never depends on a fast path firing). One direction
    /// only: a plan that never classified `single` has no fast-path
    /// buffers to turn on.
    pub(crate) fn disable_leaf_elision(&mut self) {
        self.leaf_single = false;
    }

    /// Whether the leaf fast paths are engaged — the twin's A-arm
    /// firing proof.
    pub(crate) fn leaf_elision_engaged(&self) -> bool {
        self.leaf_single
    }
}

impl Executor {
    /// The leaf fast paths. `None` = declined —
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
    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )] // the run_node context, unpacked
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
            let stop_on_skip = node.suffix_skip == crate::plan::fj::SuffixSkip::Licensed
                && sink.skip_capability() == super::SkipCapability::Licensed;
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
