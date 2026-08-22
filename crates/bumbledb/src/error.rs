//! The workspace error taxonomy, categorized per
//! .
//! Everything reachable from user input or disk returns these typed errors;
//! ids and owned fact bytes, never formatted strings — no `format!` runs on
//! a hot path; `Display` formats lazily when the host actually prints.
//! panics are reserved for programmer-invariant violations. Payloads carry

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
/// (`projection_names` pairs `projection` positionwise), so the refusal
/// speaks the caller's own vocabulary without a descriptor lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetKeyCandidate {
    pub key: KeyId,
    pub projection: Box<[FieldId]>,
    pub projection_names: Box<[Box<str>]>,
}

/// Corruption detected while decoding stored bytes — a hard error, never a
/// skip, never a default. The offline
/// sweeper reports the same facts as [`crate::StoreFinding::Corruption`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorruptionError {
    InvalidBool(u8),

    InvalidInterval([u8; 16]),

    InvalidFixedIntervalStart([u8; 8]),

    /// 2026-07-23, R18). A half-created store (no `_meta` over an empty
    MetaMissing,

    DanglingInternId(InternId),

    MissingFact {
        relation: RelationId,
        row_id: u64,
    },

    MembershipDesync {
        relation: RelationId,
        row_id: u64,
    },

    DispositionDesync {
        relation: RelationId,
    },

    WrongFactWidth {
        relation: RelationId,
        row_id: u64,
        mismatch: Mismatch<usize>,
    },

    RowCountMismatch {
        relation: RelationId,
        stored: u64,
    },

    /// over-approximates any one relation's rows. The reopen-trust
    CounterDesync {
        relation: RelationId,
        exceeded: Exceeded<u64>,
    },

    MalformedValue(&'static str),

    DictReverseIdReuse,

    NonUtf8Intern(u64),

    NonzeroFixedBytesPad([u8; 8]),

    FactWithoutMembership {
        relation: RelationId,
        row_id: u64,
        membership_key: Box<[u8]>,
    },

    MembershipWithoutFact {
        relation: RelationId,
        row_id: u64,
        membership_key: Box<[u8]>,
    },

    FactWithoutDeterminant {
        relation: RelationId,
        statement: StatementId,
        row_id: u64,
        determinant_key: Box<[u8]>,
    },

    DeterminantWithoutFact {
        relation: RelationId,
        statement: StatementId,
        determinant_key: Box<[u8]>,
    },

    PointwiseOverlap {
        relation: RelationId,
        statement: StatementId,
        first: Box<[u8]>,
        second: Box<[u8]>,
    },

    FactWithoutReverseEdge {
        statement: StatementId,
        relation: RelationId,
        row_id: u64,
        reverse_key: Box<[u8]>,
    },

    ReverseEdgeWithoutFact {
        statement: StatementId,
        reverse_key: Box<[u8]>,
    },

    ReverseEdgeWeightDesync {
        statement: StatementId,
        reverse_key: Box<[u8]>,
        stored: Box<[u8]>,
        derived: Box<[u8]>,
    },

    RowCountDesync {
        relation: RelationId,
        stored: u64,
        counted: u64,
    },

    RowIdHighWaterLow {
        relation: RelationId,
        stored: u64,
        max_row_id: u64,
    },

    FreshRowDesync {
        relation: RelationId,
        row_id: u64,
        fresh: u64,
    },

    FreshNextValueLow {
        relation: RelationId,
        field: FieldId,
        stored: u64,
        max_fresh: u64,
    },

    DictForwardDesync {
        intern_id: InternId,

        forward: Option<InternId>,
    },

    DictNextIdLow {
        stored: InternId,
        reverse_id: InternId,
    },

    FreshRowDeterminantEntry {
        relation: RelationId,
        statement: StatementId,
        determinant_key: Box<[u8]>,
    },

    InternBeyondNextId {
        relation: RelationId,
        row_id: u64,
        intern_id: InternId,
        next_id: InternId,
    },

    ClosedRelationEntry {
        relation: RelationId,
        key: Box<[u8]>,
    },

    Malformed {
        key: Box<[u8]>,
        what: &'static str,
    },
}

