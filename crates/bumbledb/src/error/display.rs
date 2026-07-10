//! `Display` rendering for every error type — formatting runs lazily, only
//! when the host actually prints.
//!
//! Statements are anonymous (`docs/architecture/30-dependencies.md`), so
//! the plain `Display` impls cite them by id; the [`Error::display_with`]
//! and [`SchemaError::display_with`] adapters pair the error with the
//! schema it speaks about and render the statement back in the `schema!`
//! algebra notation (`crate::schema::render`).

use std::fmt;

use crate::schema::{render, Schema, SchemaDescriptor, StatementId};

use super::{CorruptionError, Direction, Error, FactShapeError, SchemaError, ValidationError};

impl fmt::Display for FactShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelation { relation } => {
                write!(f, "relation {}: not in this schema", relation.0)
            }
            Self::UnknownField { relation, field } => {
                write!(f, "relation {} has no field {}", relation.0, field.0)
            }
            Self::NotAFreshField { relation, field } => write!(
                f,
                "relation {}, field {}: not a fresh field",
                relation.0, field.0
            ),
            Self::ArityMismatch {
                relation,
                expected,
                supplied,
            } => write!(
                f,
                "relation {}: {supplied} values for {expected} fields",
                relation.0
            ),
            Self::TypeMismatch { relation, field } => {
                write!(
                    f,
                    "relation {}, field {}: wrong value kind",
                    relation.0, field.0
                )
            }
            Self::EnumOrdinalOutOfRange {
                relation,
                field,
                ordinal,
            } => write!(
                f,
                "relation {}, field {}: enum ordinal {ordinal} out of range",
                relation.0, field.0
            ),
            Self::InvalidUtf8 { relation, field } => write!(
                f,
                "relation {}, field {}: string bytes are not UTF-8",
                relation.0, field.0
            ),
            Self::EmptyInterval { relation, field } => write!(
                f,
                "relation {}, field {}: interval start >= end",
                relation.0, field.0
            ),
            Self::NotAKeyStatement {
                relation,
                statement,
            } => write!(
                f,
                "statement {} is not a key of relation {}",
                statement.0, relation.0
            ),
        }
    }
}

impl fmt::Display for CorruptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBool(byte) => write!(f, "invalid Bool byte {byte:#04x}"),
            Self::EnumOrdinalOutOfRange {
                ordinal,
                variant_count,
            } => write!(f, "enum ordinal {ordinal} beyond {variant_count} variants"),
            Self::InvalidInterval(bytes) => {
                write!(f, "interval bytes {bytes:02x?}: start >= end")
            }
            Self::MetaMissing => write!(f, "the _meta database is absent or malformed"),
            Self::DanglingInternId(id) => write!(f, "intern id {id} has no dictionary entry"),
            Self::MissingFact { relation, row_id } => {
                write!(f, "relation {}: row {row_id} has no fact", relation.0)
            }
            Self::MembershipDesync { relation, row_id } => write!(
                f,
                "relation {}: membership entry for row {row_id} desynced from its F/U entries",
                relation.0
            ),
            Self::DispositionDesync { relation } => write!(
                f,
                "relation {}: base state disagrees with a net disposition the delta proved",
                relation.0
            ),
            Self::WrongFactWidth {
                relation,
                row_id,
                expected,
                actual,
            } => write!(
                f,
                "relation {}: row {row_id} is {actual} bytes, schema says {expected}",
                relation.0
            ),
            Self::RowCountMismatch { relation, stored } => write!(
                f,
                "relation {}: stored row count {stored} desynced from the facts",
                relation.0
            ),
            Self::CounterDesync {
                relation,
                claimed,
                witness,
            } => write!(
                f,
                "relation {}: stored row count {claimed} exceeds the store's {witness}-entry witness",
                relation.0
            ),
            Self::MalformedValue(kind) => write!(f, "malformed stored value: {kind}"),
            Self::NonUtf8Intern(id) => write!(f, "intern id {id}: stored bytes are not UTF-8"),
            Self::InternTagMismatch(id) => {
                write!(
                    f,
                    "intern id {id}: reverse-entry tag disagrees with the field type"
                )
            }
        }
    }
}

