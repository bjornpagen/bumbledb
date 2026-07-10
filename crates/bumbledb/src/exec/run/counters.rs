//! The execution observability counters (docs/architecture/60-validation.md).

use super::Counters;
use super::NoopCounters;
#[cfg(feature = "trace")]
use super::{JoinPhase, PhaseTimers, PHASE_NODE_CAP};

#[cfg(feature = "trace")]
impl JoinPhase {
    /// Index into per-phase tables (matches `obs::names::JOIN_PHASE`).
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Iter => 0,
            Self::Hash => 1,
            Self::Probe => 2,
            Self::Residual => 3,
            Self::Descend => 4,
            Self::Force => 5,
        }
    }
}

#[cfg(feature = "trace")]
impl PhaseTimers {
    #[must_use]
    pub fn new() -> Self {
        Self {
            acc: [[(0, 0); 6]; PHASE_NODE_CAP + 1],
            open: [[0; 6]; PHASE_NODE_CAP + 1],
            emits: 0,
        }
    }

    /// Emits one `Category::Phase` point event per touched (node, phase):
    /// `a0` = accumulated nanoseconds, `a1` = calls.
    pub fn flush(&self) {
        for (node, phases) in self.acc.iter().enumerate() {
            for (phase, &(ticks, calls)) in phases.iter().enumerate() {
                if calls == 0 {
                    continue;
                }
                crate::obs::event(
                    crate::obs::names::JOIN_PHASE[phase][node],
                    crate::obs::Category::Phase,
                    crate::obs::fastclock::ticks_to_ns(ticks),
                    calls,
                );
            }
        }
    }
}

#[cfg(feature = "trace")]
impl Default for PhaseTimers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "trace")]
impl Counters for PhaseTimers {
    #[inline]
    fn node_entry(&mut self, _: usize) {}
    #[inline]
    fn batch(&mut self, _: usize, _: usize) {}
    #[inline]
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn probe_hash(&mut self, _: usize, _: usize) {}
    #[inline]
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn residual(&mut self, _: usize, _: bool) {}
    #[inline]
    fn anti_probe(&mut self, _: usize, _: bool) {}
    #[inline]
    fn emit(&mut self) {
        self.emits += 1;
    }
    #[inline]
    fn emits(&self) -> u64 {
        self.emits
    }
    #[inline]
    fn skip(&mut self, _: usize) {}
    #[inline]
    fn phase_start(&mut self, node: usize, phase: JoinPhase) {
        self.open[node.min(PHASE_NODE_CAP)][phase.index()] = crate::obs::fastclock::ticks();
    }
    #[inline]
    fn phase_end(&mut self, node: usize, phase: JoinPhase) {
        let (node, phase) = (node.min(PHASE_NODE_CAP), phase.index());
        let cell = &mut self.acc[node][phase];
        cell.0 += crate::obs::fastclock::ticks().wrapping_sub(self.open[node][phase]);
        cell.1 += 1;
    }
}

impl Counters for NoopCounters {
    #[inline]
    fn node_entry(&mut self, _: usize) {}
    #[inline]
    fn batch(&mut self, _: usize, _: usize) {}
    #[inline]
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn probe_hash(&mut self, _: usize, _: usize) {}
    #[inline]
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn residual(&mut self, _: usize, _: bool) {}
    #[inline]
    fn anti_probe(&mut self, _: usize, _: bool) {}
    #[inline]
    fn emit(&mut self) {}
    #[inline]
    fn skip(&mut self, _: usize) {}
}
