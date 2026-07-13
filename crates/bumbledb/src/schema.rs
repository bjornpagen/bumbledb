//! Schema descriptors, declaration validation, and the fingerprint
//! (`docs/architecture/10-data-model.md`, `docs/architecture/30-dependencies.md`).
//!
//! Construction is the validation boundary (parse, don't validate): the only
//! way to obtain a [`Schema`] is [`SchemaDescriptor::validate`], and everything
//! downstream trusts the sealed witness without re-checking.

pub mod fingerprint;
pub mod manifest;
pub mod render;

mod relation;
#[cfg(test)]
mod tests;
mod type_desc;
mod validate;

use crate::encoding::FactLayout;
use crate::error::FactShapeError;
use crate::value::Value;

pub use manifest::{FieldManifest, Manifest, RelationManifest, RowManifest};

/// Dense relation id: the relation's index in schema declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelationId(pub u32);

/// Dense field id: the field's index in its relation's declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(pub u16);

/// Dense statement id: the statement's index in the schema-global
/// materialized order — fresh auto-[`StatementDescriptor::Functionality`]
/// statements first, then closed auto-keys, then declared statements in
/// declaration order ([`SchemaDescriptor::materialized_statements`] owns
/// the rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatementId(pub u16);

/// Witness index into [`Schema::keys`] — minted only by validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(pub(crate) u16);

/// Witness index into [`Schema::containments`] — minted only by validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContainmentId(pub(crate) u16);

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
    /// `Value::String` bytes are not UTF-8 (the type's contract).
    Utf8,
    /// Interval bounds with `start >= end` — the empty interval denotes
    /// no points and is unrepresentable
    /// (`docs/architecture/10-data-model.md`).
    IntervalEmpty,
}

/// The one `Value` ↔ `ValueType` compatibility check (kind, String UTF-8,
/// interval non-emptiness) — IR validation, bind-time, the dynamic write
/// path, and selection validation all call this so the rules cannot drift
/// apart. Note the membership rule is *not* here: an element-typed value
/// against an `Interval` field is a kind mismatch to this check, and the
/// IR validation boundary owns that bivalence (`ir::validate`, the
/// bivalent-anchor resolution).
pub(crate) fn value_matches(value: &Value, expected: &ValueType) -> Result<(), ValueMismatch> {
    match (value, expected) {
        (Value::Bool(_), ValueType::Bool)
        | (Value::U64(_), ValueType::U64)
        | (Value::I64(_), ValueType::I64) => Ok(()),
        // The length is the type: a bytes<N> literal of any other width
        // is a kind mismatch.
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

/// The extension-row cap: a vocabulary larger than 256 is policy data
/// wearing a vocabulary costume, and the cap keeps every compiled word-set
/// a fixed 4×u64 bitset (`docs/architecture/10-data-model.md`, the refusal —
/// *trigger* for lifting it: a census sighting).
pub const MAX_EXTENSION_ROWS: usize = 256;

/// One ground axiom of a closed relation: the handle — the row's identity,
/// NOT a column — plus one value per declared intrinsic column, in
/// field-declaration order (`docs/architecture/10-data-model.md` § closed
/// relations). The row id is the declaration index, exactly the
/// declaration-order rule relations, fields, and statements already obey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub handle: Box<str>,
    pub values: Box<[Value]>,
}

/// A closed relation's extension: its ground axioms in declaration order.
pub type Extension = Box<[Row]>;

/// One declared relation. `Some(extension)` declares it **closed** — its
/// rows are ground axioms, frozen by the fingerprint, virtual in storage,
/// write-refused; `None` is ordinary. No relation-kind enum exists: the
/// option *is* the kind (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub name: Box<str>,
    pub fields: Vec<FieldDescriptor>,
    pub extension: Option<Extension>,
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

    /// The theory's manifest: every name → id pairing as a plain Rust
    /// value ([`Manifest`]) — the id constants' runtime twin, for
    /// foreign hosts that take their numbers as data
    /// (`docs/architecture/70-api.md` § the manifest). Rendered off the
    /// descriptor; no serde anywhere — a downstream binding serializes
    /// it however it likes.
    fn manifest(self) -> Manifest {
        // The inherent method, named in full: on a `SchemaDescriptor`
        // receiver the plain `.manifest()` call would resolve to *this*
        // trait method (by-value candidates win) and recurse forever.
        SchemaDescriptor::manifest(&self.descriptor())
    }
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
    /// fresh field), then one closed auto-key `R(id) -> R` per closed
    /// relation (declaration order; projection = the synthetic id field),
    /// then the declared statements in declaration order. Fresh before
    /// closed is a fingerprint input, pinned here and never revisited
    /// (`docs/architecture/30-dependencies.md`). [`StatementId`] = index
    /// into this list, schema-global.
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
                // A closed relation's sealed field list opens with the
                // synthetic id, so its declared fields sit at idx + 1.
                let sealed_idx = field_idx + usize::from(relation.extension.is_some());
                if field.generation == Generation::Fresh {
                    statements.push(StatementDescriptor::Functionality {
                        relation: RelationId(
                            u32::try_from(rel_idx).expect("relation count fits u32"),
                        ),
                        projection: Box::new([FieldId(
                            u16::try_from(sealed_idx).expect("field count fits u16"),
                        )]),
                    });
                }
            }
        }
        // Closedness materializes `R(id) -> R` exactly as `fresh` does:
        // the handle is the identity, and the auto-key is the statement
        // containments target when a plain-u64 reference declares its
        // containment against a closed relation.
        for (rel_idx, relation) in self.relations.iter().enumerate() {
            if relation.extension.is_some() {
                statements.push(StatementDescriptor::Functionality {
                    relation: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
                    projection: Box::new([FieldId(0)]),
                });
            }
        }
        statements.extend(self.statements.iter().cloned());
        statements
    }
}

