//! Field, layout, and statement-index accessors on a validated relation.

use super::{
    ContainmentId, FactLayout, FieldDescriptor, FieldId, IntervalTail, KeyId, Relation, SealedRow,
    ValueType, WindowId,
};

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

    /// `Cardinality` statements whose source (counted child) is this
    /// relation.
    #[must_use]
    pub fn window_sources(&self) -> &[WindowId] {
        &self.window_sources
    }

    /// `Cardinality` statements whose target (parent) is this relation.
    #[must_use]
    pub fn window_targets(&self) -> &[WindowId] {
        &self.window_targets
    }

    /// The first `Fresh`-generation field — the one id allocator's mint
    /// field (R16, `docs/architecture/50-storage.md` § key layout): on a
    /// fresh-keyed relation this field's value IS the `F` row id; `None`
    /// means row ids mint from the `S` high-water.
    #[must_use]
    pub(crate) fn fresh_row_field(&self) -> Option<FieldId> {
        self.fresh_row_field
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
            .find_map(|field| match self.field(*field).value_type {
                ValueType::Interval { width, .. } => Some(IntervalTail { width }),
                _ => None,
            })
    }
}
