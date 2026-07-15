//! The fixpoint profile's counter (docs/architecture/40-execution.md
//! § the fixpoint driver): collects the driver's per-stratum, per-round
//! delta sizes and union accounting through the `Counters` seam's
//! fixpoint hooks. Node-level methods are no-ops by design — one
//! counter spans many differently shaped plan units, so per-node
//! attribution has no stable index space; the emit count and the round
//! structure are the honest whole-program surface.

use super::FixpointCounters;
use crate::api::stats::{DeltaRows, RoundStats, StratumStats};
use crate::exec::run::Counters;

impl FixpointCounters {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bindings emitted across every plan unit of the counted run.
    #[must_use]
    pub fn total_emits(&self) -> u64 {
        self.emits
    }

    /// The collected per-stratum round records, in condensation order.
    #[must_use]
    pub fn into_strata(self) -> Vec<StratumStats> {
        debug_assert!(
            self.pending_deltas.is_empty(),
            "every reported delta belongs to a closed round"
        );
        self.strata
    }
}

impl Counters for FixpointCounters {
    fn node_entry(&mut self, _node: usize) {}
    fn batch(&mut self, _node: usize, _len: usize) {}
    fn cover_choice(&mut self, _node: usize, _subatom: usize, _exact: bool) {}
    fn probe_hash(&mut self, _node: usize, _subatom: usize) {}
    fn probe(&mut self, _node: usize, _subatom: usize, _hit: bool) {}
    fn residual(&mut self, _node: usize, _pass: bool) {}
    fn anti_probe(&mut self, _node: usize, _hit: bool) {}
    fn emit(&mut self) {
        self.emits += 1;
    }
    fn emits(&self) -> u64 {
        self.emits
    }
    fn skip(&mut self, _node: usize) {}
    fn fixpoint_delta(&mut self, predicate: u16, rows: u64) {
        self.pending_deltas.push(DeltaRows { predicate, rows });
    }
    fn fixpoint_round(&mut self, stratum: u16, emitted: u64, absorbed: u64) {
        let round = RoundStats {
            deltas: std::mem::take(&mut self.pending_deltas),
            emitted,
            absorbed,
        };
        match self.strata.last_mut() {
            Some(entry) if entry.stratum == stratum => entry.rounds.push(round),
            _ => self.strata.push(StratumStats {
                stratum,
                rounds: vec![round],
            }),
        }
    }
}
