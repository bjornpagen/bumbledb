//! The reach profile's counter: collects the driver's per-round delta
//! size and union accounting through the `Counters` seam's reach hooks.
//! Node-level methods are no-ops by design — one counter spans many
//! differently shaped plan units, so per-node attribution has no stable
//! index space; the emit count and the round structure are the honest
//! surface.

use super::ReachCounters;
use crate::api::stats::{ReachStats, RoundStats};
use crate::exec::run::Counters;

impl ReachCounters {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The collected round records, round 0 = base.
    #[must_use]
    pub fn into_reach(self, rules: Vec<crate::api::stats::RuleStats>) -> ReachStats {
        debug_assert!(
            self.pending_delta == 0 || !self.rounds.is_empty(),
            "every reported delta belongs to a closed round"
        );
        ReachStats {
            rules,
            rounds: self.rounds,
        }
    }
}

impl Counters for ReachCounters {
    fn node_entry(&mut self, _node: usize) {}
    fn batch(&mut self, _node: usize, _len: usize) {}
    fn cover_choice(&mut self, _node: usize, _subatom: usize, _count: crate::exec::colt::KeyCount) {
    }
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
    fn fixpoint_delta(&mut self, rows: u64) {
        self.pending_delta = rows;
    }
    fn fixpoint_round(&mut self, emitted: u64, absorbed: u64) {
        let delta = std::mem::take(&mut self.pending_delta);
        self.rounds.push(RoundStats {
            delta,
            emitted,
            absorbed,
        });
    }
}
