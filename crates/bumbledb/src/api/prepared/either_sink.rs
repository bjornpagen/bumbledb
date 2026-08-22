use super::{Bindings, EitherSink};

use crate::exec::run::Sink;
use crate::exec::sink::FindSpec;

impl EitherSink {
    /// Empties the sink, retaining capacity — once per execution, never
    /// per rule (the seen-set spanning rules IS the union,
    /// docs/architecture/40-execution.md § the rule loop). `O(1)` in the
    /// seen-set's high-water: the maps clear by generation stamp
    /// (`exec/wordmap/clear.rs`), so a hot execution's multi-million-
    /// entry seen-set no longer taxes every later warm execute with a
    /// full-table walk.
    pub(super) fn reset(&mut self) {
        match self {
            Self::Projection(sink) => sink.reset(),
            Self::Aggregate(sink) => sink.reset(),
        }
    }

    /// Re-aims the sink's slot tables at one rule's binding layout —
    /// the rule loop's per-rule step; the shared maps (the union) are
    /// untouched. `shared_slots` is the rule's full slot array in
    /// `VarId` order — the DNF-derived union regime's re-key (R2); the
    /// projection and head-projection regimes never read it.
    pub(super) fn aim(
        &mut self,
        finds: &[FindSpec],
        slot_count: usize,
        shared_slots: &[(usize, usize)],
    ) {
        match self {
            Self::Projection(sink) => sink.aim(finds, slot_count),
            Self::Aggregate(sink) => sink.aim(finds, slot_count, shared_slots),
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
}

impl Sink for EitherSink {
    fn emit(&mut self, bindings: &Bindings) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit(bindings),
            Self::Aggregate(sink) => sink.emit(bindings),
        }
    }

    fn emit_batch(&mut self, batch: &crate::exec::run::LeafBatch<'_>) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit_batch(batch),
            Self::Aggregate(sink) => sink.emit_batch(batch),
        }
    }

    fn emit_batch_until_skip(
        &mut self,
        batch: &crate::exec::run::LeafBatch<'_>,
    ) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit_batch_until_skip(batch),
            Self::Aggregate(sink) => sink.emit_batch_until_skip(batch),
        }
    }

    fn skip_capability(&self) -> crate::exec::run::SkipCapability {
        match self {
            Self::Projection(sink) => sink.skip_capability(),
            Self::Aggregate(sink) => sink.skip_capability(),
        }
    }

    fn begin_scan(&mut self, scan: &crate::exec::run::LeafScan<'_>) -> crate::exec::run::ScanOffer {
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
