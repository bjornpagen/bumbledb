//! The workspace error taxonomy, categorized per
//! `docs/architecture/70-api.md`.
//!
//! Everything reachable from user input or disk returns these typed errors;
//! panics are reserved for programmer-invariant violations. Payloads carry
//! ids and owned fact bytes, never formatted strings — no `format!` runs on
//! a hot path; `Display` formats lazily when the host actually prints.

mod convert;
mod display;

use std::path::PathBuf;

use crate::encoding::InternId;
use crate::ir::{InteriorId, ParamId, VarId};
use crate::schema::KeyId;
use crate::schema::StatementRef;
use crate::schema::fingerprint::SchemaFingerprint;
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId, ValueType};

/// Occurrence index of an atom inside the first failing rule (positive
/// atoms first, then negated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtomIndex(pub usize);

/// Find-term / head-position index inside a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindIndex(pub usize);

/// Rule index in the query's rule list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleIndex(pub usize);

/// Extension-row index in a closed relation's ground axioms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

macro_rules! display_index {
    ($($ty:ty),* $(,)?) => {
        $(
            impl std::fmt::Display for $ty {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }
        )*
    };
}

display_index!(AtomIndex, FindIndex, RuleIndex, RowIndex);

/// A witnessed value against the bound it was required to equal.
/// Operand order is the type: `witnessed` is what was observed,
/// `required` is the bound — never `(expected, actual)` in one
/// variant and `(found, expected)` in the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mismatch<T> {
    pub witnessed: T,
    pub required: T,
}

/// A quantity that crossed its ceiling. Operand order is the type:
/// `observed` is what was measured, `ceiling` is the bound it must
/// not exceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Exceeded<T> {
    pub observed: T,
    pub ceiling: T,
}

/// One declared key offered as owned evidence in a target-key rejection.
/// The diagnostic may outlive the descriptor, so it carries no schema
/// references — the field NAMES ride beside the ids as owned strings
/// (`projection_names` pairs `projection` positionwise), so the refusal
/// speaks the caller's own vocabulary without a descriptor lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetKeyCandidate {
    pub key: KeyId,
    pub projection: Box<[FieldId]>,
    pub projection_names: Box<[Box<str>]>,
}

/// Corruption detected while decoding stored bytes — a hard error, never a
/// skip, never a default (`docs/architecture/50-storage.md`). The offline
/// sweeper reports the same facts as [`crate::StoreFinding::Corruption`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorruptionError {
    /// A Bool byte other than `0x00`/`0x01` — there is no distinct "true".
    InvalidBool(u8),
    /// Interval bytes whose `start >= end` — the empty interval is
    /// unrepresentable (a fact never denotes nothing), so a stored one is
    /// corruption, not a value. Carries the raw 16 bytes.
    InvalidInterval([u8; 16]),
    /// A fixed-width (`interval<E, w>`) start word at or past the Q2
    /// bound `start + w < MAX_END`: the derived end would reach the
    /// ceiling (ray territory — unconstructible in the fixed family) or
    /// overflow the domain, so a stored such start is corruption exactly
    /// as an inverted interval is. Carries the raw 8 start bytes.
    InvalidFixedIntervalStart([u8; 8]),
    /// The `_meta` database or one of its required keys is genuinely
    /// absent inside an initialized store: the environment is not a
    /// usable bumbledb database. A present value that fails to decode is
    /// [`CorruptionError::MalformedValue`] naming the key — the two
    /// states point at opposite remedies (initialize vs. investigate a
    /// torn write), so one error value never encodes both
    /// (`docs/architecture/50-storage.md` § the `_meta` block, ruled
    /// 2026-07-23, R18). A half-created store (no `_meta` over an empty
    /// root) is unusable at a destination path: constructors refuse
    /// [`crate::error::Error::DestinationExists`], and `open` refuses
    /// [`crate::error::Error::AlreadyInitialized`] — never corruption.
    MetaMissing,
    /// An intern id with no reverse dictionary entry — a fact referencing it
    /// is corrupt. The miss sentinel is not a stored id; the sweeper
    /// reports a stored [`InternId::SENTINEL`] as [`Self::Malformed`].
    DanglingInternId(InternId),
    /// A row id obtained from `M`/`U` has no `F` entry in the same snapshot.
    MissingFact { relation: RelationId, row_id: u64 },
    /// A live `M` entry's `F` row or `U` determinant was absent at delete time —
    /// the write-side M/F disagreement (the read side raises
    /// [`CorruptionError::MissingFact`]).
    MembershipDesync { relation: RelationId, row_id: u64 },
    /// Base state disagreed with a net disposition the delta proved at op
    /// time — a fact commit would insert already live in `M`, or one it
    /// would delete already gone. The single-writer mutex holds committed
    /// state stable for the delta's lifetime
    /// (`docs/architecture/50-storage.md`), so the disagreement is
    /// unambiguously corruption, never a race.
    DispositionDesync { relation: RelationId },
    /// A stored fact's length differs from the schema's fact width.
    WrongFactWidth {
        relation: RelationId,
        row_id: u64,
        mismatch: Mismatch<usize>,
    },
    /// The `F` scan yielded a different number of rows than the stored `S`
    /// row count — the derived counters have desynced from the facts.
    RowCountMismatch { relation: RelationId, stored: u64 },
    /// A stored `S` row count exceeds a witness the store itself provides
    /// — the `_data` DBI entry count, which spans every namespace and so
    /// over-approximates any one relation's rows. The reopen-trust
    /// ceiling (`docs/architecture/50-storage.md`): a claim above it
    /// cannot be a real row count and would otherwise size an
    /// allocation, so it is typed corruption *before* a byte is
    /// allocated (the scan cross-check,
    /// [`CorruptionError::RowCountMismatch`], stays the exactness
    /// guarantee).
    CounterDesync {
        relation: RelationId,
        exceeded: Exceeded<u64>,
    },
    /// A stored value (a counter, row id, or dictionary id) failed to
    /// decode; the static string names which width — a diagnosis, not a
    /// formatted payload. Lifecycle and integrity kinds are named
    /// variants, not strings in this arm.
    MalformedValue(&'static str),
    /// The dictionary reverse map already holds this minted id.
    DictReverseIdReuse,
    /// A stored string's bytes are not UTF-8 — distinct from a dangling id
    /// (the reverse entry exists; its content is mojibake).
    NonUtf8Intern(u64),
    /// A stored `bytes<N>` field with a nonzero byte in its trailing pad
    /// — the pad is encoding, not data, so a nonzero pad byte is exactly
    /// as corrupt as a non-0/1 Bool byte. Carries the offending trailing
    /// word's 8 bytes.
    NonzeroFixedBytesPad([u8; 8]),
    // --- Offline-sweep structural facts (twinless until this cut) ---
    // The sweeper found these at rest; they are corruption, not a second
    // vocabulary. Runtime twins that already existed stay above.
    /// A live `F` fact whose `M` entry is absent or names another row.
    FactWithoutMembership {
        relation: RelationId,
        row_id: u64,
        membership_key: Box<[u8]>,
    },
    /// An `M` entry whose row id resolves to no `F` fact hashing back to
    /// its key.
    MembershipWithoutFact {
        relation: RelationId,
        row_id: u64,
        membership_key: Box<[u8]>,
    },
    /// A live `F` fact whose determinant tuple is absent from `U` under a
    /// key statement (or held there by another row).
    FactWithoutDeterminant {
        relation: RelationId,
        statement: StatementId,
        row_id: u64,
        determinant_key: Box<[u8]>,
    },
    /// A `U` entry whose row id resolves to no live fact re-deriving the
    /// same determinant bytes.
    DeterminantWithoutFact {
        relation: RelationId,
        statement: StatementId,
        determinant_key: Box<[u8]>,
    },
    /// Two successive determinant entries of one scalar-prefix group with
    /// overlapping intervals.
    PointwiseOverlap {
        relation: RelationId,
        statement: StatementId,
        first: Box<[u8]>,
        second: Box<[u8]>,
    },
    /// A live source fact inside φ whose `R` edge is absent.
    FactWithoutReverseEdge {
        statement: StatementId,
        relation: RelationId,
        row_id: u64,
        reverse_key: Box<[u8]>,
    },
    /// An `R` edge that resolves to no live source fact still inside φ
    /// re-deriving the same key bytes.
    ReverseEdgeWithoutFact {
        statement: StatementId,
        reverse_key: Box<[u8]>,
    },
    /// A containment or capacity `R` edge whose value slot disagrees with
    /// the live source fact's weight-field encoding.
    ReverseEdgeWeightDesync {
        statement: StatementId,
        reverse_key: Box<[u8]>,
        stored: Box<[u8]>,
        derived: Box<[u8]>,
    },
    /// The stored `S` row count disagrees with the `F`-scan count. The
    /// runtime twin [`Self::RowCountMismatch`] carries no counted witness.
    RowCountDesync {
        relation: RelationId,
        stored: u64,
        counted: u64,
    },
    /// The stored `S` row-id high-water does not exceed an observed row
    /// id. Fresh-less relations only.
    RowIdHighWaterLow {
        relation: RelationId,
        stored: u64,
        max_row_id: u64,
    },
    /// A fresh-keyed relation's `F` row id disagrees with its first fresh
    /// field's value.
    FreshRowDesync {
        relation: RelationId,
        row_id: u64,
        fresh: u64,
    },
    /// The stored `Q` next-value fails the ratchet law: a committed fresh
    /// value sits at or beyond it.
    FreshNextValueLow {
        relation: RelationId,
        field: FieldId,
        stored: u64,
        max_fresh: u64,
    },
    /// A `_dict` reverse entry whose forward twin is absent, or maps the
    /// hashed bytes to a different id.
    DictForwardDesync {
        intern_id: InternId,
        /// What the forward map holds for the reverse entry's bytes.
        forward: Option<InternId>,
    },
    /// A `_dict` reverse id at or beyond the `_meta` next-id counter.
    DictNextIdLow {
        stored: InternId,
        reverse_id: InternId,
    },
    /// A `U` entry under a fresh-row auto-key — that key maintains no `U`
    /// tree, so the entry's existence is the fact.
    FreshRowDeterminantEntry {
        relation: RelationId,
        statement: StatementId,
        determinant_key: Box<[u8]>,
    },
    /// A fact references an intern id at or beyond the `_meta` dictionary
    /// next-id counter.
    InternBeyondNextId {
        relation: RelationId,
        row_id: u64,
        intern_id: InternId,
        next_id: InternId,
    },
    /// An `F`/`M`/`U`/`R` entry naming a closed relation. Closed relations
    /// are virtual — a stored entry's existence is the fact.
    ClosedRelationEntry {
        relation: RelationId,
        key: Box<[u8]>,
    },
    /// An entry that does not parse under the schema, including a fact
    /// field with a noncanonical encoding, or a stored intern equal to
    /// [`InternId::SENTINEL`]. The static string names the failing shape;
    /// the key is the offending entry. Distinct from
    /// [`Self::MalformedValue`], which names a width without a key.
    Malformed { key: Box<[u8]>, what: &'static str },
}

/// A schema declaration error (the validation boundary,
/// `docs/architecture/10-data-model.md`). Every illegal schema shape has a
/// distinct variant; an invalid schema is unconstructible, not flagged.
///
/// Two levels, and the partition is typed: declaration-scoped variants
/// live here and carry no statement id; every statement-roster rejection
/// is the one [`SchemaError::Statement`] arm — id beside its
/// [`StatementErrorKind`], one kind variant per roster line of
/// `docs/architecture/30-dependencies.md`, no catch-all. The roster's
/// "FD with selection" and "non-key FD form" lines have no variants:
/// [`crate::schema::StatementDescriptor::Functionality`] carries neither a
/// selection nor a Y side, so both shapes are unrepresentable rather than
/// rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    DuplicateRelationName {
        name: Box<str>,
    },
    DuplicateFieldName {
        relation: RelationId,
        name: Box<str>,
    },
    FreshOnNonU64 {
        relation: RelationId,
        field: FieldId,
    },
    /// A `bytes<N>` field with N = 0 or N > 64: zero bytes denote
    /// nothing, and 64 bytes (8 words, two cache lines of key material)
    /// is the width ceiling — digests in the wild are 16/20/32/64
    /// (`docs/architecture/10-data-model.md`).
    FixedBytesWidthOutOfRange {
        relation: RelationId,
        field: FieldId,
        len: u16,
    },
    /// An `interval<E, w>` field with `w = 0` (zero points denote
    /// nothing — a fact never denotes nothing) or `w = u64::MAX` (no
    /// start satisfies the Q2 bound in either element domain, so the
    /// type would be empty). Every other width is a real type whose
    /// carrier the bound narrows honestly.
    IntervalWidthOutOfRange {
        relation: RelationId,
        field: FieldId,
        width: u64,
    },
    /// A relation whose derived COLUMN count overflows the image's u16
    /// column index (`crate::image::ColumnSpan`): an interval field
    /// spans two word columns, a `bytes<N>` field its ⌈N/8⌉, every
    /// other field one — the declaration is rejected here, never
    /// discovered at image-build time (the same declaration-boundary
    /// discipline as [`StatementErrorKind::DeterminantKeyTooWide`]). The gate
    /// also keeps every `FieldId` within u16: a field occupies at
    /// least one column.
    RelationTooManyColumns {
        relation: RelationId,
        columns: usize,
    },
    /// A statement roster past the u16 id space: the MATERIALIZED
    /// statements (declared plus the fresh/closed auto-keys) number
    /// more than 65,536 — rejected typed at the declaration boundary,
    /// never the id-mint expect. Carries the materialized count.
    TooManyStatements {
        count: usize,
    },

    // --- Closed-relation roster (10-data-model § closed relations) ---
    /// A closed relation with no rows is a vocabulary of nothing — write
    /// no relation.
    EmptyExtension {
        relation: RelationId,
    },
    /// More ground axioms than [`crate::schema::MAX_EXTENSION_ROWS`]: a
    /// vocabulary larger than 256 is policy data wearing a vocabulary
    /// costume, and the cap keeps every compiled word-set a fixed 4×u64
    /// bitset.
    ExtensionTooManyRows {
        relation: RelationId,
        count: usize,
    },
    /// Two extension rows declare one handle — the handle is the row's
    /// identity, and an identity names one axiom.
    DuplicateExtensionHandle {
        relation: RelationId,
        handle: Box<str>,
    },
    /// An extension row's value count differs from the declared intrinsic
    /// columns (the handle is not a column; neither is the synthetic id).
    ExtensionArityMismatch {
        relation: RelationId,
        row: RowIndex,
        mismatch: Mismatch<usize>,
    },
    /// An extension value does not inhabit its column's structural type —
    /// the one shared value check, as selection literals.
    ExtensionValueTypeMismatch {
        relation: RelationId,
        row: RowIndex,
        field: FieldId,
    },
    /// A ray `[start, ∞)` as a ground axiom: an unbounded end says the
    /// theory's constant is still running, and a still-running span is
    /// policy, not an intrinsic property (the intrinsic-vs-policy law) —
    /// rays live in ordinary relations, where the witnessed write that
    /// eventually closes them is expressible
    /// (`docs/architecture/10-data-model.md`, the refusal).
    ExtensionIntervalRay {
        relation: RelationId,
        row: RowIndex,
        field: FieldId,
    },
    /// `str` on a closed relation: the handle IS the label, and interned
    /// columns on a virtual relation would force dictionary writes at open
    /// — the store contains zero vocabulary bytes.
    StrOnClosedRelation {
        relation: RelationId,
        field: FieldId,
    },
    /// `fresh` on a closed relation: identity is the handle, and ground
    /// axioms are never minted.
    FreshOnClosedRelation {
        relation: RelationId,
        field: FieldId,
    },

    // --- Statement roster (30-dependencies § validation roster) ---
    /// A statement-roster rejection: the offending statement's
    /// materialized-order id beside the roster line it violated
    /// ([`StatementErrorKind`]). The declaration/statement partition is
    /// the representation, not a comment banner: a statement-scoped kind
    /// cannot ship without its id, so `display_with`'s rendered citation
    /// is total by construction — misplacing a future variant is
    /// unrepresentable rather than untested.
    Statement {
        statement: StatementId,
        kind: StatementErrorKind,
    },
}

