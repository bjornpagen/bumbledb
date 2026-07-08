//! The workspace error taxonomy (the 40-storage doc; categories per
//! `docs/architecture/60-api.md`).
//!
//! Everything reachable from user input or disk returns these typed errors;
//! panics are reserved for programmer-invariant violations. Payloads carry
//! ids and owned fact bytes, never formatted strings — no `format!` runs on
//! a hot path; `Display` formats lazily when the host actually prints.

mod convert;
mod display;

use crate::ir::{ParamId, VarId};
use crate::schema::fingerprint::SchemaFingerprint;
use crate::schema::{ConstraintId, FieldId, RelationId, ValueType};

/// Corruption detected while decoding stored bytes — a hard error, never a
/// skip, never a default (`docs/architecture/40-storage.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorruptionError {
    /// A Bool byte other than `0x00`/`0x01` — there is no distinct "true".
    InvalidBool(u8),
    /// An Enum ordinal at or beyond the declared variant count.
    EnumOrdinalOutOfRange { ordinal: u8, variant_count: u16 },
    /// The `_meta` database or one of its required keys is absent or
    /// malformed: the environment is not a usable bumbledb database.
    MetaMissing,
    /// An intern id with no reverse dictionary entry — a fact referencing it
    /// is corrupt.
    DanglingInternId(u64),
    /// A row id obtained from `M`/`U` has no `F` entry in the same snapshot.
    MissingFact { relation: RelationId, row_id: u64 },
    /// A live `M` entry's `F` row or `U` guard was absent at delete time —
    /// the write-side M/F disagreement (the read side raises
    /// [`CorruptionError::MissingFact`]).
    MembershipDesync { relation: RelationId, row_id: u64 },
    /// A stored fact's length differs from the schema's fact width.
    WrongFactWidth {
        relation: RelationId,
        row_id: u64,
        expected: usize,
        actual: usize,
    },
    /// The `F` scan yielded a different number of rows than the stored `S`
    /// row count — the derived counters have desynced from the facts.
    RowCountMismatch { relation: RelationId, stored: u64 },
    /// A stored value (a counter, row id, or dictionary id) failed to
    /// decode; the static string names which kind — a diagnosis, not a
    /// formatted payload.
    MalformedValue(&'static str),
    /// A stored string's bytes are not UTF-8 — distinct from a dangling id
    /// (the reverse entry exists; its content is mojibake).
    NonUtf8Intern(u64),
    /// The reverse dictionary entry's tag byte disagrees with the field
    /// type that referenced it (a String field carrying a Bytes id, or
    /// vice versa).
    InternTagMismatch(u64),
}

/// A schema declaration error (the 10-data-model doc's validation boundary). Every illegal
/// schema shape has a distinct variant; an invalid schema is
/// unconstructible, not flagged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    DuplicateRelationName {
        name: Box<str>,
    },
    DuplicateFieldName {
        relation: RelationId,
        name: Box<str>,
    },
    DuplicateConstraintName {
        relation: RelationId,
        name: Box<str>,
    },
    EnumWithoutVariants {
        relation: RelationId,
        field: FieldId,
    },
    EnumTooManyVariants {
        relation: RelationId,
        field: FieldId,
        count: usize,
    },
    DuplicateEnumVariant {
        relation: RelationId,
        field: FieldId,
        variant: Box<str>,
    },
    SerialOnNonU64 {
        relation: RelationId,
        field: FieldId,
    },
    UnknownConstraintField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    UniqueWithoutFields {
        relation: RelationId,
        constraint: ConstraintId,
    },
    /// A field listed twice in one constraint's field list — unique or
    /// foreign-key alike.
    ConstraintDuplicateField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    /// Two unique constraints over the identical ordered field list —
    /// double guard maintenance with no added meaning.
    DuplicateConstraintFields {
        relation: RelationId,
        constraint: ConstraintId,
    },
    /// The constraint's guard key would exceed LMDB's key ceiling
    /// (`storage::keys::MAX_GUARD_WIDTH`) once embedded in a Restrict key.
    GuardKeyTooWide {
        relation: RelationId,
        constraint: ConstraintId,
        width: usize,
    },
    UnknownFkTargetRelation {
        relation: RelationId,
        constraint: ConstraintId,
        target: RelationId,
    },
    UnknownFkTargetConstraint {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkTargetNotUnique {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkArityMismatch {
        relation: RelationId,
        constraint: ConstraintId,
    },
    FkFieldTypeMismatch {
        relation: RelationId,
        constraint: ConstraintId,
        position: usize,
    },
}

/// How a foreign-key constraint failed at commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FkViolation {
    /// An inserted fact references a target unique key that does not exist
    /// in the final state (forward check).
    MissingTarget { fact_bytes: Box<[u8]> },
    /// A deleted unique key still has a live referrer in the final state
    /// (Restrict check); `relation`/`constraint` on the error name the
    /// *target* side, this carries the referencing fact itself (storage row
    /// ids never surface — `docs/architecture/10-data-model.md`).
    RemainingReference {
        source_relation: RelationId,
        fact_bytes: Box<[u8]>,
    },
}