/// The enforcement plan of a sealed containment. Keys carry their one
/// enforcement flag directly on [`KeyStatement`], so variant agreement is
/// represented by the type rather than re-checked by every consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Enforcement {
    /// Probe an ordinary target key. `key_permutation` maps statement
    /// projection order to target-key order; `coverage` selects the interval
    /// coverage walk instead of a scalar get.
    Probe {
        target_key: KeyId,
        key_permutation: Box<[u16]>,
        coverage: bool,
    },
    /// A closed target's stage-1-known answer set.
    Closed { members: [u64; 4] },
}

/// Whether `id` is inside a compiled member set — the whole judgment of a
/// closed-target containment. An out-of-range id (≥ the 256-row roster
/// cap, or ≥ the extension length: those bits are never set) is simply
/// absent — the same containment violation as any dangling reference, no
/// special error.
#[must_use]
pub(crate) fn closed_member(members: &[u64; 4], id: u64) -> bool {
    usize::try_from(id / 64)
        .ok()
        .and_then(|word| members.get(word))
        .is_some_and(|word| word & (1 << (id % 64)) != 0)
}

/// One σ-literal check compiled at validate (the staging law applied to
/// the checker, `docs/architecture/30-dependencies.md` § enforcement):
/// everything whose canonical bytes are a pure function of the value seals
/// here, once; only interned text — whose word is per-database dictionary
/// state — remains commit-resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledCheck {
    /// The literal's canonical encoding, sealed — one byte compare at
    /// judgment, zero encoding work per commit.
    Encoded { field: FieldId, bytes: Box<[u8]> },
    /// A `str` literal: resolves through the delta's pending map then the
    /// committed dictionary at commit; a double miss proves no fact can
    /// satisfy the selection.
    Interned { field: FieldId, text: Box<str> },
}

/// Both sides' compiled σ checks of one containment statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledSides {
    pub(crate) source: Box<[CompiledCheck]>,
    pub(crate) target: Box<[CompiledCheck]>,
}

/// One sealed key statement: `R(X) -> R` with its enforcement flag.
#[derive(Debug)]
pub struct KeyStatement {
    /// Materialized-order identity. It is fingerprint-pinned and embedded in
    /// storage keys and errors; it is never an arena index.
    pub id: StatementId,
    pub relation: RelationId,
    pub projection: Box<[FieldId]>,
    /// The key carries an interval (necessarily in final position), so its
    /// enforcement uses an ordered-neighbor probe.
    pub pointwise: bool,
}

/// One sealed containment: its declaration, enforcement proof, compiled
/// selections, and optional `==` partner.
#[derive(Debug)]
pub struct ContainmentStatement {
    /// Materialized-order identity. It is not an arena index.
    pub id: StatementId,
    pub source: Side,
    pub target: Side,
    pub(crate) enforcement: Enforcement,
    /// Both sides' σ literals, compiled once at validate. This is total:
    /// keys cannot reach a containment value.
    pub(crate) checks: CompiledSides,
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

/// The global materialized-order spine: a [`StatementId`] selects one typed
/// arena and one slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementRef {
    Key(KeyId),
    Containment(ContainmentId),
}

