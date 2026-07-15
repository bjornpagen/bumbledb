//! D2 origin cancellation bookkeeping.

use super::Executor;

impl Executor {
    /// Advances the per-execution cancellation epoch. On wrap-around
    /// the high-water table is cleared — one cold `O(len)` pass every
    /// 2³² executions — because a stamp from the previous epoch cycle
    /// would otherwise alias the recycled epoch value and mark a LIVE
    /// origin cancelled: the same silent-drop hazard the origin mint
    /// guard refuses with the typed `Overflow` (`probe_pass`), and a
    /// false cancellation removes members of the answer set that
    /// `lean/Bumbledb/Exec/Plan.lean: valid_plan_sound` requires.
    /// Widening the epoch to u64 was rejected for the same reason as
    /// widening origins: the `cancelled` table is measured hot-path
    /// bytes, and the clear is free amortized.
    pub(super) fn advance_cancel_epoch(&mut self) {
        self.cancel_epoch = self.cancel_epoch.wrapping_add(1);
        if self.cancel_epoch == 0 {
            self.cancelled.clear();
        }
    }

    /// Whether an origin's subtree was cancelled.
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
