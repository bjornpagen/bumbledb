//! D2 origin cancellation bookkeeping and the typed execution poison.
use super::{Executor, Poison};

impl Executor {
    /// every loop before a second site can fire) and always paired with
    pub(super) fn poison(&mut self, poison: Poison) {
        if !matches!(self.drive_state, super::DriveState::Poisoned(_)) {
            self.drive_state = super::DriveState::Poisoned(poison);
        }
    }

    /// origin cancelled: the same silent-drop hazard the origin mint
    /// `lean/Bumbledb/Exec/Plan.lean: valid_plan_sound` requires.
    pub(super) fn advance_cancel_epoch(&mut self) {
        self.cancel_epoch = self.cancel_epoch.wrapping_add(1);
        if self.cancel_epoch == 0 {
            self.cancelled.clear();
        }
    }

    pub(super) fn origin_cancelled(&self, origin: u32) -> bool {
        self.cancelled
            .get(origin as usize)
            .is_some_and(|&e| e == self.cancel_epoch)
    }

    pub(super) fn cancel_origin(&mut self, origin: u32) {
        let idx = origin as usize;
        if self.cancelled.len() <= idx {
            self.cancelled
                .resize(idx + 1, self.cancel_epoch.wrapping_sub(1));
        }
        self.cancelled[idx] = self.cancel_epoch;
    }
}