/// A borrowed sealed statement for display and other order-preserving walks.
/// Consumers that already hold a typed id use the total arena accessors.
#[derive(Debug, Clone, Copy)]
pub enum StatementView<'schema> {
    Key(&'schema KeyStatement),
    Containment(&'schema ContainmentStatement),
}

impl StatementView<'_> {
    /// The fingerprint-pinned materialized identity of either statement arm.
    #[must_use]
    pub const fn id(self) -> StatementId {
        match self {
            Self::Key(statement) => statement.id,
            Self::Containment(statement) => statement.id,
        }
    }
}

/// One sealed ground axiom: the handle plus the row's canonical fact bytes
/// — the synthetic id field (the declaration index) followed by each
/// intrinsic value's canonical encoding. Values encode ONCE, at validate,
/// and never again — the staging law applied to the feature itself
/// (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedRow {
    pub handle: Box<str>,
    pub fact: Box<[u8]>,
}

/// One relation of a validated schema.
#[derive(Debug)]
pub struct Relation {
    name: Box<str>,
    fields: Box<[FieldDescriptor]>,
    layout: FactLayout,
    /// The sealed extension of a closed relation (`None` = ordinary): rows
    /// pre-encoded at validate, in declaration order — row id = index. A
    /// closed relation's `fields` open with the synthetic (`id`, U64)
    /// field, so guards, statements, and queries address the handle's id
    /// uniformly at [`FieldId`] 0 (`docs/architecture/10-data-model.md`
    /// § closed relations).
    extension: Option<Box<[SealedRow]>>,
    /// `Functionality` statements on this relation, in materialized order.
    keys: Box<[KeyId]>,
    /// `Containment` statements whose source is this relation.
    outgoing: Box<[ContainmentId]>,
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug)]
pub struct Schema {
    relations: Box<[Relation]>,
    /// Homogeneous typed arenas. Only validation mints their witness ids.
    keys: Box<[KeyStatement]>,
    containments: Box<[ContainmentStatement]>,
    /// The materialized statement list; [`StatementId`] indexes this spine.
    order: Box<[StatementRef]>,
    /// `target_key -> dependents`, indexed by [`KeyId`].
    dependents: Box<[Box<[ContainmentId]>]>,
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

    /// All sealed keys, in typed-arena order.
    #[must_use]
    pub fn keys(&self) -> &[KeyStatement] {
        &self.keys
    }

    /// All sealed containments, in typed-arena order.
    #[must_use]
    pub fn containments(&self) -> &[ContainmentStatement] {
        &self.containments
    }

    /// A key selected by its validation-minted witness.
    #[must_use]
    pub fn key(&self, id: KeyId) -> &KeyStatement {
        &self.keys[usize::from(id.0)]
    }

    /// The bounds-checked sibling of [`Schema::key`] for ids arriving as
    /// dynamic data.
    #[must_use]
    pub fn key_checked(&self, id: KeyId) -> Option<&KeyStatement> {
        self.keys.get(usize::from(id.0))
    }

    /// A containment selected by its validation-minted witness.
    #[must_use]
    pub fn containment(&self, id: ContainmentId) -> &ContainmentStatement {
        &self.containments[usize::from(id.0)]
    }

    /// The bounds-checked sibling of [`Schema::containment`] for ids arriving
    /// as dynamic data.
    #[must_use]
    pub fn containment_checked(&self, id: ContainmentId) -> Option<&ContainmentStatement> {
        self.containments.get(usize::from(id.0))
    }

    /// Resolve a materialized-order identity through the typed arena spine.
    #[must_use]
    pub fn statement(&self, id: StatementId) -> StatementView<'_> {
        match self.order[usize::from(id.0)] {
            StatementRef::Key(key) => StatementView::Key(self.key(key)),
            StatementRef::Containment(containment) => {
                StatementView::Containment(self.containment(containment))
            }
        }
    }

    /// The bounds-checked sibling of [`Schema::statement`].
    #[must_use]
    pub fn statement_checked(&self, id: StatementId) -> Option<StatementView<'_>> {
        self.order
            .get(usize::from(id.0))
            .copied()
            .map(|statement| match statement {
                StatementRef::Key(key) => StatementView::Key(self.key(key)),
                StatementRef::Containment(containment) => {
                    StatementView::Containment(self.containment(containment))
                }
            })
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
    pub fn dependents(&self, id: KeyId) -> &[ContainmentId] {
        &self.dependents[usize::from(id.0)]
    }

    /// The bounds-checked sibling of [`Schema::dependents`].
    #[must_use]
    pub fn dependents_checked(&self, id: KeyId) -> Option<&[ContainmentId]> {
        self.dependents.get(usize::from(id.0)).map(AsRef::as_ref)
    }
}
