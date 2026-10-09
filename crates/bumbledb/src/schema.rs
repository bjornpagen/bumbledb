//! Schema validation, sealed witnesses, and compiled enforcement indexes.
//! Declaration types and [`spec`] lowering are shared with `bumbledb-theory`.
//! Validation seals declarations; [`compiled`] derives the access paths used
//! by storage, judgment, and query planning.

pub mod compiled;
/// The canonical bounded rejection-evidence codec: the one byte
/// spelling of a complete violated-statement set with labeled examples and
/// truncation evidence. The log frames these bytes verbatim into decisions
/// and receipts; strict decode plus schema interpretation reproduces the
/// judge's verdict or the public [`crate::Violations`] value.
pub mod evidence;
pub mod fingerprint;
/// Final-state judgment and the candidate-state interface shared by the
/// physical commit path and independent models.
pub(crate) mod judge;
pub mod manifest;
pub mod render;

mod relation;
#[cfg(test)]
pub(crate) mod tests;
mod validate;
mod wire;

use bumbledb_theory::Value;

pub use bumbledb_theory::schema::spec;
pub use bumbledb_theory::schema::{
    Bound, Extension, FieldDescriptor, FieldId, FixedIntervalElement, IntervalElement, LiteralSet,
    MAX_EXTENSION_ROWS, RelationDescriptor, RelationId, Row, SchemaDescriptor, SealedField, Side,
    StatementDescriptor, StatementId, StatementKind, ValueType, Weight,
};

pub use bumbledb_theory::schema::{
    MemberSet, SealedBound, SealedWeight, ValueMismatch, value_matches,
};

pub use compiled::{
    CompileError, CompiledProjection, CompiledTheory, DistinctnessWitness, KeyEncoding,
    LMDB_KEY_LIMIT, MAX_EXACT_SCALAR_BYTES, ProjectionBinding, ProjectionId, ProjectionInternKey,
    VisitControl, VisitOutcome, encode_scalar_group,
};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Enforcement {
    ScalarProbe {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
    },

    IntervalCoverage {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
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

/// A key's form: scalar, or pointwise over a final interval field whose
/// rows in one scalar group are disjoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyForm {
    Scalar,
    Pointwise { tail: ValueType },
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
}

impl KeyForm {
    #[must_use]
    pub const fn is_pointwise(&self) -> bool {
        matches!(self, Self::Pointwise { .. })
    }
}

/// One sealed containment: its canonical declaration, enforcement proof,
/// and optional `==` partner. Physical access paths live in [`CompiledTheory`].
#[derive(Debug, Clone)]
pub struct ContainmentStatement {
    pub id: StatementId,
    pub source: Side,
    pub target: Side,
    pub(crate) enforcement: Enforcement,

    pub pairing: Pairing,
}

impl ContainmentStatement {
    #[must_use]
    pub fn mirror_id(&self, schema: &Schema) -> Option<StatementId> {
        self.pairing.partner().map(|id| schema.containment(id).id)
    }
}

/// One sealed capacity statement: `B(Y | ψ) <=[w]{lo..hi} A(X | φ)`.
/// Validation resolves the target key and seals the weight and ceiling.
/// Commit-time judgment checks the final measure against the window.
#[derive(Debug, Clone)]
pub struct CapacityStatement {
    pub id: StatementId,
    pub target: Side,

    pub weight: SealedWeight,

    pub lo: u64,

    pub hi: SealedBound,
    pub source: Side,

    pub(crate) enforcement: CapacityEnforcement,
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

/// One row of a closed relation: its handle, its values in sealed field
/// order (the handle id, the row's declaration index, first) and the same
/// values as a canonical row, the codec of stored rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedRow {
    pub handle: Box<str>,
    pub values: Box<[Value]>,
    pub row: crate::canonical::CanonicalRow,
}

/// A relation's kind: ordinary, or closed with its rows. Closed relations
/// are never written.
#[derive(Debug, Clone)]
pub enum RelationBody {
    Ordinary,

    Closed { extension: Box<[SealedRow]> },
}

impl RelationBody {
    #[must_use]
    pub fn closed_rows(&self) -> Option<&[SealedRow]> {
        match self {
            Self::Closed { extension } => Some(extension),
            Self::Ordinary => None,
        }
    }
}

/// One relation of a validated schema.
#[derive(Debug, Clone)]
pub struct Relation {
    name: Box<str>,
    fields: Box<[FieldDescriptor]>,

    keys: Box<[KeyId]>,

    outgoing: Box<[ContainmentId]>,

    capacity_sources: Box<[CapacityId]>,

    /// Rejudge every target group touched by the final delta.
    capacity_targets: Box<[CapacityId]>,
    body: RelationBody,
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug, Clone)]
pub struct Schema {
    identity: std::sync::OnceLock<fingerprint::SchemaFingerprint>,
    compiled: std::sync::OnceLock<Result<compiled::SharedCompiledTheory, compiled::CompileError>>,
    relations: Box<[Relation]>,

    keys: Box<[KeyStatement]>,
    containments: Box<[ContainmentStatement]>,
    capacities: Box<[CapacityStatement]>,

    order: Box<[StatementRef]>,

    dependents: Box<[Box<[ContainmentId]>]>,
}

impl Schema {
    /// The sealed schema's compiled projection/law machine (C1). Compiled
    /// once and shared by storage, admission and planning consumers.
    ///
    /// # Errors
    /// [`CompileError::ProjectionIdExhausted`] when interned projection ids run out.
    pub fn compiled_theory(&self) -> Result<&compiled::CompiledTheory, compiled::CompileError> {
        self.compiled
            .get_or_init(|| compiled::shared_compile(self))
            .as_ref()
            .map(std::convert::AsRef::as_ref)
            .map_err(|error| *error)
    }

    pub(crate) fn shared_compiled_theory(
        &self,
    ) -> Result<compiled::SharedCompiledTheory, compiled::CompileError> {
        self.compiled
            .get_or_init(|| compiled::shared_compile(self))
            .as_ref()
            .cloned()
            .map_err(|error| *error)
    }

    #[must_use]
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }

    /// The rows of a closed relation; `None` for an ordinary or unknown one.
    #[must_use]
    pub fn closed_rows(&self, id: RelationId) -> Option<&[SealedRow]> {
        self.relation_checked(id)?.body().closed_rows()
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
    #[cfg(test)]
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

    /// The closed extension is fixed by the schema.
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

    /// Instance-dependent laws in declaration order. Closed-constant laws
    /// were discharged by validation and cannot be affected by a delta.
    pub(crate) fn complete_obligations(&self) -> impl Iterator<Item = StatementView<'_>> + '_ {
        self.statements()
            .filter(|view| !self.closed_constant(*view))
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
}
