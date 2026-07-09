use super::ValidatedPlan;
use crate::ir::VarId;

impl ValidatedPlan {
    /// The first slot index of a variable (its only slot for scalars; an
    /// interval variable's end word sits at `slot_of(var) + 1` — the
    /// two-slot layout, [`crate::ir::normalize::SlotWidth`]).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        let mut slot = 0;
        for (candidate, width) in &self.slots {
            if *candidate == var {
                return slot;
            }
            slot += width.slots();
        }
        panic!("validated plan binds every variable")
    }
}
