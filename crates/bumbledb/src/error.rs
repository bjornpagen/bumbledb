//! The workspace error taxonomy, categorized per
//! `docs/architecture/70-api.md`.
//!
//! Everything reachable from user input or disk returns these typed errors;
//! panics are reserved for programmer-invariant violations. Payloads carry
//! ids and owned fact bytes, never formatted strings — no `format!` runs on
//! a hot path; `Display` formats lazily when the host actually prints.

mod convert;
mod display;

use crate::ir::{ParamId, PredId, VarId};
use crate::schema::fingerprint::SchemaFingerprint;
use crate::schema::{FieldId, KeyId, RelationId, StatementId, ValueType};

/// One declared key offered as owned evidence in a target-key rejection.
/// The diagnostic may outlive the descriptor, so it carries no schema
/// references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetKeyCandidate {
    pub key: KeyId,
    pub projection: Box<[FieldId]>,
}

/// Corruption detected while decoding stored bytes — a hard error, never a
/// skip, never a default (`docs/architecture/50-storage.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// The `_meta` database or one of its required keys is absent or
    /// malformed: the environment is not a usable bumbledb database.
    MetaMissing,
    /// The `_meta` store-kind marker is PRESENT but undecodable — a
    /// wrong-width value or a byte no [`crate::StoreKind`] encodes to.
    /// Distinct from [`CorruptionError::MetaMissing`]: the key exists,
    /// so this is corrupt data, not a missing key.
    StoreKindInvalid,
    /// An intern id with no reverse dictionary entry — a fact referencing it
    /// is corrupt.
    DanglingInternId(u64),
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
    /// A stored `bytes<N>` field with a nonzero byte in its trailing pad
    /// — the pad is encoding, not data, so a nonzero pad byte is exactly
    /// as corrupt as a non-0/1 Bool byte. Carries the offending trailing
    /// word's 8 bytes.
    NonzeroFixedBytesPad([u8; 8]),
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
        row: usize,
        expected: usize,
        supplied: usize,
    },
    /// An extension value does not inhabit its column's structural type —
    /// the one shared value check, as selection literals.
    ExtensionValueTypeMismatch {
        relation: RelationId,
        row: usize,
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
        row: usize,
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
    /// Roster "a degenerate literal set": a `Many` binding with fewer than
    /// two literals — the empty set selects nothing (write no statement)
    /// and the one-literal set is the `One` spelling, kept the only
    /// singleton by representation
    /// (`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff` —
    /// a singleton set is exactly today's equality).
    DegenerateSelectionSet {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
        len: usize,
    },
    /// Roster "a duplicate literal within one binding's set": the set is
    /// canonical — sorted, duplicate-free — so a repeated literal is
    /// rejected, not silently collapsed (write it once).
    DuplicateSelectionLiteral {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "an inverted window": `hi < lo` is satisfied by no count —
    /// the statement is unsatisfiable as declared. The canonical bounds
    /// are `lo < hi` (an exact count is `lo = hi`, the `{n}` spelling).
    CardinalityInvertedWindow {
        statement: StatementId,
        lo: u64,
        hi: u64,
    },
    /// Roster "the vacuous window": `0..*` admits every count — the
    /// statement provably says nothing
    /// (`lean/Bumbledb/Cardinality.lean: cardinality_zero_star`), and a
    /// statement that says nothing is not a statement (the
    /// canonical-utterance law, `docs/architecture/70-api.md`).
    CardinalityVacuousWindow {
        statement: StatementId,
    },
    /// Roster "the containment respelled": `1..*` says exactly what the
    /// bare containment `target <= source` says
    /// (`lean/Bumbledb/Subsumption.lean: window_floor_containment`) — one
    /// meaning, one spelling: drop the window and declare the
    /// containment.
    CardinalityContainmentWindow {
        statement: StatementId,
    },
    /// Roster "an interval position in a window projection" — refused v0:
    /// a window counts FACTS per parent, and an interval position would
    /// make the count ambiguous between facts and points
    /// (`lean/Bumbledb/Cardinality.lean` § v0 refusals; *trigger* for
    /// lifting: a sighted counting-over-denotation workload — counting
    /// points, not rows).
    CardinalityIntervalPosition {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster ">1 interval position": two interval fields in one FD
    /// projection would be 2-D exclusion, which the ordered determinant cannot
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
    /// second determinant is pure write amplification, and rejecting it makes
    /// containment target-key resolution unambiguous.
    DuplicateFunctionality {
        statement: StatementId,
        earlier: StatementId,
    },
    /// Roster "determinant width overflow": Σ projected field widths exceeds
    /// [`crate::storage::keys::MAX_DETERMINANT_WIDTH`] — rejected at declaration,
    /// never discovered at write time.
    DeterminantKeyTooWide {
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
    /// Roster "selection literal type mismatch (… non-UTF-8 string
    /// literals)".
    SelectionLiteralNotUtf8 {
        statement: StatementId,
        relation: RelationId,
        field: FieldId,
    },
    /// Roster "IND whose target projection matches no key of the target":
    /// probe-ability requires Y to be a permutation of a declared key.
    NoMatchingTargetKey {
        statement: StatementId,
        target: RelationId,
        projection: Box<[FieldId]>,
        available: Box<[TargetKeyCandidate]>,
    },
    /// Roster "IND … (or, with an interval position, no pointwise key
    /// carrying it)": the coverage walk needs the target's own key to keep
    /// its intervals disjoint and ordered.
    NoPointwiseTargetKey {
        statement: StatementId,
        target: RelationId,
        projection: Box<[FieldId]>,
        available: Box<[TargetKeyCandidate]>,
    },
    /// An interval position on a containment with a closed side — refused
    /// v0: a pointwise judgment against a closed relation would mix the
    /// coverage walk with virtual storage, and a constant source's
    /// coverage demand has no delete to re-judge it under
    /// (`docs/architecture/30-dependencies.md`, the refusal —
    /// *trigger* for lifting it: a census sighting). Carries the closed
    /// relation.
    ClosedContainmentInterval {
        statement: StatementId,
        relation: RelationId,
    },
    /// A statement between constants that the ground axioms refute: both
    /// sides of the judgment are sealed at validate, so its truth is
    /// decidable here — and a theory whose axioms refute its own statement
    /// has no model to commit (`docs/architecture/30-dependencies.md`,
    /// "a committed database is a model of its theory, always"). For a
    /// containment, `row` is the source axiom outside the compiled member
    /// set; for a functionality, the second axiom of the colliding pair.
    ClosedStatementRefuted {
        statement: StatementId,
        relation: RelationId,
        row: usize,
    },
    /// Roster "duplicate statements (identical normalized sides and form —
    /// write it once)": selections compare sorted by field id.
    DuplicateStatement {
        statement: StatementId,
        earlier: StatementId,
    },
}

/// A mis-shaped dynamic fact on the untyped write surface
/// (`insert_dyn`/`delete_dyn`/`bulk_load_dyn`): ETL input is data, so shape
/// problems are typed errors, not panics (`docs/architecture/70-api.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactShapeError {
    /// The relation id is outside the schema — ETL input is data, so an
    /// out-of-range id at the dynamic surface (`insert_dyn`/`delete_dyn`/
    /// `bulk_load_dyn`/`scan`/`fresh_field`) is a typed error, never an index
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
        produced: usize,
        cap: usize,
    },
    /// A rule's condition trees nest deeper than
    /// [`crate::ir::MAX_CONDITION_DEPTH`] — the boundary check for every
    /// recursive tree walk (the trust-boundary law: hostile nesting must
    /// be a typed rejection, never a stack exhaustion). Judged
    /// iteratively, before any recursion sees the tree.
    ConditionNestingTooDeep {
        rule: usize,
        depth: usize,
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
    /// An order operator with a Bool operand. Boolean ordering is noise;
    /// Bool is equality-only.
    OrderComparisonOnBool {
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
    /// (`docs/architecture/20-query-ir.md` § the Allen operator; the
    /// bind-time sibling is [`Error::EmptyAllenMaskParam`]).
    EmptyAllenMask {
        index: usize,
    },
    /// An `Allen` comparison whose literal mask is all 13 basics — every
    /// pair satisfies it, so the condition is "always": write no
    /// condition (the bind-time sibling is [`Error::FullAllenMaskParam`]).
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
    /// A second `Pack` term in one head: the multi-`Pack` product has no
    /// sighting and is refused. *Trigger* for admitting it: a real query
    /// needing two coalesced columns in one row
    /// (`docs/architecture/20-query-ir.md` § aggregation).
    MultiplePackTerms {
        find: usize,
    },
    /// `Pack` beside a fold aggregate (Sum/Min/Max/Count/CountDistinct):
    /// `Pack` is relation-shaped — a fold column repeated per segment row
    /// is a join in aggregate costume. Coalesced-time accounting
    /// (`Sum∘Duration∘Pack`) is two prepared queries or a host fold over
    /// packed answers; *trigger* for a composed form: a measured two-pass
    /// budget violation.
    MixedPackAndFold {
        find: usize,
    },
    /// `Pack` beside Arg terms — the two relation-shaped aggregates do
    /// not compose in one head (the Arg/fold mixing rule, extended).
    MixedPackAndArg {
        find: usize,
    },
    /// `Pack` over a non-interval variable: the coalesce is defined by
    /// the interval point-set denotation and by nothing else.
    PackInputType {
        find: usize,
    },
    /// A `Term::Measure` in an atom binding: the measure is a
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
    /// A `FindTerm::AggregateMeasure` whose op is not `Sum`/`Min`/`Max`
    /// — `Count` is nullary, `CountDistinct` over a measure is a count
    /// over derived values with no sighted use, and the Arg ops key on
    /// variables, not computations.
    DurationAggregateOp {
        find: usize,
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

    // --- The program roster (20-query-ir.md § engine recursion; the strata judge
    // and the well-formedness screen, `ir/validate/strata.rs`) ---
    /// The predicate-count cap ([`crate::ir::MAX_PREDICATES`]) — the
    /// program sibling of [`ValidationError::TooManyRules`].
    TooManyPredicates {
        count: usize,
    },
    /// The program's `output` names no predicate — the answer position
    /// of the well-formedness screen.
    UnknownOutputPredicate {
        pred: PredId,
    },
    /// An `Idb` atom names a predicate outside the program — the
    /// well-formedness screen (`lean/Bumbledb/Query/Syntax.lean:
    /// Program.WellFormed`, spent by `lean/Bumbledb/Exec/Fixpoint.lean:
    /// wellFormed_reads_real`): without it a phantom `idb` read denotes
    /// the empty fact set, and a NEGATED phantom read would be
    /// vacuously satisfied — the stratification witness never refuses
    /// the shape, so the screen must. `atom` is the occurrence index
    /// (positives first, then negated) inside the failing rule.
    UnknownPredicate {
        atom: usize,
        pred: PredId,
    },
    /// An `Idb` binding's `FieldId` sits at or beyond the target
    /// predicate's arity — head positions are the whole address space
    /// (the arity roster item beside the screen; `FieldId(i)` is column
    /// `i`, positional, never nominal).
    PredicateColumnOutOfRange {
        atom: usize,
        field: FieldId,
    },
    /// A negated atom whose target shares the atom's own SCC — negation
    /// through a cycle. Negation *of* lower strata is legal: a lower
    /// stratum is a finished set before this stratum's operator runs,
    /// which is exactly what keeps the operator monotone
    /// (`lean/Bumbledb/Exec/Fixpoint.lean: stratumOp_mono` spends the
    /// strictly-lower premise; `lean/Bumbledb/Countermodels.lean:
    /// odd_not_monotone` is the wall without it).
    NegationThroughCycle {
        pred: PredId,
        via: PredId,
    },
    /// A fold in a head whose rule body reads the head's own SCC —
    /// aggregation through a cycle. Aggregation *of* lower strata is
    /// legal for the same reason negation is: an `Idb` atom under a
    /// fold reads a finished set.
    AggregationThroughCycle {
        pred: PredId,
        via: PredId,
    },
    /// A `Measure` find in a recursive predicate's head. Two
    /// derivations (20-query-ir.md § engine recursion): the safety theorem requires
    /// recursive heads to project **bound** variables — the measure is
    /// a computation, not a binding — and the error-timing ruling: the
    /// round at which a ray reaches a recursive head would depend on
    /// iteration order, so the same store would error after differing
    /// partial work. The measure over a *lower* stratum from a
    /// non-recursive head stays legal.
    MeasureInRecursiveHead {
        pred: PredId,
    },
    /// A predicate whose signature never seals: every one of its rules
    /// reads a same-SCC predicate whose own signature is still
    /// underived, so some column's type is anchored only through the
    /// cycle (e.g. `p(x) | p(x)` — no stored column ever names `x`'s
    /// type). The signature fixpoint's honest bottom.
    UnresolvedPredicateSignature {
        pred: PredId,
    },
    /// A fold- or fold-measure-headed predicate below the program's
    /// output. The executable program class is the Lean cut's
    /// (`lean/Bumbledb/Query/Syntax.lean: PRule` — `finds : List VarId`,
    /// so a program-level fold head is unrepresentable in the model;
    /// `lean/Bumbledb/Exec/Fixpoint.lean: evalProgram` computes
    /// projection heads only): an interior predicate's answers are a
    /// transient word-row table read by `Idb` occurrences, and a fold's
    /// answers only materialize at finalize — the OUTPUT predicate,
    /// where the ordinary head-owned sink and finalize already live.
    /// Aggregation *of* lower strata therefore stays legal exactly as
    /// 20-query-ir.md § engine recursion records it (a fold rule reading finished
    /// `Idb` sets), while a fold predicate *feeding* another predicate
    /// is refused with this typed error. A projected `Measure` head is
    /// NOT a fold — it is a value column (u64 per row) — but it is
    /// likewise interior-refused, with its own typed error
    /// ([`Self::MeasureInteriorPredicate`]).
    AggregateInteriorPredicate {
        pred: PredId,
    },
    /// A `Measure` find in an interior (non-output) predicate's head,
    /// recursive or not. The executable program class is the Lean cut's
    /// (`lean/Bumbledb/Query/Syntax.lean: PRule` — `finds : List VarId`,
    /// so an interior measure head is unrepresentable in the model;
    /// `lean/Bumbledb/Exec/Fixpoint.lean: evalProgram` computes
    /// projection heads only): the engine narrows to the model rather
    /// than executing a class with zero oracle coverage. The measure at
    /// the OUTPUT predicate's head stays legal — it evaluates on the
    /// query surface exactly as a degenerate program does, where the
    /// `MeasureOfRay` timing ruling already lives. The recursive form is
    /// caught first by the strata roster
    /// ([`Self::MeasureInRecursiveHead`]); this error names the
    /// non-recursive interior remainder.
    MeasureInteriorPredicate {
        pred: PredId,
    },
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

/// One violated statement of a rejected commit — the element of
/// [`Violations`]. Payloads carry the statement id and canonical fact
/// bytes, never storage row ids (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A `Functionality` statement violated by the final state: two live
    /// facts claim one key — the same determinant bytes (scalar put-conflict),
    /// or overlapping intervals within one scalar-prefix group (the
    /// pointwise neighbor probe).
    Functionality {
        statement: StatementId,
        /// The fact whose insert violated the statement.
        fact: Box<[u8]>,
        /// The already-standing fact, for the pointwise arm — the probe
        /// names both parties. `None` for a scalar put-conflict, where
        /// the determinant bytes inside `fact` already identify the collision.
        incumbent: Option<Box<[u8]>>,
    },
    /// A `Containment` statement violated by the final state
    /// (`docs/architecture/30-dependencies.md` § judged on final states).
    /// `fact` is canonical source-fact bytes on either side: the judgment
    /// speaks about sources — a missing target is named by the source
    /// that requires it.
    Containment {
        statement: StatementId,
        direction: Direction,
        /// The source fact: the inserted fact whose target is missing
        /// (`SourceUnsatisfied`), or the surviving fact still requiring a
        /// deleted target key (`TargetRequired`).
        fact: Box<[u8]>,
    },
    /// A `Cardinality` statement violated by the final state: a selected
    /// parent fact whose child-group count falls outside the window —
    /// below the floor or above the ceiling
    /// (`lean/Bumbledb/Cardinality.lean: CardinalityWindow`).
    Cardinality {
        statement: StatementId,
        /// The convicting parent fact: the ψ-selected holder of the
        /// touched key tuple whose group count is out of window.
        fact: Box<[u8]>,
        /// The observed child-group count (the walk stops as soon as the
        /// verdict is decided, so a ceiling conviction reports the first
        /// count past the ceiling).
        count: u64,
    },
}

impl Violation {
    /// The violated statement.
    #[must_use]
    pub fn statement(&self) -> StatementId {
        match self {
            Self::Functionality { statement, .. }
            | Self::Containment { statement, .. }
            | Self::Cardinality { statement, .. } => *statement,
        }
    }

    /// The citation identity — [`Violations`]' sort and dedup key:
    /// statement id (materialized order), then direction (source before
    /// target; key and window statements have none). Witness
    /// facts, counts, and defect kinds are deliberately outside the
    /// identity: a statement is cited once per direction, whatever the
    /// count of facts convicting it.
    fn citation(&self) -> (StatementId, Option<Direction>) {
        match self {
            Self::Functionality { statement, .. } | Self::Cardinality { statement, .. } => {
                (*statement, None)
            }
            Self::Containment {
                statement,
                direction,
                ..
            } => (*statement, Some(*direction)),
        }
    }
}

/// The complete violation set of one rejected commit — sealed: nonempty,
/// one citation per statement (per direction for a containment), sorted
/// by materialized statement order. The only constructors sort and dedup
/// and refuse emptiness, so an empty, unsorted, or duplicated set is
/// unrepresentable — a rejection IS this set, never an arbitrary
/// representative (`docs/architecture/30-dependencies.md` § judged on
/// final states).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violations(Box<[Violation]>);

impl Violations {
    /// Seals a collector's raw finds: stable-sorts by citation (so the
    /// first-discovered witness of each citation survives), dedups by
    /// citation, and returns `None` for the empty collection — the
    /// accept path, never an empty rejection.
    pub(crate) fn seal(mut found: Vec<Violation>) -> Option<Self> {
        if found.is_empty() {
            return None;
        }
        found.sort_by_key(Violation::citation);
        found.dedup_by_key(|violation| violation.citation());
        Some(Self(found.into_boxed_slice()))
    }

    /// The singleton set — a lone violation is trivially sealed. The
    /// judgment probes convict through this shape and the collectors
    /// flatten it ([`Violations::seal`] re-sorts the union).
    pub(crate) fn one(violation: Violation) -> Self {
        Self(Box::new([violation]))
    }

    /// Every violation, in citation order.
    #[must_use]
    pub fn as_slice(&self) -> &[Violation] {
        &self.0
    }

    /// Iterates the violations, in citation order.
    pub fn iter(&self) -> std::slice::Iter<'_, Violation> {
        self.0.iter()
    }
}

impl IntoIterator for Violations {
    type Item = Violation;
    type IntoIter = std::vec::IntoIter<Violation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a Violations {
    type Item = &'a Violation;
    type IntoIter = std::slice::Iter<'a, Violation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Which computation crossed its representation — [`Error::Overflow`]'s
/// payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowKind {
    /// An aggregate's final value exceeds its result type (the once-at-
    /// finalization range check; deterministic under any fold order).
    /// Carries the find-position index.
    Aggregate { find: usize },
    /// The executor's D2 origin counter would cross u32 — more than 2³²
    /// absorb-node survivors in one execution. Beyond the scale axiom,
    /// but valid input, so it errors; checked at batch granularity
    /// (`exec/run/probe_pass.rs`).
    OriginCapacity,
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
    /// The store on disk is not the KIND this constructor opens
    /// (`docs/architecture/50-storage.md` § the ephemeral store kind):
    /// `Db::open` reached an ephemeral store, or `Db::ephemeral` reached
    /// a durable one. The kind is a property of the store, marked in
    /// `_meta` at creation — never a mode of a handle — so the durable
    /// surface can never quietly read a store that skipped its fsyncs,
    /// and the ephemeral surface can never quietly strip a durable
    /// store's guarantee. Checked after the format version, before the
    /// fingerprint.
    StoreKindMismatch {
        found: crate::storage::env::StoreKind,
        expected: crate::storage::env::StoreKind,
    },
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
    /// A commit rejected by the dependency judgment: the payload is the
    /// COMPLETE violation set — every violated statement, cited once
    /// (per direction for a containment), in materialized statement
    /// order — never an arbitrary representative among simultaneous
    /// violations (`docs/architecture/30-dependencies.md` § judged on
    /// final states). Key (`Functionality`) violations preempt the
    /// containment judgment: the containment probes are defined over the
    /// keyed final state, which exists only when every key statement
    /// holds — so one rejection is all-key or all-containment, complete
    /// within its phase.
    CommitRejected {
        violations: Violations,
    },
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
    /// [`crate::Db::write_from`]'s witness compare failed: a
    /// state-changing commit landed after the witness snapshot was taken,
    /// so the premises the host computed from are stale. Raised before
    /// any page is touched; the delta drops exactly as any abort does.
    /// Payload is the two generations (ids, never strings) — the same
    /// generation the image cache keys on, so a counters-only/no-op
    /// commit never raises this. Retry is host policy: re-run the query,
    /// re-compute, `write_from` again (`docs/architecture/70-api.md`
    /// § conditional writes).
    GenerationMoved {
        /// The witness snapshot's generation.
        witnessed: crate::storage::env::GenerationId,
        /// The current committed generation.
        current: crate::storage::env::GenerationId,
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
    /// A witness snapshot of a different database than the one being
    /// written ([`crate::Db::write_from`]) — the same environment-identity
    /// key-probe prepared queries run at every execution entry, on the write
    /// side: another database's generation clock proves nothing about
    /// this one.
    ForeignSnapshot,
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
    /// an interval position — a membership binding or a `PointIn`
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
    /// Bind-time: a mask param bound to the empty mask — the condition
    /// would be "never" (the validation-time sibling is
    /// [`ValidationError::EmptyAllenMask`]).
    EmptyAllenMaskParam {
        param: ParamId,
    },
    /// Bind-time: a mask param bound to the full mask — the condition
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
    /// A stratum's fixpoint crossed the driver's iteration/tuple budget
    /// — the one new trust boundary the recursion campaign added
    /// (`docs/architecture/40-execution.md` § the fixpoint driver).
    /// Termination is a
    /// theorem of the validation roster
    /// (`lean/Bumbledb/Exec/Fixpoint.lean: program_den_finite`), but
    /// the fixpoint's *size* is data-shaped: a foreign query may
    /// legally demand a quadratic closure, and an unbounded round count
    /// crossing the trust boundary is what the recorded v0 OS-backstop
    /// argument never priced. On `MeasureOfRay`'s model: aborts the
    /// query, the snapshot stays usable, the payload is ids and counts
    /// — never strings. Policy stays host-owned
    /// ([`crate::PreparedQuery::set_fixpoint_budget`] — the staleness
    /// doctrine verbatim: the engine ships the typed condition, never a
    /// threshold loop); the documented default
    /// ([`crate::api::prepared::fixpoint::DEFAULT_FIXPOINT_ROUNDS`] /
    /// [`crate::api::prepared::fixpoint::DEFAULT_FIXPOINT_TUPLES`])
    /// exists so the boundary is never unguarded.
    FixpointBudgetExceeded {
        /// The stratum whose fixpoint crossed the budget (the SCC
        /// condensation index, `ir/validate/strata.rs`).
        stratum: u16,
        /// Rounds the stratum had run when the budget tripped.
        rounds: u32,
        /// Distinct tuples the stratum's predicates had derived.
        tuples: u64,
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
