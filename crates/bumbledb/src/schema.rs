//! Schema descriptors, declaration validation, and the fingerprint (docs/architecture).
//!
//! Construction is the validation boundary (parse, don't validate): the only
//! way to obtain a [`Schema`] is [`SchemaDescriptor::validate`], and everything
//! downstream trusts the sealed witness without re-checking.

pub mod fingerprint;
pub(crate) mod runtime;

mod relation;
mod type_desc;
mod validate;
#[cfg(test)]
mod tests;

use crate::encoding::FactLayout;

/// Dense relation id: the relation's index in schema declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelationId(pub u32);

/// Dense field id: the field's index in its relation's declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(pub u16);

/// Dense constraint id: the constraint's index in its relation's constraint
/// list — auto-materialized serial uniques first (in field declaration
/// order), then declared constraints in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstraintId(pub u16);

/// The element domain of an Interval: closed to the two orderable scalars.
/// A flat enum, deliberately — no `Interval(Box<ValueType>)` recursion, so
/// illegal elements are unrepresentable rather than rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalElement {
    U64,
    I64,
}

/// A structural value type: the description *is* the identity — structural
/// equality of the description is type equality, and there is no name field
/// anywhere (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    Bool,
    /// Identity is the ordered variant-name list: two fields declaring the
    /// same list are the same type, whatever the schema calls them.
    Enum {
        variants: Box<[Box<str>]>,
    },
    U64,
    I64,
    String,
    Bytes,
    /// A half-open `[start, end)` over the element domain, strictly
    /// `start < end` — a finite set of points, written as its bounds
    /// (`docs/architecture/10-data-model.md`).
    Interval {
        element: IntervalElement,
    },
}

/// Field generation: a storage behavior, not a type
/// (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Generation {
    /// Ordinary field: the application supplies every value.
    None,
    /// The database mints values: monotonic per (relation, field), never
    /// re-issuing a value observable in a committed state. Must be `U64`.
    Serial,
}

/// One field: name + structural type + generation attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: Box<str>,
    pub value_type: ValueType,
    pub generation: Generation,
}

/// One declared constraint. Field lists are ordered (the order defines the
/// guard key and the FK target shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintDescriptor {
    Unique {
        name: Box<str>,
        fields: Box<[FieldId]>,
    },
    ForeignKey {
        name: Box<str>,
        fields: Box<[FieldId]>,
        target_relation: RelationId,
        /// Must name a `Unique` constraint of the target relation. Note the
        /// id numbering rule on [`ConstraintId`]: auto-materialized serial
        /// uniques come first.
        target_constraint: ConstraintId,
    },
}

impl ConstraintDescriptor {
    /// The constraint's name (scoped per relation).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Unique { name, .. } | Self::ForeignKey { name, .. } => name,
        }
    }

    /// The constraint's ordered field list.
    #[must_use]
    pub fn fields(&self) -> &[FieldId] {
        match self {
            Self::Unique { fields, .. } | Self::ForeignKey { fields, .. } => fields,
        }
    }
}

/// One declared relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub name: Box<str>,
    pub fields: Vec<FieldDescriptor>,
    pub constraints: Vec<ConstraintDescriptor>,
}

/// The schema as declared: input to validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDescriptor {
    pub relations: Vec<RelationDescriptor>,
}

/// One relation of a validated schema.
#[derive(Debug)]
pub struct Relation {
    name: Box<str>,
    fields: Box<[FieldDescriptor]>,
    /// Auto-materialized serial uniques first, then declared constraints.
    constraints: Box<[ConstraintDescriptor]>,
    layout: FactLayout,
    /// Ids of this relation's `Unique` constraints.
    unique_constraints: Box<[ConstraintId]>,
    /// Unique constraints of *this* relation targeted by some FK anywhere in
    /// the schema — the delete-side Restrict scan set (the 40-storage doc's reader).
    fk_targeted: Box<[ConstraintId]>,
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug)]
pub struct Schema {
    relations: Box<[Relation]>,
}

impl Schema {
    #[must_use]
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }

    /// The relation for a plan- or macro-derived id (every internal id
    /// is dense and validated).
    ///
    /// # Panics
    ///
    /// On an out-of-range id — internal callers only; the dynamic (ETL)
    /// surface bounds-checks through [`Schema::relation_checked`] first.
    #[must_use]
    pub fn relation(&self, id: RelationId) -> &Relation {
        &self.relations[id.0 as usize]
    }

    /// The bounds-checked sibling of [`Schema::relation`], for the
    /// dynamic surface where the id is data (`60-api.md`).
    #[must_use]
    pub fn relation_checked(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(id.0 as usize)
    }
}
