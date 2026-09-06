//! The scan-pushdown leaf arm and its residual position filter.
use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, LeafScan, Operand, Sink, Source,
    ValidatedPlan,
};
use std::ops::ControlFlow;

impl Executor {
    /// Scan physical suffix runs, polling bounded work before sink delivery.
    /// Unsupported scans may fall back; cancellation and sink errors may not.
    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )]
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
        // A physical set traversal licenses deduplicated COLT keys, never
        // raw source-position multiplicity. The generic leaf forces keys.
        if S::may_use_distinct_traversal() && self.physical_distinct.is_some() {
            return None;
        }
        let node = &plan.nodes()[node_idx];
        let super::LeafPrecompute::Fast {
            const_residuals,
            scan_residuals,
            ..
        } = &self.leaf
        else {
            unreachable!("fast path is classified Fast");
        };
        if !colts[occ].suffix_scannable(cursor)
            || (node.suffix_skip == crate::plan::fj::SuffixSkip::Licensed
                && sink.skip_capability() == super::SkipCapability::Licensed)
        {
            return None;
        }

        for (op, lhs, rhs) in const_residuals {
            if !op.compare(&bindings.get(*lhs), &bindings.get(*rhs)) {
                counters.node_entry(node_idx);
                counters.cover_choice(node_idx, 0, crate::exec::colt::KeyCount::Estimate(0));
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
        if sink.begin_scan(&scan) != super::ScanOffer::Open {
            return None;
        }
        counters.node_entry(node_idx);
        counters.cover_choice(node_idx, 0, crate::exec::colt::KeyCount::Estimate(0));
        let n_residuals = scan_residuals.len();
        let mut filtered = std::mem::take(&mut self.scan_filter);
        let ledger = &mut self.ledger;
        let mut work_refusal = None;
        let drove = scan.colt.for_each_suffix_run(cursor, |run| {
            // Keep physical-run statistics stable; bounded windows are
            // an internal polling detail, not new COLT batches.
            counters.batch(node_idx, run.len());
            let mut remaining = run;
            while !remaining.is_empty() {
                // A prior short run may have left pending work. Align
                // the first window to that quantum, then use full windows.
                let available = ledger
                    .as_ref()
                    .map_or(crate::exec::sink::STEP_QUANTUM, |ledger| {
                        crate::exec::sink::STEP_QUANTUM - ledger.pending
                    }) as usize;
                let (run, tail) = remaining.split_at(remaining.len().min(available));
                remaining = tail;
                if n_residuals != 0 {
                    filtered.clear();
                    if run.len() >= crate::exec::SCAN_HOIST_THRESHOLD {
                        for (idx, (op, lhs_src, rhs_src)) in scan_residuals.iter().enumerate() {
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
                                let pass =
                                    op.compare(&value(&lhs, position), &value(&rhs, position));
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
                            for (op, lhs_src, rhs_src) in scan_residuals {
                                let value = |src: &Source| match *src {
                                    Source::Batch(word) => {
                                        match scan.colt.suffix_column(scan.level, word) {
                                            crate::image::ColumnView::Words(w) => {
                                                w[position as usize]
                                            }
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
                }
                // Charge every explored window, including all-rejected
                // windows, before handing its surviving rows to the sink.
                if let Some(ledger) = ledger
                    && let Err(error) = ledger.note_explored(run.len())
                {
                    work_refusal = Some(error);
                    return ControlFlow::Break(Flow::Error);
                }
                if n_residuals == 0 {
                    sink.scan_run(&scan, run);
                } else if !filtered.is_empty() {
                    sink.scan_run(&scan, crate::exec::colt::SuffixRun::Positions(&filtered));
                }
                let flow = Flow::from_sink_progress(sink.progress());
                if flow.is_terminal() {
                    return ControlFlow::Break(flow);
                }
            }
            ControlFlow::Continue(())
        });
        let flow = match drove {
            ControlFlow::Continue(supported) => {
                debug_assert!(supported, "suffix_scannable pre-checked");
                let emitted = sink.end_scan(&scan);
                for _ in 0..emitted {
                    counters.emit();
                }
                Flow::from_sink_progress(sink.progress())
            }
            ControlFlow::Break(flow) => flow,
        };
        self.scan_filter = filtered;
        if let Some(error) = work_refusal {
            self.poison(super::Poison::Work(error));
        }
        // None means unsupported; refusal is a terminal scan outcome
        // and must never fall through into the generic leaf.
        Some(flow)
    }
}

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