/// One violated line of the statement-validation roster
/// (`docs/architecture/30-dependencies.md` § validation roster) — the
/// kind half of [`SchemaError::Statement`]: one variant per roster line,
/// no catch-all; each doc comment cites its line. Payloads carry ids and
/// owned evidence; the statement id lives on the carrier, so no variant
/// can forget it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementErrorKind {
    /// Roster "unknown relation … ids": a statement names a relation
    /// outside the schema.
    UnknownRelation { relation: RelationId },
    /// Roster "unknown … field ids": a projection or selection names a
    /// field outside its relation.
    UnknownField {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "empty … projections": a projection with no fields.
    EmptyProjection { relation: RelationId },
    /// Roster "duplicate-carrying projections": a field twice in one
    /// projection.
    DuplicateProjectionField {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "duplicate-carrying projections", the selection sibling: a
    /// field bound twice in one selection σ (a set of bindings).
    DuplicateSelectionField {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "a degenerate literal set": a `Many` binding with fewer than
    /// two literals — the empty set selects nothing (write no statement)
    /// and the one-literal set is the `One` spelling, kept the only
    /// singleton by representation
    /// (`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff` —
    /// a singleton set is exactly today's equality).
    DegenerateSelectionSet {
        relation: RelationId,
        field: FieldId,
        len: usize,
    },
    /// Roster "a duplicate literal within one binding's set": the set is
    /// canonical — sorted, duplicate-free — so a repeated literal is
    /// rejected, not silently collapsed (write it once).
    DuplicateSelectionLiteral {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "an inverted window": literal `hi < lo` is satisfied by no
    /// measure — the statement is unsatisfiable as declared. The
    /// canonical bounds are `lo < hi` (an exact measure is `lo = hi`,
    /// the `{n}` spelling). Literal-gated: a dependent ceiling resolves
    /// per row and is not statically invertible.
    CapacityInvertedWindow { lo: u64, hi: u64 },
    /// Roster "the vacuous window": `0..*` admits every measure at any
    /// weight — the statement provably says nothing
    /// (`lean/Bumbledb/Capacity.lean: capacity_zero_star`), and a
    /// statement that says nothing is not a statement (the
    /// canonical-utterance law, `docs/architecture/70-api.md`).
    CapacityVacuousWindow,
    /// Roster "the containment respelled": unit `1..*` says exactly what
    /// the bare containment `target <= source` says
    /// (`lean/Bumbledb/Subsumption.lean: window_floor_containment`) — one
    /// meaning, one spelling: drop the window and declare the
    /// containment. Fires on the count instance ONLY (the per-aggregate
    /// ban law): `<=[w]{1..*}` — "positive total" — is no existence
    /// claim over rows and stays legal.
    CapacityContainmentWindow,
    /// Roster "an interval position in a capacity projection" — refused
    /// v0: the group key identifies FACTS per parent, and an interval
    /// position would make the group ambiguous between facts and points;
    /// intervals enter through the MEASURE argument
    /// (`[Duration(field)]`), never the group key
    /// (`lean/Bumbledb/Capacity.lean` § v0 refusals; *trigger* for
    /// lifting: a sighted counting-over-denotation workload — counting
    /// points, not rows).
    CapacityIntervalPosition {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "a signed or non-u64 weight field": a `[field]` weight
    /// must name a u64-encoded SOURCE position — a signed encoding is
    /// the typed polarity refusal (a negative weight would let an insert
    /// lower a sum, breaking the delta scheduler), and every other
    /// encoding equally fails to measure.
    CapacityWeightNotU64 {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "a Duration weight over a non-interval field":
    /// `[Duration(field)]` reads an interval position's measure — the
    /// named SOURCE field must be interval-typed.
    CapacityWeightNotDuration {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "a dependent bound not a u64 field of TARGET's row": a
    /// bound ident resolves by name against the target's whole field
    /// roster (ruled 2026-07-24, C1) and must be u64-encoded — signed
    /// and non-scalar encodings are refused.
    CapacityBoundNotU64 {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "a Duration bound over a non-interval field":
    /// `{..Duration(field)}` bounds by a TARGET interval's measure — the
    /// named field must be interval-typed.
    CapacityBoundNotDuration {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "dimension mixing": a unit (count) window against a
    /// `Duration` bound — a count of facts bounded by a span of time is
    /// a dimension error (ruled 2026-07-24, C18; the legal pairings:
    /// Duration weights under Duration or literal bounds, u64 weights
    /// under u64-field or literal bounds — u64 is u64).
    CapacityDimensionMixing { field: FieldId },
    /// Roster ">1 interval position": two interval fields in one FD
    /// projection would be 2-D exclusion, which the ordered determinant cannot
    /// answer. Carries the second interval field.
    FunctionalityMultipleIntervals {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "interval not in final position": the neighbor probe needs
    /// the scalar prefix as its group.
    FunctionalityIntervalNotLast {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "duplicate statements", the Functionality-specific form: two
    /// FDs over one field set on one relation assert one judgment — the
    /// second determinant is pure write amplification, and rejecting it makes
    /// containment target-key resolution unambiguous.
    DuplicateFunctionality { earlier: StatementId },
    /// Roster "determinant width overflow": Σ projected field widths exceeds
    /// [`crate::storage::keys::MAX_DETERMINANT_WIDTH`] — rejected at declaration,
    /// never discovered at write time.
    DeterminantKeyTooWide { width: usize },
    /// Roster "arity mismatch between sides": |X| ≠ |Y|.
    ContainmentArityMismatch { mismatch: Mismatch<usize> },
    /// Roster "positional structural-type mismatch" — including its
    /// called-out instance, an interval position against a scalar one.
    ContainmentTypeMismatch { position: usize },
    /// Roster "a selected field also projected": a constant column — write
    /// the statement you mean.
    SelectedFieldProjected {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "selection literal type mismatch": the literal's variant is
    /// not the field's structural type.
    SelectionLiteralTypeMismatch {
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "IND whose target projection matches no key of the target":
    /// probe-ability requires Y to be a permutation of a declared key.
    /// Names ride beside the ids (`target_name`, and `projection_names`
    /// pairing `projection` positionwise) as owned strings — validation
    /// has the descriptor in hand, and the refusal must speak the
    /// caller's vocabulary at every boundary (60-containment-parity).
    NoMatchingTargetKey {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
        available: Box<[TargetKeyCandidate]>,
    },
    /// Roster "IND … (or, with an interval position, no pointwise key
    /// carrying it)": the coverage walk needs the target's own key to keep
    /// its intervals disjoint and ordered. Same owned-name payloads as
    /// [`StatementErrorKind::NoMatchingTargetKey`].
    NoPointwiseTargetKey {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
        available: Box<[TargetKeyCandidate]>,
    },
    /// An interval position on a containment with a closed side — refused
    /// v0: a pointwise judgment against a closed relation would mix the
    /// coverage walk with virtual storage, and a constant source's
    /// coverage demand has no delete to re-judge it under
    /// (`docs/architecture/30-dependencies.md`, the refusal —
    /// *trigger* for lifting it: a census sighting). Carries the closed
    /// relation.
    ClosedContainmentInterval { relation: RelationId },
    /// Roster "a closed-target projection that is not the synthetic id":
    /// the handle id is the ONE probe-able identity of a closed target
    /// (the auto-key `R(id) -> R`), so the projection must be exactly
    /// `{id}`. Its own variant, not a reused
    /// [`StatementErrorKind::NoMatchingTargetKey`]: declared payload keys on a
    /// closed relation are legal, point-read-served objects whose field
    /// set may equal the refused projection — the refusal reason is
    /// closedness, a different fact than key absence, and the two carry
    /// different encodings. Names ride beside the ids exactly as on
    /// [`StatementErrorKind::NoMatchingTargetKey`].
    ClosedTargetNotHandle {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
    },
    /// A statement between constants that the ground axioms refute: both
    /// sides of the judgment are sealed at validate, so its truth is
    /// decidable here — and a theory whose axioms refute its own statement
    /// has no model to commit (`docs/architecture/30-dependencies.md`,
    /// "a committed database is a model of its theory, always"). For a
    /// containment, `row` is the source axiom outside the compiled member
    /// set; for a functionality, the second axiom of the colliding pair.
    ClosedStatementRefuted { relation: RelationId, row: RowIndex },
    /// Roster "duplicate statements (identical normalized sides and form —
    /// write it once)": selections compare sorted by field id.
    DuplicateStatement { earlier: StatementId },
}

impl StatementErrorKind {
    /// The kind at its statement — the one construction site of
    /// [`SchemaError::Statement`].
    #[must_use]
    pub fn at(self, statement: StatementId) -> SchemaError {
        SchemaError::Statement {
            statement,
            kind: self,
        }
    }
}

/// A dynamic-surface id that does not resolve: relation, field, fresh
/// field, or key statement. These are not fact shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynIdError {
    /// The relation id is outside the schema — ETL input is data, so an
    /// out-of-range id at the dynamic surface (`insert_dyn`/`delete_dyn`/
    /// `scan`/`fresh_field`) is a typed error, never an index
    /// panic.
    UnknownRelation { relation: RelationId },
    /// The field id is outside its relation — the field sibling of
    /// [`DynIdError::UnknownRelation`], raised by the id-addressed
    /// dynamic surface ([`crate::Db::fresh_field`]).
    UnknownField {
        relation: RelationId,
        field: FieldId,
    },
    /// [`crate::Db::fresh_field`]'s field is not `Fresh` generation — no
    /// witness exists, so no mint can be asked for. Raised at resolution,
    /// and again by the mint's per-transaction sequence init at the dyn
    /// boundary, where `Db<SchemaDescriptor>` handles share one typestate
    /// and a foreign descriptor's witness arrives well-typed (the
    /// schema-bound witness law — [`crate::FreshField`]).
    NotAFreshField {
        relation: RelationId,
        field: FieldId,
    },
    /// [`crate::WriteTx::get_dyn`]'s statement id is not a `Functionality`
    /// on the queried relation (out of range, a containment, or another
    /// relation's key) — the dynamic point-read surface is data, so the
    /// mismatch is a typed error, never an index panic.
    NotAKeyStatement {
        relation: RelationId,
        statement: StatementId,
    },
}

/// A mis-shaped dynamic fact on the untyped write surface
/// (`insert_dyn`/`delete_dyn`): ETL input is data, so shape
/// problems are typed errors, not panics (`docs/architecture/70-api.md`).
/// Id-resolution failures are [`DynIdError`], nested as [`Self::Id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactShapeError {
    /// A dynamic-surface id that does not resolve.
    Id(DynIdError),
    ArityMismatch {
        relation: RelationId,
        mismatch: Mismatch<usize>,
    },
    TypeMismatch {
        relation: RelationId,
        field: FieldId,
    },
    /// A collection's variable-width payload (string or `bytes<N>` cells)
    /// crossed the 4 GiB transport bound of the accepted collection's u32
    /// arena spans (`api/db/collection.rs` — one collection is one call's
    /// facts, so the bound is transport, not capacity: split the
    /// collection). ETL input is data, so the bound is a typed refusal,
    /// never a panic (`docs/architecture/70-api.md`).
    PayloadBound { relation: RelationId },
}

impl From<DynIdError> for FactShapeError {
    fn from(err: DynIdError) -> Self {
        Self::Id(err)
    }
}

/// A query validation error (the IR boundary): one variant per roster item
/// in `docs/architecture/20-query-ir.md`, returned at prepare time.
///
/// Rules validate one at a time, in order: every rule-local payload (an
/// `atom` occurrence index, a comparison `index`, a `find` position, a
/// `var`) names a position **inside the first failing rule**. An `atom`
/// payload is an *occurrence* index within that rule: positive atoms first
/// in rule order, then negated atoms — negated atoms are checked under the
/// same per-atom rules and share the same diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    /// A query with no rules denotes nothing — the empty union is not a
    /// query; write no query (`docs/architecture/20-query-ir.md`, the
    /// rules shape).
    EmptyRuleSet,
    /// The rule-count cap ([`crate::ir::MAX_RULES`]), counted
    /// independently of the per-rule occurrence cap.
    TooManyRules {
        count: usize,
    },
    /// DNF distribution of the rules' condition trees would produce more
    /// rules than the cap ([`crate::ir::MAX_RULES`]) — the exponential
    /// case is rejected at declaration, exactly like determinant-width
    /// overflow. `produced` names the blowup: the structural term count
    /// across all rules, judged before a single disjunct is materialized
    /// (so before duplicate collapse).
    DnfExceedsRules {
        exceeded: Exceeded<usize>,
    },
    /// A rule's condition trees nest deeper than
    /// [`crate::ir::MAX_CONDITION_DEPTH`] — the boundary check for every
    /// recursive tree walk (the trust-boundary law: hostile nesting must
    /// be a typed rejection, never a stack exhaustion). Judged
    /// iteratively, before any recursion sees the tree.
    ConditionNestingTooDeep {
        rule: RuleIndex,
        exceeded: Exceeded<usize>,
    },
    /// A rule's find-term count differs from the head's arity — rules
    /// align against the head position by position.
    HeadArityMismatch {
        rule: RuleIndex,
        mismatch: Mismatch<usize>,
    },
    /// A rule's find term at `position` resolves to a different
    /// structural type than the head's pinned positional type row (rule
    /// 0's row pins it; every later rule must agree).
    HeadTypeMismatch {
        rule: RuleIndex,
        position: FindIndex,
    },
    /// A rule's find term at `position` has the wrong *shape* against the
    /// head: a variable where the head names an aggregate, an aggregate
    /// where it names a variable, or a different aggregate-op kind.
    HeadAggregateMismatch {
        rule: RuleIndex,
        position: FindIndex,
    },
    /// A nullary `Count` in a fold-free head of a hand-written 2+-rule
    /// query (ruled 2026-07-23, R1): under the head-projection law a
    /// fold-free head admits one projection per group, so the Count is
    /// definitionally the constant 1 — an uninformative query, made
    /// unrepresentable. The modeling answer: one Count per disjunct,
    /// host-merged. DNF-derived
    /// rule sets are exempt — or-transparency (R2) keeps their fold
    /// domain the written rule's full binding set, so their Count counts
    /// (`docs/architecture/20-query-ir.md` § aggregation).
    CountAcrossRules {
        rules: usize,
    },
    UnknownRelation {
        atom: AtomIndex,
        relation: RelationId,
    },
    UnknownField {
        atom: AtomIndex,
        field: FieldId,
    },
    DuplicateFieldBinding {
        atom: AtomIndex,
        field: FieldId,
    },
    VariableTypeConflict {
        var: VarId,
    },
    LiteralTypeMismatch {
        atom: AtomIndex,
        field: FieldId,
    },
    /// The point-domain law (`docs/architecture/10-data-model.md`): points
    /// are `MIN ..= MAX−1`; `end == MAX` denotes the ray `[s, ∞)`, so an
    /// element-typed literal equal to the domain ceiling in a membership
    /// binding can never be inside any interval — rejected typed, never
    /// silently unmatchable (comparison sites report
    /// [`ValidationError::ComparisonPointLiteralAtCeiling`]).
    PointLiteralAtCeiling {
        atom: AtomIndex,
        field: FieldId,
    },
    /// Param ids must be dense (0..n) across scalars and sets jointly: a
    /// gap would be a positional slot whose supplied value is never
    /// type-checked.
    ParamIdGap {
        param: ParamId,
    },
    ParamTypeConflict {
        param: ParamId,
    },
    /// A `ParamId` used both as a scalar (`Term::Param`) and as a set
    /// (`Term::ParamSet`) — a param is one or the other, never both.
    ParamScalarAndSet {
        param: ParamId,
    },
    /// A `ParamSet` under any comparison operator but `Eq` — `Ne(x, set)`
    /// reads as ambiguous quantification; "not in set" is a negated atom
    /// or the host's complement, written explicitly.
    ParamSetComparison {
        index: usize,
    },
    /// A `ParamSet` anchored at an interval type: param sets hold points;
    /// interval-set params are not a thing.
    IntervalParamSet {
        param: ParamId,
    },
    /// Type rules violated: mixed-type operands or an interval operator over
    /// non-interval sides. Single-type order refusals have dedicated variants.
    IllegalComparison {
        index: usize,
    },
    /// An order operator (`Lt`/`Le`/`Gt`/`Ge`) with an interval operand —
    /// intervals are unordered; the predictable mistake gets the dedicated
    /// diagnostic (`docs/architecture/20-query-ir.md` § comparison rules).
    OrderComparisonOnInterval {
        index: usize,
    },
    /// An order operator with a `bytes<N>` operand — a digest's
    /// lexicographic order is an encoding artifact, and admitting it
    /// would make hash-function choice semantically visible. Identity
    /// only: `Eq`/`Ne` and membership (`docs/architecture/10-data-model.md`,
    /// the order-on-bytes refusal).
    OrderComparisonOnFixedBytes {
        index: usize,
    },
    /// An order operator with a String operand. Intern ids are an equality
    /// representation, not a collation; String is equality-only.
    OrderComparisonOnString {
        index: usize,
    },
    /// An order operator over a closed-bound variable (ruled 2026-07-23,
    /// R4): a closed reference's declaration-id order is a
    /// declaration-order accident, not semantics — refused exactly as
    /// the enum's ordinal order was, judged once in the engine so the
    /// wall is identical on every surface. Bool is the one orderability
    /// carve-out (R3) and cannot be closed-bound.
    OrderComparisonOnClosedReference {
        index: usize,
    },
    /// Neither side is a variable — write the query you mean.
    ConstantComparison {
        index: usize,
    },
    /// Both sides are the same variable — constant-valued; write the
    /// query you mean.
    SelfComparison {
        index: usize,
    },
    /// An element-typed literal equal to the domain ceiling as a
    /// comparison operand against an interval side (the comparison-site
    /// sibling of [`ValidationError::PointLiteralAtCeiling`]): `MAX` is
    /// the ray's ∞, never a point.
    ComparisonPointLiteralAtCeiling {
        index: usize,
    },
    /// An `Allen` comparison whose literal mask is empty — no basic
    /// relation can hold, so the condition is "never": write no query
    /// (`docs/architecture/20-query-ir.md` § the Allen operator).
    EmptyAllenMask {
        index: usize,
    },
    /// An `Allen` comparison whose literal mask is all 13 basics — every
    /// pair satisfies it, so the condition is "always": write no
    /// condition.
    FullAllenMask {
        index: usize,
    },
    /// An element-typed variable whose positive atom bindings are all
    /// interval-field memberships: membership binds no enumerable domain,
    /// so every point variable needs at least one positive scalar-field
    /// binding.
    MembershipOnlyVariable {
        var: VarId,
    },
    /// Negation safety: a variable occurring in a negated atom must occur
    /// in some positive atom — a negated atom binds nothing, it only
    /// rejects.
    NegatedVariableUnbound {
        var: VarId,
    },
    /// Datalog safety: a find (or aggregate-input) variable bound by no
    /// positive atom.
    UnboundFindVariable {
        var: VarId,
    },
    ComparisonOnlyVariable {
        var: VarId,
    },
    EmptyFinds,
    DuplicateFindTerm {
        index: usize,
    },
    /// A query with no positive atoms is invalid — negated atoms alone
    /// bind nothing.
    NoPositiveAtoms,
    /// Sum/Min/Max over a variable outside the fold's roster — Sum takes
    /// U64/I64; Min/Max take the orderable types, bool included (`Max`
    /// over bool is Any, `Min` is All — ruled 2026-07-23, R3).
    AggregateInputType {
        find: FindIndex,
    },
    /// A `Sum`/`Min`/`Max` fold over a closed-bound
    /// variable (ruled 2026-07-23, R4): its words are declaration
    /// indices, so folding or sweeping their order is ordering an
    /// accident — refused exactly as the order comparison is
    /// ([`Self::OrderComparisonOnClosedReference`]).
    AggregateOverClosedReference {
        find: FindIndex,
    },
    /// Count is nullary; it carries no variable.
    CountWithVariable {
        find: FindIndex,
    },
    /// Sum/Min/Max require a variable.
    AggregateWithoutVariable {
        find: FindIndex,
    },
    AggregateOverGroupKey {
        find: FindIndex,
    },
    /// A second `Pack` term in one head: the multi-`Pack` product has no
    /// sighting and is refused. *Trigger* for admitting it: a real query
    /// needing two coalesced columns in one row
    /// (`docs/architecture/20-query-ir.md` § aggregation).
    MultiplePackTerms {
        find: FindIndex,
    },
    /// `Pack` beside a fold aggregate (Sum/Min/Max/Count):
    /// `Pack` is relation-shaped — a fold column repeated per segment row
    /// is a join in aggregate costume. Coalesced-time accounting
    /// (`Sum∘Duration∘Pack`) is two prepared queries or a host fold over
    /// packed answers; *trigger* for a composed form: a measured two-pass
    /// budget violation.
    MixedPackAndFold {
        find: FindIndex,
    },
    /// `Pack` over a non-interval variable: the coalesce is defined by
    /// the interval point-set denotation and by nothing else.
    PackInputType {
        find: FindIndex,
    },
    /// A `Term::Measure` in an atom binding: the measure is a
    /// computation over a bound interval variable, not a bindable value
    /// — its legal positions are a find term, the aggregated input of
    /// `Sum`/`Min`/`Max`, and one side of an order comparison
    /// (`docs/architecture/20-query-ir.md`, § the measure).
    DurationInBinding {
        atom: AtomIndex,
        field: FieldId,
    },
    /// `Duration(v)` over a variable that did not resolve to an interval
    /// type: the measure is defined by the interval denotation and by
    /// nothing else.
    DurationOverNonInterval {
        var: VarId,
    },
    /// A `FindTerm::AggregateMeasure` whose op is not `Sum`/`Min`/`Max`
    /// — `Count` is nullary and Pack coalesces intervals, not measures.
    DurationAggregateOp {
        find: FindIndex,
    },
    /// A `Term::Measure` under any operator but the order comparisons
    /// (`Lt`/`Le`/`Gt`/`Ge`) — the measure's one comparison position
    /// (`docs/architecture/20-query-ir.md`, § the measure).
    DurationComparisonOperator {
        index: usize,
    },
    /// `Duration` on both sides of one comparison: the legal shape is one
    /// measure side against a u64 term or literal — write two
    /// comparisons against a shared bound, or compute in the host.
    DurationBothSides {
        index: usize,
    },
    /// Planner cap: the exhaustive left-deep DP accepts at most
    /// `plan::planner::MAX_OCCURRENCES` atom occurrences — negated
    /// occurrences counted, they consume plan-time work.
    TooManyAtoms {
        count: usize,
    },
    /// Planner cap: at most 128 distinct variables (dense bitset width).
    TooManyVariables {
        count: usize,
    },

    // --- Interiors and rec (20-query-ir.md § interiors / rec; 01-language.md roster) ---
    /// Derived-table count does not fit `u32` — id-width, not a product
    /// cap. CQ counts `interiors.len()`; Reach counts `interiors.len() + 1`
    /// (the rec). Counted with `usize` before any
    /// [`crate::ir::InteriorId`] is constructed.
    InteriorIdOverflow {
        count: usize,
    },
    /// An [`crate::ir::Interior`] with zero rules — not
    /// [`ValidationError::EmptyRuleSet`] (that name is the main query).
    EmptyInterior {
        interior: InteriorId,
    },
    /// `Rec.base` is empty: a constantly-empty rec (math: `T(∅) = ∅`).
    EmptyRecursiveBase,
    /// `Rec.rec` is empty: that is an interior — write an interior.
    EmptyRecursiveStep,
    /// A base arm whose body names the rec (positive or negated).
    SelfInBase,
    /// A rec arm with zero positive self-atoms.
    RecArmMissingSelf,
    /// A rec arm with two or more positive self-atoms.
    NonlinearRecArm,
    /// A negated atom anywhere in the rec SCC (base or rec, EDB or
    /// interior or self). Self-negation is the wall; finished-table
    /// negation is this-cut scope.
    NegationInRec,
    /// An `Interior` atom names a derived table outside the query.
    UnknownInterior {
        atom: AtomIndex,
        interior: InteriorId,
    },
    /// An `Interior` binding's `FieldId` sits at or beyond the target
    /// derived head's arity.
    InteriorColumnOutOfRange {
        atom: AtomIndex,
        field: FieldId,
    },
    /// Interior `at` reads `interior` where `interior ≥ at`, or any
    /// interior reads the rec id.
    InteriorNotPrior {
        interior: InteriorId,
        at: InteriorId,
    },
    /// A fold on an interior or rec **head** (bound-var law).
    AggregateInInterior {
        interior: InteriorId,
    },
    /// A measure find on an interior or rec **head**.
    MeasureInInterior {
        interior: InteriorId,
    },
    /// A measure site in a rec **body** (comparison / binding).
    MeasureInRec,
}

/// Which side of a containment statement the commit-time judgment found
/// unsatisfied (`docs/architecture/30-dependencies.md` § enforcement).
///
/// `Ord` is citation order: within one statement cited in both
/// directions, source before target ([`Violations`]' sort key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    /// An inserted source fact inside σ has no target: the key probe
    /// missed, or the coverage walk found a gap.
    SourceUnsatisfied,
    /// A deleted target key tuple is still required by a surviving
    /// source fact (the reverse-edge scan).
    TargetRequired,
}

/// Theory rejection inside a successful outer [`Result`]: the candidate
/// formed correctly and either holds or fails the declared theory.
/// Infrastructure failure stays `Err(Error)`; an unadmitted value cannot
/// occupy an admitted slot.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission<T> {
    Accepted(T),
    Rejected(Violations),
}

impl<T> Admission<T> {
    /// Unwraps an accepted admission. Panics on rejection — hosts that
    /// have already proved the theory, and tests.
    ///
    /// # Panics
    ///
    /// When `self` is [`Admission::Rejected`].
    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            Self::Accepted(value) => value,
            Self::Rejected(violations) => panic!("admission rejected: {violations}"),
        }
    }

    /// [`Self::unwrap`] with a caller message.
    ///
    /// # Panics
    ///
    /// When `self` is [`Admission::Rejected`].
    #[track_caller]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Self::Accepted(value) => value,
            Self::Rejected(violations) => {
                panic!("{msg}: admission rejected: {violations}")
            }
        }
    }

    /// Maps the accepted payload. Rejection is unchanged.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Admission<U> {
        match self {
            Self::Accepted(value) => Admission::Accepted(f(value)),
            Self::Rejected(violations) => Admission::Rejected(violations),
        }
    }
}

/// One probe's witnessed judgment: either the obligation holds or it
/// cites exactly one violation. Checkers return this; they never mint
/// an error as a semantic verdict.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Holds,
    Violated(Violation),
}

/// A durable write that admitted: the callback value and the generation
/// after the commit. Rejection carries no callback value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed<R> {
    pub value: R,
    pub generation: GenerationId,
}

