//! Schema descriptors, declaration validation, and the fingerprint (docs/architecture).
//!
//! Construction is the validation boundary (parse, don't validate): the only
//! way to obtain a [`Schema`] is [`SchemaDescriptor::validate`], and everything
//! downstream trusts the sealed witness without re-checking.

pub mod fingerprint;
pub mod render;
pub(crate) mod runtime;

mod relation;
#[cfg(test)]
mod tests;
mod type_desc;
mod validate;

use crate::encoding::FactLayout;

/// Dense relation id: the relation's index in schema declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelationId(pub u32);

/// Dense field id: the field's index in its relation's declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(pub u16);

/// Dense statement id: the statement's index in the schema-global
/// materialized order — serial auto-[`StatementDescriptor::Functionality`]
/// statements first, then declared statements in declaration order
/// ([`SchemaDescriptor::materialized_statements`] owns the rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatementId(pub u16);

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

/// A selection literal: one variant per structural type
/// (`docs/architecture/30-dependencies.md` — any type's literal binds in σ).
/// Enum carries the resolved ordinal; String carries UTF-8 bytes; Interval
/// carries `(start, end)` in the element domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralValue {
    Bool(bool),
    U64(u64),
    I64(i64),
    Enum(u8),
    IntervalU64(u64, u64),
    IntervalI64(i64, i64),
    String(Box<[u8]>),
    Bytes(Box<[u8]>),
}

/// One side of a containment: the single-atom query `R(X | φ)`
/// (`docs/architecture/30-dependencies.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    pub relation: RelationId,
    /// π — ordered, the statement's written order.
    pub projection: Box<[FieldId]>,
    /// σ — a set of (field, literal) equality bindings; empty = unselected.
    pub selection: Box<[(FieldId, LiteralValue)]>,
}

/// One dependency statement: a judgment about queries
/// (`docs/architecture/30-dependencies.md`). Statements are anonymous —
/// their identity is their materialized-order [`StatementId`]. There is no
/// bidirectional variant: `==` is lowered to two `Containment` statements
/// with the sides swapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementDescriptor {
    /// `R(X) -> R`: πX is injective on R. X is ordered (the order defines
    /// the guard key), non-empty, duplicate-free.
    Functionality {
        relation: RelationId,
        projection: Box<[FieldId]>,
    },
    /// `A(X | φ) <= B(Y | ψ)`: πX(σφ(A)) ⊆ πY(σψ(B)) as sets of tuples.
    Containment { source: Side, target: Side },
}

/// One declared relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub name: Box<str>,
    pub fields: Vec<FieldDescriptor>,
}

/// The schema as declared: input to validation. Statements are
/// schema-level, between relations, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDescriptor {
    pub relations: Vec<RelationDescriptor>,
    pub statements: Vec<StatementDescriptor>,
}

impl SchemaDescriptor {
    /// The materialized statement list — the one owner of the ordering rule
    /// pinned by the fingerprint (`docs/architecture/10-data-model.md`,
    /// § fingerprint inputs): one auto-`Functionality` per `Serial` field
    /// (relation declaration order, then field order; projection = the one
    /// serial field), followed by the declared statements in declaration
    /// order. [`StatementId`] = index into this list, schema-global.
    ///
    /// # Panics
    ///
    /// When a relation or field ordinal exceeds the id space (`u32`/`u16`)
    /// — impossible for a descriptor the acceptance gate admitted.
    #[must_use]
    pub fn materialized_statements(&self) -> Vec<StatementDescriptor> {
        let mut statements: Vec<StatementDescriptor> = Vec::new();
        for (rel_idx, relation) in self.relations.iter().enumerate() {
            for (field_idx, field) in relation.fields.iter().enumerate() {
                if field.generation == Generation::Serial {
                    statements.push(StatementDescriptor::Functionality {
                        relation: RelationId(
                            u32::try_from(rel_idx).expect("relation count fits u32"),
                        ),
                        projection: Box::new([FieldId(
                            u16::try_from(field_idx).expect("field count fits u16"),
                        )]),
                    });
                }
            }
        }
        statements.extend(self.statements.iter().cloned());
        statements
    }
}

/// The enforcement-plan data validation attaches to an accepted statement
/// (computed by the acceptance gate, `docs/architecture/30-dependencies.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Functionality {
        /// Index into the projection of its one interval field;
        /// `None` = scalar key.
        interval_position: Option<usize>,
    },
    Containment {
        /// The `Functionality` statement probed on the target.
        target_key: StatementId,
        /// Statement projection order -> target key order.
        key_permutation: Box<[u16]>,
        /// Positional index shared by both sides; `None` = scalar.
        interval_position: Option<usize>,
    },
}

/// One sealed statement: the descriptor plus its resolved enforcement data.
#[derive(Debug)]
pub struct Statement {
    pub descriptor: StatementDescriptor,
    pub resolved: Resolved,
}

/// One relation of a validated schema.
#[derive(Debug)]
pub struct Relation {
    name: Box<str>,
    fields: Box<[FieldDescriptor]>,
    layout: FactLayout,
    /// `Functionality` statements on this relation, in materialized order.
    keys: Box<[StatementId]>,
    /// `Containment` statements whose source is this relation.
    outgoing: Box<[StatementId]>,
    /// `Containment` statements whose target is this relation — the
    /// delete-side reverse-edge scan set (`docs/architecture/50-storage.md`).
    incoming: Box<[StatementId]>,
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug)]
pub struct Schema {
    relations: Box<[Relation]>,
    /// The materialized statement list; [`StatementId`] indexes it.
    statements: Box<[Statement]>,
    /// `target_key -> dependents`: per statement, the `Containment`
    /// statements whose resolved [`Resolved::Containment::target_key`] is
    /// that statement — the target-side reverse-edge check set
    /// (`docs/architecture/30-dependencies.md` § enforcement). Empty for
    /// every non-key statement. [`StatementId`] indexes it.
    dependents: Box<[Box<[StatementId]>]>,
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
    /// dynamic surface where the id is data (`70-api.md`).
    #[must_use]
    pub fn relation_checked(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(id.0 as usize)
    }

    /// The sealed statements, in materialized order.
    #[must_use]
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    /// The statement for a validated id.
    ///
    /// # Panics
    ///
    /// On an out-of-range id — internal callers only.
    #[must_use]
    pub fn statement(&self, id: StatementId) -> &Statement {
        &self.statements[usize::from(id.0)]
    }

    /// The `Containment` statements whose resolved target key is `id` —
    /// the set the commit pipeline's target side walks when a key tuple is
    /// disestablished (`docs/architecture/30-dependencies.md`
    /// § enforcement). Empty unless `id` is a `Functionality` statement
    /// some containment resolved to.
    ///
    /// # Panics
    ///
    /// On an out-of-range id — internal callers only.
    #[must_use]
    pub fn dependents(&self, id: StatementId) -> &[StatementId] {
        &self.dependents[usize::from(id.0)]
    }
}