impl fmt::Display for SchemaError {
    #[allow(clippy::too_many_lines)] // a rendering table: one arm per variant
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Short bindings: r = relation, fd = field.
        match self {
            Self::DuplicateRelationName { name } => write!(f, "duplicate relation name `{name}`"),
            Self::DuplicateFieldName { relation: r, name } => {
                write!(f, "relation {}: duplicate field name `{name}`", r.0)
            }
            Self::EnumWithoutVariants {
                relation: r,
                field: fd,
            } => {
                write!(f, "relation {}, field {}: enum with no variants", r.0, fd.0)
            }
            Self::EnumTooManyVariants {
                relation: r,
                field: fd,
                count,
            } => write!(
                f,
                "relation {}, field {}: {count} enum variants exceed the u8 ordinal",
                r.0, fd.0
            ),
            Self::DuplicateEnumVariant {
                relation: r,
                field: fd,
                variant,
            } => write!(
                f,
                "relation {}, field {}: duplicate enum variant `{variant}`",
                r.0, fd.0
            ),
            Self::FreshOnNonU64 {
                relation: r,
                field: fd,
            } => {
                write!(f, "relation {}, field {}: fresh requires u64", r.0, fd.0)
            }
            Self::StatementUnknownRelation {
                statement: s,
                relation: r,
            } => write!(f, "statement {}: unknown relation {}", s.0, r.0),
            Self::StatementUnknownField {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: relation {} has no field {}",
                s.0, r.0, fd.0
            ),
            Self::EmptyProjection {
                statement: s,
                relation: r,
            } => write!(f, "statement {}: empty projection on relation {}", s.0, r.0),
            Self::DuplicateProjectionField {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: field {} projected twice on relation {}",
                s.0, fd.0, r.0
            ),
            Self::DuplicateSelectionField {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: field {} selected twice on relation {}",
                s.0, fd.0, r.0
            ),
            Self::FunctionalityMultipleIntervals {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: second interval field {} on relation {} — the ordered guard answers one dimension",
                s.0, fd.0, r.0
            ),
            Self::FunctionalityIntervalNotLast {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: interval field {} on relation {} must be the final projection position",
                s.0, fd.0, r.0
            ),
            Self::DuplicateFunctionality {
                statement: s,
                earlier,
            } => write!(
                f,
                "statement {}: statement {} already keys this field set",
                s.0, earlier.0
            ),
            Self::GuardKeyTooWide { statement: s, width } => write!(
                f,
                "statement {}: {width}-byte guard key exceeds the key-size ceiling",
                s.0
            ),
            Self::ContainmentArityMismatch {
                statement: s,
                source,
                target,
            } => write!(
                f,
                "statement {}: {source} source positions against {target} target positions",
                s.0
            ),
            Self::ContainmentTypeMismatch {
                statement: s,
                position,
            } => write!(
                f,
                "statement {}: structural type mismatch at position {position}",
                s.0
            ),
            Self::SelectedFieldProjected {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: field {} on relation {} is both selected and projected",
                s.0, fd.0, r.0
            ),
            Self::SelectionLiteralTypeMismatch {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: selection literal type mismatch at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::SelectionEnumOrdinalOutOfRange {
                statement: s,
                relation: r,
                field: fd,
                ordinal,
            } => write!(
                f,
                "statement {}: enum ordinal {ordinal} out of range at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::SelectionLiteralNotUtf8 {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: string literal is not UTF-8 at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::SelectionIntervalEmpty {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: interval literal start >= end at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::NoMatchingTargetKey {
                statement: s,
                relation: r,
            } => write!(
                f,
                "statement {}: target projection matches no key of relation {}",
                s.0, r.0
            ),
            Self::NoPointwiseTargetKey {
                statement: s,
                relation: r,
            } => write!(
                f,
                "statement {}: no pointwise key of relation {} carries the interval position",
                s.0, r.0
            ),
            Self::DuplicateStatement {
                statement: s,
                earlier,
            } => write!(
                f,
                "statement {}: duplicates statement {} — write it once",
                s.0, earlier.0
            ),
        }
    }
}

