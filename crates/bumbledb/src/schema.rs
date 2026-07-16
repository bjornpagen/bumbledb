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

/// Witness index into [`Schema::windows`] — minted only by validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub(crate) u16);

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
    /// (`docs/architecture/10-data-model.md`). `width` selects within
    /// the interval FAMILY: `None` is the general type (16-byte
    /// `start ‖ end` encoding, rays representable); `Some(w)` is
    /// `interval<E, w>` — the width is the type (the `bytes<N>`
    /// precedent), the encoding stores ONLY the start (8 bytes; the
    /// end derives as `start + w`), wide values are unrepresentable,
    /// and the Q2 bound `start + w < MAX_END` bars ray-hood by
    /// construction (`lean/Bumbledb/Values.lean: FixedU64.not_ray`).
    /// Admitted under the admission rule: a type parameter is
    /// admitted iff it changes the encoding — `w` does; a parameter
    /// that merely checks is a CHECK constraint, refused
    /// (`docs/architecture/10-data-model.md` § the admission rule).
    /// The width is a fingerprint input — a width change is a new
    /// theory. `w ≥ 1`, validated at declaration.
    Interval {
        element: IntervalElement,
        width: Option<u64>,
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
/// schema `S` — the handle of the untyped mint path
/// ([`crate::WriteTx::alloc_at`]). Fields are private and
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
}

/// The one `Value` ↔ `ValueType` compatibility check (kind and String UTF-8)
/// — IR validation, bind-time, the dynamic write
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
        // The interval family: the general type takes any checked
        // interval of its element; a fixed-width type takes exactly the
        // declared width, never a ray (Q2: `start + w < MAX_END` — the
        // ray end IS `MAX_END`, so `!is_ray()` is the bound;
        // `lean/Bumbledb/Values.lean: FixedU64.not_ray`). A wide or
        // narrow value is a kind mismatch — the width is the type.
        (
            Value::IntervalU64(interval),
            ValueType::Interval {
                element: IntervalElement::U64,
                width,
            },
        ) => match width {
            None => Ok(()),
            Some(w) if interval.end() - interval.start() == *w && !interval.is_ray() => Ok(()),
            Some(_) => Err(ValueMismatch::Type),
        },
        (
            Value::IntervalI64(interval),
            ValueType::Interval {
                element: IntervalElement::I64,
                width,
            },
        ) => match width {
            None => Ok(()),
            Some(w) if interval.end().abs_diff(interval.start()) == *w && !interval.is_ray() => {
                Ok(())
            }
            Some(_) => Err(ValueMismatch::Type),
        },
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
        _ => Err(ValueMismatch::Type),
    }
}

/// One σ binding's literal set — the disjunctive selection fragment
/// (`lean/Bumbledb/Schema.lean: Selection`): the selected field's value is
/// a MEMBER of the spelled set, bindings read conjunctively. The singleton
/// arm is today's equality by representation
/// (`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff`) and
/// stays zero-cost — no per-literal indirection on the one-literal path.
/// The `Many` arm's canonical form is sorted and duplicate-free with at
/// least two literals; validation canonicalizes the order and rejects the
/// degenerate spellings (`docs/architecture/30-dependencies.md`
/// § validation roster).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralSet {
    /// One literal: the equality binding — the whole accepted σ fragment
    /// before the disjunctive extension, unchanged in meaning.
    One(Value),
    /// Two or more literals, read disjunctively. The sets are first-class,
    /// not per-literal sugar: a window over a disjunctive selection is not
    /// any conjunction of per-literal windows
    /// (`lean/Bumbledb/Countermodels.lean:
    /// disjunctive_window_not_literal_conjunction`).
    Many(Box<[Value]>),
}

impl LiteralSet {
    /// The literals, one or more — the `One` arm borrows in place
    /// (`std::slice::from_ref`), so the singleton path allocates and
    /// indirects nothing.
    #[must_use]
    pub fn literals(&self) -> &[Value] {
        match self {
            Self::One(literal) => std::slice::from_ref(literal),
            Self::Many(literals) => literals,
        }
    }

    /// The singleton reading, when this binding is today's equality.
    #[must_use]
    pub fn as_equality(&self) -> Option<&Value> {
        match self {
            Self::One(literal) => Some(literal),
            Self::Many(_) => None,
        }
    }
}

impl From<Value> for LiteralSet {
    fn from(literal: Value) -> Self {
        Self::One(literal)
    }
}

/// One side of a containment: the single-atom query `R(X | φ)`
/// (`docs/architecture/30-dependencies.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    pub relation: RelationId,
    /// π — ordered, the statement's written order.
    pub projection: Box<[FieldId]>,
    /// σ — a set of (field, literal-set) bindings read conjunctively,
    /// each binding a disjunction over its spelled set; empty =
    /// unselected. Literals are the one shared [`Value`] sum
    /// (`docs/architecture/30-dependencies.md` — any type's literal binds
    /// in σ; dependencies and queries share one representation).
    pub selection: Box<[(FieldId, LiteralSet)]>,
}