/// [`crate::Db::write_from`]'s proved outcomes: admission plus the
/// compare-and-swap miss. A moved generation is an answer, not an
/// error — the caller proceeds on the two generations in the arm.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalWrite<R> {
    Accepted(Committed<R>),
    Rejected(Violations),
    Moved {
        witnessed: GenerationId,
        current: GenerationId,
    },
}

impl<R> ConditionalWrite<R> {
    /// Unwraps an accepted conditional write. Panics on rejection or a
    /// moved generation.
    ///
    /// # Panics
    ///
    /// When `self` is not [`ConditionalWrite::Accepted`].
    #[track_caller]
    pub fn unwrap(self) -> Committed<R> {
        match self {
            Self::Accepted(committed) => committed,
            Self::Rejected(violations) => {
                panic!("conditional write rejected: {violations}")
            }
            Self::Moved { witnessed, current } => {
                panic!("conditional write moved ({witnessed} → {current})")
            }
        }
    }

    /// [`Self::unwrap`] with a caller message.
    ///
    /// # Panics
    ///
    /// When `self` is not [`ConditionalWrite::Accepted`].
    #[track_caller]
    pub fn expect(self, msg: &str) -> Committed<R> {
        match self {
            Self::Accepted(committed) => committed,
            Self::Rejected(violations) => {
                panic!("{msg}: conditional write rejected: {violations}")
            }
            Self::Moved { witnessed, current } => {
                panic!("{msg}: conditional write moved ({witnessed} → {current})")
            }
        }
    }
}