impl fmt::Display for ValidationError {
    #[allow(clippy::too_many_lines)] // a rendering table: one arm per variant
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRuleSet => write!(f, "the rule set is empty — the empty union is no query"),
            Self::TooManyRules { count } => {
                write!(f, "{count} rules exceed the rule cap")
            }
            Self::DnfExceedsRules { produced, cap } => write!(
                f,
                "DNF distribution produces {produced} rules against the cap of {cap}"
            ),
            Self::HeadArityMismatch {
                rule,
                expected,
                found,
            } => write!(
                f,
                "rule {rule}: {found} find terms against a head of arity {expected}"
            ),
            Self::HeadTypeMismatch { rule, position } => write!(
                f,
                "rule {rule}: find term {position} disagrees with the head's positional type"
            ),
            Self::HeadAggregateMismatch { rule, position } => write!(
                f,
                "rule {rule}: find term {position} disagrees with the head's shape at that position"
            ),
            Self::ArgAcrossRules { rules } => write!(
                f,
                "Arg-restriction over a {rules}-rule program: the restriction key is \
                 rule-scoped and the union's extreme is undefined — write one Arg query \
                 per disjunct and merge in the host"
            ),
            Self::UnknownRelation { atom, relation } => {
                write!(f, "atom {atom}: unknown relation {}", relation.0)
            }
            Self::UnknownField { atom, field } => {
                write!(f, "atom {atom}: unknown field {}", field.0)
            }
            Self::DuplicateFieldBinding { atom, field } => {
                write!(f, "atom {atom}: field {} bound twice", field.0)
            }
            Self::VariableTypeConflict { var } => {
                write!(f, "variable {} bound at conflicting types", var.0)
            }
            Self::LiteralTypeMismatch { atom, field } => {
                write!(f, "atom {atom}: literal type mismatch at field {}", field.0)
            }
            Self::EnumOrdinalOutOfRange {
                atom,
                field,
                ordinal,
            } => write!(
                f,
                "atom {atom}: enum ordinal {ordinal} out of range at field {}",
                field.0
            ),
            Self::EmptyIntervalLiteral { atom, field } => write!(
                f,
                "atom {atom}: interval literal start >= end at field {}",
                field.0
            ),
            Self::PointLiteralAtCeiling { atom, field } => write!(
                f,
                "atom {atom}: point literal at the domain ceiling at field {} — \
                 points are MIN..=MAX-1; MAX is the ray's \u{221e}",
                field.0
            ),
            Self::ParamIdGap { param } => {
                write!(f, "parameter ids are not dense: {} is unused", param.0)
            }
            Self::ParamTypeConflict { param } => {
                write!(f, "parameter {} anchored at conflicting types", param.0)
            }
            Self::ParamScalarAndSet { param } => {
                write!(
                    f,
                    "parameter {} used both as a scalar and as a set",
                    param.0
                )
            }
            Self::ParamSetComparison { index } => {
                write!(f, "comparison {index}: a param set is legal only under Eq")
            }
            Self::IntervalParamSet { param } => write!(
                f,
                "parameter {}: param sets hold points, not intervals",
                param.0
            ),
            Self::IllegalComparison { index } => {
                write!(f, "comparison {index}: type rules violated")
            }
            Self::OrderComparisonOnInterval { index } => write!(
                f,
                "comparison {index}: order operator on an interval — intervals are unordered"
            ),
            Self::ConstantComparison { index } => {
                write!(f, "comparison {index}: neither side is a variable")
            }
            Self::SelfComparison { index } => {
                write!(f, "comparison {index}: a variable compared with itself")
            }
            Self::ComparisonEnumOrdinalOutOfRange { index, ordinal } => {
                write!(f, "comparison {index}: enum ordinal {ordinal} out of range")
            }
            Self::ComparisonEmptyIntervalLiteral { index } => {
                write!(f, "comparison {index}: interval literal start >= end")
            }
            Self::ComparisonPointLiteralAtCeiling { index } => write!(
                f,
                "comparison {index}: point literal at the domain ceiling — \
                 points are MIN..=MAX-1; MAX is the ray's \u{221e}"
            ),
            Self::EmptyAllenMask { index } => write!(
                f,
                "comparison {index}: empty Allen mask — no basic relation can hold; \
                 write no query"
            ),
            Self::FullAllenMask { index } => write!(
                f,
                "comparison {index}: full Allen mask — every pair satisfies it; \
                 write no predicate"
            ),
            Self::MembershipOnlyVariable { var } => write!(
                f,
                "variable {} is bound only by membership — no enumerable domain",
                var.0
            ),
            Self::NegatedVariableUnbound { var } => write!(
                f,
                "variable {} occurs in a negated atom but in no positive atom",
                var.0
            ),
            Self::UnboundFindVariable { var } => {
                write!(f, "find variable {} bound by no positive atom", var.0)
            }
            Self::ComparisonOnlyVariable { var } => {
                write!(f, "variable {} appears only in comparisons", var.0)
            }
            Self::EmptyFinds => write!(f, "the find list is empty"),
            Self::DuplicateFindTerm { index } => write!(f, "find term {index} is a duplicate"),
            Self::NoPositiveAtoms => write!(f, "the query has no positive atoms"),
            Self::AggregateInputType { find } => {
                write!(f, "find {find}: aggregate over a non-integer variable")
            }
            Self::CountWithVariable { find } => {
                write!(f, "find {find}: Count is nullary")
            }
            Self::AggregateWithoutVariable { find } => {
                write!(f, "find {find}: this aggregate requires a variable")
            }
            Self::AggregateOverGroupKey { find } => {
                write!(f, "find {find}: aggregate over a group-key variable")
            }
            Self::MixedArgAndFold { find } => {
                write!(f, "find {find}: Arg terms and fold aggregates may not mix")
            }
            Self::ArgKeyMismatch { find } => write!(
                f,
                "find {find}: Arg terms must share one key variable and one direction"
            ),
            Self::NonOrderableArgKey { find } => {
                write!(f, "find {find}: the Arg key must be U64 or I64")
            }
            Self::TooManyAtoms { count } => {
                write!(f, "{count} atom occurrences exceed the planner cap")
            }
            Self::TooManyVariables { count } => {
                write!(f, "{count} distinct variables exceed the 128-bit bitset")
            }
        }
    }
}

