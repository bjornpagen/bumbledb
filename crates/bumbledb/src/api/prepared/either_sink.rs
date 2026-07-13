use super::{Bindings, EitherSink};

use crate::exec::run::Sink;
use crate::exec::sink::FindSpec;

impl EitherSink {
    /// Empties the sink, retaining capacity — once per execution, never
    /// per rule (the seen-set spanning rules IS the union,
    /// docs/architecture/40-execution.md § the rule loop).
    pub(super) fn reset(&mut self) {
        match self {
            Self::Projection(sink) => sink.reset(),
            Self::Aggregate(sink) => sink.reset(),
        }
    }

    /// Re-aims the sink's slot tables at one rule's binding layout —
    /// the rule loop's per-rule step; the shared maps (the union) are
    /// untouched.
    pub(super) fn aim(&mut self, finds: &[FindSpec], slot_count: usize) {
        match self {
            Self::Projection(sink) => sink.aim(finds, slot_count),
            Self::Aggregate(sink) => sink.aim(finds, slot_count),
        }
    }

    /// Distinct head tuples (projection) or seen bindings (aggregate)
    /// held — the union observable behind per-rule absorbed accounting.
    /// `None` when the aggregate seen-set is elided (the distinct proof:
    /// nothing is ever absorbed).
    pub(super) fn distinct_seen(&self) -> Option<usize> {
        match self {
            Self::Projection(sink) => Some(sink.len()),
            Self::Aggregate(sink) => sink.distinct_seen(),
        }
    }

    /// The measure poison: the first ray a measure position reached
    /// during the rule loop — checked once after the rules run, before
    /// finalize, and raised as the typed
    /// [`crate::Error::MeasureOfRay`](crate::error::Error::MeasureOfRay).
    pub(super) fn measure_of_ray(&self) -> Option<[u64; 2]> {
        match self {
            Self::Projection(sink) => sink.measure_of_ray(),
            Self::Aggregate(sink) => sink.measure_of_ray(),
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