/// Scalar put-conflict vs pointwise neighbor probe — two conviction
/// shapes, not an optional incumbent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    /// The determinant bytes inside the cited fact already identify the
    /// collision.
    Scalar,
    /// The probe names both parties.
    Pointwise { incumbent: Box<[u8]> },
}

/// One violated statement of a rejected admission — the element of
/// [`Violations`]. One body: the typed spine slot, the convicting fact
/// bytes, and a per-law detail. Storage row ids never appear
/// (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A `Functionality` statement violated by the final state: two live
    /// facts claim one key — the same determinant bytes (scalar put-conflict),
    /// or overlapping intervals within one scalar-prefix group (the
    /// pointwise neighbor probe).
    Functionality {
        statement: StatementRef,
        fact: Box<[u8]>,
        conflict: Conflict,
    },
    /// A `Containment` statement violated by the final state
    /// (`docs/architecture/30-dependencies.md` § judged on final states).
    /// `fact` is canonical source-fact bytes on either side: the judgment
    /// speaks about sources — a missing target is named by the source
    /// that requires it.
    Containment {
        statement: StatementRef,
        direction: Direction,
        /// The source fact: the inserted fact whose target is missing
        /// (`SourceUnsatisfied`), or the surviving fact still requiring a
        /// deleted target key (`TargetRequired`).
        fact: Box<[u8]>,
    },
    /// A `Capacity` statement violated by the final state: a selected
    /// parent fact whose child-group MEASURE (Σ weight over the
    /// deduplicated group; unit weight = count) falls outside the
    /// window — below the floor or above the resolved ceiling
    /// (`lean/Bumbledb/Capacity.lean: CapacityLaw`).
    Capacity {
        statement: StatementRef,
        /// The convicting parent fact: the ψ-selected holder of the
        /// touched key tuple whose group measure is out of window.
        fact: Box<[u8]>,
        /// The witnessed group measure, u128 whole — the accumulator's
        /// width crosses untruncated (ruled 2026-07-24, C3). On
        /// conviction the judge completes the full walk, so the reported
        /// measure is the group's total, walk-order-independent (ruled
        /// 2026-07-24, C14: the clip serves the verdict, the full sum
        /// serves the witness).
        measure: u128,
    },
}

