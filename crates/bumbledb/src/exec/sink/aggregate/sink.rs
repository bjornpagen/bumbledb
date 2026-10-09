use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, ScanOffer, Sink};
use crate::exec::sink::{AggregateSink, DedupState, GroupState};
use crate::image::ColumnView;

impl Sink for AggregateSink {
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
        if count == 0 || self.error.is_some() || self.cardinality_overflow {
            return 0;
        }

        let Some(group_idx) = self.probe_group() else {
            return 0;
        };
        if !self.advance_group(group_idx, count) {
            return 0;
        }

        self.merge_partials(group_idx, count, |slot| scan.bindings.get(slot));
        if self.cardinality_overflow {
            return 0;
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
