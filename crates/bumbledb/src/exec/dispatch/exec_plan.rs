use super::ExecPlan;
use crate::ir::VarId;

impl ExecPlan {
    /// The slot index of a variable.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        match self {
            Self::GuardProbe(guard) => guard
                .vars
                .iter()
                .position(|(_, v)| *v == var)
                .expect("guard plans bind every variable"),
            Self::FreeJoin(plan) => plan.slot_of(var),
        }
    }

    /// A variable's slot width in words — the layout map's companion to
    /// [`Self::slot_of`] (2 for an interval variable, the `SlotWidth`
    /// layout). Guard-plan slots are field-indexed and one word wide
    /// (the statement-driven guard path is PRD 19's).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn width_of(&self, var: VarId) -> usize {
        match self {
            Self::GuardProbe(_) => 1,
            Self::FreeJoin(plan) => plan.width_of(var),
        }
    }

    /// The distinct-bindings elision flag (trivially true for a guard
    /// probe: at most one binding exists).
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        match self {
            Self::GuardProbe(_) => true,
            Self::FreeJoin(plan) => plan.distinct_bindings(),
        }
    }
}