impl Violation {
    pub(crate) fn functionality(
        statement: StatementRef,
        fact: Box<[u8]>,
        conflict: Conflict,
    ) -> Self {
        Self::Functionality {
            statement,
            fact,
            conflict,
        }
    }

    pub(crate) fn containment(
        statement: StatementRef,
        direction: Direction,
        fact: Box<[u8]>,
    ) -> Self {
        Self::Containment {
            statement,
            direction,
            fact,
        }
    }

    pub(crate) fn capacity(statement: StatementRef, fact: Box<[u8]>, measure: u128) -> Self {
        Self::Capacity {
            statement,
            fact,
            measure,
        }
    }

    /// The typed spine slot of the violated statement.
    #[must_use]
    pub const fn statement(&self) -> StatementRef {
        match *self {
            Self::Functionality { statement, .. }
            | Self::Containment { statement, .. }
            | Self::Capacity { statement, .. } => statement,
        }
    }

    /// The materialized-order ordinal — host citation, fingerprints, and
    /// [`Violations`]' sort key. Derived from [`Self::statement`] through
    /// the schema that minted the ref.
    #[must_use]
    pub fn statement_id(&self, schema: &crate::schema::Schema) -> StatementId {
        schema.id_of(self.statement())
    }

    /// Canonical bytes of the convicting fact.
    #[must_use]
    pub fn fact(&self) -> &[u8] {
        match self {
            Self::Functionality { fact, .. }
            | Self::Containment { fact, .. }
            | Self::Capacity { fact, .. } => fact,
        }
    }

    /// The incumbent fact of a pointwise key collision, if this citation
    /// names one.
    #[must_use]
    pub fn incumbent(&self) -> Option<&[u8]> {
        match self {
            Self::Functionality {
                conflict: Conflict::Pointwise { incumbent },
                ..
            } => Some(incumbent),
            Self::Functionality {
                conflict: Conflict::Scalar,
                ..
            }
            | Self::Containment { .. }
            | Self::Capacity { .. } => None,
        }
    }

