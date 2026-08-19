//! Schema declaration validation, the sealed witness, and the fingerprint
//! (`docs/architecture/10-data-model.md`, `docs/architecture/30-dependencies.md`).
//!
//! The schema-as-declared vocabulary — the ids, [`ValueType`],
//! [`SchemaDescriptor`] and its descriptor family, [`LiteralSet`]/[`Side`],
//! the [`spec`] lowering, and the shared [`value_matches`] check — lives in
//! `bumbledb-theory` (the parity roster is normative there) and is
//! re-exported here as this crate's own surface: hosts depend on this one
//! crate, and every established path (`crate::schema::SchemaDescriptor`,
//! `crate::SchemaSpec`, …) keeps resolving.
//!
//! What stays engine-side is the admission boundary and the sealed half:
//! construction is the validation boundary (parse, don't validate) — the
//! only way to obtain a [`Schema`] is [`SchemaDescriptor::validate`], and
//! everything downstream trusts the sealed witness without re-checking.

pub mod fingerprint;
pub mod manifest;
pub mod render;

pub(crate) mod descriptor_codec;
mod relation;
#[cfg(test)]
pub(crate) mod tests;
mod validate;
mod wire;

use crate::encoding::FactLayout;
use crate::error::{DynIdError, FactShapeError};
// The submodules (`render`, `validate`) address the literal sum as
// `super::Value`, exactly as before the theory extraction.
use bumbledb_theory::Value;

// The theory vocabulary, re-exported as this crate's public schema
// surface (`docs/architecture/70-api.md` § the SchemaSpec bindings
// contract): the facade is the permanent API, not a shim — hosts import
// these names from here; internal engine code imports `bumbledb_theory::`
// directly (zero internal shim usage, grep-enforced).
pub use bumbledb_theory::schema::spec;
pub use bumbledb_theory::schema::{
    Bound, Extension, FieldDescriptor, FieldId, Generation, IntervalElement, LiteralSet,
    MAX_EXTENSION_ROWS, RelationDescriptor, RelationId, Row, SchemaDescriptor, SealedField, Side,
    StatementDescriptor, StatementId, StatementKind, ValueType, Weight,
};
// The shared Value ↔ ValueType check — crate-internal here exactly as it
// was when it lived in this module (public in the theory crate).
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
/// schema `S` — the handle of the untyped mint path
/// ([`crate::WriteTx::reserve_at`]). Fields are private and
/// [`crate::Db::fresh_field`] is the one construction site; the ETL access
/// pattern is resolve once per relation, mint per row (`70-api.md` § ETL).
///
/// The witness carries a **binding** proof: `S` is the resolving handle's
/// schema typestate, so a witness of one `schema!` schema cannot reach a
/// transaction of another — a compile error, the hard-structural-typing
/// answer (nominal safety = host Rust newtypes; pinned by
/// `tests/schema-compile-fail/foreign_fresh_witness.rs`). This REVERSES
/// the earlier "the witness carries the proof" decision (2026-07-15): a
/// value-level proof bound to no schema let a foreign witness mint
/// silently. At the dyn boundary — every `Db<SchemaDescriptor>` shares
/// one typestate — the binding proves nothing across descriptors, so the
/// mint's per-transaction sequence init re-checks the generation and
/// refuses typed ([`crate::error::FactShapeError`]); the steady-state
/// mint path still re-checks nothing.
pub struct FreshField<S> {
    relation: RelationId,
    field: FieldId,
    /// The schema binding (`fn() -> S` keeps auto-traits independent of
    /// `S`, the [`crate::Db`] marker's precedent).
    marker: std::marker::PhantomData<fn() -> S>,
}

// Manual impls: a derive would bound `S` (`S: Copy` etc.), and the
// phantom binding must not inherit the schema type's own traits.
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
    /// The one construction site's plumbing ([`crate::Db::fresh_field`]
    /// validates first — nothing else constructs).
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
        // The extension trait, named in full: on a `SchemaDescriptor`
        // receiver the plain `.manifest()` call would resolve to *this*
        // trait method (by-value candidates win) and recurse forever.
        ManifestDescriptor::manifest(&self.descriptor())
    }
}

