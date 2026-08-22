//! Schema declaration validation, the sealed witness, and the fingerprint
//! .
//! The schema-as-declared vocabulary — the ids, [`ValueType`],
//! [`SchemaDescriptor`] and its descriptor family, [`LiteralSet`]/[`Side`],
//! the [`spec`] lowering, and the shared [`value_matches`] check — lives in
//! `bumbledb-theory` (the parity roster is normative there) and is

pub mod fingerprint;
pub mod manifest;
pub mod render;

mod relation;
#[cfg(test)]
pub(crate) mod tests;
mod validate;
mod wire;

use crate::encoding::FactLayout;
use crate::error::{DynIdError, FactShapeError};
// `super::Value`, exactly as before the theory extraction.
use bumbledb_theory::Value;

pub use bumbledb_theory::schema::spec;
pub use bumbledb_theory::schema::{
    Bound, Extension, FieldDescriptor, FieldId, Generation, IntervalElement, LiteralSet,
    MAX_EXTENSION_ROWS, RelationDescriptor, RelationId, Row, SchemaDescriptor, SealedField, Side,
    StatementDescriptor, StatementId, StatementKind, ValueType, Weight,
};

pub(crate) use bumbledb_theory::schema::{ValueMismatch, value_matches};

pub use manifest::{
    FieldManifest, Manifest, ManifestDescriptor, RelationManifest, RowManifest, StatementManifest,
};
pub use render::{RenderedFact, RenderedViolation, render_rejection};
pub use spec::{
    BoundSpec, CapacityWindowSpec, FaceNewtype, FieldSpec, LiteralSetSpec, LiteralSpec,
    RelationSpec, RowSpec, SchemaSpec, SchemaSpecError, SideSpec, SpecIssue, StatementSpec,
    WeightSpec,
};
pub use validate::ValidateDescriptor;

/// Witness index into [`Schema::keys`] — minted only by validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(pub(crate) u16);

/// Witness index into [`Schema::containments`] — minted only by validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContainmentId(pub(crate) u16);

/// Witness index into [`Schema::capacities`] — minted only by validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CapacityId(pub(crate) u16);

/// A witness that `(relation, field)` names a `Fresh`-generation field of
/// ([`crate::WriteTx::reserve_at`]). Fields are private and
/// [`crate::Db::fresh_field`] is the one construction site; the ETL access
/// The witness carries a **binding** proof: `S` is the resolving handle's
/// schema typestate, so a witness of one `schema!` schema cannot reach a
/// transaction of another — a compile error, the hard-structural-typing
/// answer (nominal safety = host Rust newtypes; pinned by
/// `tests/schema-compile-fail/foreign_fresh_witness.rs`). This REVERSES
pub struct FreshField<S> {
    relation: RelationId,
    field: FieldId,

    marker: std::marker::PhantomData<fn() -> S>,
}

impl<S> std::fmt::Debug for FreshField<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FreshField")
            .field("relation", &self.relation)
            .field("field", &self.field)
            .finish()
    }
}

impl<S> Clone for FreshField<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for FreshField<S> {}

impl<S> PartialEq for FreshField<S> {
    fn eq(&self, other: &Self) -> bool {
        self.relation == other.relation && self.field == other.field
    }
}

impl<S> Eq for FreshField<S> {}