/// A schema declaration error. Every illegal schema shape has a
/// distinct variant; an invalid schema is unconstructible, not flagged.
/// Two levels, and the partition is typed: declaration-scoped variants
/// live here and carry no statement id; every statement-roster rejection
/// is the one [`SchemaError::Statement`] arm — id beside its
/// [`StatementErrorKind`], one kind variant per roster line of, no catch-all. The roster's
/// "FD with selection" and "non-key FD form" lines have no variants:
/// [`crate::schema::StatementDescriptor::Functionality`] carries neither a
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

    FixedBytesWidthOutOfRange {
        relation: RelationId,
        field: FieldId,
        len: u16,
    },

    IntervalWidthOutOfRange {
        relation: RelationId,
        field: FieldId,
        width: u64,
    },

    RelationTooManyColumns {
        relation: RelationId,
        columns: usize,
    },

    TooManyStatements {
        count: usize,
    },

    EmptyExtension {
        relation: RelationId,
    },

    ExtensionTooManyRows {
        relation: RelationId,
        count: usize,
    },

    DuplicateExtensionHandle {
        relation: RelationId,
        handle: Box<str>,
    },

    ExtensionArityMismatch {
        relation: RelationId,
        row: RowIndex,
        mismatch: Mismatch<usize>,
    },

    ExtensionValueTypeMismatch {
        relation: RelationId,
        row: RowIndex,
        field: FieldId,
    },

    ExtensionIntervalRay {
        relation: RelationId,
        row: RowIndex,
        field: FieldId,
    },

    StrOnClosedRelation {
        relation: RelationId,
        field: FieldId,
    },

    FreshOnClosedRelation {
        relation: RelationId,
        field: FieldId,
    },

    Statement {
        statement: StatementId,
        kind: StatementErrorKind,
    },
}

/// One violated line of the statement-validation roster
/// — the
/// kind half of [`SchemaError::Statement`]: one variant per roster line,
/// no catch-all; each doc comment cites its line. Payloads carry ids and
/// owned evidence; the statement id lives on the carrier, so no variant
/// can forget it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementErrorKind {
    UnknownRelation {
        relation: RelationId,
    },

    UnknownField {
        relation: RelationId,
        field: FieldId,
    },

    EmptyProjection {
        relation: RelationId,
    },

    DuplicateProjectionField {
        relation: RelationId,
        field: FieldId,
    },

    DuplicateSelectionField {
        relation: RelationId,
        field: FieldId,
    },

    /// (`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff` —
    DegenerateSelectionSet {
        relation: RelationId,
        field: FieldId,
        len: usize,
    },

    DuplicateSelectionLiteral {
        relation: RelationId,
        field: FieldId,
    },

    CapacityInvertedWindow {
        lo: u64,
        hi: u64,
    },

    /// (`lean/Bumbledb/Capacity.lean: capacity_zero_star`), and a
    CapacityVacuousWindow,

    /// (`lean/Bumbledb/Subsumption.lean: window_floor_containment`) — one
    /// meaning, one spelling: drop the window and declare the
    CapacityContainmentWindow,
    /// Roster "an interval position in a capacity projection" — refused

    /// (`lean/Bumbledb/Capacity.lean` § v0 refusals; *trigger* for
    CapacityIntervalPosition {
        relation: RelationId,
        field: FieldId,
    },

    /// the typed polarity refusal (a negative weight would let an insert
    CapacityWeightNotU64 {
        relation: RelationId,
        field: FieldId,
    },

    CapacityWeightNotDuration {
        relation: RelationId,
        field: FieldId,
    },

    /// roster (ruled 2026-07-24, C1) and must be u64-encoded — signed
    /// and non-scalar encodings are refused.
    CapacityBoundNotU64 {
        relation: RelationId,
        field: FieldId,
    },

    CapacityBoundNotDuration {
        relation: RelationId,
        field: FieldId,
    },

    /// a dimension error (ruled 2026-07-24, C18; the legal pairings:
    CapacityDimensionMixing {
        field: FieldId,
    },

    FunctionalityMultipleIntervals {
        relation: RelationId,
        field: FieldId,
    },

    FunctionalityIntervalNotLast {
        relation: RelationId,
        field: FieldId,
    },

    DuplicateFunctionality {
        earlier: StatementId,
    },

    DeterminantKeyTooWide {
        width: usize,
    },

    ContainmentArityMismatch {
        mismatch: Mismatch<usize>,
    },

    ContainmentTypeMismatch {
        position: usize,
    },

    SelectedFieldProjected {
        relation: RelationId,
        field: FieldId,
    },

    SelectionLiteralTypeMismatch {
        relation: RelationId,
        field: FieldId,
    },

    /// has the descriptor in hand, and the refusal must speak the
    NoMatchingTargetKey {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
        available: Box<[TargetKeyCandidate]>,
    },

    NoPointwiseTargetKey {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
        available: Box<[TargetKeyCandidate]>,
    },
    /// An interval position on a containment with a closed side — refused
    ClosedContainmentInterval {
        relation: RelationId,
    },

    /// set may equal the refused projection — the refusal reason is
    ClosedTargetNotHandle {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
    },

    ClosedStatementRefuted {
        relation: RelationId,
        row: RowIndex,
    },

    DuplicateStatement {
        earlier: StatementId,
    },
}

