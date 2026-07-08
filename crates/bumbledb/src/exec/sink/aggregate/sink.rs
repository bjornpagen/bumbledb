use crate::exec::colt::SuffixRun;
use crate::exec::kernel;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, Sink};
use crate::exec::sink::{word_to_i64, Acc, AggregateSink, FindSpec};
use crate::image::ColumnView;
use crate::ir::AggOp;

impl Sink for AggregateSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..bindings.slot_count() {
            self.binding_scratch[slot] = bindings.get(slot);
        }
        self.fold_scratch_row();
        Flow::Continue
    }

    /// The scan-fold pushdown (docs/perf/ PRD 05): supported for the
    /// elided constant-group regime over word columns — positions fold
    /// straight through the kernels with no key batch materialized.
    /// Partials are identity-seeded and merged at `end_scan`, so an
    /// empty scan creates no group row (matching the batch paths).
    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> bool {
        if self.seen.is_some() {
            return false;
        }
        if self
            .group_slots
            .iter()
            .any(|slot| scan.key_slots.contains(slot))
        {
            return false;
        }
        self.scan_sources.clear();
        for find in &self.finds {
            let FindSpec::Agg { over_slot, .. } = find else {
                continue;
            };
            let source = over_slot.and_then(|slot| scan.key_slots.iter().position(|k| *k == slot));
            if let Some(word) = source {
                // Aggregates fold integer columns; a byte-backed column
                // here would be a validation hole — decline, don't trust.
                if !matches!(
                    scan.colt.suffix_column(scan.level, word),
                    ColumnView::Words(_)
                ) {
                    return false;
                }
            }
            self.scan_sources.push(source);
        }
        // Identity-seeded partials; the group key resolves now (outer
        // bindings are constant for this node entry).
        self.acc_scratch.clear();
        for find in &self.finds {
            if let FindSpec::Agg { op, signed, .. } = find {
                self.acc_scratch.push(match (op, signed) {
                    (AggOp::Sum, true) => Acc::SumSigned(0),
                    (AggOp::Sum, false) => Acc::SumUnsigned(0),
                    (AggOp::Min, _) => Acc::Min(u64::MAX),
                    (AggOp::Max, _) => Acc::Max(u64::MIN),
                    (AggOp::Count, _) => Acc::Count(0),
                });
            }
        }
        self.scan_count = 0;
        for (i, slot) in self.group_slots.iter().enumerate() {
            self.key_scratch[i] = scan.bindings.get(*slot);
        }
        true
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.scan_count += run.len() as u64;
        let mut cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg { op, .. } = find else {
                continue;
            };
            let source = self.scan_sources[cursor];
            let acc = &mut self.acc_scratch[cursor];
            cursor += 1;
            let Some(word) = source else {
                continue; // outer-constant / Count: finished at end_scan
            };
            let ColumnView::Words(col) = scan.colt.suffix_column(scan.level, word) else {
                unreachable!("begin_scan declined byte columns")
            };
            match (op, acc, run) {
                (AggOp::Sum, Acc::SumSigned(total), SuffixRun::Identity { start, len }) => {
                    *total += kernel::fold_sum_biased_i64(col, 1, start, len);
                }
                (AggOp::Sum, Acc::SumSigned(total), SuffixRun::Positions(p)) => {
                    *total += kernel::fold_sum_biased_i64_idx(col, 1, 0, p);
                }
                (AggOp::Sum, Acc::SumUnsigned(total), SuffixRun::Identity { start, len }) => {
                    *total += kernel::fold_sum_u64(col, 1, start, len);
                }
                (AggOp::Sum, Acc::SumUnsigned(total), SuffixRun::Positions(p)) => {
                    *total += kernel::fold_sum_u64_idx(col, 1, 0, p);
                }
                (AggOp::Min, Acc::Min(best), SuffixRun::Identity { start, len }) => {
                    *best = (*best).min(kernel::fold_min_max_u64(col, 1, start, len).0);
                }
                (AggOp::Min, Acc::Min(best), SuffixRun::Positions(p)) => {
                    *best = (*best).min(kernel::fold_min_max_u64_idx(col, 1, 0, p).0);
                }
                (AggOp::Max, Acc::Max(best), SuffixRun::Identity { start, len }) => {
                    *best = (*best).max(kernel::fold_min_max_u64(col, 1, start, len).1);
                }
                (AggOp::Max, Acc::Max(best), SuffixRun::Positions(p)) => {
                    *best = (*best).max(kernel::fold_min_max_u64_idx(col, 1, 0, p).1);
                }
                _ => unreachable!("accumulators are seeded per op; Count has no source"),
            }
        }
    }

    fn end_scan(&mut self, scan: &LeafScan<'_>) -> u64 {
        let count = self.scan_count;
        if count == 0 {
            return 0;
        }
        // Finish the outer-sourced and Count partials.
        let mut cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg { op, over_slot, .. } = find else {
                continue;
            };
            let source = self.scan_sources[cursor];
            let acc = &mut self.acc_scratch[cursor];
            cursor += 1;
            if source.is_some() {
                continue;
            }
            match (op, acc) {
                (AggOp::Count, Acc::Count(n)) => *n += count,
                (AggOp::Sum, Acc::SumSigned(total)) => {
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += i128::from(word_to_i64(scan.bindings.get(slot))) * i128::from(count);
                }
                (AggOp::Sum, Acc::SumUnsigned(total)) => {
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += u128::from(scan.bindings.get(slot)) * u128::from(count);
                }
                (AggOp::Min, Acc::Min(best)) => {
                    let slot = over_slot.expect("validated: Min has a variable");
                    *best = (*best).min(scan.bindings.get(slot));
                }
                (AggOp::Max, Acc::Max(best)) => {
                    let slot = over_slot.expect("validated: Max has a variable");
                    *best = (*best).max(scan.bindings.get(slot));
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
        // Merge the partials into the group's row (identity seeds make
        // the merge exact for every op).
        // Once per batch (docs/silicon2/10: the group-run memo that
        // skipped this probe measured < 2% under the const-arity map
        // and was deleted — the probe IS the fast path now).
        let group_idx = self.probe_group();
        let range = group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs;
        for (acc, partial) in self.accs[range].iter_mut().zip(&self.acc_scratch) {
            match (acc, partial) {
                (Acc::SumSigned(t), Acc::SumSigned(p)) => *t += p,
                (Acc::SumUnsigned(t), Acc::SumUnsigned(p)) => *t += p,
                (Acc::Min(t), Acc::Min(p)) => *t = (*t).min(*p),
                (Acc::Max(t), Acc::Max(p)) => *t = (*t).max(*p),
                (Acc::Count(t), Acc::Count(p)) => *t += p,
                _ => unreachable!("partials are seeded from the same finds"),
            }
        }
        count
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
        // Aggregate plans mark every node sink-relevant (hardening
        // PRD 05), so the executor never asks a fold to stop on skip.
        debug_assert!(!stop_on_skip, "folds never stop on skip");
        if batch.survivors.is_empty() {
            return Flow::Continue;
        }
        // Group-key classification, cached on the leaf shape: every
        // group slot outer means the whole batch folds into ONE
        // accumulator row — the trie already grouped it (PRD 02).
        self.refresh_shape_cache(batch);
        match (self.seen.is_some(), self.cached_constant_group) {
            // Dedup required (the plan could not prove distinctness):
            // the seen-set pass runs per row, but the group probe still
            // hoists — surviving entries gather-fold like the elided
            // path.
            (true, true) => self.fold_batch_dedup_constant_group(batch),
            (false, true) => self.fold_batch_constant_group(batch, batch.survivors),
            // Varying group keys: the per-row correctness arm.
            (_, false) => self.fold_batch_rows(batch),
        }
        Flow::Continue
    }
}