/// A mis-shaped dynamic fact on the untyped write surface
/// (`insert_dyn`/`delete_dyn`/`bulk_load`): ETL input is data, so shape
/// problems are typed errors, not panics (`docs/architecture/60-api.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactShapeError {
    /// The relation id is outside the schema — ETL input is data, so an
    /// out-of-range id at the dynamic surface (`insert_dyn`/`delete_dyn`/
    /// `bulk_load`/`scan`) is a typed error, never an index panic.
    UnknownRelation { relation: RelationId },
    ArityMismatch {
        relation: RelationId,
        expected: usize,
        supplied: usize,
    },
    TypeMismatch {
        relation: RelationId,
        field: FieldId,
    },
    EnumOrdinalOutOfRange {
        relation: RelationId,
        field: FieldId,
        ordinal: u8,
    },
    /// `Value::String` bytes are not UTF-8.
    InvalidUtf8 {
        relation: RelationId,
        field: FieldId,
    },
}

/// A query validation error (the IR boundary, the 20-query-ir doc): one variant per
/// roster item in `docs/architecture/20-query-ir.md`, returned at prepare
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    UnknownRelation {
        atom: usize,
        relation: RelationId,
    },
    UnknownField {
        atom: usize,
        field: FieldId,
    },
    DuplicateFieldBinding {
        atom: usize,
        field: FieldId,
    },
    VariableTypeConflict {
        var: VarId,
    },
    LiteralTypeMismatch {
        atom: usize,
        field: FieldId,
    },
    EnumOrdinalOutOfRange {
        atom: usize,
        field: FieldId,
        ordinal: u8,
    },
    /// Param ids must be dense (0..n): a gap would be a positional slot
    /// whose supplied value is never type-checked.
    ParamIdGap {
        param: ParamId,
    },
    ParamTypeConflict {
        param: ParamId,
    },
    /// Type rules violated: cross-type comparison, or an order operator on
    /// a non-integer type.
    IllegalComparison {
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
    /// An enum literal in a comparison carries an ordinal beyond the
    /// variable's variant list.
    ComparisonEnumOrdinalOutOfRange {
        index: usize,
        ordinal: u8,
    },
    /// Datalog safety: a find (or aggregate-input) variable bound by no atom.
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
    NoAtoms,
    /// Sum/Min/Max over a non-integer variable.
    AggregateInputType {
        find: usize,
    },
    /// Count is nullary; it carries no variable.
    CountWithVariable {
        find: usize,
    },
    /// Sum/Min/Max require a variable.
    AggregateWithoutVariable {
        find: usize,
    },
    AggregateOverGroupKey {
        find: usize,
    },
    /// Planner cap: the exhaustive left-deep DP accepts at most
    /// `plan::planner::MAX_OCCURRENCES` atom occurrences.
    TooManyAtoms {
        count: usize,
    },
    /// Planner cap: at most 128 distinct variables (dense bitset width).
    TooManyVariables {
        count: usize,
    },
}

/// The one workspace error type, categorized per
/// `docs/architecture/60-api.md`.
#[derive(Debug)]
pub enum Error {
    // --- Open errors ---
    /// Storage format version mismatch — checked before the fingerprint.
    FormatMismatch {
        found: u32,
        expected: u32,
    },
    /// Schema fingerprint mismatch: the compiled schema is not the stored one.
    SchemaMismatch {
        found: SchemaFingerprint,
        expected: SchemaFingerprint,
    },
    /// `create` refused a directory that already holds an LMDB
    /// environment — a bumbledb one (re-initializing `_meta` over live
    /// data would be silent corruption; open it instead) or anyone
    /// else's (a non-`_meta` environment is not ours to move into).
    AlreadyInitialized,
    /// Another live handle — a second process, or a second `Db` in this
    /// one — holds the environment's advisory lock. One writer, many
    /// reader threads, one handle, one process (`00-product.md`).
    EnvironmentLocked,
    Io(std::io::Error),
    Lmdb(heed::Error),

    // --- Declaration / validation errors ---
    Schema(SchemaError),
    Validation(ValidationError),
    /// A mis-shaped dynamic fact on the ETL surface (data, not code).
    FactShape(FactShapeError),

    // --- Write errors ---
    /// A foreign-key invariant would be violated by the committed state:
    /// the whole transaction aborts.
    ForeignKeyViolation {
        relation: RelationId,
        constraint: ConstraintId,
        violation: FkViolation,
    },
    /// Two live facts claimed one unique key: the commit-time invariant is
    /// violated and the whole transaction aborts.
    UniqueViolation {
        relation: RelationId,
        constraint: ConstraintId,
        fact_bytes: Box<[u8]>,
    },
    /// A serial sequence reached `u64::MAX`; the generator can issue no
    /// further values for this field.
    SerialExhausted {
        relation: RelationId,
        field: FieldId,
    },

    // --- Runtime errors ---
    /// A prepared query executed against a snapshot of a different
    /// database than the one that prepared it. A prepared query's plan,
    /// statistics, and view memo all belong to one environment — it
    /// executes only against snapshots of the database that prepared it.
    ForeignPreparedQuery,
    /// Bind-time: the supplied parameter count does not match the query's.
    ParamCountMismatch {
        expected: usize,
        supplied: usize,
    },
    /// Bind-time: a supplied parameter's structural type does not match
    /// the anchor-inferred one.
    ParamTypeMismatch {
        param: ParamId,
        expected: ValueType,
    },
    /// An aggregate's final value exceeds its result type (the once-at-
    /// finalization range check; deterministic under any fold order).
    Overflow {
        find: usize,
    },
    /// The result buffer's byte heap crossed the u32 offset space —
    /// more than 4 GiB of distinct string/bytes payload in one result.
    /// Absurd under the scale axiom, but it is valid input, so it
    /// errors rather than panics.
    ResultBytesOverflow,
    /// Hard corruption error, never a skip.
    Corruption(CorruptionError),
}

pub type Result<T> = std::result::Result<T, Error>;
