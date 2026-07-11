//! Schema descriptors, declaration validation, and the fingerprint
//! (`docs/architecture/10-data-model.md`, `docs/architecture/30-dependencies.md`).
//!
//! Construction is the validation boundary (parse, don't validate): the only
//! way to obtain a [`Schema`] is [`SchemaDescriptor::validate`], and everything
//! downstream trusts the sealed witness without re-checking.

pub mod fingerprint;
pub mod render;

mod relation;
#[cfg(test)]
mod tests;
mod type_desc;
mod validate;

use crate::encoding::FactLayout;
use crate::error::FactShapeError;
use crate::value::Value;

/// Dense relation id: the relation's index in schema declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelationId(pub u32);

/// Dense field id: the field's index in its relation's declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(pub u16);

/// Dense statement id: the statement's index in the schema-global
/// materialized order — fresh auto-[`StatementDescriptor::Functionality`]
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
    /// `bytes<N>`: exactly `len` raw bytes, identity-shaped — stored
    /// inline in the fact, word-padded, never interned (*intern what
    /// repeats; inline what identifies* —
    /// `docs/architecture/10-data-model.md`). The length is part of the
    /// type: `bytes<16>` and `bytes<32>` are different types, and the
    /// fingerprint feeds the length (a width change is a new theory).
    /// `len` is validated to `1..=64` at declaration.
    FixedBytes {
        len: u16,
    },
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
    Fresh,
}

/// A witness that `(relation, field)` names a `Fresh`-generation field of
/// this schema: the proof-carrying handle of the untyped mint path
/// ([`crate::WriteTx::alloc_at`]). Fields are private and
/// [`Schema::fresh_field`] is the one construction site — holding a value
/// *is* the proof, so the mint path never re-checks the generation (parse,
/// don't validate; the ETL access pattern is resolve once per relation,
/// mint per row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshField {
    relation: RelationId,
    field: FieldId,
}

impl FreshField {
    pub(crate) fn relation(self) -> RelationId {
        self.relation
    }

    pub(crate) fn field(self) -> FieldId {
        self.field
    }
}

/// One field: name + structural type + generation attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: Box<str>,
    pub value_type: ValueType,
    pub generation: Generation,
}

/// How a [`Value`] failed to match an expected [`ValueType`] — the shared
/// vocabulary of the checking boundaries (query literals, bound params,
/// dynamic facts, statement selections).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueMismatch {
    /// Wrong structural kind.
    Type,
    /// Enum ordinal at or beyond the variant count.
    EnumOrdinal(u8),
    /// `Value::String` bytes are not UTF-8 (the type's contract).
    Utf8,
    /// Interval bounds with `start >= end` — the empty interval denotes
    /// no points and is unrepresentable
    /// (`docs/architecture/10-data-model.md`).
    IntervalEmpty,
}

/// The one `Value` ↔ `ValueType` compatibility check (kind, enum ordinal
/// range, String UTF-8, interval non-emptiness) — IR validation, bind-time,
/// the dynamic write path, and selection validation all call this so the
/// rules cannot drift apart. Note the membership rule is *not* here: an
/// element-typed value against an `Interval` field is a kind mismatch to
/// this check, and the IR validation boundary owns that bivalence
/// (`ir::validate`, the bivalent-anchor resolution).
pub(crate) fn value_matches(value: &Value, expected: &ValueType) -> Result<(), ValueMismatch> {
    match (value, expected) {
        (Value::Bool(_), ValueType::Bool)
        | (Value::U64(_), ValueType::U64)
        | (Value::I64(_), ValueType::I64) => Ok(()),
        // The length is the type: a bytes<N> literal of any other width
        // is a kind mismatch, exactly like a wrong variant.
        (Value::FixedBytes(raw), ValueType::FixedBytes { len }) => {
            if raw.len() == usize::from(*len) {
                Ok(())
            } else {
                Err(ValueMismatch::Type)
            }
        }
        (Value::String(raw), ValueType::String) => {
            if std::str::from_utf8(raw).is_ok() {
                Ok(())
            } else {
                Err(ValueMismatch::Utf8)
            }
        }
        (Value::Enum(ordinal), ValueType::Enum { variants }) => {
            if usize::from(*ordinal) < variants.len() {
                Ok(())
            } else {
                Err(ValueMismatch::EnumOrdinal(*ordinal))
            }
        }
        (
            Value::IntervalU64(start, end),
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        ) => {
            if start < end {
                Ok(())
            } else {
                Err(ValueMismatch::IntervalEmpty)
            }
        }
        (
            Value::IntervalI64(start, end),
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ) => {
            if start < end {
                Ok(())
            } else {
                Err(ValueMismatch::IntervalEmpty)
            }
        }
        _ => Err(ValueMismatch::Type),
    }
}

/// One side of a containment: the single-atom query `R(X | φ)`
/// (`docs/architecture/30-dependencies.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    pub relation: RelationId,
    /// π — ordered, the statement's written order.
    pub projection: Box<[FieldId]>,
    /// σ — a set of (field, literal) equality bindings; empty = unselected.
    /// Literals are the one shared [`Value`] sum
    /// (`docs/architecture/30-dependencies.md` — any type's literal binds
    /// in σ; dependencies and queries share one representation).
    pub selection: Box<[(FieldId, Value)]>,
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

