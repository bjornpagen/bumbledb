//! The scan-pushdown leaf arm and its residual position filter.

use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, LeafScan, Operand, Sink, Source,
    ValidatedPlan,
};

impl Executor {
    /// The scan-pushdown arm: positions flow straight from the trie into
    /// the sink's kernels; no key batch exists. Leaf residuals filter
    /// positions per run before the sink sees them; a leaf that could
    /// skip (D2) stays on the batch path.
    #[allow(clippy::too_many_arguments)] // the run_node context, unpacked
    #[allow(clippy::too_many_lines)] // the two measured eval arms are siblings by design
    pub(super) fn run_leaf_scan<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        occ: usize,
        level: usize,
        cursor: Cursor,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Option<Flow> {
        let node = &plan.nodes()[node_idx];
        {
            if !colts[occ].suffix_scannable(cursor) || (!node.sink_relevant && sink.may_skip()) {
                return None;
            }
            // Batch-constant residuals (both sides outer) decide the
            // whole leaf at once.
            for (op, lhs, rhs) in &self.leaf_const_residuals {
                if !op.compare(&bindings.get(*lhs), &bindings.get(*rhs)) {
                    counters.node_entry(node_idx);
                    counters.cover_choice(node_idx, 0, false);
                    counters.residual(node_idx, false);
                    return Some(Flow::Continue);
                }
            }
            let scan = LeafScan {
                colt: &colts[occ],
                level,
                key_slots: &self.slot_map[node_idx][0],
                bindings,
            };
            if !sink.begin_scan(&scan) {
                return None;
            }
            counters.node_entry(node_idx);
            counters.cover_choice(node_idx, 0, false);
            counters.phase_start(node_idx, JoinPhase::Descend);
            let n_residuals = self.leaf_scan_residuals.len();
            let mut filtered = std::mem::take(&mut self.scan_filter);
            let drove = scan.colt.for_each_suffix_run(cursor, |run| {
                counters.batch(node_idx, run.len());
                if n_residuals == 0 {
                    sink.scan_run(&scan, run);
                    return;
                }
                // Filter positions through the leaf residuals — run-
                // length-adaptive (see SCAN_HOIST_THRESHOLD): big runs
                // resolve each residual's operands once; small runs
                // resolve per position (both directions measured, both
                // real).
                filtered.clear();
                if run.len() >= crate::exec::SCAN_HOIST_THRESHOLD {
                    // Residual-hoisted evaluation (the column-hoisted
                    // idiom turned on the plan's own list): each leaf
                    // residual resolves its two operands ONCE per run —
                    // a live column view or the outer constant — then
                    // filters positions, survivors compacting in place
                    // exactly like the batch path's residual passes. No
                    // fixed-size residual table exists: the witness
                    // list is iterated directly, at any length.
                    for (idx, (op, lhs_src, rhs_src)) in self.leaf_scan_residuals.iter().enumerate()
                    {
                        let side = |src: &Source| match *src {
                            Source::Batch(word) => {
                                Operand::Col(scan.colt.suffix_column(scan.level, word))
                            }
                            Source::Slot(slot) => Operand::Const(bindings.get(slot)),
                        };
                        let (lhs, rhs) = (side(lhs_src), side(rhs_src));
                        let value = |operand: &Operand<'_>, position: u32| match operand {
                            Operand::Col(crate::image::ColumnView::Words(w)) => {
                                w[position as usize]
                            }
                            Operand::Col(crate::image::ColumnView::Bytes(b)) => {
                                u64::from(b[position as usize])
                            }
                            Operand::Const(word) => *word,
                        };
                        let mut eval = |position: u32| {
                            let pass = op.compare(&value(&lhs, position), &value(&rhs, position));
                            counters.residual(node_idx, pass);
                            pass
                        };
                        if idx == 0 {
                            push_surviving(run, &mut filtered, &mut eval);
                        } else {
                            retain_surviving(&mut filtered, &mut eval);
                        }
                        if filtered.is_empty() {
                            break;
                        }
                    }
                } else {
                    let mut eval = |position: u32| {
                        for (op, lhs_src, rhs_src) in &self.leaf_scan_residuals {
                            let value = |src: &Source| match *src {
                                Source::Batch(word) => {
                                    match scan.colt.suffix_column(scan.level, word) {
                                        crate::image::ColumnView::Words(w) => w[position as usize],
                                        crate::image::ColumnView::Bytes(b) => {
                                            u64::from(b[position as usize])
                                        }
                                    }
                                }
                                Source::Slot(slot) => bindings.get(slot),
                            };
                            let pass = op.compare(&value(lhs_src), &value(rhs_src));
                            counters.residual(node_idx, pass);
                            if !pass {
                                return false;
                            }
                        }
                        true
                    };
                    push_surviving(run, &mut filtered, &mut eval);
                }
                if !filtered.is_empty() {
                    sink.scan_run(&scan, crate::exec::colt::SuffixRun::Positions(&filtered));
                }
            });
            debug_assert!(drove, "suffix_scannable pre-checked");
            let emitted = sink.end_scan(&scan);
            for _ in 0..emitted {
                counters.emit();
            }
            counters.phase_end(node_idx, JoinPhase::Descend);
            self.scan_filter = filtered;
            Some(Flow::Continue)
        }
    }
}

/// Appends the positions of `run` that pass `eval` to `out`.
fn push_surviving(
    run: crate::exec::colt::SuffixRun<'_>,
    out: &mut Vec<u32>,
    eval: &mut impl FnMut(u32) -> bool,
) {
    match run {
        crate::exec::colt::SuffixRun::Identity { start, len } => {
            for position in start..start + len {
                let position = u32::try_from(position).expect("positions fit u32");
                if eval(position) {
                    out.push(position);
                }
            }
        }
        crate::exec::colt::SuffixRun::Positions(positions) => {
            for &position in positions {
                if eval(position) {
                    out.push(position);
                }
            }
        }
    }
}

/// Compacts `out` in place, keeping the positions that pass `eval` —
/// [`push_surviving`]'s in-place twin for the residuals past the first
/// (one residual's survivors are the next one's input).
fn retain_surviving(out: &mut Vec<u32>, eval: &mut impl FnMut(u32) -> bool) {
    let mut kept = 0;
    for idx in 0..out.len() {
        let position = out[idx];
        if eval(position) {
            out[kept] = position;
            kept += 1;
        }
    }
    out.truncate(kept);
}