impl Theory for SchemaDescriptor {
    fn descriptor(self) -> SchemaDescriptor {
        self
    }
}

/// Trailing interval encoding of a sealed projection (CONTRACT C9):
/// the interval-restricted [`ValueType`]. Width has one owner —
/// [`ValueType::width`]. General stores both order-preserving halves; a
/// fixed-width type stores the START word only — the bias of both
/// element encodings is additive, so `start_word + w` IS the encoded end
/// (`lean/Bumbledb/Values.lean: encode_fixed_order_u64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IntervalTail(ValueType);

impl IntervalTail {
    /// Restricts `ty` to an interval-family type.
    #[must_use]
    pub(crate) const fn of(ty: ValueType) -> Option<Self> {
        if ty.is_interval() {
            Some(Self(ty))
        } else {
            None
        }
    }

    /// Trailing encoded bytes: [`ValueType::width`] of the interval type.
    pub(crate) const fn bytes(self) -> usize {
        self.0.width()
    }

    /// The `(start, end)` order-preserving words of a tail slice —
    /// `None` on a malformed tail (wrong width, or a fixed start at or
    /// past the Q2 bound; callers convict corruption). Both element
    /// ceilings encode to `u64::MAX`, so the fixed bound is one word
    /// compare in either domain.
    pub(crate) fn words(self, tail: &[u8]) -> Option<(u64, u64)> {
        if tail.len() != self.bytes() {
            return None;
        }
        match self.0 {
            ValueType::Interval { .. } => {
                let start = u64::from_be_bytes(tail[..8].try_into().ok()?);
                let end = u64::from_be_bytes(tail[8..].try_into().ok()?);
                Some((start, end))
            }
            ValueType::FixedInterval { width, .. } => {
                let bytes: [u8; 8] = tail.try_into().ok()?;
                crate::encoding::decode_fixed_interval_start(bytes, width).ok()
            }
            _ => None,
        }
    }
}

impl Schema {
    /// The interval-tail descriptor of a pointwise key's determinant;
    /// `None` for scalar keys. A read of the sealed witness — validation
    /// minted the tail once, so no commit or sweep re-walks the
    /// projection.
    #[expect(
        clippy::unused_self,
        reason = "the schema is the witness's minting authority — readers go through it"
    )]
    pub(crate) fn key_tail(&self, key: &KeyStatement) -> Option<IntervalTail> {
        key.tail()
    }
}

/// Validator-minted evidence that a functionality's interval position is
/// final and unique. That shape makes every scalar-prefix determinant group
/// disjoint and start-ordered under the functionality judgment, which is
/// precisely the precondition the interval coverage sweep consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisjointDeterminantProof(());

impl DisjointDeterminantProof {
    /// Consumes the validator witness at the coverage boundary. The method
    /// is intentionally zero-cost; possession of `self` is the check.
    pub(crate) const fn authorize_coverage(self) {
        let Self(()) = self;
    }
}

/// The enforcement plan of a sealed containment. The variant records which
/// judgment is valid; interval coverage carries its load-bearing proof rather
/// than hiding the obligation in a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Enforcement {
    /// Probe an ordinary target key for one scalar tuple.
    /// `key_projection` is the source fields in target-key determinant
    /// order — the sealed permuted projection, so the per-fact encoder
    /// is a straight [`crate::storage::keys::determinant_image`] gather.
    ScalarProbe {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
    },
    /// Sweep the target's pointwise interval segments. `disjoint` proves the
    /// resolved target key enforces disjoint, start-ordered prefix groups.
    /// `key_projection` as in [`Enforcement::ScalarProbe`]. Both interval
    /// encodings travel with the arm — no consumer re-reads
    /// [`Schema::key_tail`].
    IntervalCoverage {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
        disjoint: DisjointDeterminantProof,
        source_tail: IntervalTail,
        target_tail: IntervalTail,
    },
    /// A closed target's stage-1-known answer set.
    Closed { members: MemberSet },
}