impl StatementErrorKind {
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
    UnknownRelation {
        relation: RelationId,
    },

    UnknownField {
        relation: RelationId,
        field: FieldId,
    },

    NotAFreshField {
        relation: RelationId,
        field: FieldId,
    },

    NotAKeyStatement {
        relation: RelationId,
        statement: StatementId,
    },
}

/// A mis-shaped dynamic fact on the untyped write surface
/// (`insert_dyn`/`delete_dyn`): ETL input is data, so shape
/// problems are typed errors, not panics.
/// Id-resolution failures are [`DynIdError`], nested as [`Self::Id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactShapeError {
    Id(DynIdError),
    ArityMismatch {
        relation: RelationId,
        mismatch: Mismatch<usize>,
    },
    TypeMismatch {
        relation: RelationId,
        field: FieldId,
    },

    /// collection). ETL input is data, so the bound is a typed refusal,
    PayloadBound {
        relation: RelationId,
    },
}

impl From<DynIdError> for FactShapeError {
    fn from(err: DynIdError) -> Self {
        Self::Id(err)
    }
}

/// A query validation error (the IR boundary): one variant per roster item
/// in, returned at prepare time.
/// Rules validate one at a time, in order: every rule-local payload (an
/// `atom` occurrence index, a comparison `index`, a `find` position, a
/// `var`) names a position **inside the first failing rule**. An `atom`
/// payload is an *occurrence* index within that rule: positive atoms first
/// in rule order, then negated atoms — negated atoms are checked under the
/// same per-atom rules and share the same diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    EmptyRuleSet,

    TooManyRules {
        count: usize,
    },

    /// across all rules, judged before a single disjunct is materialized
    /// (so before duplicate collapse).
    DnfExceedsRules {
        exceeded: Exceeded<usize>,
    },

    /// iteratively, before any recursion sees the tree.
    ConditionNestingTooDeep {
        rule: RuleIndex,
        exceeded: Exceeded<usize>,
    },

    HeadArityMismatch {
        rule: RuleIndex,
        mismatch: Mismatch<usize>,
    },

    HeadTypeMismatch {
        rule: RuleIndex,
        position: FindIndex,
    },

    HeadAggregateMismatch {
        rule: RuleIndex,
        position: FindIndex,
    },

    /// query (ruled 2026-07-23, R1): under the head-projection law a
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

    PointLiteralAtCeiling {
        atom: AtomIndex,
        field: FieldId,
    },

    ParamIdGap {
        param: ParamId,
    },
    ParamTypeConflict {
        param: ParamId,
    },

    ParamScalarAndSet {
        param: ParamId,
    },

    ParamSetComparison {
        index: usize,
    },

    IntervalParamSet {
        param: ParamId,
    },

    IllegalComparison {
        index: usize,
    },

    OrderComparisonOnInterval {
        index: usize,
    },

    /// the order-on-bytes refusal).
    OrderComparisonOnFixedBytes {
        index: usize,
    },

    OrderComparisonOnString {
        index: usize,
    },
    /// An order operator over a closed-bound variable (ruled 2026-07-23,

    /// declaration-order accident, not semantics — refused exactly as
    OrderComparisonOnClosedReference {
        index: usize,
    },

    ConstantComparison {
        index: usize,
    },

    SelfComparison {
        index: usize,
    },

    ComparisonPointLiteralAtCeiling {
        index: usize,
    },

    EmptyAllenMask {
        index: usize,
    },

    FullAllenMask {
        index: usize,
    },

    MembershipOnlyVariable {
        var: VarId,
    },
    /// Negation safety: a variable occurring in a negated atom must occur
    NegatedVariableUnbound {
        var: VarId,
    },
    /// Datalog safety: a find (or aggregate-input) variable bound by no
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

    NoPositiveAtoms,

    /// over bool is Any, `Min` is All — ruled 2026-07-23, R3).
    AggregateInputType {
        find: FindIndex,
    },

    /// variable (ruled 2026-07-23, R4): its words are declaration

    /// accident — refused exactly as the order comparison is
    AggregateOverClosedReference {
        find: FindIndex,
    },

    CountWithVariable {
        find: FindIndex,
    },

    AggregateWithoutVariable {
        find: FindIndex,
    },
    AggregateOverGroupKey {
        find: FindIndex,
    },

    /// sighting and is refused. *Trigger* for admitting it: a real query
    MultiplePackTerms {
        find: FindIndex,
    },

    MixedPackAndFold {
        find: FindIndex,
    },

    PackInputType {
        find: FindIndex,
    },

    TooManyAtoms {
        count: usize,
    },

    TooManyVariables {
        count: usize,
    },

    /// (the rec). Counted with `usize` before any
    InteriorIdOverflow {
        count: usize,
    },

    EmptyInterior {
        interior: InteriorId,
    },

    EmptyRecursiveBase,

    EmptyRecursiveStep,

    SelfInBase,

    RecArmMissingSelf,

    NonlinearRecArm,

    NegationInRec,

    UnknownInterior {
        atom: AtomIndex,
        interior: InteriorId,
    },

    InteriorColumnOutOfRange {
        atom: AtomIndex,
        field: FieldId,
    },

    InteriorNotPrior {
        interior: InteriorId,
        at: InteriorId,
    },

    AggregateInInterior {
        interior: InteriorId,
    },
}

