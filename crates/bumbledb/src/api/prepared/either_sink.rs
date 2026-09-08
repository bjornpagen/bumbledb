use super::{Bindings, EitherSink};

use crate::exec::run::Sink;
use crate::exec::sink::FindSpec;

impl EitherSink {
    /// Install this execution's cooperative cancellation context.
    pub(super) fn begin_execution(&mut self, work: Option<crate::work::WorkContext>) {
        match self {
            Self::Computed(sink) => sink.inner.begin_execution(work),
            Self::Projection(sink) => sink.begin(work),
            Self::Aggregate(sink) => sink.begin(work),
        }
    }

    pub(super) fn reset(&mut self) {
        match self {
            Self::Computed(sink) => sink.reset(),
            Self::Projection(sink) => sink.reset(),
            Self::Aggregate(sink) => sink.reset(),
        }
    }

    pub(super) fn release_memory(&mut self) {
        match self {
            Self::Computed(sink) => sink.release_memory(),
            Self::Projection(sink) => sink.release_memory(),
            Self::Aggregate(sink) => sink.release_memory(),
        }
    }

    pub(super) fn aim(
        &mut self,
        finds: &[FindSpec],
        slot_count: usize,
        shared_slots: &[(usize, usize)],
    ) {
        match self {
            Self::Computed(sink) => sink.aim(finds, slot_count, shared_slots),
            Self::Projection(sink) => sink.aim(finds),
            Self::Aggregate(sink) => sink.aim(finds, slot_count, shared_slots),
        }
    }

    #[cfg(test)]
    pub(super) fn distinct_seen(&self) -> Option<usize> {
        match self {
            Self::Computed(sink) => sink.inner.distinct_seen(),
            Self::Projection(sink) => Some(sink.len()),
            Self::Aggregate(sink) => sink.distinct_seen(),
        }
    }

    pub(super) fn progress(&self) -> crate::exec::sink::SinkProgress {
        match self {
            Self::Computed(sink) => {
                if sink.error.is_some() {
                    crate::exec::sink::SinkProgress::Error
                } else {
                    sink.inner.progress()
                }
            }
            Self::Projection(sink) => sink.progress(),
            Self::Aggregate(sink) => sink.progress(),
        }
    }

    pub(super) fn take_error(&mut self) -> Option<crate::error::Error> {
        match self {
            Self::Computed(sink) => sink.error.take().or_else(|| sink.inner.take_error()),
            Self::Projection(sink) => sink.take_error(),
            Self::Aggregate(sink) => sink.take_error(),
        }
    }
}

impl Sink for EitherSink {
    fn retains_binding_slot(&self, slot: usize) -> bool {
        match self {
            Self::Computed(sink) => sink.retains_binding_slot(slot),
            Self::Projection(sink) => sink.retains_binding_slot(slot),
            Self::Aggregate(sink) => sink.retains_binding_slot(slot),
        }
    }

    fn emit(&mut self, bindings: &Bindings) -> crate::exec::run::Flow {
        let flow = match self {
            Self::Computed(sink) => sink.emit(bindings),
            Self::Projection(sink) => sink.emit(bindings),
            Self::Aggregate(sink) => sink.emit(bindings),
        };
        crate::exec::run::Flow::from_sink_progress(self.progress()).or_skip(flow)
    }

    fn emit_batch(&mut self, batch: &crate::exec::run::LeafBatch<'_>) -> crate::exec::run::Flow {
        let flow = match self {
            Self::Computed(sink) => sink.emit_batch(batch),
            Self::Projection(sink) => sink.emit_batch(batch),
            Self::Aggregate(sink) => sink.emit_batch(batch),
        };
        crate::exec::run::Flow::from_sink_progress(self.progress()).or_skip(flow)
    }

    fn progress(&self) -> crate::exec::sink::SinkProgress {
        EitherSink::progress(self)
    }

    fn take_error(&mut self) -> Option<crate::error::Error> {
        EitherSink::take_error(self)
    }

    fn emit_batch_until_skip(
        &mut self,
        batch: &crate::exec::run::LeafBatch<'_>,
    ) -> crate::exec::run::Flow {
        match self {
            Self::Computed(sink) => sink.emit_batch(batch),
            Self::Projection(sink) => sink.emit_batch_until_skip(batch),
            Self::Aggregate(sink) => sink.emit_batch_until_skip(batch),
        }
    }

    fn skip_capability(&self) -> crate::exec::run::SkipCapability {
        match self {
            Self::Computed(_) => crate::exec::run::SkipCapability::Forbidden,
            Self::Projection(sink) => sink.skip_capability(),
            Self::Aggregate(sink) => sink.skip_capability(),
        }
    }

    fn prepare_scan(&mut self, key_slots: &[usize]) {
        if let Self::Projection(sink) = self {
            sink.prepare_scan(key_slots);
        }
    }

    fn begin_scan(&mut self, scan: &crate::exec::run::LeafScan<'_>) -> crate::exec::run::ScanOffer {
        match self {
            Self::Computed(_) => crate::exec::run::ScanOffer::Declined,
            Self::Projection(sink) => sink.begin_scan(scan),
            Self::Aggregate(sink) => sink.begin_scan(scan),
        }
    }

    fn scan_run(
        &mut self,
        scan: &crate::exec::run::LeafScan<'_>,
        run: crate::exec::colt::SuffixRun<'_>,
    ) {
        match self {
            Self::Computed(_) => unreachable!("computed scans use complete binding batches"),
            Self::Projection(sink) => sink.scan_run(scan, run),
            Self::Aggregate(sink) => sink.scan_run(scan, run),
        }
    }

    fn end_scan(&mut self, scan: &crate::exec::run::LeafScan<'_>) -> u64 {
        match self {
            Self::Computed(_) => unreachable!("computed scans use complete binding batches"),
            Self::Projection(sink) => sink.end_scan(scan),
            Self::Aggregate(sink) => sink.end_scan(scan),
        }
    }
}
