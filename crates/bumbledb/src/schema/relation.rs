//! Field, layout, and statement-index accessors on a validated relation.

use super::{FactLayout, FieldDescriptor, FieldId, Relation, StatementId};

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

    /// The relation's fact byte layout (fields in declaration order).
    #[must_use]
    pub const fn layout(&self) -> &FactLayout {
        &self.layout
    }

    /// `Functionality` statements on this relation (auto-materialized and
    /// declared alike), in materialized order.
    #[must_use]
    pub fn keys(&self) -> &[StatementId] {
        &self.keys
    }

    /// `Containment` statements whose source is this relation.
    #[must_use]
    pub fn outgoing(&self) -> &[StatementId] {
        &self.outgoing
    }

    /// `Containment` statements whose target is this relation — the
    /// delete-side reverse-edge scan set (`docs/architecture/50-storage.md`).
    #[must_use]
    pub fn incoming(&self) -> &[StatementId] {
        &self.incoming
    }
}
