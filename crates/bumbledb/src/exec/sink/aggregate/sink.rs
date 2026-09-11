use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, ScanOffer, Sink};
use crate::exec::sink::{Acc, AggSpec, AggregateSink, DedupState, GroupState, SinkSpec};
use crate::image::ColumnView;

use super::super::FoldSource;
use super::reduce::{Partial, merge};

impl Sink for AggregateSink {
    fn retains_binding_slot(&self, slot: usize) -> bool {
        let in_spans = |spans: &[(usize, usize)]| {
            spans
                .iter()
                .any(|&(start, width)| slot >= start && slot - start < width)
        };
        in_spans(&self.group_spans)
            || match &self.dedup {
                DedupState::Bindings { .. } => slot < self.real_slots,
                DedupState::Union { spans, .. } | DedupState::DnfUnion { spans, .. } => {
                    in_spans(spans)
                }
                DedupState::Elided { .. } => false,
            }
    }

    #[inline]
    fn may_use_distinct_traversal() -> bool {
        true
    }

    fn emit(&mut self, bindings: &Bindings) -> Flow {
        if self.distinct_bindings() {
            self.fold_row(&[], |slot, _| bindings.get(slot));
        } else {
            let mut row = std::mem::take(&mut self.binding_scratch);
            for (slot, word) in row[..bindings.slot_count()].iter_mut().enumerate() {
                *word = bindings.get(slot);
            }
            self.fold_row(&row, |slot, _| bindings.get(slot));
            self.binding_scratch = row;
        }
        Flow::from_sink_progress(AggregateSink::progress(self))
    }

    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> ScanOffer {
        if !matches!(self.dedup, DedupState::Elided { .. }) {
            return ScanOffer::Declined;
        }

        if matches!(self.group_state, GroupState::Pack { .. }) {
            return ScanOffer::Declined;
        }
        // Exact floating folds use the constant-group batch path, which
        // already preserves shared Sum/Mean lanes and binding distinctness.
        if self
            .finds
            .iter()
            .any(|find| matches!(find, SinkSpec::Agg(AggSpec::Float { .. })))
        {
            return ScanOffer::Declined;
        }

        self.refresh_shape_cache(scan.key_slots);
        if !self.cached_constant_group {
            return ScanOffer::Declined;
        }
        for input in &mut self.fold_inputs {
            if !matches!(
                scan.colt.suffix_column(scan.level, input.word),
                ColumnView::Words(_)
            ) {
                return ScanOffer::Declined;
            }
            input.partial.reset();
        }
        self.scan_count = 0;
        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            scan.bindings.get(slot)
        });
        ScanOffer::Open
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        if self.cardinality_overflow {
            return;
        }
        let Some(count) = self.scan_count.checked_add(run.len() as u64) else {
            self.cardinality_overflow = true;
            return;
        };
        self.scan_count = count;
        for input in &mut self.fold_inputs {
            let ColumnView::Words(column) = scan.colt.suffix_column(scan.level, input.word) else {
                unreachable!("begin_scan declined byte columns")
            };
            input.partial.fold(column, 1, 0, run);
        }
    }

    fn end_scan(&mut self, scan: &LeafScan<'_>) -> u64 {
        let count = self.scan_count;
        self.maybe_spill_groups();
        if count == 0 || self.error.is_some() || self.cardinality_overflow {
            return 0;
        }

        let group_idx = self.probe_group();
        if !self.advance_group(group_idx, count) {
            return 0;
        }

        let GroupState::Folds { accs, n_aggs } = &mut self.group_state else {
            unreachable!("scan merge is the Folds arm");
        };
        let range = group_idx * *n_aggs..(group_idx + 1) * *n_aggs;
        let mut accumulators = accs[range].iter_mut();
        let mut fold_i = 0;
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let acc = accumulators.next().expect("one accumulator per aggregate");
            match spec {
                AggSpec::Float { .. } => unreachable!("float scans use the batch path"),
                AggSpec::Count => {
                    let Acc::Count(n) = acc else {
                        unreachable!("accumulators are seeded per op");
                    };
                    *n = n.saturating_add(count);
                }
                AggSpec::Fold { slot, .. } => {
                    let source = self.fold_sources[fold_i];
                    fold_i += 1;
                    let partial = match source {
                        FoldSource::Column(input) => self.fold_inputs[input].partial,
                        FoldSource::Outer => {
                            Partial::seed(*spec).repeated(scan.bindings.get(*slot), count)
                        }
                    };
                    merge(acc, partial.output(*spec, count));
                }
            }
        }

        count
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        if batch.survivors.is_empty() {
            return Flow::from_sink_progress(AggregateSink::progress(self));
        }

        self.refresh_shape_cache(batch.key_slots);

        if matches!(self.group_state, GroupState::Pack { .. }) {
            self.fold_batch_rows(batch);
            return Flow::from_sink_progress(AggregateSink::progress(self));
        }
        match (!self.distinct_bindings(), self.cached_constant_group) {
            (true, true) => self.fold_batch_dedup_constant_group(batch),
            // Raw multiplicity is legal only with the checked distinct-binding
            // witness. Exact accumulation is not idempotent: replaying a partition
            // doubles its contribution. The seen set, a semantic `DistinctWitness`,
            // or deduplicated resident COLT traversal must exclude overlapping
            // derivations before push_repeated. Resident dedup does not license raw scans.
            (false, true) => {
                debug_assert!(
                    self.distinct_bindings(),
                    "raw batch multiplicity requires the distinct-binding witness"
                );
                self.fold_batch_constant_group(batch, batch.survivors);
            }

            (_, false) => self.fold_batch_rows(batch),
        }
        Flow::from_sink_progress(AggregateSink::progress(self))
    }

    fn progress(&self) -> crate::exec::sink::SinkProgress {
        AggregateSink::progress(self)
    }

    fn take_error(&mut self) -> Option<crate::error::Error> {
        AggregateSink::take_error(self)
    }
}