impl Enforcement {
    /// The ordinary target key both probe forms resolve; closed targets
    /// compile to membership and therefore have no stored key.
    pub(crate) const fn target_key(&self) -> Option<KeyId> {
        match self {
            Self::ScalarProbe { target_key, .. } | Self::IntervalCoverage { target_key, .. } => {
                Some(*target_key)
            }
            Self::Closed { .. } => None,
        }
    }

    /// Source fields in target-key determinant order; closed targets
    /// have none.
    pub(crate) fn key_projection(&self) -> Option<&[FieldId]> {
        match self {
            Self::ScalarProbe { key_projection, .. }
            | Self::IntervalCoverage { key_projection, .. } => Some(key_projection),
            Self::Closed { .. } => None,
        }
    }
}

/// Capacity's two-arm target plan (CONTRACT C9). Containments keep
/// three-arm [`Enforcement`]; capacity projections refuse interval
/// positions, so coverage is unrepresentable here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CapacityEnforcement {
    /// Probe an ordinary parent key for one scalar tuple.
    /// `key_projection` is the child fields in parent-key determinant
    /// order.
    ScalarProbe {
        target_key: KeyId,
        key_projection: Box<[FieldId]>,
    },
    /// A closed parent's stage-1-known answer set.
    Closed { members: MemberSet },
}

impl CapacityEnforcement {
    /// Child fields in parent-key determinant order; closed parents
    /// project through the statement's source as-is.
    pub(crate) fn key_projection(&self) -> Option<&[FieldId]> {
        match self {
            Self::ScalarProbe { key_projection, .. } => Some(key_projection),
            Self::Closed { .. } => None,
        }
    }
}

/// How the target-side judgment finds surviving sources of one
/// containment. Sealed at validation from the source relation's body —
/// [`check_target`] matches these arms instead of re-asking
/// `body().closed_rows()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Survivors {
    /// Ordinary source: walk stored `R` edges.
    ReverseEdges,
    /// Closed source: scan sealed φ-rows. Interval coverage is
    /// unrepresentable (refused at validation).
    SealedRows,
}

/// The `==` partner of a containment, typed to the containment arena.
/// [`StatementId`] is the materialized-order ordinal, not this pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pairing {
    /// A one-way containment, or a functionality/capacity (those never
    /// occupy this field).
    OneWay,
    /// The `==` partner, by its validation-minted containment witness.
    Mirror(ContainmentId),
}

impl Pairing {
    /// The partner's containment witness, if this is a pair.
    #[must_use]
    pub const fn partner(self) -> Option<ContainmentId> {
        match self {
            Self::OneWay => None,
            Self::Mirror(id) => Some(id),
        }
    }
}

/// Index of a ground axiom in a sealed closed extension. The 256-axiom
/// domain is the type: arbitrary `u64` fact values narrow through
/// [`TryFrom`]; values beyond `u8::MAX` are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AxiomIndex(pub(crate) u8);

impl TryFrom<u64> for AxiomIndex {
    type Error = std::num::TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u8::try_from(value).map(Self)
    }
}

/// A closed relation's compiled member set: one bit per sealed ground
/// axiom, in extension order. The four words encode the declaration-time
/// 256-axiom bound enforced by `schema::validate::validate_extension` and
/// [`MAX_EXTENSION_ROWS`]. Out-of-range indices are absent by contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemberSet {
    words: [u64; 4],
}

impl MemberSet {
    pub(crate) const fn empty() -> Self {
        Self { words: [0; 4] }
    }

    /// Tests membership. Every [`AxiomIndex`] is in the four-word domain.
    #[must_use]
    pub(crate) fn contains(&self, index: AxiomIndex) -> bool {
        let word = usize::from(index.0 / 64);
        self.words[word] & (1 << (index.0 % 64)) != 0
    }

