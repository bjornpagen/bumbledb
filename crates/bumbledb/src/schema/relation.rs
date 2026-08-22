//! Field, layout, and statement-index accessors on a validated relation.
use super::{
    CapacityId, ContainmentId, FactLayout, FieldDescriptor, FieldId, KeyId, Relation, RelationBody,
    ValueType,
};

impl Relation {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// (ground axioms). Match; do not re-test a flag (CONTRACT C9).
    #[must_use]
    pub fn body(&self) -> &RelationBody {
        &self.body
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
    pub const fn layout(&self) -> &FactLayout {
        &self.layout
    }

    #[must_use]
    pub fn keys(&self) -> &[KeyId] {
        &self.keys
    }

    #[must_use]
    pub fn outgoing(&self) -> &[ContainmentId] {
        &self.outgoing
    }

    #[must_use]
    pub fn capacity_sources(&self) -> &[CapacityId] {
        &self.capacity_sources
    }

    #[must_use]
    pub fn capacity_targets(&self) -> &[CapacityId] {
        &self.capacity_targets
    }

    #[must_use]
    pub(crate) fn fresh_key(&self) -> Option<KeyId> {
        match self.body {
            RelationBody::Ordinary { fresh } => fresh,
            RelationBody::Closed { .. } => None,
        }
    }

    #[must_use]
    pub(crate) fn interval_tail(&self, projection: &[FieldId]) -> Option<ValueType> {
        projection.iter().find_map(|field| {
            let ty = self.field(*field).value_type;
            ty.is_interval().then_some(ty)
        })
    }
}
