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
use crate::schema::{FieldId, RelationId, StatementId, ValueType};

/// Corruption detected while decoding stored bytes — a hard error, never a
/// skip, never a default (`docs/architecture/40-storage.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorruptionError {
    /// A Bool byte other than `0x00`/`0x01` — there is no distinct "true".
    InvalidBool(u8),
    /// An Enum ordinal at or beyond the declared variant count.
    EnumOrdinalOutOfRange { ordinal: u8, variant_count: u16 },
    /// Interval bytes whose `start >= end` — the empty interval is
    /// unrepresentable (a fact never denotes nothing), so a stored one is
    /// corruption, not a value. Carries the raw 16 bytes.
    InvalidInterval([u8; 16]),
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
///
/// Statement variants implement the validation roster of
/// `docs/architecture/30-dependencies.md` — one variant per roster line,
/// no catch-all. Each doc comment cites its line. The roster's "FD with
/// selection" and "non-key FD form" lines have no variants: PRD 02's
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

    // --- Statement roster (30-dependencies § validation roster) ---
    /// Roster "unknown relation … ids": a statement names a relation
    /// outside the schema.
    StatementUnknownRelation {
        statement: StatementId,
        relation: RelationId,
    },
    /// Roster "unknown … field ids": a projection or selection names a
    /// field outside its relation.
    StatementUnknownField {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "empty … projections": a projection with no fields.
    EmptyProjection {
        statement: StatementId,
        relation: RelationId,
    },
    /// Roster "duplicate-carrying projections": a field twice in one
    /// projection.
    DuplicateProjectionField {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "duplicate-carrying projections", the selection sibling: a
    /// field bound twice in one selection σ (a set of bindings).
    DuplicateSelectionField {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster ">1 interval position": two interval fields in one FD
    /// projection would be 2-D exclusion, which the ordered guard cannot
    /// answer. Carries the second interval field.
    FunctionalityMultipleIntervals {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "interval not in final position": the neighbor probe needs
    /// the scalar prefix as its group.
    FunctionalityIntervalNotLast {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "duplicate statements", the Functionality-specific form: two
    /// FDs over one field set on one relation assert one judgment — the
    /// second guard is pure write amplification, and rejecting it makes
    /// containment target-key resolution unambiguous.
    DuplicateFunctionality {
        statement: StatementId,
        earlier: StatementId,
    },
    /// Roster "guard width overflow": Σ projected field widths exceeds
    /// [`crate::storage::keys::MAX_GUARD_WIDTH`] — rejected at declaration,
    /// never discovered at write time.
    GuardKeyTooWide {
        statement: StatementId,
        width: usize,
    },
    /// Roster "arity mismatch between sides": |X| ≠ |Y|.
    ContainmentArityMismatch {
        statement: StatementId,
        source: usize,
        target: usize,
    },
    /// Roster "positional structural-type mismatch" — including its
    /// called-out instance, an interval position against a scalar one.
    ContainmentTypeMismatch {
        statement: StatementId,
        position: usize,
    },
    /// Roster "a selected field also projected": a constant column — write
    /// the statement you mean.
    SelectedFieldProjected {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "selection literal type mismatch": the literal's variant is
    /// not the field's structural type.
    SelectionLiteralTypeMismatch {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "selection literal type mismatch (including out-of-range
    /// enum ordinals …)".
    SelectionEnumOrdinalOutOfRange {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
        ordinal: u8,
    },
    /// Roster "selection literal type mismatch (… non-UTF-8 string
    /// literals)".
    SelectionLiteralNotUtf8 {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "selection literal type mismatch", the interval bound rule:
    /// `start >= end` denotes no points, and a fact never denotes nothing.
    SelectionIntervalEmpty {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "IND whose target projection matches no key of the target":
    /// probe-ability requires Y to be a permutation of a declared key.
    NoMatchingTargetKey {
        statement: StatementId,
        relation: RelationId,
    },
    /// Roster "IND … (or, with an interval position, no pointwise key
    /// carrying it)": the coverage walk needs the target's own key to keep
    /// its intervals disjoint and ordered.
    NoPointwiseTargetKey {
        statement: StatementId,
        relation: RelationId,
    },
    /// Roster "duplicate statements (identical normalized sides and form —
    /// write it once)": selections compare sorted by field id.
    DuplicateStatement {
        statement: StatementId,
        earlier: StatementId,
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
    /// [`crate::WriteTx::get_dyn`]'s statement id is not a `Functionality`
    /// on the queried relation (out of range, a containment, or another
    /// relation's key) — the dynamic point-read surface is data, so the
    /// mismatch is a typed error, never an index panic.
    NotAKeyStatement {
        relation: RelationId,
        statement: StatementId,
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

/// Which side of a containment statement the commit-time judgment found
/// unsatisfied (`docs/architecture/30-dependencies.md` § enforcement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// An inserted source fact inside σ has no target: the guard probe
    /// missed, or the coverage walk found a gap.
    SourceUnsatisfied,
    /// A deleted target key tuple is still required by a surviving
    /// source fact (the reverse-edge scan).
    TargetRequired,
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
    /// A `Functionality` statement violated by the committed final state:
    /// two live facts claim one key — the same guard bytes (scalar
    /// put-conflict), or overlapping intervals within one scalar-prefix
    /// group (the pointwise neighbor probe). Payloads are canonical fact
    /// bytes, never row ids (`docs/architecture/10-data-model.md`).
    FunctionalityViolation {
        statement: StatementId,
        /// The fact whose insert violated the statement.
        fact: Box<[u8]>,
        /// The already-standing fact, for the pointwise arm — the probe
        /// names both parties. `None` for a scalar put-conflict, where
        /// the guard bytes inside `fact` already identify the collision.
        incumbent: Option<Box<[u8]>>,
    },
    /// A `Containment` statement violated by the committed final state
    /// (`docs/architecture/30-dependencies.md` § judged on final states).
    /// `fact` is canonical source-fact bytes on either side: the judgment
    /// speaks about sources — a missing target is named by the source
    /// that requires it.
    ContainmentViolation {
        statement: StatementId,
        side: Direction,
        /// The source fact: the inserted fact whose target is missing
        /// (`SourceUnsatisfied`), or the surviving fact still requiring a
        /// deleted target key (`TargetRequired`).
        fact: Box<[u8]>,
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