    /// Inserts a sealed axiom. Every [`AxiomIndex`] is in the four-word
    /// domain; the caller has already enforced [`MAX_EXTENSION_ROWS`].
    pub(crate) fn insert(&mut self, index: AxiomIndex) {
        let word = usize::from(index.0 / 64);
        self.words[word] |= 1 << (index.0 % 64);
    }
}

/// One σ-binding whose encoding is a pure function of the value —
/// interned text is unrepresentable. Closed-side walks take this type
/// so a validator-refuted interned arm cannot answer with a silent
/// `false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EncodableCheck {
    /// The literal's canonical encoding, sealed — one byte compare.
    Encoded { field: FieldId, bytes: Box<[u8]> },
    /// A disjunctive binding of encodable literals: satisfaction = any-of.
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

/// One σ-binding check compiled at validate (the staging law applied to
/// the checker, `docs/architecture/30-dependencies.md` § enforcement):
/// everything whose canonical bytes are a pure function of the value seals
/// here, once; only interned text — whose word is per-database dictionary
/// state — remains commit-resolved. The singleton arms are the classic
/// one-compare paths, byte-identical to the pre-set engine; the `Set`
/// arms carry the disjunctive binding's alternatives (canonical order,
/// deduplicated), and satisfaction is membership among them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledCheck {
    /// The literal's canonical encoding, sealed — one byte compare at
    /// judgment, zero encoding work per commit.
    Encoded { field: FieldId, bytes: Box<[u8]> },
    /// A disjunctive binding of encodable literals: the sealed canonical
    /// encodings, satisfaction = any-of.
    EncodedSet {
        field: FieldId,
        alternatives: Box<[Box<[u8]>]>,
    },
    /// A `str` literal: resolves through the delta's pending map then the
    /// committed dictionary at commit; a double miss proves no fact can
    /// satisfy the selection.
    Interned { field: FieldId, text: Box<str> },
    /// A disjunctive binding of `str` literals: each resolves at commit;
    /// a never-interned literal drops out of the disjunction (that arm is
    /// provably unsatisfiable), and all missing proves the binding — and
    /// so the side — unsatisfiable.
    InternedSet {
        field: FieldId,
        texts: Box<[Box<str>]>,
    },
}

/// One side's compiled σ: interned text is unrepresentable on a closed
/// relation (closed columns refuse `str`).
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

/// Both sides' compiled σ checks of one containment or capacity statement.
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
    /// The relation's FIRST fresh field's auto-key (the one id allocator,
    /// `docs/architecture/50-storage.md` § key layout; ruled 2026-07-23,
    /// R16): its determinant IS the `F` row id, it maintains no `U` tree,
    /// and its functionality judgment is the `F` put-conflict itself.
    /// Probes against it read `F` directly — one B-tree descent. One
    /// word by type. The statement's `projection` is that one field.
    FreshRow { field: FieldId },
    /// A scalar (interval-free) functionality.
    Scalar,
    /// An interval-final functionality. The trailing encoding and the
    /// disjointness proof the gate derived travel with the arm — no
    /// consumer re-walks the projection, and no boolean licenses the
    /// sweep.
    Pointwise {
        tail: IntervalTail,
        disjoint: DisjointDeterminantProof,
    },
}

/// One sealed key statement: `R(X) -> R` with its form.
#[derive(Debug, Clone)]
pub struct KeyStatement {
    /// Materialized-order identity. It is fingerprint-pinned and embedded in
    /// storage keys and errors; it is never an arena index.
    pub id: StatementId,
    pub relation: RelationId,
    pub projection: Box<[FieldId]>,
    pub(crate) form: KeyForm,
}

impl KeyStatement {
    /// The form this statement sealed as — match, don't re-test flags.
    #[must_use]
    pub fn form(&self) -> &KeyForm {
        &self.form
    }

    /// The trailing interval encoding of a Pointwise key; `None` on the
    /// other arms.
    #[must_use]
    pub(crate) fn tail(&self) -> Option<IntervalTail> {
        self.form.as_pointwise()
    }
}

