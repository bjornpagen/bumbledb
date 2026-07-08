use super::{Bindings, EitherSink};

use crate::exec::run::Sink;

impl EitherSink {
    pub(super) fn reset(&mut self) {
        match self {
            Self::Projection(sink) => sink.reset(),
            Self::Aggregate(sink) => sink.reset(),
        }
    }
}

impl Sink for EitherSink {
    fn emit(&mut self, bindings: &Bindings) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit(bindings),
            Self::Aggregate(sink) => sink.emit(bindings),
        }
    }

    fn emit_batch(
        &mut self,
        batch: &crate::exec::run::LeafBatch<'_>,
        stop_on_skip: bool,
    ) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit_batch(batch, stop_on_skip),
            Self::Aggregate(sink) => sink.emit_batch(batch, stop_on_skip),
        }
    }

    fn may_skip(&self) -> bool {
        match self {
            Self::Projection(sink) => sink.may_skip(),
            Self::Aggregate(sink) => sink.may_skip(),
        }
    }

    fn begin_scan(&mut self, scan: &crate::exec::run::LeafScan<'_>) -> bool {
        match self {
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
            Self::Projection(sink) => sink.scan_run(scan, run),
            Self::Aggregate(sink) => sink.scan_run(scan, run),
        }
    }

    fn end_scan(&mut self, scan: &crate::exec::run::LeafScan<'_>) -> u64 {
        match self {
            Self::Projection(sink) => sink.end_scan(scan),
            Self::Aggregate(sink) => sink.end_scan(scan),
        }
    }
}
