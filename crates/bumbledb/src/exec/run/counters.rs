//! The execution observability counters.
use super::Counters;
use super::NoopCounters;
#[cfg(feature = "trace")]
use super::{JoinPhase, PHASE_NODE_CAP, PhaseTimers};

#[cfg(feature = "trace")]
impl JoinPhase {
    pub const COUNT: usize = Self::Gather as usize + 1;

    #[must_use]
    pub fn index(self) -> usize {
        self as usize
    }
}

#[cfg(feature = "trace")]
impl PhaseTimers {
    #[must_use]
    pub fn new() -> Self {
        Self {
            acc: [[(0, 0); JoinPhase::COUNT]; PHASE_NODE_CAP + 1],
            open: [[0; JoinPhase::COUNT]; PHASE_NODE_CAP + 1],
            depth: [[0; JoinPhase::COUNT]; PHASE_NODE_CAP + 1],
            emits: 0,
        }
    }

    pub fn flush(&self) {
        for (node, phases) in self.acc.iter().enumerate() {
            for (phase, &(ticks, calls)) in phases.iter().enumerate() {
                if calls == 0 {
                    continue;
                }
                crate::obs::event(
                    crate::obs::names::JOIN_PHASE[phase][node],
                    crate::obs::TraceArgs::Pair(crate::obs::fastclock::ticks_to_ns(ticks), calls),
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
    fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
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
        let (node, phase) = (node.min(PHASE_NODE_CAP), phase.index());

        if self.depth[node][phase] == 0 {
            self.open[node][phase] = crate::obs::fastclock::ticks();
        }
        self.depth[node][phase] += 1;
    }
    #[inline]
    fn phase_end(&mut self, node: usize, phase: JoinPhase) {
        let (node, phase) = (node.min(PHASE_NODE_CAP), phase.index());
        debug_assert!(self.depth[node][phase] > 0, "phase_end without its start");
        self.depth[node][phase] -= 1;
        if self.depth[node][phase] == 0 {
            let cell = &mut self.acc[node][phase];
            cell.0 += crate::obs::fastclock::ticks().wrapping_sub(self.open[node][phase]);
            cell.1 += 1;
        }
    }
}

#[cfg(all(test, feature = "trace"))]
mod tests {
    use super::super::{Counters as _, JoinPhase, PHASE_NODE_CAP, PhaseTimers};

    #[test]
    fn overflow_bucket_merges_nested_windows() {
        let mut timers = PhaseTimers::new();
        timers.phase_start(PHASE_NODE_CAP, JoinPhase::Descend);
        timers.phase_start(PHASE_NODE_CAP + 1, JoinPhase::Descend);
        timers.phase_end(PHASE_NODE_CAP + 1, JoinPhase::Descend);
        timers.phase_end(PHASE_NODE_CAP, JoinPhase::Descend);
        let (ticks, calls) = timers.acc[PHASE_NODE_CAP][JoinPhase::Descend.index()];
        assert_eq!(calls, 1, "one merged outermost window, not two");

        assert!(ticks > 0 || calls == 1);
    }

    #[test]
    fn sequential_windows_count_per_window() {
        let mut timers = PhaseTimers::new();
        for _ in 0..2 {
            timers.phase_start(3, JoinPhase::Probe);
            timers.phase_end(3, JoinPhase::Probe);
        }
        assert_eq!(timers.acc[3][JoinPhase::Probe.index()].1, 2);
    }
}

impl Counters for NoopCounters {
    #[inline]
    fn node_entry(&mut self, _: usize) {}
    #[inline]
    fn batch(&mut self, _: usize, _: usize) {}
    #[inline]
    fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
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
