use super::ExecPlan;
use crate::ir::VarId;

impl ExecPlan {
    /// The first slot index of a variable.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        match self {
            Self::GuardProbe(guard) => {
                guard
                    .vars
                    .iter()
                    .find(|v| v.var == var)
                    .expect("guard plans bind every variable")
                    .slot
            }
            Self::FreeJoin(plan) => plan.slot_of(var),
        }
    }

    /// A variable's slot width in words — the layout map's companion to
    /// [`Self::slot_of`] (2 for an interval variable, the `SlotWidth`
    /// layout; guard plans carry it per decoded variable).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn width_of(&self, var: VarId) -> usize {
        match self {
            Self::GuardProbe(guard) => {
                guard
                    .vars
                    .iter()
                    .find(|v| v.var == var)
                    .expect("guard plans bind every variable")
                    .width
            }
            Self::FreeJoin(plan) => plan.width_of(var),
        }
    }

    /// The plan's binding-slot count in **words** (the `SlotWidth`
    /// layout) — the rule loop sizes the shared binding scratch with it.
    #[must_use]
    pub fn slot_count(&self) -> usize {
        match self {
            Self::GuardProbe(guard) => guard.slot_count(),
            Self::FreeJoin(plan) => plan.slot_count(),
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
