use super::{OccId, PlanOccurrence, ValidatedPlan};

impl ValidatedPlan {
    /// # Panics
    ///
    /// On a programmer-invariant violation: an occurrence outside the plan.
    #[cfg(test)]
    #[must_use]
    pub fn occurrence(&self, occ: OccId) -> &PlanOccurrence {
        self.occurrences
            .iter()
            .find(|o| o.occ_id == occ)
            .expect("validated plan covers its occurrences")
    }
}