impl KeyForm {
    /// The mint field when this key is [`KeyForm::FreshRow`].
    #[must_use]
    pub const fn as_fresh_row(&self) -> Option<FieldId> {
        match *self {
            Self::FreshRow { field } => Some(field),
            Self::Scalar | Self::Pointwise { .. } => None,
        }
    }

    /// Whether this key is [`KeyForm::Pointwise`].
    #[must_use]
    pub const fn is_pointwise(&self) -> bool {
        matches!(self, Self::Pointwise { .. })
    }

    /// The trailing encoding when this key is [`KeyForm::Pointwise`].
    #[must_use]
    pub(crate) const fn as_pointwise(&self) -> Option<IntervalTail> {
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
    /// Materialized-order identity. It is not an arena index.
    pub id: StatementId,
    pub source: Side,
    pub target: Side,
    pub(crate) enforcement: Enforcement,
    /// Target-side survivor scan, sealed from the source relation's body.
    pub(crate) survivors: Survivors,
    /// Both sides' σ literals, compiled once at validate. This is total:
    /// keys cannot reach a containment value.
    pub(crate) checks: CompiledSides,
    /// The `==` partner: the containment whose NORMALIZED sides (the one
    /// statement identity — selections sorted, literal sets canonical)
    /// are exactly this statement's normalized sides swapped, anywhere in
    /// the materialized list — `==` lowers to two containments and the
    /// pairing is a fact of the declaration, sealed here rather than
    /// re-discovered by render-time search
    /// (`docs/architecture/30-dependencies.md`). Typed to the containment
    /// arena — no re-resolution through [`StatementView`]. Normalized, not
    /// raw: statement identity ignores spelling, so a respelled literal
    /// set cannot fork the links of two fingerprint-equal schemas. At most
    /// one partner can exist because [`StatementErrorKind::DuplicateStatement`]
    /// rejects identical normalized statements (two candidate mirrors
    /// would be identical to each other), which makes the links
    /// symmetric. [`Pairing::OneWay`] for every FD and one-way containment.
    ///
    /// [`StatementErrorKind::DuplicateStatement`]: crate::error::StatementErrorKind::DuplicateStatement
    pub pairing: Pairing,
}

impl ContainmentStatement {
    /// The partner's materialized-order ordinal — host citation and
    /// render. The stored identity is [`Self::pairing`].
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
    Duration { field: FieldId, tail: IntervalTail },
}

impl SealedWeight {
    /// The descriptor-facing spelling — fingerprint and render.
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
    Duration { field: FieldId, tail: IntervalTail },
}

impl SealedBound {
    /// The descriptor-facing spelling (`None` = `*`).
    #[must_use]
    pub const fn to_bound(self) -> Option<Bound> {
        match self {
            Self::Unbounded => None,
            Self::Lit(n) => Some(Bound::Lit(n)),
            Self::TargetField(field) => Some(Bound::TargetField(field)),
            Self::Duration { field, .. } => Some(Bound::TargetDuration(field)),
        }
    }

    /// Whether resolving this ceiling needs the parent fact bytes.
    #[must_use]
    pub(crate) const fn needs_parent_fact(self) -> bool {
        matches!(self, Self::TargetField(_) | Self::Duration { .. })
    }
}

/// A resolved capacity ceiling: unbounded vs a finite measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundCeiling {
    Unbounded,
    Finite(u64),
}

