use super::ValidatedPlan;
use crate::ir::VarId;

impl ValidatedPlan {
    /// The slot index of a variable.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        self.slots
            .iter()
            .position(|v| *v == var)
            .expect("validated plan binds every variable")
    }
}
