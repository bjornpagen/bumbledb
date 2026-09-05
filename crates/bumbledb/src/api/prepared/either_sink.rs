use super::{Bindings, EitherSink};

use crate::exec::run::Sink;
use crate::exec::sink::FindSpec;

impl EitherSink {
    pub(super) fn reset(&mut self) {
        match self {
            Self::Computed(sink) => { sink.error = None; sink.inner.reset(); },
            Self::Projection(sink) => sink.reset(),
            Self::Aggregate(sink) => sink.reset(),
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
            Self::Projection(sink) => sink.aim(finds, slot_count),
            Self::Aggregate(sink) => sink.aim(finds, slot_count, shared_slots),
        }
    }

    pub(super) fn distinct_seen(&self) -> Option<usize> {
        match self {
            Self::Computed(sink) => sink.inner.distinct_seen(),
            Self::Projection(sink) => Some(sink.len()),
            Self::Aggregate(sink) => sink.distinct_seen(),
        }
    }
}

impl Sink for EitherSink {
    fn emit(&mut self, bindings: &Bindings) -> crate::exec::run::Flow {
        match self {
            Self::Computed(sink) => sink.emit(bindings),
            Self::Projection(sink) => sink.emit(bindings),
            Self::Aggregate(sink) => sink.emit(bindings),
        }
    }

    fn emit_batch(&mut self, batch: &crate::exec::run::LeafBatch<'_>) -> crate::exec::run::Flow {
        match self {
            Self::Computed(sink) => sink.emit_batch(batch),
            Self::Projection(sink) => sink.emit_batch(batch),
            Self::Aggregate(sink) => sink.emit_batch(batch),
        }
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