/// Which side of a containment statement the commit-time judgment found
/// unsatisfied.
/// `Ord` is citation order: within one statement cited in both
/// directions, source before target ([`Violations`]' sort key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    SourceUnsatisfied,

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
    /// # Panics

    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            Self::Accepted(value) => value,
            Self::Rejected(violations) => panic!("admission rejected: {violations}"),
        }
    }

    /// # Panics

    #[track_caller]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Self::Accepted(value) => value,
            Self::Rejected(violations) => {
                panic!("{msg}: admission rejected: {violations}")
            }
        }
    }

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
    /// # Panics

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

    /// # Panics

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
    Scalar,

    Pointwise { incumbent: Box<[u8]> },
}

/// [`Violations`]. One body: the typed spine slot, the convicting fact
/// bytes, and a per-law detail. Storage row ids never appear
/// .
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Functionality {
        statement: StatementRef,
        fact: Box<[u8]>,
        conflict: Conflict,
    },

    Containment {
        statement: StatementRef,
        direction: Direction,

        fact: Box<[u8]>,
    },

    /// (`lean/Bumbledb/Capacity.lean: CapacityLaw`).
    Capacity {
        statement: StatementRef,

        fact: Box<[u8]>,

        /// width crosses untruncated (ruled 2026-07-24, C3). On

        /// 2026-07-24, C14: the clip serves the verdict, the full sum
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

    #[must_use]
    pub const fn statement(&self) -> StatementRef {
        match *self {
            Self::Functionality { statement, .. }
            | Self::Containment { statement, .. }
            | Self::Capacity { statement, .. } => statement,
        }
    }

    #[must_use]
    pub fn statement_id(&self, schema: &crate::schema::Schema) -> StatementId {
        schema.id_of(self.statement())
    }

    #[must_use]
    pub fn fact(&self) -> &[u8] {
        match self {
            Self::Functionality { fact, .. }
            | Self::Containment { fact, .. }
            | Self::Capacity { fact, .. } => fact,
        }
    }

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
/// .
/// Decoding happens AT rejection time, inside the commit boundary,
/// because that is the only time it is possible: an inserted fact's
/// nowhere, so a post-hoc decode helper would misread genuine rejections
/// as corruption. `values` are in sealed field order (a closed
/// relation's synthetic id first), `str` fields resolved to owned
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedFact {
    relation: RelationId,
    values: Box<[bumbledb_theory::Value]>,
}

impl CitedFact {
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

    #[must_use]
    pub const fn relation(&self) -> RelationId {
        self.relation
    }

    #[must_use]
    pub fn values(&self) -> &[bumbledb_theory::Value] {
        &self.values
    }
}

/// ```compile_fail
/// let _ = bumbledb::Violations { citations: Box::new([]) };
/// ```
pub(crate) type CitedCitations = Box<[(Violation, Box<[CitedFact]>)]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violations {
    citations: CitedCitations,
}

impl Violations {
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

    pub(crate) fn from_pairs(citations: CitedCitations) -> Self {
        debug_assert!(
            !citations.is_empty(),
            "a sealed rejection is nonempty by construction"
        );
        Self { citations }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.citations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.citations.is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Violation> {
        self.citations.get(index).map(|(violation, _)| violation)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[(Violation, Box<[CitedFact]>)] {
        &self.citations
    }

    pub fn iter(&self) -> impl Iterator<Item = &Violation> {
        self.citations.iter().map(|(violation, _)| violation)
    }

    #[must_use]
    pub fn cited_facts(&self, index: usize) -> &[CitedFact] {
        self.citations
            .get(index)
            .map_or(&[], |(_, cited)| cited.as_ref())
    }

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
    Aggregate { find: FindIndex },

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

/// inner boxed payload (it was never clone-faithful); the variant remains.
/// An LMDB/heed failure owned by [`Error`]. Encoding/decoding drop the
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
/// .
/// (`Io`, `Lmdb`, `CommitSync`, `TransactionPoisoned`); the structured
/// variants (`Corruption`, `Schema`, `Validation`, `FactShape`, …) carry
/// data payloads, not nested errors — a decision, not an omission:
/// chain-walkers see exactly the real causes, and the structured detail
/// renders through `Display`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Storage format version mismatch — checked before the fingerprint.
    FormatMismatch {
        mismatch: Mismatch<u32>,
    },

    SchemaMismatch {
        mismatch: Mismatch<SchemaFingerprint>,
    },
    /// `create` refused a directory that already holds an LMDB

    /// Open-time: a foreign LMDB environment, or a half-created empty
    AlreadyInitialized,

    DestinationExists {
        path: PathBuf,
    },

    PublishedButUnsynced {
        path: PathBuf,
        source: IoFailure,
    },

    EnvironmentLocked,
    Io(IoFailure),

    #[doc(hidden)]
    Hatch(Hatch),
    Lmdb(LmdbFailure),

    /// not diagnosing LMDB.
    ReadersFull {
        max_readers: u32,
    },

    Schema(SchemaError),
    Validation(ValidationError),

    FactShape(FactShapeError),

    FreshExhausted {
        relation: RelationId,
        field: FieldId,
    },

    /// delta. Checked at every write-surface entry before any encoding
    ClosedRelationWrite {
        relation: RelationId,
    },

    /// a raw OS errno from its write/sync path — on macOS the data-page

    /// meta write; LMDB reports one errno for the phase and names no
    CommitSync {
        /// Bounded retries consumed before the error escaped.
        retries: u32,
        error: IoFailure,
    },

    /// after catching, still abort — no prefix reaches LMDB.
    TransactionPoisoned {
        source: Box<Error>,
    },

    ForeignPreparedQuery,

    ForeignWitness,

    ParamCountMismatch {
        mismatch: Mismatch<usize>,
    },

    ParamTypeMismatch {
        param: ParamId,
        expected: ValueType,
    },

    ParamSetExpected {
        param: ParamId,
    },

    ParamScalarExpected {
        param: ParamId,
    },

    ParamElementTypeMismatch {
        param: ParamId,
        element: usize,
        expected: ValueType,
    },

    PointParamAtCeiling {
        param: ParamId,
    },

    /// refusal naming the row (ruled 2026-07-24, C10). Boundedness is
    CapacityRayMeasure {
        statement: StatementId,

        fact: Box<[u8]>,
    },

    /// (`lean/Bumbledb/Exec/Reach.lean: reach_den_finite`), but derived

    /// refusal: aborts the query, the snapshot
    DerivedBudgetExceeded {
        rounds: u32,

        tuples: u64,
    },

    Overflow(OverflowKind),

    /// limit — NOT the map size; do not sweep it with the map constant.)
    ResultBytesOverflow,

    Corruption(CorruptionError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Per-variant taxonomy: the one exhaustive table [`Error::descriptor`]
/// walks. `source`, the C kind map, and any future Clone-like fold
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
    CapacityRayMeasure,
    DerivedBudgetExceeded,
    Overflow,
    ResultBytesOverflow,
    Corruption,
}

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
    #[doc(hidden)]
    #[must_use]
    pub fn hatch() -> Self {
        Self::Hatch(Hatch)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn is_hatch(&self) -> bool {
        matches!(self, Self::Hatch(_))
    }

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
            Self::CapacityRayMeasure { .. } => family_only(ErrorFamily::CapacityRayMeasure),
            Self::DerivedBudgetExceeded { .. } => family_only(ErrorFamily::DerivedBudgetExceeded),
            Self::Overflow(_) => family_only(ErrorFamily::Overflow),
            Self::ResultBytesOverflow => family_only(ErrorFamily::ResultBytesOverflow),
            Self::Corruption(_) => family_only(ErrorFamily::Corruption),
        }
    }

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
