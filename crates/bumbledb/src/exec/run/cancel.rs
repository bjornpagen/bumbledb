//! D2 origin cancellation bookkeeping (docs/perf/ PRD 10).

use super::Executor;

impl Executor {
    /// Whether an origin's subtree was cancelled (PRD 10).
    pub(super) fn origin_cancelled(&self, origin: u32) -> bool {
        self.cancelled
            .get(origin as usize)
            .is_some_and(|&e| e == self.cancel_epoch)
    }

    /// Cancels one origin's subtree.
    pub(super) fn cancel_origin(&mut self, origin: u32) {
        let idx = origin as usize;
        if self.cancelled.len() <= idx {
            self.cancelled
                .resize(idx + 1, self.cancel_epoch.wrapping_sub(1));
        }
        self.cancelled[idx] = self.cancel_epoch;
    }
}