    /// The citation identity — [`Violations`]' sort and dedup key:
    /// statement id (materialized order), then direction (source before
    /// target; key and capacity statements have none). Witness
    /// facts, measures, and defect kinds are deliberately outside the
    /// identity: a statement is cited once per direction, whatever the
    /// count of facts convicting it.
    fn citation(&self, schema: &crate::schema::Schema) -> (StatementId, Option<Direction>) {
        let id = schema.id_of(self.statement());
        match self {
            Self::Functionality { .. } | Self::Capacity { .. } => (id, None),
            Self::Containment { direction, .. } => (id, Some(*direction)),
        }
    }
}

/// One cited fact of a violation, decoded to owned field values — the
/// bindings-consumable twin of the violation's canonical fact bytes
/// (`docs/architecture/30-dependencies.md` § rendering the rejection).
/// Decoding happens AT rejection time, inside the commit boundary,
/// because that is the only time it is possible: an inserted fact's
/// `str` fields may carry provisional intern ids minted by the very
/// transaction the rejection aborts — after the abort those ids resolve
/// nowhere, so a post-hoc decode helper would misread genuine rejections
/// as corruption. `values` are in sealed field order (a closed
/// relation's synthetic id first), `str` fields resolved to owned
/// strings; the allocation is acceptable at rejection time (the
/// rejection IS the repair diagnostic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedFact {
    relation: RelationId,
    values: Box<[bumbledb_theory::Value]>,
}

impl CitedFact {
    /// Sealed constructor: `values` is one decoded field per sealed
    /// position of `relation`. The caller supplies the relation's field
    /// count as the layout proof — a length mismatch is a programmer
    /// error, never a host-constructible cited fact.
    pub(crate) fn new(
        relation: RelationId,
        field_count: usize,
        values: Box<[bumbledb_theory::Value]>,
    ) -> Self {
        debug_assert_eq!(
            values.len(),
            field_count,
            "cited-fact values are one per sealed field"
        );
        let _ = field_count;
        Self { relation, values }
    }

    /// The cited fact's relation: the statement's own relation for a
    /// key, the SOURCE relation for a containment (the judgment speaks
    /// about sources), the TARGET (parent) relation for a capacity
    /// statement.
    #[must_use]
    pub const fn relation(&self) -> RelationId {
        self.relation
    }

    /// One decoded [`bumbledb_theory::Value`] per sealed field, in
    /// declaration order.
    #[must_use]
    pub fn values(&self) -> &[bumbledb_theory::Value] {
        &self.values
    }
}

/// The complete violation set of one rejected admission — sealed: nonempty,
/// one citation per statement (per direction for a containment), sorted
/// by materialized statement order. The only constructors sort and dedup
/// and refuse emptiness, so an empty, unsorted, or duplicated set is
/// unrepresentable — a rejection IS this set, never an arbitrary
/// representative (`docs/architecture/30-dependencies.md` § judged on
/// final states).
///
/// Alongside each citation ride its **decoded cited facts**
/// ([`CitedFact`], via [`Violations::cited_facts`] /
/// [`Violations::citations`]): the commit boundary attaches them at
/// rejection time — while the rejecting transaction's pending interns
/// are still resolvable — so a bindings layer renders the whole
/// rejection from plain data without the typed fact structs
/// (`docs/architecture/30-dependencies.md` § rendering the rejection).
/// Decoration is best-effort: a decode failure degrades the citation
/// and never converts a `Rejected` into an `Err`.
///
/// Complete admission cites containments source-to-target (the sweeper's
/// convention: a candidate, like a committed store, has no just-inserted
/// side). The incremental checker keeps its two-direction citations.
/// The dedup key remains `(StatementId, Option<Direction>)`.
///
/// Host construction is unrepresentable — fields are private and the
/// only constructors seal:
///
/// ```compile_fail
/// let _ = bumbledb::Violations { citations: Box::new([]) };
/// ```
/// Sealed citations paired with their cited facts — the stored
/// [`Violations`] shape. Undecorated entries carry an empty cited slice.
pub(crate) type CitedCitations = Box<[(Violation, Box<[CitedFact]>)]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violations {
    /// Sealed citations paired with their cited facts — nonempty,
    /// sorted, unique by citation key. Undecorated entries carry an
    /// empty cited slice.
    citations: CitedCitations,
}

impl Violations {
    /// Seals a collector's raw finds: stable-sorts by citation (so the
    /// first-discovered witness of each citation survives), dedups by
    /// citation, and returns [`Admission::Accepted`] for the empty
    /// collection — the accept path, never an empty rejection.
    pub(crate) fn seal(schema: &crate::schema::Schema, mut found: Vec<Violation>) -> Admission<()> {
        if found.is_empty() {
            return Admission::Accepted(());
        }
        found.sort_by_key(|violation| violation.citation(schema));
        found.dedup_by_key(|violation| violation.citation(schema));
        Admission::Rejected(Self {
            citations: found
                .into_iter()
                .map(|violation| (violation, Box::<[CitedFact]>::from([])))
                .collect(),
        })
    }

    /// Rebuilds a sealed rejection from already-paired citations.
    /// The decoration loop builds the pairs where it decodes — this is
    /// the one constructor that takes the stored shape
    /// (`storage/catalog/decorate.rs`, `storage/commit/write.rs`).
    pub(crate) fn from_pairs(citations: CitedCitations) -> Self {
        debug_assert!(
            !citations.is_empty(),
            "a sealed rejection is nonempty by construction"
        );
        Self { citations }
    }

    /// Number of citations in sealed order.
    #[must_use]
    pub fn len(&self) -> usize {
        self.citations.len()
    }

    /// Always false: a sealed rejection is nonempty by construction.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.citations.is_empty()
    }

    /// The citation at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Violation> {
        self.citations.get(index).map(|(violation, _)| violation)
    }

    /// Citations paired with their cited facts — the stored shape.
    #[must_use]
    pub fn as_slice(&self) -> &[(Violation, Box<[CitedFact]>)] {
        &self.citations
    }

    /// Every violation, in citation order.
    pub fn iter(&self) -> impl Iterator<Item = &Violation> {
        self.citations.iter().map(|(violation, _)| violation)
    }

    /// The decoded cited facts of the citation at `index` — the
    /// violation's `fact` first, then its `incumbent` where one exists.
    /// Empty for a set no decode pass decorated (the sweeper's re-play
    /// findings) and for an out-of-range index.
    #[must_use]
    pub fn cited_facts(&self, index: usize) -> &[CitedFact] {
        self.citations
            .get(index)
            .map_or(&[], |(_, cited)| cited.as_ref())
    }

    /// Iterates citations paired with their decoded cited facts.
    pub fn citations(&self) -> impl Iterator<Item = (&Violation, &[CitedFact])> {
        self.citations
            .iter()
            .map(|(violation, cited)| (violation, cited.as_ref()))
    }
}

fn take_citation((violation, _): (Violation, Box<[CitedFact]>)) -> Violation {
    violation
}

fn citation_ref((violation, _): &(Violation, Box<[CitedFact]>)) -> &Violation {
    violation
}

impl IntoIterator for Violations {
    type Item = Violation;
    type IntoIter = std::iter::Map<
        std::vec::IntoIter<(Violation, Box<[CitedFact]>)>,
        fn((Violation, Box<[CitedFact]>)) -> Violation,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.citations.into_vec().into_iter().map(take_citation)
    }
}

impl<'a> IntoIterator for &'a Violations {
    type Item = &'a Violation;
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (Violation, Box<[CitedFact]>)>,
        fn(&'a (Violation, Box<[CitedFact]>)) -> &'a Violation,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.citations.iter().map(citation_ref)
    }
}

/// Which computation crossed its representation — [`Error::Overflow`]'s
/// payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowKind {
    /// An aggregate's final value exceeds its result type (the once-at-
    /// finalization range check; deterministic under any fold order).
    /// Carries the find-position index.
    Aggregate { find: FindIndex },
    /// The executor's D2 origin counter would cross u32 — more than 2³²
    /// absorb-node survivors in one execution. Beyond any validated
    /// workload — and survivors are per-execution and can exceed live
    /// rows via joins, so at map-ceiling-scale stores (~10⁸–10⁹ live
    /// rows) 2³² is merely large, not absurd — which is exactly why this
    /// is a typed error, never a panic; checked at batch granularity
    /// (`exec/run/probe_pass.rs`).
    OriginCapacity,
}

/// An OS I/O failure owned by [`Error`]: kind plus raw errno, never a
/// foreign `std::io::Error` whose clone is lossy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoFailure {
    pub kind: std::io::ErrorKind,
    pub raw_os: Option<i32>,
}

impl IoFailure {
    #[must_use]
    pub fn from_io(err: &std::io::Error) -> Self {
        Self {
            kind: err.kind(),
            raw_os: err.raw_os_error(),
        }
    }

    #[must_use]
    pub fn raw_os_error(&self) -> Option<i32> {
        self.raw_os
    }
}

impl From<&std::io::Error> for IoFailure {
    fn from(err: &std::io::Error) -> Self {
        Self::from_io(err)
    }
}

impl From<std::io::Error> for IoFailure {
    fn from(err: std::io::Error) -> Self {
        Self::from_io(&err)
    }
}

/// Concrete bridge decline. Maps to [`ErrorFamily::Io`] so the C ABI
/// kind table does not grow. Only [`Error::hatch`] mints it; only
/// [`Error::is_hatch`] matches it. Unforgeable from real I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub struct Hatch;

/// An LMDB/heed failure owned by [`Error`]. Encoding/decoding drop the
/// inner boxed payload (it was never clone-faithful); the variant remains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LmdbFailure {
    Io(IoFailure),
    Mdb(heed::MdbError),
    Encoding,
    Decoding,
    EnvAlreadyOpened,
}

impl From<heed::Error> for LmdbFailure {
    fn from(err: heed::Error) -> Self {
        match err {
            heed::Error::Io(io) => Self::Io(IoFailure::from_io(&io)),
            heed::Error::Mdb(mdb) => Self::Mdb(mdb),
            heed::Error::Encoding(_) => Self::Encoding,
            heed::Error::Decoding(_) => Self::Decoding,
            heed::Error::EnvAlreadyOpened => Self::EnvAlreadyOpened,
        }
    }
}