impl fmt::Display for Error {
    #[allow(clippy::too_many_lines)] // a rendering table: one arm per variant
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FormatMismatch { found, expected } => {
                write!(
                    f,
                    "storage format version {found}, this build expects {expected}"
                )
            }
            Self::SchemaMismatch { .. } => {
                write!(f, "the compiled schema's fingerprint is not the stored one")
            }
            Self::AlreadyInitialized => {
                write!(
                    f,
                    "the directory already holds an LMDB environment; open it instead"
                )
            }
            Self::EnvironmentLocked => {
                write!(f, "another live handle holds this environment's lock")
            }
            Self::Io(err) => write!(f, "io: {err}"),
            Self::Lmdb(err) => write!(f, "lmdb: {err}"),
            Self::ReadersFull { max_readers } => {
                write!(f, "all {max_readers} reader slots hold open snapshots")
            }
            Self::Schema(err) => write!(f, "schema declaration: {err}"),
            Self::Validation(err) => write!(f, "query validation: {err}"),
            Self::FactShape(err) => write!(f, "dynamic fact: {err}"),
            Self::FunctionalityViolation { statement, .. } => write!(
                f,
                "statement {}: functionality violated — two live facts claim one key",
                statement.0
            ),
            Self::ContainmentViolation {
                statement,
                direction,
                ..
            } => match direction {
                Direction::SourceUnsatisfied => write!(
                    f,
                    "statement {}: containment violated — an inserted source fact has no target",
                    statement.0
                ),
                Direction::TargetRequired => write!(
                    f,
                    "statement {}: containment violated — a deleted target key is still required",
                    statement.0
                ),
            },
            Self::FreshExhausted { relation, field } => write!(
                f,
                "fresh sequence exhausted (relation {}, field {})",
                relation.0, field.0
            ),
            Self::CommitSync { retries, error } => write!(
                f,
                "commit durability boundary (page pwrite / F_FULLFSYNC) failed after {retries} retries: {error}"
            ),
            Self::ForeignPreparedQuery => {
                write!(
                    f,
                    "a prepared query executes only against snapshots of the database that prepared it"
                )
            }
            Self::ParamCountMismatch { expected, supplied } => {
                write!(
                    f,
                    "{supplied} parameters supplied, the query takes {expected}"
                )
            }
            Self::ParamTypeMismatch { param, expected } => {
                write!(f, "parameter {}: expected {expected:?}", param.0)
            }
            Self::ParamSetExpected { param } => write!(
                f,
                "parameter {}: the query binds a set — supply a slice",
                param.0
            ),
            Self::ParamScalarExpected { param } => write!(
                f,
                "parameter {}: the query binds a scalar — a set was supplied",
                param.0
            ),
            Self::ParamElementTypeMismatch {
                param,
                element,
                expected,
            } => write!(
                f,
                "parameter {}, element {element}: expected {expected:?}",
                param.0
            ),
            Self::PointParamAtCeiling { param } => write!(
                f,
                "parameter {}: point value at the domain ceiling — \
                 points are MIN..=MAX-1; MAX is the ray's \u{221e}",
                param.0
            ),
            Self::AllenMaskParamExpected { param } => write!(
                f,
                "parameter {}: expected an Allen mask (BindValue::AllenMask)",
                param.0
            ),
            Self::EmptyAllenMaskParam { param } => write!(
                f,
                "parameter {}: empty Allen mask — no basic relation can hold; \
                 write no query",
                param.0
            ),
            Self::FullAllenMaskParam { param } => write!(
                f,
                "parameter {}: full Allen mask — every pair satisfies it; \
                 write no predicate",
                param.0
            ),
            Self::Overflow(super::OverflowKind::Aggregate { find }) => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::Overflow(super::OverflowKind::Origins) => {
                write!(
                    f,
                    "origin mint space exhausted: more than 2^32 absorb-node survivors in one execution"
                )
            }
            Self::BulkLoad { committed, error } => {
                write!(
                    f,
                    "bulk load failed after {committed} committed facts: {error}"
                )
            }
            Self::ResultBytesOverflow => {
                write!(
                    f,
                    "the result buffer's byte heap exceeds u32 offsets (4 GiB)"
                )
            }
            Self::Corruption(err) => write!(f, "corruption: {err}"),
        }
    }
}

