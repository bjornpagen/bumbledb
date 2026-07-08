//! Field, constraint, layout, and FK-target accessors on a validated relation.

use super::{ConstraintDescriptor, ConstraintId, FactLayout, FieldDescriptor, FieldId, Relation};

impl Relation {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn fields(&self) -> &[FieldDescriptor] {
        &self.fields
    }

    #[must_use]
    pub fn field(&self, id: FieldId) -> &FieldDescriptor {
        &self.fields[usize::from(id.0)]
    }

    #[must_use]
    pub fn constraints(&self) -> &[ConstraintDescriptor] {
        &self.constraints
    }

    #[must_use]
    pub fn constraint(&self, id: ConstraintId) -> &ConstraintDescriptor {
        &self.constraints[usize::from(id.0)]
    }

    /// The relation's fact byte layout (fields in declaration order).
    #[must_use]
    pub const fn layout(&self) -> &FactLayout {
        &self.layout
    }

    /// Ids of this relation's `Unique` constraints (auto-materialized and
    /// declared alike).
    #[must_use]
    pub fn unique_constraints(&self) -> &[ConstraintId] {
        &self.unique_constraints
    }

    /// Unique constraints of this relation that some FK targets.
    #[must_use]
    pub fn fk_targeted(&self) -> &[ConstraintId] {
        &self.fk_targeted
    }
}