/// One dependency statement: a judgment about queries
/// (`docs/architecture/30-dependencies.md`). Statements are anonymous —
/// their identity is their materialized-order [`StatementId`]. There is no
/// bidirectional variant: `==` is lowered to two `Containment` statements
/// with the sides swapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementDescriptor {
    /// `R(X) -> R`: πX is injective on R. X is ordered (the order defines
    /// the determinant key), non-empty, duplicate-free.
    Functionality {
        relation: RelationId,
        projection: Box<[FieldId]>,
    },
    /// `A(X | φ) <= B(Y | ψ)`: πX(σφ(A)) ⊆ πY(σψ(B)) as sets of tuples.
    Containment { source: Side, target: Side },
    /// `B(Y | ψ) <={lo..hi} A(X | φ)` (B-family, target-left — the left
    /// side is `target`): the cardinality window — per selected target
    /// fact, the count of selected source facts sharing its projected
    /// tuple lies in the window
    /// (`lean/Bumbledb/Cardinality.lean: CardinalityWindow`;
    /// `lean/Bumbledb/Schema.lean: Statement.cardinality`). `hi = None`
    /// is the `*` spelling — the only spelling of "no upper bound";
    /// `lo = hi` is the `{n}` exact-count spelling (`{0}` the exclusion).
    Cardinality {
        source: Side,
        /// The inclusive lower count bound.
        lo: u64,
        /// The inclusive upper count bound; `None` is `*`.
        hi: Option<u64>,
        target: Side,
    },
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

/// The trailing interval encoding of a pointwise determinant or an
/// interval-final projection: how many encoded bytes the interval
/// position occupies and how its exclusive end derives. The general type
/// stores both order-preserving halves; a fixed-width type stores the
/// START word only — the width is the type's, and the bias of both
/// element encodings is additive, so `start_word + w` IS the encoded end
/// (`lean/Bumbledb/Values.lean: encode_fixed_order_u64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IntervalTail {
    /// `Some(w)` = the fixed width; `None` = general (`start ‖ end`).
    pub(crate) width: Option<u64>,
}

impl IntervalTail {
    /// Trailing encoded bytes: 16 general, 8 fixed.
    pub(crate) const fn bytes(self) -> usize {
        match self.width {
            None => 16,
            Some(_) => 8,
        }
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
        match self.width {
            None => {
                let start = u64::from_be_bytes(tail[..8].try_into().ok()?);
                let end = u64::from_be_bytes(tail[8..].try_into().ok()?);
                Some((start, end))
            }
            Some(width) => {
                let bytes: [u8; 8] = tail.try_into().ok()?;
                crate::encoding::decode_fixed_interval_start(bytes, width).ok()
            }
        }
    }
}

impl Schema {
    /// The interval-tail descriptor of a pointwise key's determinant;
    /// `None` for scalar keys.
    pub(crate) fn key_tail(&self, key: &KeyStatement) -> Option<IntervalTail> {
        self.relation(key.relation).interval_tail(&key.projection)
    }

