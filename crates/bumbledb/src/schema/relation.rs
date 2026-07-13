//! Field, layout, and statement-index accessors on a validated relation.

use super::{ContainmentId, FactLayout, FieldDescriptor, FieldId, KeyId, Relation, SealedRow};

impl Relation {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The sealed ground axioms of a closed relation, in declaration order
    /// (row id = index); `None` = ordinary. The option *is* the kind —
    /// there is no relation-kind enum
    /// (`docs/architecture/10-data-model.md` § closed relations).
    #[must_use]
    pub fn extension(&self) -> Option<&[SealedRow]> {
        self.extension.as_deref()
    }

    /// Whether the relation is closed: rows are ground axioms — frozen by
    /// the fingerprint, virtual in storage, write-refused.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.extension.is_some()
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
    pub fn keys(&self) -> &[KeyId] {
        &self.keys
    }

    /// `Containment` statements whose source is this relation.
    #[must_use]
    pub fn outgoing(&self) -> &[ContainmentId] {
        &self.outgoing
    }
}