/// The one workspace error type, categorized per
/// `docs/architecture/70-api.md`.
///
/// `source()` chains only where the payload *is* an underlying error
/// (`Io`, `Lmdb`, `CommitSync`, `TransactionPoisoned`); the structured
/// variants (`Corruption`, `Schema`, `Validation`, `FactShape`, …) carry
/// data payloads, not nested errors — a decision, not an omission:
/// chain-walkers see exactly the real causes, and the structured detail
/// renders through `Display`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    // --- Open errors ---
    /// Storage format version mismatch — checked before the fingerprint.
    FormatMismatch {
        mismatch: Mismatch<u32>,
    },
    /// Schema fingerprint mismatch: the compiled schema is not the stored one.
    SchemaMismatch {
        mismatch: Mismatch<SchemaFingerprint>,
    },
    /// `create` refused a directory that already holds an LMDB
    /// environment — a bumbledb one (re-initializing `_meta` over live
    /// data would be silent corruption; open it instead) or anyone
    /// else's (a non-`_meta` environment is not ours to move into).
    /// Open-time: a foreign LMDB environment, or a half-created empty
    /// root that is not a usable store.
    AlreadyInitialized,
    /// A fresh-store constructor (`create`, `from_instance`, `compact`) was asked
    /// to claim a path that already exists — including as an empty
    /// directory. An existing path is a previous claim on the name; the
    /// engine does not guess. Payloads carry the path, not formatted
    /// prose.
    DestinationExists {
        path: PathBuf,
    },
    /// Publication finished (the destination contains a complete store)
    /// but the directory-entry durability witness did not. The
    /// implementation does not rename back or delete. A caller may open
    /// the visible destination or repair the directory sync.
    PublishedButUnsynced {
        path: PathBuf,
        source: IoFailure,
    },
    /// Another live handle — a second process, or a second `Db` in this
    /// one — holds the environment's advisory lock. One writer, many
    /// reader threads, one handle, one process (`00-product.md`).
    EnvironmentLocked,
    Io(IoFailure),
    /// Hidden concrete bridge decline. Reuses [`ErrorFamily::Io`].
    /// Only [`Error::hatch`] mints it; only [`Error::is_hatch`]
    /// matches it. Not a public failure kind and not an ABI family arm.
    #[doc(hidden)]
    Hatch(Hatch),
    Lmdb(LmdbFailure),

    // --- Runtime resource errors ---
    /// Every reader slot holds an open snapshot. The environment opens
    /// with a fixed 1024-slot reader table
    /// (`crate::storage::env::MAX_READERS` — a decision, not a knob), and
    /// `MDB_NOTLS` binds slots to transaction objects, so this names one
    /// snapshot too many — not one thread too many. Named instead of a
    /// raw `Lmdb` passthrough because the remedy is releasing snapshots,
    /// not diagnosing LMDB.
    ReadersFull {
        /// The configured reader-table size.
        max_readers: u32,
    },

    // --- Declaration / validation errors ---
    Schema(SchemaError),
    Validation(ValidationError),
    /// A mis-shaped dynamic fact on the ETL surface (data, not code).
    FactShape(FactShapeError),

    // --- Write errors ---
    /// A fresh sequence reached `u64::MAX`; the generator can issue no
    /// further values for this field.
    FreshExhausted {
        relation: RelationId,
        field: FieldId,
    },
    /// A write operation named a closed relation: its rows are ground
    /// axioms — changing them is a new theory (fingerprint), never a
    /// delta. Checked at every write-surface entry before any encoding
    /// runs (`docs/architecture/10-data-model.md` § closed relations).
    ClosedRelationWrite {
        relation: RelationId,
    },
    /// The commit's durability boundary failed: `mdb_txn_commit` surfaced
    /// a raw OS errno from its write/sync path — on macOS the data-page
    /// `pwrite`s, the `fcntl(F_FULLFSYNC)` data flush, or the `O_DSYNC`
    /// meta write; LMDB reports one errno for the phase and names no
    /// syscall, so the type names the phase exactly and the syscall
    /// class honestly. Parsed once at the boundary
    /// ([`Error::from_commit`]) — never a bare `Lmdb(Io(...))` a caller
    /// can only call flaky. The transient form is retried, bounded and
    /// observable, before this escapes (`docs/architecture/50-storage.md`
    /// § write path, phase 5); nothing persisted — the failed commit
    /// aborted its transaction.
    CommitSync {
        /// Bounded retries consumed before the error escaped.
        retries: u32,
        error: IoFailure,
    },
    /// A write transaction's apply phase recorded a prefix of facts then
    /// failed. The first failing call returns the original error; later
    /// mutation in the same transaction, and a closure that returns `Ok`
    /// after catching, still abort — no prefix reaches LMDB.
    TransactionPoisoned {
        /// The original apply error. Nested, never a formatted string.
        source: Box<Error>,
    },

    // --- Runtime errors ---
    /// A prepared query executed against a snapshot of a different
    /// database than the one that prepared it. A prepared query's plan,
    /// statistics, and view memo all belong to one environment — it
    /// executes only against snapshots of the database that prepared it.
    ForeignPreparedQuery,
    /// A witness of a different database than the one being written
    /// ([`crate::Db::write_from`]) — the same environment-identity
    /// key-probe prepared queries run at every execution entry, on the write
    /// side: another database's generation clock proves nothing about
    /// this one.
    ForeignWitness,
    /// Bind-time: the supplied parameter count does not match the query's.
    ParamCountMismatch {
        mismatch: Mismatch<usize>,
    },
    /// Bind-time: a supplied parameter's structural type does not match
    /// the anchor-inferred one.
    ParamTypeMismatch {
        param: ParamId,
        expected: ValueType,
    },
    /// Bind-time: a scalar value supplied where the query binds this
    /// parameter as a set (`Term::ParamSet`) — supply
    /// [`crate::ParamArg::Set`].
    ParamSetExpected {
        param: ParamId,
    },
    /// Bind-time: a set slice supplied where the query binds this
    /// parameter as a scalar (`Term::Param`) — supply
    /// [`crate::ParamArg::Scalar`].
    ParamScalarExpected {
        param: ParamId,
    },
    /// Bind-time: a set element's structural type does not match the
    /// anchor-inferred element type. `element` indexes the supplied slice.
    ParamElementTypeMismatch {
        param: ParamId,
        element: usize,
        expected: ValueType,
    },
    /// Bind-time: a point-position param (an element-typed param meeting
    /// an interval position — a membership binding or a `PointIn`
    /// operand) bound to its domain ceiling. The point domain is
    /// `MIN ..= MAX−1`; `MAX` is the ray's ∞, never a point
    /// (`docs/architecture/10-data-model.md`, the point-domain law) — the
    /// bind-time sibling of
    /// [`ValidationError::PointLiteralAtCeiling`].
    PointParamAtCeiling {
        param: ParamId,
    },
    /// `Duration` reached a ray: an interval with `end == MAX` denotes
    /// `[s, ∞)`, and a ray has no finite measure — **the engine's one
    /// runtime type error** (`docs/architecture/10-data-model.md`, the
    /// point-domain law). Boundedness is not provable at validation, so
    /// the subtraction path tests `end == MAX` and raises here, carrying
    /// the offending fact's two encoded interval words (order-preserving
    /// column form — I64 endpoints are the sign-flipped biased words).
    /// The alternative — silently yielding `MAX` — would fabricate
    /// arithmetic. Hosts exclude rays first: an `Allen` predicate
    /// (`DISJOINT` from the ray-detecting probe `[MAX−1, MAX)`) or a
    /// bounded-end filter on the measured atom runs before the measure
    /// by the filter-order law (`docs/architecture/20-query-ir.md`,
    /// § the measure).
    MeasureOfRay {
        /// The offending interval's encoded start word.
        start: u64,
        /// The offending interval's encoded end word (`u64::MAX` — the
        /// ray's ∞ in both element encodings).
        end: u64,
    },
    /// A capacity statement's Duration weight or dependent Duration bound
    /// reached a ray at judge time: an interval with `end == MAX` denotes
    /// `[s, ∞)`, and a ray has no finite measure — the typed COMMIT
    /// refusal naming the row (ruled 2026-07-24, C10; the
    /// [`Error::MeasureOfRay`] precedent enforced at the law site).
    /// Boundedness is not provable at validation, so the measure path
    /// tests `end == MAX` and refuses the commit whole — never a
    /// violation (the law is not judged false; its measure is undefined)
    /// and never a silent `MAX` (fabricated arithmetic).
    CapacityRayMeasure {
        /// The capacity statement whose measure met the ray.
        statement: StatementId,
        /// The offending row — the weighed SOURCE fact or the
        /// bound-carrying TARGET fact, canonical bytes.
        fact: Box<[u8]>,
    },
    /// A derived-tuples budget crossed — interiors and rec share one
    /// ledger (`docs/architecture/40-execution.md` § the reach driver).
    /// Termination of the rec is a theorem of the validation roster
    /// (`lean/Bumbledb/Exec/Reach.lean: reach_den_finite`), but derived
    /// *size* is data-shaped: a foreign query may legally demand a
    /// quadratic closure, and an unbounded table crossing the trust
    /// boundary is what the recorded v0 OS-backstop argument never
    /// priced. On `MeasureOfRay`'s model: aborts the query, the snapshot
    /// stays usable, the payload is counts — never strings. The
    /// documented default
    /// ([`crate::api::prepared::reach::DEFAULT_REACH_ROUNDS`] /
    /// [`crate::api::prepared::reach::DEFAULT_DERIVED_TUPLES`]) is the
    /// decision, not a knob — the boundary is never unguarded. `rounds`
    /// is rec rounds so far (`0` on an interiors-only / preamble trip).
    DerivedBudgetExceeded {
        /// Rec rounds run when the budget tripped (`0` if no rec ran).
        rounds: u32,
        /// Distinct derived tuples (interior emits plus rec table).
        tuples: u64,
    },
    /// A computed value crossed its representation — valid input whose
    /// result cannot be represented, so a typed error, never a panic.
    /// The payload names which computation.
    Overflow(OverflowKind),
    /// The result buffer's byte heap crossed the u32 offset space —
    /// more than 4 GiB of distinct string/bytes payload in one result.
    /// (This 4 GiB is the u32 byte-heap offset ceiling, a representation
    /// limit — NOT the map size; do not sweep it with the map constant.)
    /// Beyond any validated workload, but it is valid input, so it
    /// errors rather than panics.
    ResultBytesOverflow,
    /// Hard corruption error, never a skip.
    Corruption(CorruptionError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Per-variant taxonomy: the one exhaustive table [`Error::descriptor`]
/// walks. `source()`, the C kind map, and any future Clone-like fold
/// read this — adding a variant is one arm here plus its `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorFamily {
    FormatMismatch,
    SchemaMismatch,
    AlreadyInitialized,
    DestinationExists,
    PublishedButUnsynced,
    EnvironmentLocked,
    Io,
    Lmdb,
    ReadersFull,
    Schema,
    Validation,
    FactShape,
    FreshExhausted,
    ClosedRelationWrite,
    CommitSync,
    TransactionPoisoned,
    ForeignPreparedQuery,
    ForeignWitness,
    Param,
    MeasureOfRay,
    CapacityRayMeasure,
    DerivedBudgetExceeded,
    Overflow,
    ResultBytesOverflow,
    Corruption,
}

