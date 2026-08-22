use crate::exec::colt::SuffixRun;
use crate::exec::kernel;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, ScanOffer, Sink};
use crate::exec::sink::{
    Acc, AggSpec, AggregateSink, DedupState, FoldOp, GroupState, SinkSpec, word_to_i64,
};
use crate::image::ColumnView;

use super::super::FoldSource;

impl Sink for AggregateSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..bindings.slot_count() {
            self.binding_scratch[slot] = bindings.get(slot);
        }
        self.fold_scratch_row();
        Flow::Continue
    }

    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> ScanOffer {
        if !matches!(self.dedup, DedupState::Elided { .. }) {
            return ScanOffer::Declined;
        }

        if matches!(self.group_state, GroupState::Pack { .. }) {
            return ScanOffer::Declined;
        }

        if self
            .group_spans
            .iter()
            .any(|(slot, width)| (*slot..slot + width).any(|word| scan.key_slots.contains(&word)))
        {
            return ScanOffer::Declined;
        }
        self.scan_sources.clear();
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let AggSpec::Fold { slot, .. } = spec else {
                continue;
            };
            let source = match scan.key_slots.iter().position(|k| *k == *slot) {
                Some(word) => {
                    if !matches!(
                        scan.colt.suffix_column(scan.level, word),
                        ColumnView::Words(_)
                    ) {
                        return ScanOffer::Declined;
                    }
                    FoldSource::Column(word)
                }
                None => FoldSource::Outer,
            };
            self.scan_sources.push(source);
        }

        self.acc_scratch.clear();
        for find in &self.finds {
            if let SinkSpec::Agg(spec) = find {
                self.acc_scratch.push(spec.seed_acc());
            }
        }
        self.scan_count = 0;
        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            scan.bindings.get(slot)
        });
        ScanOffer::Open
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.scan_count += run.len() as u64;
        let mut acc_i = 0;
        let mut fold_i = 0;
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let acc = &mut self.acc_scratch[acc_i];
            acc_i += 1;
            let AggSpec::Fold { op, .. } = spec else {
                continue;
            };
            let source = self.scan_sources[fold_i];
            fold_i += 1;
            let FoldSource::Column(word) = source else {
                continue;
            };
            let ColumnView::Words(col) = scan.colt.suffix_column(scan.level, word) else {
                unreachable!("begin_scan declined byte columns")
            };
            match (op, acc, run) {
                (FoldOp::Sum, Acc::SumSigned(total), SuffixRun::Identity { start, len }) => {
                    *total += kernel::fold_sum_biased_i64(col, 1, start, len);
                }
                (FoldOp::Sum, Acc::SumSigned(total), SuffixRun::Positions(p)) => {
                    *total += kernel::fold_sum_biased_i64_idx(col, 1, 0, p);
                }
                (FoldOp::Sum, Acc::SumUnsigned(total), SuffixRun::Identity { start, len }) => {
                    *total += kernel::fold_sum_u64(col, 1, start, len);
                }
                (FoldOp::Sum, Acc::SumUnsigned(total), SuffixRun::Positions(p)) => {
                    *total += kernel::fold_sum_u64_idx(col, 1, 0, p);
                }
                (FoldOp::Min, Acc::Min(best), SuffixRun::Identity { start, len }) => {
                    *best = (*best).min(kernel::fold_min_max_u64(col, 1, start, len).0);
                }
                (FoldOp::Min, Acc::Min(best), SuffixRun::Positions(p)) => {
                    *best = (*best).min(kernel::fold_min_max_u64_idx(col, 1, 0, p).0);
                }
                (FoldOp::Max, Acc::Max(best), SuffixRun::Identity { start, len }) => {
                    *best = (*best).max(kernel::fold_min_max_u64(col, 1, start, len).1);
                }
                (FoldOp::Max, Acc::Max(best), SuffixRun::Positions(p)) => {
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

        let mut acc_i = 0;
        let mut fold_i = 0;
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let acc = &mut self.acc_scratch[acc_i];
            acc_i += 1;
            match spec {
                AggSpec::Count => {
                    let Acc::Count(n) = acc else {
                        unreachable!("accumulators are seeded per op");
                    };
                    *n += count;
                }
                AggSpec::Fold { op, slot, .. } => {
                    let source = self.scan_sources[fold_i];
                    fold_i += 1;
                    if matches!(source, FoldSource::Column(_)) {
                        continue;
                    }
                    match (op, acc) {
                        (FoldOp::Sum, Acc::SumSigned(total)) => {
                            *total += i128::from(word_to_i64(scan.bindings.get(*slot)))
                                * i128::from(count);
                        }
                        (FoldOp::Sum, Acc::SumUnsigned(total)) => {
                            *total += u128::from(scan.bindings.get(*slot)) * u128::from(count);
                        }
                        (FoldOp::Min, Acc::Min(best)) => {
                            *best = (*best).min(scan.bindings.get(*slot));
                        }
                        (FoldOp::Max, Acc::Max(best)) => {
                            *best = (*best).max(scan.bindings.get(*slot));
                        }
                        _ => unreachable!("accumulators are seeded per op"),
                    }
                }
            }
        }

        let group_idx = self.probe_group();
        let GroupState::Folds { accs, n_aggs } = &mut self.group_state else {
            unreachable!("scan merge is the Folds arm");
        };
        let range = group_idx * *n_aggs..(group_idx + 1) * *n_aggs;
        for (acc, partial) in accs[range].iter_mut().zip(&self.acc_scratch) {
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

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        if batch.survivors.is_empty() {
            return Flow::Continue;
        }

        self.refresh_shape_cache(batch);

        if matches!(self.group_state, GroupState::Pack { .. }) {
            self.fold_batch_rows(batch);
            return Flow::Continue;
        }
        match (
            !matches!(self.dedup, DedupState::Elided { .. }),
            self.cached_constant_group,
        ) {
            (true, true) => self.fold_batch_dedup_constant_group(batch),
            (false, true) => self.fold_batch_constant_group(batch, batch.survivors),

            (_, false) => self.fold_batch_rows(batch),
        }
        Flow::Continue
    }
}