impl<S> FreshField<S> {
    pub(crate) fn new(relation: RelationId, field: FieldId) -> Self {
        Self {
            relation,
            field,
            marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn relation(self) -> RelationId {
        self.relation
    }

    pub(crate) fn field(self) -> FieldId {
        self.field
    }
}

/// A named theory — a schema names a theory (relations plus statements)
/// and a store models it: the
/// value [`crate::Db::create`] and [`crate::Db::open`] take, and the
/// type that names the database in [`crate::Db<S>`]'s typestate. The
/// `schema!` macro emits one unit
/// struct per invocation (`pub Ledger;` → `pub struct Ledger;` with this
/// impl), so a fact of schema A cannot reach a database of schema B —
/// the mismatch is a compile error, not a lucky width check.
pub trait Theory: Sized {
    fn descriptor(self) -> SchemaDescriptor;

    fn manifest(self) -> Manifest {
        ManifestDescriptor::manifest(&self.descriptor())
    }
}

impl Theory for SchemaDescriptor {
    fn descriptor(self) -> SchemaDescriptor {
        self
    }
}

impl Schema {
    #[expect(
        clippy::unused_self,
        reason = "the schema is the witness's minting authority — readers go through it"
    )]
    pub(crate) fn key_tail(&self, key: &KeyStatement) -> Option<ValueType> {
        key.tail()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisjointDeterminantProof(());

impl DisjointDeterminantProof {
    pub(crate) const fn authorize_coverage(self) {
        let Self(()) = self;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Enforcement {
    ScalarProbe {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
    },

    IntervalCoverage {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
        disjoint: DisjointDeterminantProof,
        source_tail: ValueType,
        target_tail: ValueType,
    },

    Closed {
        members: MemberSet,
    },
}

impl Enforcement {
    pub(crate) const fn target_key(&self) -> Option<KeyId> {
        match self {
            Self::ScalarProbe { target_key, .. } | Self::IntervalCoverage { target_key, .. } => {
                Some(*target_key)
            }
            Self::Closed { .. } => None,
        }
    }

    pub(crate) fn key_projection(&self) -> Option<&[FieldId]> {
        match self {
            Self::ScalarProbe { key_projection, .. }
            | Self::IntervalCoverage { key_projection, .. } => Some(key_projection),
            Self::Closed { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CapacityEnforcement {
    ScalarProbe {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
    },

    Closed {
        members: MemberSet,
    },
}

impl CapacityEnforcement {
    pub(crate) fn key_projection(&self) -> Option<&[FieldId]> {
        match self {
            Self::ScalarProbe { key_projection, .. } => Some(key_projection),
            Self::Closed { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Survivors {
    ReverseEdges,

    /// unrepresentable (refused at validation).
    SealedRows,
}

/// The `==` partner of a containment, typed to the containment arena.
/// [`StatementId`] is the materialized-order ordinal, not this pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pairing {
    OneWay,

    Mirror(ContainmentId),
}

impl Pairing {
    #[must_use]
    pub const fn partner(self) -> Option<ContainmentId> {
        match self {
            Self::OneWay => None,
            Self::Mirror(id) => Some(id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AxiomIndex(pub(crate) u8);

impl TryFrom<u64> for AxiomIndex {
    type Error = std::num::TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u8::try_from(value).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemberSet {
    words: [u64; 4],
}

impl MemberSet {
    pub(crate) const fn empty() -> Self {
        Self { words: [0; 4] }
    }

    #[must_use]
    pub(crate) fn contains(&self, index: AxiomIndex) -> bool {
        let word = usize::from(index.0 / 64);
        self.words[word] & (1 << (index.0 % 64)) != 0
    }

    pub(crate) fn insert(&mut self, index: AxiomIndex) {
        let word = usize::from(index.0 / 64);
        self.words[word] |= 1 << (index.0 % 64);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EncodableCheck {
    Encoded {
        field: FieldId,
        bytes: Box<[u8]>,
    },

    EncodedSet {
        field: FieldId,
        alternatives: Box<[Box<[u8]>]>,
    },
}

impl EncodableCheck {
    pub(crate) fn matches(&self, layout: &FactLayout, fact: &[u8]) -> bool {
        use crate::encoding::field_bytes;
        match self {
            Self::Encoded { field, bytes } => {
                field_bytes(layout.encoded(fact), usize::from(field.0)) == &bytes[..]
            }
            Self::EncodedSet {
                field,
                alternatives,
            } => {
                let actual = field_bytes(layout.encoded(fact), usize::from(field.0));
                alternatives.iter().any(|bytes| actual == &bytes[..])
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledCheck {
    Encoded {
        field: FieldId,
        bytes: Box<[u8]>,
    },

    EncodedSet {
        field: FieldId,
        alternatives: Box<[Box<[u8]>]>,
    },

    Interned {
        field: FieldId,
        text: Box<str>,
    },

    InternedSet {
        field: FieldId,
        texts: Box<[Box<str>]>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledSide {
    Ordinary(Box<[CompiledCheck]>),
    Closed(Box<[EncodableCheck]>),
}

impl CompiledSide {
    pub(crate) fn ordinary(&self) -> Option<&[CompiledCheck]> {
        match self {
            Self::Ordinary(checks) => Some(checks),
            Self::Closed(_) => None,
        }
    }

    pub(crate) fn closed(&self) -> Option<&[EncodableCheck]> {
        match self {
            Self::Closed(checks) => Some(checks),
            Self::Ordinary(_) => None,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Ordinary(checks) => checks.is_empty(),
            Self::Closed(checks) => checks.is_empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledSides {
    pub(crate) source: CompiledSide,
    pub(crate) target: CompiledSide,
}

/// The sealed key form: three behaviors, three arms. Fresh-row ×
/// pointwise is unrepresentable; the disjointness proof lives on the
/// Pointwise arm that needs it (CONTRACT C9).
#[allow(private_interfaces)]
#[derive(Debug, Clone)]
pub enum KeyForm {
    FreshRow {
        field: FieldId,
    },

    Scalar,

    Pointwise {
        tail: ValueType,
        disjoint: DisjointDeterminantProof,
    },
}

/// One sealed key statement: `R(X) -> R` with its form.
#[derive(Debug, Clone)]
pub struct KeyStatement {
    pub id: StatementId,
    pub relation: RelationId,
    pub projection: Box<[FieldId]>,
    pub(crate) form: KeyForm,
}

impl KeyStatement {
    #[must_use]
    pub fn form(&self) -> &KeyForm {
        &self.form
    }

    #[must_use]
    pub(crate) fn tail(&self) -> Option<ValueType> {
        self.form.as_pointwise()
    }
}

impl KeyForm {
    #[must_use]
    pub const fn as_fresh_row(&self) -> Option<FieldId> {
        match *self {
            Self::FreshRow { field } => Some(field),
            Self::Scalar | Self::Pointwise { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_pointwise(&self) -> bool {
        matches!(self, Self::Pointwise { .. })
    }

    #[must_use]
    pub(crate) const fn as_pointwise(&self) -> Option<ValueType> {
        match *self {
            Self::Pointwise { tail, .. } => Some(tail),
            Self::FreshRow { .. } | Self::Scalar => None,
        }
    }
}

/// One sealed containment: its declaration, enforcement proof, compiled
/// selections, and optional `==` partner.
#[derive(Debug, Clone)]
pub struct ContainmentStatement {
    pub id: StatementId,
    pub source: Side,
    pub target: Side,
    pub(crate) enforcement: Enforcement,

    pub(crate) survivors: Survivors,

    pub(crate) checks: CompiledSides,

    pub pairing: Pairing,
}

impl ContainmentStatement {
    #[must_use]
    pub fn mirror_id(&self, schema: &Schema) -> Option<StatementId> {
        self.pairing.partner().map(|id| schema.containment(id).id)
    }
}

/// Sealed capacity measure (CONTRACT C9): Duration carries its tail
/// in-arm. [`Weight::Unit`] is a case, not an absence.
#[allow(private_interfaces)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealedWeight {
    Unit,
    Field(FieldId),
    Duration { field: FieldId, tail: ValueType },
}

impl SealedWeight {
    #[must_use]
    pub const fn to_weight(self) -> Weight {
        match self {
            Self::Unit => Weight::Unit,
            Self::Field(field) => Weight::Field(field),
            Self::Duration { field, .. } => Weight::DurationOf(field),
        }
    }
}

/// Sealed capacity ceiling (CONTRACT C9): `*` is [`SealedBound::Unbounded`],
/// not a missing bound. Duration carries its tail in-arm.
#[allow(private_interfaces)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealedBound {
    Unbounded,
    Lit(u64),
    TargetField(FieldId),
    Duration { field: FieldId, tail: ValueType },
}

impl SealedBound {
    #[must_use]
    pub const fn to_bound(self) -> Option<Bound> {
        match self {
            Self::Unbounded => None,
            Self::Lit(n) => Some(Bound::Lit(n)),
            Self::TargetField(field) => Some(Bound::TargetField(field)),
            Self::Duration { field, .. } => Some(Bound::TargetDuration(field)),
        }
    }

    #[must_use]
    pub(crate) const fn needs_parent_fact(self) -> bool {
        matches!(self, Self::TargetField(_) | Self::Duration { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundCeiling {
    Unbounded,
    Finite(u64),
}

/// One sealed capacity statement: `B(Y | ψ) <=[w]{lo..hi} A(X | φ)`.
/// Accepted at declaration with its sealed target-key plan handle
/// (the same probe-ability rule containments resolve —
/// plan); commit-time judging is the enforcement stage's work. Fields
/// sit in the operator's read order — target, weight, window, source
/// (ruled 2026-07-24, C2).
/// `lean/Bumbledb/Oracle.lean: capacity_plan_decides` is the promised
#[derive(Debug, Clone)]
pub struct CapacityStatement {
    pub id: StatementId,
    pub target: Side,

    pub weight: SealedWeight,

    pub lo: u64,

    pub hi: SealedBound,
    pub source: Side,

    pub(crate) enforcement: CapacityEnforcement,

    pub(crate) checks: CompiledSides,
}

/// The global materialized-order spine: a [`StatementId`] selects one typed
/// arena and one slot. This is the one stored statement identity —
/// [`StatementId`] is the materialized-order ordinal for fingerprints,
/// rendering, and host citation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatementRef {
    Key(KeyId),
    Containment(ContainmentId),
    Capacity(CapacityId),
}

impl std::fmt::Display for StatementRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Key(id) => write!(f, "key {}", id.0),
            Self::Containment(id) => write!(f, "containment {}", id.0),
            Self::Capacity(id) => write!(f, "capacity {}", id.0),
        }
    }
}

/// A borrowed sealed statement for display and other order-preserving walks.
/// Consumers that already hold a typed id use the total arena accessors.
#[derive(Debug, Clone, Copy)]
pub enum StatementView<'schema> {
    Key(KeyId, &'schema KeyStatement),
    Containment(ContainmentId, &'schema ContainmentStatement),
    Capacity(CapacityId, &'schema CapacityStatement),
}

impl StatementView<'_> {
    #[must_use]
    pub const fn id(self) -> StatementId {
        match self {
            Self::Key(_, statement) => statement.id,
            Self::Containment(_, statement) => statement.id,
            Self::Capacity(_, statement) => statement.id,
        }
    }
}

/// One sealed ground axiom: the handle plus the row's canonical fact bytes
/// — the synthetic id field (the declaration index) followed by each
/// intrinsic value's canonical encoding. Values encode ONCE, at validate,
/// .
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedRow {
    pub handle: Box<str>,
    pub fact: Box<[u8]>,
}

/// The sealed relation kind (CONTRACT C9). Shared layout lives on
/// [`Relation`]; the kind-carrying payloads (`fresh` vs `extension`)
/// live in the arms. Closed cannot be written.
#[derive(Debug, Clone)]
pub enum RelationBody {
    Ordinary { fresh: Option<KeyId> },

    Closed { extension: Box<[SealedRow]> },
}

impl RelationBody {
    #[must_use]
    pub fn closed_rows(&self) -> Option<&[SealedRow]> {
        match self {
            Self::Closed { extension } => Some(extension),
            Self::Ordinary { .. } => None,
        }
    }
}

/// One relation of a validated schema.
#[derive(Debug, Clone)]
pub struct Relation {
    name: Box<str>,
    fields: Box<[FieldDescriptor]>,
    layout: FactLayout,

    keys: Box<[KeyId]>,

    outgoing: Box<[ContainmentId]>,

    capacity_sources: Box<[CapacityId]>,

    /// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`).
    capacity_targets: Box<[CapacityId]>,
    body: RelationBody,
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug, Clone)]
pub struct Schema {
    relations: Box<[Relation]>,

    keys: Box<[KeyStatement]>,
    containments: Box<[ContainmentStatement]>,
    capacities: Box<[CapacityStatement]>,

    order: Box<[StatementRef]>,

    dependents: Box<[Box<[ContainmentId]>]>,
}

impl Schema {
    #[must_use]
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }

    /// # Panics

    #[must_use]
    pub fn relation(&self, id: RelationId) -> &Relation {
        &self.relations[id.0 as usize]
    }

    #[must_use]
    pub fn relation_checked(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(id.0 as usize)
    }

    /// # Errors

    pub(crate) fn check_fresh_field(
        &self,
        relation: RelationId,
        field: FieldId,
    ) -> Result<(), FactShapeError> {
        let Some(rel) = self.relation_checked(relation) else {
            return Err(DynIdError::UnknownRelation { relation }.into());
        };
        let Some(descriptor) = rel.fields().get(usize::from(field.0)) else {
            return Err(DynIdError::UnknownField { relation, field }.into());
        };
        if descriptor.generation != Generation::Fresh {
            return Err(DynIdError::NotAFreshField { relation, field }.into());
        }
        Ok(())
    }

    #[must_use]
    pub fn keys(&self) -> &[KeyStatement] {
        &self.keys
    }

    #[must_use]
    pub fn containments(&self) -> &[ContainmentStatement] {
        &self.containments
    }

    #[must_use]
    pub fn capacities(&self) -> &[CapacityStatement] {
        &self.capacities
    }

    #[must_use]
    pub fn capacity(&self, id: CapacityId) -> &CapacityStatement {
        &self.capacities[usize::from(id.0)]
    }

    #[must_use]
    pub fn capacity_checked(&self, id: CapacityId) -> Option<&CapacityStatement> {
        self.capacities.get(usize::from(id.0))
    }

    #[must_use]
    pub fn key(&self, id: KeyId) -> &KeyStatement {
        &self.keys[usize::from(id.0)]
    }

    #[must_use]
    pub fn key_checked(&self, id: KeyId) -> Option<&KeyStatement> {
        self.keys.get(usize::from(id.0))
    }

    #[must_use]
    pub fn containment(&self, id: ContainmentId) -> &ContainmentStatement {
        &self.containments[usize::from(id.0)]
    }

    #[must_use]
    pub fn containment_checked(&self, id: ContainmentId) -> Option<&ContainmentStatement> {
        self.containments.get(usize::from(id.0))
    }

    #[must_use]
    pub fn statement(&self, id: StatementId) -> StatementView<'_> {
        self.view(self.order[usize::from(id.0)])
    }

    #[must_use]
    pub fn id_of(&self, statement: StatementRef) -> StatementId {
        self.view(statement).id()
    }

    #[must_use]
    pub(crate) fn cite(&self, id: StatementId) -> StatementRef {
        self.order[usize::from(id.0)]
    }

    #[must_use]
    pub fn statement_checked(&self, id: StatementId) -> Option<StatementView<'_>> {
        self.order
            .get(usize::from(id.0))
            .copied()
            .map(|statement| self.view(statement))
    }

    fn view(&self, statement: StatementRef) -> StatementView<'_> {
        match statement {
            StatementRef::Key(key) => StatementView::Key(key, self.key(key)),
            StatementRef::Containment(containment) => {
                StatementView::Containment(containment, self.containment(containment))
            }
            StatementRef::Capacity(capacity) => {
                StatementView::Capacity(capacity, self.capacity(capacity))
            }
        }
    }

    pub fn statements(&self) -> impl Iterator<Item = StatementView<'_>> + '_ {
        self.order
            .iter()
            .copied()
            .map(|statement| self.view(statement))
    }

    /// Mirrors `lean/Bumbledb/Schema.lean: Statement.closedConstant`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn closed_constant(&self, view: StatementView<'_>) -> bool {
        let closed = |relation| self.relation(relation).body().closed_rows().is_some();
        match view {
            StatementView::Key(_, statement) => closed(statement.relation),
            StatementView::Containment(_, statement) => {
                closed(statement.source.relation) && closed(statement.target.relation)
            }
            StatementView::Capacity(_, statement) => {
                closed(statement.source.relation) && closed(statement.target.relation)
            }
        }
    }

    #[must_use]
    pub(crate) fn complete_obligations(&self) -> CompleteObligations<'_> {
        CompleteObligations { schema: self }
    }

    /// # Panics

    #[must_use]
    pub fn dependents(&self, id: KeyId) -> &[ContainmentId] {
        &self.dependents[usize::from(id.0)]
    }

    #[must_use]
    pub fn dependents_checked(&self, id: KeyId) -> Option<&[ContainmentId]> {
        self.dependents.get(usize::from(id.0)).map(AsRef::as_ref)
    }

    #[must_use]
    pub(crate) fn fresh_mint_field(&self, id: RelationId) -> Option<FieldId> {
        let key = self.relation(id).fresh_key()?;
        match self.key(key).form() {
            KeyForm::FreshRow { field } => Some(*field),
            KeyForm::Scalar | KeyForm::Pointwise { .. } => None,
        }
    }
}

/// Hand-enumerated rosters beside this spine are refused.
pub(crate) struct CompleteObligations<'schema> {
    schema: &'schema Schema,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CompleteObligation<'schema> {
    Key {
        id: KeyId,
        statement: &'schema KeyStatement,
    },
    Containment {
        id: ContainmentId,
        statement: &'schema ContainmentStatement,
    },
    Capacity {
        id: CapacityId,
        statement: &'schema CapacityStatement,
    },
}

impl CompleteObligation<'_> {
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "roster identity; tests and later consumers")
    )]
    pub(crate) fn statement_ref(self) -> StatementRef {
        match self {
            Self::Key { id, .. } => StatementRef::Key(id),
            Self::Containment { id, .. } => StatementRef::Containment(id),
            Self::Capacity { id, .. } => StatementRef::Capacity(id),
        }
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "roster identity; tests and later consumers")
    )]
    pub(crate) fn statement_id(self) -> StatementId {
        match self {
            Self::Key { statement, .. } => statement.id,
            Self::Containment { statement, .. } => statement.id,
            Self::Capacity { statement, .. } => statement.id,
        }
    }
}

impl<'schema> CompleteObligations<'schema> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = CompleteObligation<'schema>> + '_ {
        self.schema.order.iter().copied().filter_map(|slot| {
            let view = self.schema.view(slot);
            if self.schema.closed_constant(view) {
                None
            } else {
                Some(Self::classify(view))
            }
        })
    }

    fn classify(view: StatementView<'schema>) -> CompleteObligation<'schema> {
        match view {
            StatementView::Key(id, statement) => match statement.form() {
                KeyForm::FreshRow { .. } | KeyForm::Scalar | KeyForm::Pointwise { .. } => {
                    CompleteObligation::Key { id, statement }
                }
            },
            StatementView::Containment(id, statement) => match &statement.enforcement {
                Enforcement::ScalarProbe { .. }
                | Enforcement::IntervalCoverage { .. }
                | Enforcement::Closed { .. } => CompleteObligation::Containment { id, statement },
            },
            StatementView::Capacity(id, statement) => match &statement.enforcement {
                CapacityEnforcement::ScalarProbe { .. } | CapacityEnforcement::Closed { .. } => {
                    CompleteObligation::Capacity { id, statement }
                }
            },
        }
    }
}