/// One sealed capacity statement: `B(Y | ψ) <=[w]{lo..hi} A(X | φ)`.
/// Accepted at declaration with its sealed target-key plan handle
/// (the same probe-ability rule containments resolve —
/// `lean/Bumbledb/Oracle.lean: capacity_plan_decides` is the promised
/// plan); commit-time judging is the enforcement stage's work. Fields
/// sit in the operator's read order — target, weight, window, source
/// (ruled 2026-07-24, C2).
#[derive(Debug, Clone)]
pub struct CapacityStatement {
    /// Materialized-order identity. It is not an arena index.
    pub id: StatementId,
    pub target: Side,
    /// The measure of one source fact; [`SealedWeight::Unit`] is the count
    /// instance (the surviving `<={lo..hi}` utterance). Duration tails
    /// live in-arm (CONTRACT C9).
    pub weight: SealedWeight,
    /// The inclusive lower measure bound — a literal by representation
    /// (C6: dependent floors are unrepresentable).
    pub lo: u64,
    /// The inclusive upper measure bound; [`SealedBound::Unbounded`] is `*`.
    pub hi: SealedBound,
    pub source: Side,
    /// The target-key plan handle: [`CapacityEnforcement::ScalarProbe`]
    /// or [`CapacityEnforcement::Closed`]. Interval coverage is
    /// unrepresentable — capacity projections refuse interval positions
    /// at the gate. Consumed by the commit judge's touched-parent probe
    /// and the sweeper's global re-verification
    /// (`storage/commit/judgment.rs::check_capacities`).
    pub(crate) enforcement: CapacityEnforcement,
    /// Both sides' σ bindings, compiled once at validate — resolved per
    /// commit into [`crate::storage::commit::judgment::Selections`]
    /// exactly as containments' are.
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
    /// The fingerprint-pinned materialized identity of either statement arm.
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
/// and never again — the staging law applied to the feature itself
/// (`docs/architecture/10-data-model.md`).
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
    /// An ordinary row-store. `fresh` is the [`KeyForm::FreshRow`] key
    /// when this relation is the one id allocator's mint (R16); `None`
    /// means row ids mint from the `S` high-water.
    Ordinary { fresh: Option<KeyId> },
    /// Ground axioms, frozen by the fingerprint, virtual in storage.
    /// A closed relation's `fields` open with the synthetic (`id`, U64)
    /// field, so determinants, statements, and queries address the
    /// handle's id uniformly at [`FieldId`] 0
    /// (`docs/architecture/10-data-model.md` § closed relations).
    Closed { extension: Box<[SealedRow]> },
}

impl RelationBody {
    /// Closed extension rows; ordinary has none.
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
    /// `Functionality` statements on this relation, in materialized order.
    keys: Box<[KeyId]>,
    /// `Containment` statements whose source is this relation.
    outgoing: Box<[ContainmentId]>,
    /// `Capacity` statements whose SOURCE (weighed child) is this
    /// relation — the plan derivation walks it per fact op, exactly as
    /// `outgoing`.
    capacity_sources: Box<[CapacityId]>,
    /// `Capacity` statements whose TARGET (parent) is this relation —
    /// a delta parent touches its own key tuple
    /// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`).
    capacity_targets: Box<[CapacityId]>,
    body: RelationBody,
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug, Clone)]
pub struct Schema {
    relations: Box<[Relation]>,
    /// Homogeneous typed arenas. Only validation mints their witness ids.
    keys: Box<[KeyStatement]>,
    containments: Box<[ContainmentStatement]>,
    capacities: Box<[CapacityStatement]>,
    /// The materialized statement list; [`StatementId`] indexes this spine.
    order: Box<[StatementRef]>,
    /// `target_key -> dependents`, indexed by [`KeyId`].
    dependents: Box<[Box<[ContainmentId]>]>,
    /// Non-fatal declaration diagnostics sealed alongside the witness.
    /// Warnings never change acceptance or enforcement.
    warnings: Box<[SchemaWarning]>,
}

