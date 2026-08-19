//! Field, layout, and statement-index accessors on a validated relation.

use super::{
    CapacityId, ContainmentId, FactLayout, FieldDescriptor, FieldId, IntervalTail, KeyId, Relation,
    RelationBody,
};

impl Relation {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The sealed kind: ordinary (optional fresh-row mint) or closed
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

    /// `Capacity` statements whose source (weighed child) is this
    /// relation.
    #[must_use]
    pub fn capacity_sources(&self) -> &[CapacityId] {
        &self.capacity_sources
    }

    /// `Capacity` statements whose target (parent) is this relation.
    #[must_use]
    pub fn capacity_targets(&self) -> &[CapacityId] {
        &self.capacity_targets
    }

    /// The [`KeyForm::FreshRow`] key on an ordinary relation, if this
    /// relation is the one id allocator's mint (R16). Closed relations
    /// have none — identity is the handle.
    #[must_use]
    pub(crate) fn fresh_key(&self) -> Option<KeyId> {
        match self.body {
            RelationBody::Ordinary { fresh } => fresh,
            RelationBody::Closed { .. } => None,
        }
    }

    /// The interval-tail descriptor of a projection over this relation:
    /// `Some` when the projection carries an interval-typed field (the
    /// acceptance gate makes it unique and final for keys, so the tail is
    /// the determinant's trailing encoding), describing how many trailing
    /// bytes the interval occupies and how its end derives — 16 general
    /// (`start ‖ end`), 8 fixed (`interval<E, w>`: the start word; the
    /// end is `start + w`, the width being the type's).
    #[must_use]
    pub(crate) fn interval_tail(&self, projection: &[FieldId]) -> Option<IntervalTail> {
        projection
            .iter()
            .find_map(|field| IntervalTail::of(self.field(*field).value_type))
    }
}
