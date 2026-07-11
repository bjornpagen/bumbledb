//! The workspace error taxonomy, categorized per
//! `docs/architecture/70-api.md`.
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
/// skip, never a default (`docs/architecture/50-storage.md`).
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
        expected: usize,
        actual: usize,
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
        /// The stored `S` value.
        claimed: u64,
        /// The `_data` entry count that bounds it.
        witness: u64,
    },
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

/// A schema declaration error (the validation boundary,
/// `docs/architecture/10-data-model.md`). Every illegal schema shape has a
/// distinct variant; an invalid schema is unconstructible, not flagged.
///
/// Statement variants implement the validation roster of
/// `docs/architecture/30-dependencies.md` — one variant per roster line,
/// no catch-all. Each doc comment cites its line. The roster's "FD with
/// selection" and "non-key FD form" lines have no variants:
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
    FreshOnNonU64 {
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
/// problems are typed errors, not panics (`docs/architecture/70-api.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactShapeError {
    /// The relation id is outside the schema — ETL input is data, so an
    /// out-of-range id at the dynamic surface (`insert_dyn`/`delete_dyn`/
    /// `bulk_load`/`scan`/`fresh_field`) is a typed error, never an index
    /// panic.
    UnknownRelation { relation: RelationId },
    /// The field id is outside its relation — the field sibling of
    /// [`FactShapeError::UnknownRelation`], raised by the id-addressed
    /// dynamic surface ([`crate::Schema::fresh_field`]).
    UnknownField {
        relation: RelationId,
        field: FieldId,
    },
    /// [`crate::Schema::fresh_field`]'s field is not `Fresh` generation —
    /// no witness exists, so no mint can be asked for (parse, don't
    /// validate: the check runs once at resolution, never per mint).
    NotAFreshField {
        relation: RelationId,
        field: FieldId,
    },
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
    /// An interval value with `start >= end`: the empty interval denotes
    /// no points, and a fact never denotes nothing
    /// (`docs/architecture/10-data-model.md`).
    EmptyInterval {
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
    /// DNF distribution of the rules' predicate trees would produce more
    /// rules than the cap ([`crate::ir::MAX_RULES`]) — the exponential
    /// case is rejected at declaration, exactly like guard-width
    /// overflow. `produced` names the blowup: the structural term count
    /// across all rules, judged before a single disjunct is materialized
    /// (so before duplicate collapse).
    DnfExceedsRules {
        produced: usize,
        cap: usize,
    },
    /// A rule's find-term count differs from the head's arity — rules
    /// align against the head position by position.
    HeadArityMismatch {
        rule: usize,
        expected: usize,
        found: usize,
    },
    /// A rule's find term at `position` resolves to a different
    /// structural type than the head's pinned positional type row (rule
    /// 0's row pins it; every later rule must agree).
    HeadTypeMismatch {
        rule: usize,
        position: usize,
    },
    /// A rule's find term at `position` has the wrong *shape* against the
    /// head: a variable where the head names an aggregate, an aggregate
    /// where it names a variable, or a different aggregate-op kind.
    HeadAggregateMismatch {
        rule: usize,
        position: usize,
    },
    /// Arg-restriction (`ArgMax`/`ArgMin`) in a multi-rule program
    /// (DNF-lowered rules included): the restriction key is a rule-scoped
    /// variable outside the head's vocabulary — rules need not even
    /// agree on its type — so "the extreme over the union" is undefined
    /// and refused at the boundary. The modeling answer is one Arg query
    /// per disjunct, host-merged; the trigger for defining a cross-rule
    /// restriction is a real query
    /// (`docs/architecture/20-query-ir.md` § aggregation).
    ArgAcrossRules {
        rules: usize,
    },
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
    /// An interval literal with `start >= end` in a binding position — it
    /// denotes no points, and no field value equals or contains nothing
    /// (comparison sites report
    /// [`ValidationError::ComparisonEmptyIntervalLiteral`]).
    EmptyIntervalLiteral {
        atom: usize,
        field: FieldId,
    },
    /// The point-domain law (`docs/architecture/10-data-model.md`): points
    /// are `MIN ..= MAX−1`; `end == MAX` denotes the ray `[s, ∞)`, so an
    /// element-typed literal equal to the domain ceiling in a membership
    /// binding can never be inside any interval — rejected typed, never
    /// silently unmatchable (comparison sites report
    /// [`ValidationError::ComparisonPointLiteralAtCeiling`]).
    PointLiteralAtCeiling {
        atom: usize,
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
    /// Type rules violated: cross-type comparison, an order operator on a
    /// non-integer type, or an interval operator over non-interval sides.
    IllegalComparison {
        index: usize,
    },
    /// An order operator (`Lt`/`Le`/`Gt`/`Ge`) with an interval operand —
    /// intervals are unordered; the predictable mistake gets the dedicated
    /// diagnostic (`docs/architecture/20-query-ir.md` § comparison rules).
    OrderComparisonOnInterval {
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
    /// An interval literal with `start >= end` in a comparison position
    /// (the binding-site sibling of
    /// [`ValidationError::EmptyIntervalLiteral`]).
    ComparisonEmptyIntervalLiteral {
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
    /// relation can hold, so the predicate is "never": write no query
    /// (`docs/architecture/20-query-ir.md` § the Allen operator; the
    /// bind-time sibling is [`Error::EmptyAllenMaskParam`]).
    EmptyAllenMask {
        index: usize,
    },
    /// An `Allen` comparison whose literal mask is all 13 basics — every
    /// pair satisfies it, so the predicate is "always": write no
    /// predicate (the bind-time sibling is [`Error::FullAllenMaskParam`]).
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
    /// Sum/Min/Max over a non-integer variable (`CountDistinct` is legal
    /// over every type — equality is all it needs).
    AggregateInputType {
        find: usize,
    },
    /// Count is nullary; it carries no variable.
    CountWithVariable {
        find: usize,
    },
    /// Sum/Min/Max/CountDistinct require a variable, and Arg terms a
    /// carried variable.
    AggregateWithoutVariable {
        find: usize,
    },
    AggregateOverGroupKey {
        find: usize,
    },
    /// Arg terms and fold aggregates (Sum/Min/Max/Count/CountDistinct)
    /// may not mix in one query — "sum of the latest" is two queries.
    MixedArgAndFold {
        find: usize,
    },
    /// All Arg terms in one query share one key variable and one
    /// direction; this find disagrees with an earlier Arg term.
    ArgKeyMismatch {
        find: usize,
    },
    /// An Arg key must be orderable: U64 or I64.
    NonOrderableArgKey {
        find: usize,
    },
    /// A `Term::Duration` in an atom binding: the measure is a
    /// computation over a bound interval variable, not a bindable value
    /// — its legal positions are a find term, the aggregated input of
    /// `Sum`/`Min`/`Max`, and one side of an order comparison
    /// (`docs/architecture/20-query-ir.md`, § the measure).
    DurationInBinding {
        atom: usize,
        field: FieldId,
    },
    /// `Duration(v)` over a variable that did not resolve to an interval
    /// type: the measure is defined by the interval denotation and by
    /// nothing else.
    DurationOverNonInterval {
        var: VarId,
    },
    /// A `FindTerm::AggregateDuration` whose op is not `Sum`/`Min`/`Max`
    /// — `Count` is nullary, `CountDistinct` over a measure is a count
    /// over derived values with no sighted use, and the Arg ops key on
    /// variables, not computations.
    DurationAggregateOp {
        find: usize,
    },
    /// A `Term::Duration` under any operator but the order comparisons
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

/// Which computation crossed its representation — [`Error::Overflow`]'s
/// payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowKind {
    /// An aggregate's final value exceeds its result type (the once-at-
    /// finalization range check; deterministic under any fold order).
    /// Carries the find-clause index.
    Aggregate { find: usize },
    /// The executor's D2 origin counter would cross u32 — more than 2³²
    /// absorb-node survivors in one execution. Beyond the scale axiom,
    /// but valid input, so it errors; checked at batch granularity
    /// (`exec/run/probe_pass.rs`).
    Origins,
}

/// The one workspace error type, categorized per
/// `docs/architecture/70-api.md`.
///
/// `source()` chains only where the payload *is* an underlying error
/// (`Io`, `Lmdb`, `CommitSync`, `BulkLoad`); the structured variants (`Corruption`,
/// `Schema`, `Validation`, `FactShape`, …) carry data payloads, not
/// nested errors — a decision, not an omission: chain-walkers see
/// exactly the real causes, and the structured detail renders through
/// `Display`.
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
        direction: Direction,
        /// The source fact: the inserted fact whose target is missing
        /// (`SourceUnsatisfied`), or the surviving fact still requiring a
        /// deleted target key (`TargetRequired`).
        fact: Box<[u8]>,
    },
    /// A fresh sequence reached `u64::MAX`; the generator can issue no
    /// further values for this field.
    FreshExhausted {
        relation: RelationId,
        field: FieldId,
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
        error: std::io::Error,
    },
    /// A bulk load failed mid-stream: the underlying error plus how many
    /// facts were already durable in the chunks committed before it —
    /// the resumability payload, carried through the `?` conversion from
    /// [`crate::BulkLoadError`] rather than dropped (the count is the
    /// whole reason that type exists).
    BulkLoad {
        /// Facts that changed state in the chunks committed before the
        /// error.
        committed: u64,
        error: Box<Error>,
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
    /// an interval position — a membership binding or a `Contains`
    /// operand) bound to its domain ceiling. The point domain is
    /// `MIN ..= MAX−1`; `MAX` is the ray's ∞, never a point
    /// (`docs/architecture/10-data-model.md`, the point-domain law) — the
    /// bind-time sibling of
    /// [`ValidationError::PointLiteralAtCeiling`].
    PointParamAtCeiling {
        param: ParamId,
    },
    /// Bind-time: a non-mask value supplied for an `Allen` comparison's
    /// mask param — supply [`crate::BindValue::AllenMask`].
    AllenMaskParamExpected {
        param: ParamId,
    },
    /// Bind-time: a mask param bound to the empty mask — the predicate
    /// would be "never" (the validation-time sibling is
    /// [`ValidationError::EmptyAllenMask`]).
    EmptyAllenMaskParam {
        param: ParamId,
    },
    /// Bind-time: a mask param bound to the full mask — the predicate
    /// would be "always" (the validation-time sibling is
    /// [`ValidationError::FullAllenMask`]).
    FullAllenMaskParam {
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
    /// arithmetic. Hosts exclude rays first: an `Allen` guard
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
    /// A computed value crossed its representation — valid input whose
    /// result cannot be represented, so a typed error, never a panic.
    /// The payload names which computation.
    Overflow(OverflowKind),
    /// The result buffer's byte heap crossed the u32 offset space —
    /// more than 4 GiB of distinct string/bytes payload in one result.
    /// Absurd under the scale axiom, but it is valid input, so it
    /// errors rather than panics.
    ResultBytesOverflow,
    /// Hard corruption error, never a skip.
    Corruption(CorruptionError),
}

pub type Result<T> = std::result::Result<T, Error>;