impl Error {
    /// Pairs the error with the schema it speaks about: the violation
    /// variants (`FunctionalityViolation`/`ContainmentViolation`) `Display`
    /// with their statement rendered back in the `schema!` algebra
    /// notation; every other variant renders as its plain `Display`.
    /// Formatting allocates — `Display` is never the hot path; the error
    /// payload itself stays ids and fact bytes.
    #[must_use]
    pub fn display_with<'a>(&'a self, schema: &'a Schema) -> impl fmt::Display + 'a {
        DisplayWith {
            error: self,
            schema,
        }
    }
}

/// [`Error::display_with`]'s adapter.
struct DisplayWith<'a> {
    error: &'a Error,
    schema: &'a Schema,
}

impl fmt::Display for DisplayWith<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.error {
            Error::FunctionalityViolation { statement, .. } => write!(
                f,
                "functionality violated: `{}` — two live facts claim one key",
                render::render(self.schema, *statement)
            ),
            Error::ContainmentViolation {
                statement,
                direction,
                ..
            } => {
                let rendered = render::render(self.schema, *statement);
                match direction {
                    Direction::SourceUnsatisfied => write!(
                        f,
                        "containment violated (source side): `{rendered}` — an inserted source fact has no target"
                    ),
                    Direction::TargetRequired => write!(
                        f,
                        "containment violated (target side): `{rendered}` — a deleted target key is still required"
                    ),
                }
            }
            other => write!(f, "{other}"),
        }
    }
}

impl SchemaError {
    /// The offending statement, for the variants that carry one.
    fn statement(&self) -> Option<StatementId> {
        match self {
            Self::DuplicateRelationName { .. }
            | Self::DuplicateFieldName { .. }
            | Self::EnumWithoutVariants { .. }
            | Self::EnumTooManyVariants { .. }
            | Self::DuplicateEnumVariant { .. }
            | Self::FreshOnNonU64 { .. } => None,
            Self::StatementUnknownRelation { statement, .. }
            | Self::StatementUnknownField { statement, .. }
            | Self::EmptyProjection { statement, .. }
            | Self::DuplicateProjectionField { statement, .. }
            | Self::DuplicateSelectionField { statement, .. }
            | Self::FunctionalityMultipleIntervals { statement, .. }
            | Self::FunctionalityIntervalNotLast { statement, .. }
            | Self::DuplicateFunctionality { statement, .. }
            | Self::GuardKeyTooWide { statement, .. }
            | Self::ContainmentArityMismatch { statement, .. }
            | Self::ContainmentTypeMismatch { statement, .. }
            | Self::SelectedFieldProjected { statement, .. }
            | Self::SelectionLiteralTypeMismatch { statement, .. }
            | Self::SelectionEnumOrdinalOutOfRange { statement, .. }
            | Self::SelectionLiteralNotUtf8 { statement, .. }
            | Self::SelectionIntervalEmpty { statement, .. }
            | Self::NoMatchingTargetKey { statement, .. }
            | Self::NoPointwiseTargetKey { statement, .. }
            | Self::DuplicateStatement { statement, .. } => Some(*statement),
        }
    }

    /// Pairs the rejection with the declaration it judged: statement
    /// variants `Display` with the offending statement rendered back in
    /// the `schema!` algebra notation (a rejected declaration never seals
    /// a [`Schema`], so diagnostics render from the descriptor).
    #[must_use]
    pub fn display_with<'a>(&'a self, descriptor: &'a SchemaDescriptor) -> impl fmt::Display + 'a {
        SchemaDisplayWith {
            error: self,
            descriptor,
        }
    }
}

/// [`SchemaError::display_with`]'s adapter.
struct SchemaDisplayWith<'a> {
    error: &'a SchemaError,
    descriptor: &'a SchemaDescriptor,
}

impl fmt::Display for SchemaDisplayWith<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.error.statement() {
            Some(statement) => write!(
                f,
                "{} — in `{}`",
                self.error,
                render::render_declared(self.descriptor, statement)
            ),
            None => write!(f, "{}", self.error),
        }
    }
}