/// Shared per-variant facts produced by the one exhaustive [`Error`] match.
pub(crate) struct ErrorDescriptor<'a> {
    pub family: ErrorFamily,
    pub source: Option<&'a (dyn std::error::Error + 'static)>,
}

fn family_only<'a>(family: ErrorFamily) -> ErrorDescriptor<'a> {
    ErrorDescriptor {
        family,
        source: None,
    }
}

fn family_source<'a>(
    family: ErrorFamily,
    source: &'a (dyn std::error::Error + 'static),
) -> ErrorDescriptor<'a> {
    ErrorDescriptor {
        family,
        source: Some(source),
    }
}

impl Error {
    /// Mint the bridge decline. The engine never constructs one on any
    /// engine path.
    #[doc(hidden)]
    #[must_use]
    pub fn hatch() -> Self {
        Self::Hatch(Hatch)
    }

    /// Match the bridge decline. `false` for every engine `Error`,
    /// including a genuine [`Self::Io`].
    #[doc(hidden)]
    #[must_use]
    pub fn is_hatch(&self) -> bool {
        matches!(self, Self::Hatch(_))
    }

    /// The one per-variant descriptor. Display still formats payloads;
    /// every other taxonomy fold reads this.
    #[must_use]
    pub(crate) fn descriptor(&self) -> ErrorDescriptor<'_> {
        match self {
            Self::FormatMismatch { .. } => family_only(ErrorFamily::FormatMismatch),
            Self::SchemaMismatch { .. } => family_only(ErrorFamily::SchemaMismatch),
            Self::AlreadyInitialized => family_only(ErrorFamily::AlreadyInitialized),
            Self::DestinationExists { .. } => family_only(ErrorFamily::DestinationExists),
            Self::PublishedButUnsynced { source, .. } => {
                family_source(ErrorFamily::PublishedButUnsynced, source)
            }
            Self::EnvironmentLocked => family_only(ErrorFamily::EnvironmentLocked),
            Self::Io(err) => family_source(ErrorFamily::Io, err),
            Self::Hatch(_) => family_only(ErrorFamily::Io),
            Self::Lmdb(err) => family_source(ErrorFamily::Lmdb, err),
            Self::ReadersFull { .. } => family_only(ErrorFamily::ReadersFull),
            Self::Schema(_) => family_only(ErrorFamily::Schema),
            Self::Validation(_) => family_only(ErrorFamily::Validation),
            Self::FactShape(_) => family_only(ErrorFamily::FactShape),
            Self::FreshExhausted { .. } => family_only(ErrorFamily::FreshExhausted),
            Self::ClosedRelationWrite { .. } => family_only(ErrorFamily::ClosedRelationWrite),
            Self::CommitSync { error, .. } => family_source(ErrorFamily::CommitSync, error),
            Self::TransactionPoisoned { source } => {
                family_source(ErrorFamily::TransactionPoisoned, source.as_ref())
            }
            Self::ForeignPreparedQuery => family_only(ErrorFamily::ForeignPreparedQuery),
            Self::ForeignWitness => family_only(ErrorFamily::ForeignWitness),
            Self::ParamCountMismatch { .. }
            | Self::ParamTypeMismatch { .. }
            | Self::ParamSetExpected { .. }
            | Self::ParamScalarExpected { .. }
            | Self::ParamElementTypeMismatch { .. }
            | Self::PointParamAtCeiling { .. } => family_only(ErrorFamily::Param),
            Self::MeasureOfRay { .. } => family_only(ErrorFamily::MeasureOfRay),
            Self::CapacityRayMeasure { .. } => family_only(ErrorFamily::CapacityRayMeasure),
            Self::DerivedBudgetExceeded { .. } => family_only(ErrorFamily::DerivedBudgetExceeded),
            Self::Overflow(_) => family_only(ErrorFamily::Overflow),
            Self::ResultBytesOverflow => family_only(ErrorFamily::ResultBytesOverflow),
            Self::Corruption(_) => family_only(ErrorFamily::Corruption),
        }
    }

    /// The C/TS kind tag — derived from [`Error::descriptor`].
    #[must_use]
    pub fn family(&self) -> ErrorFamily {
        self.descriptor().family
    }
}

#[cfg(test)]
mod hatch_tests {
    use super::*;

    #[test]
    fn hatch_reuses_io_family_and_downcasts() {
        let error = Error::hatch();
        assert_eq!(error.family(), ErrorFamily::Io);
        assert!(error.is_hatch());
        let interrupted = Error::from(std::io::Error::from(std::io::ErrorKind::Interrupted));
        assert!(!interrupted.is_hatch());
        assert_eq!(interrupted.family(), ErrorFamily::Io);
        assert_ne!(interrupted, error);
    }
}

/// Zero-dyn engine census (audit/27). `dyn` in production engine src is
/// legal only on `std::error::Error::source` and the `ErrorDescriptor`
/// mirror that feeds it. Test modules (`tests.rs`, `tests/`) follow the
/// `FilterPredicate` gate's skip — they are not the law's surface.
#[cfg(test)]
mod zero_dyn_census {
    use std::fs;
    use std::path::{Path, PathBuf};

    const ENGINE_SRC: &[&str] = &[
        "crates/bumbledb/src",
        "crates/bumbledb-theory/src",
        "crates/bumbledb-query/src",
        "crates/bumbledb-macros/src",
    ];

    /// Pinned exemption: the `Error::source` signature plus the two
    /// `ErrorDescriptor` lines that carry the same `dyn Error` the
    /// impl returns. A new `dyn` fails with <file:line>.
    const EXEMPT: &[(&str, &str)] = &[
        (
            "crates/bumbledb/src/error.rs",
            "pub source: Option<&'a (dyn std::error::Error + 'static)>,",
        ),
        (
            "crates/bumbledb/src/error.rs",
            "source: &'a (dyn std::error::Error + 'static),",
        ),
        (
            "crates/bumbledb/src/error/convert.rs",
            "fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {",
        ),
    ];

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) == Some("tests") {
                    continue;
                }
                rust_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
                && path.file_name().and_then(|s| s.to_str()) != Some("tests.rs")
            {
                out.push(path);
            }
        }
    }

    fn strip_strings(line: &str) -> String {
        let mut out = String::with_capacity(line.len());
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                out.push(' ');
                while let Some(d) = chars.next() {
                    if d == '\\' {
                        chars.next();
                        continue;
                    }
                    if d == '"' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn code_span(line: &str) -> String {
        let stripped = strip_strings(line);
        let trim = stripped.trim_start();
        if trim.starts_with("//") {
            return String::new();
        }
        match stripped.find("//") {
            Some(at) => stripped[..at].trim().to_owned(),
            None => stripped.trim().to_owned(),
        }
    }

    fn has_type_dyn(code: &str) -> bool {
        let bytes = code.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i..].starts_with(b"dyn") {
                let before = i == 0 || {
                    let b = bytes[i - 1];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                let after = i + 3;
                let after_ok = after < bytes.len() && bytes[after].is_ascii_whitespace();
                if before && after_ok {
                    return true;
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        false
    }

    fn exempt(rel: &str, line: &str) -> bool {
        let trimmed = line.trim();
        EXEMPT
            .iter()
            .any(|(path, snippet)| *path == rel && trimmed == *snippet)
    }

    #[test]
    fn zero_dyn_engine_pins_error_source_exemption() {
        let root = workspace_root();
        let mut files = Vec::new();
        for rel in ENGINE_SRC {
            rust_files(&root.join(rel), &mut files);
        }
        files.sort();
        let mut unexpected = Vec::new();
        let mut exempt_hits = 0usize;
        for path in &files {
            let rel = path
                .strip_prefix(&root)
                .expect("under workspace")
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(path).expect("read");
            for (idx, line) in text.lines().enumerate() {
                let code = code_span(line);
                if !has_type_dyn(&code) {
                    continue;
                }
                if exempt(&rel, line) {
                    exempt_hits += 1;
                    continue;
                }
                unexpected.push(format!("{rel}:{}: {line}", idx + 1));
            }
        }
        assert_eq!(
            exempt_hits,
            EXEMPT.len(),
            "exemption list drifted — expected {} Error::source lines, found {exempt_hits}",
            EXEMPT.len()
        );
        assert!(
            unexpected.is_empty(),
            "engine dyn outside the Error::source exemption:\n{}",
            unexpected.join("\n")
        );
    }
}
