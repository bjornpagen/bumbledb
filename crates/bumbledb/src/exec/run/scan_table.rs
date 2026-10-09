//! The scan-pushdown leaf arm and its residual position filter.
use super::{
    Counters, Cursor, Executor, Flow, JoinCtx, LeafScan, Operand, ScanBuffers, Sink, Source,
};
use std::ops::ControlFlow;

impl Executor {
    /// Scan physical suffix runs, polling bounded work before sink delivery.
    /// Unsupported scans may fall back; cancellation and sink errors may not.
    pub(super) fn run_leaf_scan<S: Sink, C: Counters>(
        &mut self,
        cx: &mut JoinCtx<'_, S, C>,
        node_idx: usize,
        occ: usize,
        level: usize,
        cursor: Cursor,
    ) -> Option<Flow> {
        let JoinCtx {
            plan,
            colts,
            bindings,
            sink,
            counters,
        } = cx;
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
        let mut buffers = std::mem::take(&mut self.scan_buffers);
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
                let ScanBuffers {
                    filtered,
                    words,
                    kept,
                } = &mut buffers;
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
                            if let Some(range) = word_range(*op, lhs, rhs) {
                                let before = if idx == 0 { run.len() } else { filtered.len() };
                                if idx == 0 {
                                    push_in_range(range, run, words, filtered);
                                } else {
                                    retain_in_range(range, filtered, words, kept);
                                }
                                for k in 0..before {
                                    counters.residual(node_idx, k < filtered.len());
                                }
                            } else {
                                let mut eval = |position: u32| {
                                    let pass =
                                        op.compare(&value(&lhs, position), &value(&rhs, position));
                                    counters.residual(node_idx, pass);
                                    pass
                                };
                                if idx == 0 {
                                    push_surviving(run, filtered, &mut eval);
                                } else {
                                    retain_surviving(filtered, &mut eval);
                                }
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
                        push_surviving(run, filtered, &mut eval);
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
                    sink.scan_run(&scan, crate::exec::colt::SuffixRun::Positions(filtered));
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
        self.scan_buffers = buffers;
        if let Some(error) = work_refusal {
            self.poison(super::Poison::Work(error));
        }
        // None means unsupported; refusal is a terminal scan outcome
        // and must never fall through into the generic leaf.
        Some(flow)
    }
}

/// A residual that compares a word column with a constant: the column and
/// the inclusive word range it keeps. `None` for a byte column, two columns,
/// or `Ne`.
fn word_range<'c>(
    op: crate::ir::WordCmp,
    lhs: Operand<'c>,
    rhs: Operand<'c>,
) -> Option<(&'c [u64], u64, u64)> {
    let (column, constant, op) = match (lhs, rhs) {
        (Operand::Col(crate::image::ColumnView::Words(column)), Operand::Const(constant)) => {
            (column, constant, op)
        }
        (Operand::Const(constant), Operand::Col(crate::image::ColumnView::Words(column))) => {
            (column, constant, op.converse())
        }
        _ => return None,
    };
    let (lo, hi) = op.kept_range(constant)?;
    Some((column, lo, hi))
}

/// Appends the run's positions whose word lies in the range, through one
/// range-filter kernel scan (a position run is gathered first).
fn push_in_range(
    (column, lo, hi): (&[u64], u64, u64),
    run: crate::exec::colt::SuffixRun<'_>,
    words: &mut Vec<u64>,
    out: &mut Vec<u32>,
) {
    let from = out.len();
    match run {
        crate::exec::colt::SuffixRun::Identity { start, len } => {
            crate::exec::kernel::filter_range_u64(&column[start..start + len], lo, hi, out);
            let base = u32::try_from(start).expect("positions fit u32");
            for position in &mut out[from..] {
                *position += base;
            }
        }
        crate::exec::colt::SuffixRun::Positions(positions) => {
            words.clear();
            words.extend(positions.iter().map(|&p| column[p as usize]));
            crate::exec::kernel::filter_range_u64(words, lo, hi, out);
            for position in &mut out[from..] {
                *position = positions[*position as usize];
            }
        }
    }
}

/// Keeps the positions of `filtered` whose word lies in the range.
fn retain_in_range(
    (column, lo, hi): (&[u64], u64, u64),
    filtered: &mut Vec<u32>,
    words: &mut Vec<u64>,
    kept: &mut Vec<u32>,
) {
    words.clear();
    words.extend(filtered.iter().map(|&p| column[p as usize]));
    kept.clear();
    crate::exec::kernel::filter_range_u64(words, lo, hi, kept);
    for (to, &from) in kept.iter().enumerate() {
        filtered[to] = filtered[from as usize];
    }
    filtered.truncate(kept.len());
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