/// A named theory — a schema names a theory (relations plus statements)
/// and a store models it (`docs/architecture/10-data-model.md`): the
/// value [`crate::Db::create`] and [`crate::Db::open`] take, and the
/// type that names the database in [`crate::Db<S>`]'s typestate. The
/// `schema!` macro emits one unit
/// struct per invocation (`pub Ledger;` → `pub struct Ledger;` with this
/// impl), so a fact of schema A cannot reach a database of schema B —
/// the mismatch is a compile error, not a lucky width check.
///
/// Validation happens where the definition is consumed:
/// `Db::create`/`open` run [`SchemaDescriptor::validate`] and surface an
/// invalid declaration as the typed [`crate::error::SchemaError`] — no
/// panic path, no memoization.
///
/// [`SchemaDescriptor`] implements the trait as itself: a runtime-built
/// descriptor (ETL tooling, test fixtures) is its own definition. All
/// such databases share the `Db<SchemaDescriptor>` state — dynamic
/// schemas get the dynamic surface's runtime checks, not typestate.
pub trait Theory: Sized {
    /// The schema as declared. Consumes the definition value —
    /// implementers are unit structs or one-shot carriers.
    fn descriptor(self) -> SchemaDescriptor;
}

impl Theory for SchemaDescriptor {
    fn descriptor(self) -> SchemaDescriptor {
        self
    }
}

impl SchemaDescriptor {
    /// The materialized statement list — the one owner of the ordering rule
    /// pinned by the fingerprint (`docs/architecture/10-data-model.md`,
    /// § fingerprint inputs): one auto-`Functionality` per `Fresh` field
    /// (relation declaration order, then field order; projection = the one
    /// fresh field), followed by the declared statements in declaration
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
                if field.generation == Generation::Fresh {
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

/// One sealed statement: the descriptor plus its resolved enforcement data
/// and its `==` pairing.
#[derive(Debug)]
pub struct Statement {
    pub descriptor: StatementDescriptor,
    pub resolved: Resolved,
    /// The `==` partner: the containment whose sides are exactly this
    /// statement's sides swapped, anywhere in the materialized list —
    /// `==` lowers to two containments and the pairing is a fact of the
    /// declaration, sealed here rather than re-discovered by render-time
    /// search (`docs/architecture/30-dependencies.md`). At most one
    /// partner can exist because [`SchemaError::DuplicateStatement`]
    /// rejects identical normalized statements (two candidate mirrors
    /// would be identical to each other), which makes the links
    /// symmetric. `None` for every FD and one-way containment.
    ///
    /// [`SchemaError::DuplicateStatement`]: crate::error::SchemaError::DuplicateStatement
    pub mirror: Option<StatementId>,
}

impl Statement {
    /// The projection of a key (`Functionality`) statement — the guard
    /// tuple's field order (readers: the commit applier's guard
    /// derivation, `Db::verify_store`'s re-derivation).
    ///
    /// # Panics
    ///
    /// On a `Containment` — callers hold ids from [`Relation::keys`],
    /// which the validated schema fills with `Functionality` statements.
    #[must_use]
    pub(crate) fn key_projection(&self) -> &[FieldId] {
        let StatementDescriptor::Functionality { projection, .. } = &self.descriptor else {
            unreachable!("validated schema: relation keys are Functionality statements")
        };
        projection
    }
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

    /// Resolves `(relation, field)` to the [`FreshField`] witness — ids
    /// and generation validated here, once; every
    /// [`crate::WriteTx::alloc_at`] mint thereafter carries the proof
    /// instead of re-checking it (`70-api.md` § ETL).
    ///
    /// # Errors
    ///
    /// `UnknownRelation`/`UnknownField` on an out-of-range id;
    /// `NotAFreshField` when the field's generation is not `Fresh`.
    pub fn fresh_field(
        &self,
        relation: RelationId,
        field: FieldId,
    ) -> Result<FreshField, FactShapeError> {
        let Some(rel) = self.relation_checked(relation) else {
            return Err(FactShapeError::UnknownRelation { relation });
        };
        let Some(descriptor) = rel.fields().get(usize::from(field.0)) else {
            return Err(FactShapeError::UnknownField { relation, field });
        };
        if descriptor.generation != Generation::Fresh {
            return Err(FactShapeError::NotAFreshField { relation, field });
        }
        Ok(FreshField { relation, field })
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

    /// The projection of a key statement — the one place an id from
    /// [`Relation::keys`] is unpacked to its field list (guard byte
    /// order, coverage checks, the planner's key var sets).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an id naming a non-key
    /// statement — `Relation::keys()` indexes `Functionality` statements
    /// only.
    #[must_use]
    pub fn key_projection(&self, id: StatementId) -> &[FieldId] {
        match &self.statement(id).descriptor {
            StatementDescriptor::Functionality { projection, .. } => projection,
            StatementDescriptor::Containment { .. } => {
                unreachable!("Relation::keys() indexes Functionality statements")
            }
        }
    }
}