/// A non-fatal schema diagnostic. Unlike [`crate::error::SchemaError`], a
/// warning accompanies an accepted, fully enforcing [`Schema`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaWarning {
    /// `key` strictly contains `implied_by` on the same relation. The
    /// smaller determinant already implies the larger one, so the latter
    /// adds determinant writes without strengthening the theory.
    RedundantSuperkey {
        relation: RelationId,
        key: KeyId,
        implied_by: KeyId,
    },
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

    /// The `Fresh`-generation check behind the [`FreshField`] witness: ids
    /// and generation, typed. Two callers, one law —
    /// [`crate::Db::fresh_field`] at resolution (mints the schema-bound
    /// witness), and the mint's per-transaction sequence init
    /// (`WriteDelta::fresh_mark`) at the dyn boundary, where
    /// `Db<SchemaDescriptor>` handles share one typestate and the
    /// witness's binding proves nothing across descriptors.
    ///
    /// # Errors
    ///
    /// `UnknownRelation`/`UnknownField` on an out-of-range id;
    /// `NotAFreshField` when the field's generation is not `Fresh`.
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

    /// All sealed capacity statements, in typed-arena order.
    #[must_use]
    pub fn capacities(&self) -> &[CapacityStatement] {
        &self.capacities
    }

    /// A capacity statement selected by its validation-minted witness.
    #[must_use]
    pub fn capacity(&self, id: CapacityId) -> &CapacityStatement {
        &self.capacities[usize::from(id.0)]
    }

    /// The bounds-checked sibling of [`Schema::capacity`] for ids
    /// arriving as dynamic data.
    #[must_use]
    pub fn capacity_checked(&self, id: CapacityId) -> Option<&CapacityStatement> {
        self.capacities.get(usize::from(id.0))
    }

    /// Non-fatal diagnostics recorded while sealing this schema.
    #[must_use]
    pub fn warnings(&self) -> &[SchemaWarning] {
        &self.warnings
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
        self.view(self.order[usize::from(id.0)])
    }

    /// The materialized-order ordinal of a typed spine slot.
    #[must_use]
    pub fn id_of(&self, statement: StatementRef) -> StatementId {
        self.view(statement).id()
    }

    /// The typed spine slot a materialized-order identity selects.
    #[must_use]
    pub(crate) fn cite(&self, id: StatementId) -> StatementRef {
        self.order[usize::from(id.0)]
    }

    /// The bounds-checked sibling of [`Schema::statement`].
    #[must_use]
    pub fn statement_checked(&self, id: StatementId) -> Option<StatementView<'_>> {
        self.order
            .get(usize::from(id.0))
            .copied()
            .map(|statement| self.view(statement))
    }

    /// The borrowed arm a spine slot selects.
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

    /// The materialized statement spine, in fingerprint-pinned order.
    pub fn statements(&self) -> impl Iterator<Item = StatementView<'_>> + '_ {
        self.order
            .iter()
            .copied()
            .map(|statement| self.view(statement))
    }

    /// Whether every relation this statement consults is closed — the
    /// instance-independent obligations schema validation discharges.
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

    /// The instance-dependent complete roster: every statement whose
    /// truth can still depend on ordinary facts, derived by exhaustive
    /// match over [`StatementView`]. Closed-constant statements are
    /// skipped — validation already refuted self-refuting theories.
    /// This is not the delta-restricted empty-plan judgment.
    #[must_use]
    pub(crate) fn complete_obligations(&self) -> CompleteObligations<'_> {
        CompleteObligations { schema: self }
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

    /// The mint field of an ordinary fresh-keyed relation (R16), read
    /// off [`KeyForm::FreshRow`]. Closed and fresh-less relations have
    /// none — one site names the field (CONTRACT C9 / schema-006).
    #[must_use]
    pub(crate) fn fresh_mint_field(&self, id: RelationId) -> Option<FieldId> {
        let key = self.relation(id).fresh_key()?;
        match self.key(key).form() {
            KeyForm::FreshRow { field } => Some(*field),
            KeyForm::Scalar | KeyForm::Pointwise { .. } => None,
        }
    }
}

/// The complete initial-admission roster, derived by exhaustive match
/// over the materialized [`StatementView`] spine. A statement form
/// absent from [`CompleteObligations::classify`] is a compile error.
/// Hand-enumerated rosters beside this spine are refused.
pub(crate) struct CompleteObligations<'schema> {
    schema: &'schema Schema,
}

/// One instance-dependent obligation. Inner matches on [`KeyForm`],
/// [`Enforcement`], and [`CapacityEnforcement`] force the roster to grow
/// with a new arm.
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
    /// The stored identity of this obligation — arena id, not the
    /// materialized-order ordinal.
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

    /// The fingerprint-pinned materialized identity.
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

    /// Exhaustive on every statement arm and every enforcement arm.
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