    /// The interval-tail descriptor of a containment's SOURCE projection
    /// — the shape of the reverse-edge key-bytes tail (the source fact's
    /// interval encodes at its own field's width).
    pub(crate) fn source_tail(&self, statement: &ContainmentStatement) -> Option<IntervalTail> {
        self.relation(statement.source.relation)
            .interval_tail(&statement.source.projection)
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
    ScalarProbe {
        target_key: KeyId,
        key_permutation: Box<[u16]>,
    },
    /// Sweep the target's pointwise interval segments. `disjoint` proves the
    /// resolved target key enforces disjoint, start-ordered prefix groups.
    IntervalCoverage {
        target_key: KeyId,
        key_permutation: Box<[u16]>,
        disjoint: DisjointDeterminantProof,
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
}

/// Index of a ground axiom in a sealed closed extension. Arbitrary `u64`
/// fact values narrow through [`TryFrom`]; values beyond `u16` are absent,
/// and [`MemberSet::contains`] makes indices `256..=u16::MAX` absent too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AxiomIndex(pub(crate) u16);

impl TryFrom<u64> for AxiomIndex {
    type Error = std::num::TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u16::try_from(value).map(Self)
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

    /// Tests membership; an index outside the four-word domain is absent.
    #[must_use]
    pub(crate) fn contains(&self, index: AxiomIndex) -> bool {
        let word = usize::from(index.0 / 64);
        self.words
            .get(word)
            .is_some_and(|bits| bits & (1 << (index.0 % 64)) != 0)
    }

    /// Inserts a sealed axiom. The caller has already enforced
    /// [`MAX_EXTENSION_ROWS`], so its declaration index is below 256.
    pub(crate) fn insert(&mut self, index: AxiomIndex) {
        let word = usize::from(index.0 / 64);
        self.words[word] |= 1 << (index.0 % 64);
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

/// One sealed cardinality window: `B(Y | ψ) <={lo..hi} A(X | φ)`.
/// Accepted at declaration with its sealed target-key plan handle
/// (the same probe-ability rule containments resolve —
/// `lean/Bumbledb/Oracle.lean: cardinality_plan_decides` is the promised
/// plan); commit-time judging is the enforcement stage's work.
#[derive(Debug)]
pub struct CardinalityStatement {
    /// Materialized-order identity. It is not an arena index.
    pub id: StatementId,
    pub source: Side,
    /// The inclusive lower count bound.
    pub lo: u64,
    /// The inclusive upper count bound; `None` is `*`.
    pub hi: Option<u64>,
    pub target: Side,
    /// The target-key plan handle (`ScalarProbe` or `Closed`; windows
    /// refuse interval positions, so `IntervalCoverage` is unreachable).
    /// Consumed by the commit judge's touched-parent probe and the
    /// sweeper's global re-verification
    /// (`storage/commit/judgment.rs::check_windows`).
    pub(crate) enforcement: Enforcement,
    /// Both sides' σ bindings, compiled once at validate — resolved per
    /// commit into [`crate::storage::commit::judgment::Selections`]
    /// exactly as containments' are.
    pub(crate) checks: CompiledSides,
}

/// The global materialized-order spine: a [`StatementId`] selects one typed
/// arena and one slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementRef {
    Key(KeyId),
    Containment(ContainmentId),
    Cardinality(WindowId),
}

/// A borrowed sealed statement for display and other order-preserving walks.
/// Consumers that already hold a typed id use the total arena accessors.
#[derive(Debug, Clone, Copy)]
pub enum StatementView<'schema> {
    Key(KeyId, &'schema KeyStatement),
    Containment(ContainmentId, &'schema ContainmentStatement),
    Cardinality(WindowId, &'schema CardinalityStatement),
}

impl StatementView<'_> {
    /// The fingerprint-pinned materialized identity of either statement arm.
    #[must_use]
    pub const fn id(self) -> StatementId {
        match self {
            Self::Key(_, statement) => statement.id,
            Self::Containment(_, statement) => statement.id,
            Self::Cardinality(_, statement) => statement.id,
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
    /// field, so determinants, statements, and queries address the handle's id
    /// uniformly at [`FieldId`] 0 (`docs/architecture/10-data-model.md`
    /// § closed relations).
    extension: Option<Box<[SealedRow]>>,
    /// `Functionality` statements on this relation, in materialized order.
    keys: Box<[KeyId]>,
    /// `Containment` statements whose source is this relation.
    outgoing: Box<[ContainmentId]>,
    /// `Cardinality` statements whose SOURCE (counted child) is this
    /// relation — the plan derivation walks it per fact op, exactly as
    /// `outgoing`.
    window_sources: Box<[WindowId]>,
    /// `Cardinality` statements whose TARGET (parent) is this relation —
    /// a delta parent touches its own key tuple
    /// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`).
    window_targets: Box<[WindowId]>,
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug)]
pub struct Schema {
    relations: Box<[Relation]>,
    /// Homogeneous typed arenas. Only validation mints their witness ids.
    keys: Box<[KeyStatement]>,
    containments: Box<[ContainmentStatement]>,
    windows: Box<[CardinalityStatement]>,
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
            return Err(FactShapeError::UnknownRelation { relation });
        };
        let Some(descriptor) = rel.fields().get(usize::from(field.0)) else {
            return Err(FactShapeError::UnknownField { relation, field });
        };
        if descriptor.generation != Generation::Fresh {
            return Err(FactShapeError::NotAFreshField { relation, field });
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

    /// All sealed cardinality windows, in typed-arena order.
    #[must_use]
    pub fn windows(&self) -> &[CardinalityStatement] {
        &self.windows
    }

    /// A cardinality window selected by its validation-minted witness.
    #[must_use]
    pub fn window(&self, id: WindowId) -> &CardinalityStatement {
        &self.windows[usize::from(id.0)]
    }

    /// The bounds-checked sibling of [`Schema::window`] for ids arriving
    /// as dynamic data.
    #[must_use]
    pub fn window_checked(&self, id: WindowId) -> Option<&CardinalityStatement> {
        self.windows.get(usize::from(id.0))
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
            StatementRef::Cardinality(window) => {
                StatementView::Cardinality(window, self.window(window))
            }
        }
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
