//! `Display` rendering for every error type — formatting runs lazily, only
//! when the host actually prints.
//!
//! Statements are anonymous (`docs/architecture/30-dependencies.md`), so
//! the plain `Display` impls cite them by id; the [`Error::display_with`]
//! and [`SchemaError::display_with`] adapters pair the error with the
//! schema it speaks about and render the statement back in the `schema!`
//! algebra notation (`crate::schema::render`).

use std::fmt;

use crate::schema::{Schema, render};
use bumbledb_theory::schema::{SchemaDescriptor, StatementId};

use super::{
    CorruptionError, Direction, Error, FactShapeError, SchemaError, TargetKeyCandidate,
    ValidationError, Violation,
};

fn field_set(
    f: &mut fmt::Formatter<'_>,
    projection: &[bumbledb_theory::schema::FieldId],
) -> fmt::Result {
    let mut fields = projection.to_vec();
    fields.sort_unstable();
    write!(f, "{{")?;
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}", field.0)?;
    }
    write!(f, "}}")
}

fn target_key_rejection(
    f: &mut fmt::Formatter<'_>,
    statement: bumbledb_theory::schema::StatementId,
    target: bumbledb_theory::schema::RelationId,
    projection: &[bumbledb_theory::schema::FieldId],
    available: &[TargetKeyCandidate],
    pointwise: bool,
) -> fmt::Result {
    write!(
        f,
        "statement {}: target relation {} projection ",
        statement.0, target.0
    )?;
    field_set(f, projection)?;
    write!(f, " matches no declared key; available keys: ")?;
    if available.is_empty() {
        write!(f, "none")?;
    } else {
        for (index, candidate) in available.iter().enumerate() {
            if index > 0 {
                write!(f, "; ")?;
            }
            write!(f, "key {} ", candidate.key.0)?;
            field_set(f, &candidate.projection)?;
        }
    }
    if pointwise {
        write!(
            f,
            "; hint: declare the exact pointwise key `R(prefix…, interval) -> R`"
        )?;
    }
    Ok(())
}

/// The violation message's shared parts — their ONE home: both
/// renderers — the plain `Display` below, which cites the statement by
/// id, and [`Error::display_with`], which cites the rendered `schema!`
/// notation — compose these three accessors, so no message body exists
/// twice (the former tandem-edit coupling between the two renderers).
impl Violation {
    /// The violated law's name.
    fn law(&self) -> &'static str {
        match self {
            Self::Functionality { .. } => "functionality",
            Self::Containment { .. } => "containment",
            Self::Cardinality { .. } => "cardinality",
        }
    }

    /// The side parenthetical `display_with` cites (empty for the
    /// undirected laws; the plain renderer's tail already names the
    /// side's meaning).
    fn side(&self) -> &'static str {
        match self {
            Self::Containment {
                direction: Direction::SourceUnsatisfied,
                ..
            } => " (source side)",
            Self::Containment {
                direction: Direction::TargetRequired,
                ..
            } => " (target side)",
            Self::Functionality { .. } | Self::Cardinality { .. } => "",
        }
    }

    /// The factual tail after the em-dash: what happened.
    fn tail(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Functionality { .. } => write!(f, "two live facts claim one key"),
            Self::Containment {
                direction: Direction::SourceUnsatisfied,
                ..
            } => write!(f, "an inserted source fact has no target"),
            Self::Containment {
                direction: Direction::TargetRequired,
                ..
            } => write!(f, "a deleted target key is still required"),
            Self::Cardinality { count, .. } => write!(
                f,
                "a parent's child-group count ({count}) falls outside the window"
            ),
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "statement {}: {} violated — ",
            self.statement().0,
            self.law()
        )?;
        self.tail(f)
    }
}

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
            Self::InvalidUtf8 { relation, field } => write!(
                f,
                "relation {}, field {}: string bytes are not UTF-8",
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
            Self::InvalidInterval(bytes) => {
                write!(f, "interval bytes {bytes:02x?}: start >= end")
            }
            Self::InvalidFixedIntervalStart(bytes) => write!(
                f,
                "fixed-width interval start {bytes:02x?}: start + w at or past the domain ceiling"
            ),
            Self::MetaMissing => write!(f, "the _meta database is absent or malformed"),
            Self::StoreKindInvalid => write!(
                f,
                "the _meta store-kind marker is present but not a valid kind encoding"
            ),
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
            Self::NonzeroFixedBytesPad(tail) => write!(
                f,
                "bytes<N> trailing word {tail:02x?}: nonzero pad byte — the pad is encoding, not data"
            ),
            Self::DescriptorFingerprintDesync { .. } => write!(
                f,
                "the persisted schema descriptor hashes to something other than the stored fingerprint"
            ),
        }
    }
}

impl fmt::Display for SchemaError {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // a rendering table: one arm per variant
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Short bindings: r = relation, fd = field.
        match self {
            Self::DuplicateRelationName { name } => write!(f, "duplicate relation name `{name}`"),
            Self::DuplicateFieldName { relation: r, name } => {
                write!(f, "relation {}: duplicate field name `{name}`", r.0)
            }
            Self::FreshOnNonU64 {
                relation: r,
                field: fd,
            } => {
                write!(f, "relation {}, field {}: fresh requires u64", r.0, fd.0)
            }
            Self::FixedBytesWidthOutOfRange {
                relation: r,
                field: fd,
                len,
            } => write!(
                f,
                "relation {}, field {}: bytes<{len}> outside the 1..=64 width range",
                r.0, fd.0
            ),
            Self::IntervalWidthOutOfRange {
                relation: r,
                field: fd,
                width,
            } => write!(
                f,
                "relation {}, field {}: interval<E, {width}> — the width must be \
                 1..=u64::MAX-1 (zero points denote nothing; u64::MAX leaves no \
                 start under the Q2 bound)",
                r.0, fd.0
            ),
            Self::RelationTooManyColumns {
                relation: r,
                columns,
            } => write!(
                f,
                "relation {}: {columns} derived columns exceed the 65,535-column \
                 image cap (an interval field spans two columns, bytes<N> its ⌈N/8⌉)",
                r.0
            ),
            Self::TooManyStatements { count } => write!(
                f,
                "{count} materialized statements exceed the 65,536-statement id space"
            ),
            Self::EmptyExtension { relation: r } => write!(
                f,
                "relation {}: a closed relation with no rows is a vocabulary of nothing — write no relation",
                r.0
            ),
            Self::ExtensionTooManyRows { relation: r, count } => write!(
                f,
                "relation {}: {count} ground axioms exceed the 256-row extension cap",
                r.0
            ),
            Self::DuplicateExtensionHandle {
                relation: r,
                handle,
            } => write!(f, "relation {}: duplicate handle `{handle}`", r.0),
            Self::ExtensionArityMismatch {
                relation: r,
                row,
                expected,
                supplied,
            } => write!(
                f,
                "relation {}, row {row}: {supplied} values for {expected} columns",
                r.0
            ),
            Self::ExtensionValueTypeMismatch {
                relation: r,
                row,
                field: fd,
            } => write!(
                f,
                "relation {}, row {row}: value type mismatch at field {}",
                r.0, fd.0
            ),
            Self::ExtensionIntervalRay {
                relation: r,
                row,
                field: fd,
            } => write!(
                f,
                "relation {}, row {row}: ray axiom at field {} — a still-running span is policy, not an intrinsic property",
                r.0, fd.0
            ),
            Self::StrOnClosedRelation {
                relation: r,
                field: fd,
            } => write!(
                f,
                "relation {}, field {}: str on a closed relation — the handle is the label",
                r.0, fd.0
            ),
            Self::FreshOnClosedRelation {
                relation: r,
                field: fd,
            } => write!(
                f,
                "relation {}, field {}: fresh on a closed relation — identity is the handle",
                r.0, fd.0
            ),
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
                "statement {}: second interval field {} on relation {} — the ordered determinant answers one dimension",
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
            Self::DeterminantKeyTooWide {
                statement: s,
                width,
            } => write!(
                f,
                "statement {}: {width}-byte determinant key exceeds the key-size ceiling",
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
            Self::SelectionLiteralNotUtf8 {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: string literal is not UTF-8 at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::NoMatchingTargetKey {
                statement: s,
                target,
                projection,
                available,
            } => target_key_rejection(f, *s, *target, projection, available, false),
            Self::NoPointwiseTargetKey {
                statement: s,
                target,
                projection,
                available,
            } => target_key_rejection(f, *s, *target, projection, available, true),
            Self::ClosedContainmentInterval {
                statement: s,
                relation: r,
            } => write!(
                f,
                "statement {}: interval position on a containment with closed relation {} — \
                 pointwise judgments against a virtual extension are refused",
                s.0, r.0
            ),
            Self::ClosedStatementRefuted {
                statement: s,
                relation: r,
                row,
            } => write!(
                f,
                "statement {}: refuted by ground axiom {} of closed relation {} — \
                 a theory whose axioms refute its own statement has no model",
                s.0, row, r.0
            ),
            Self::DuplicateStatement {
                statement: s,
                earlier,
            } => write!(
                f,
                "statement {}: duplicates statement {} — write it once",
                s.0, earlier.0
            ),
            Self::DegenerateSelectionSet {
                statement: s,
                relation: r,
                field: fd,
                len,
            } => write!(
                f,
                "statement {}: literal set of {len} on relation {}, field {} — a set binding \
                 carries at least two literals (one literal is the equality spelling)",
                s.0, r.0, fd.0
            ),
            Self::DuplicateSelectionLiteral {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: duplicate literal in the set binding on relation {}, field {} — \
                 write it once",
                s.0, r.0, fd.0
            ),
            Self::CardinalityInvertedWindow {
                statement: s,
                lo,
                hi,
            } => write!(
                f,
                "statement {}: the window {lo}..{hi} is inverted — no count satisfies \
                 hi < lo; the canonical bounds are lo < hi ({{lo..hi}}), an exact count \
                 lo = hi (the {{n}} spelling)",
                s.0
            ),
            Self::CardinalityVacuousWindow { statement: s } => write!(
                f,
                "statement {}: the 0..* window admits every count — it provably says \
                 nothing (lean/Bumbledb/Cardinality.lean: cardinality_zero_star); delete \
                 the statement",
                s.0
            ),
            Self::CardinalityContainmentWindow { statement: s } => write!(
                f,
                "statement {}: the 1..* window says only what the bare containment says — \
                 drop the annotation and declare `target <= source`",
                s.0
            ),
            Self::CardinalityIntervalPosition {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: interval field {} on relation {} in a cardinality window — \
                 a window counts facts per parent, and an interval position would make the \
                 count ambiguous between facts and points",
                s.0, fd.0, r.0
            ),
        }
    }
}

impl fmt::Display for ValidationError {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // a rendering table: one arm per variant
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
            Self::ConditionNestingTooDeep { rule, depth, cap } => write!(
                f,
                "rule {rule}: condition trees nest {depth} deep against the cap of {cap}"
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
            Self::OrderComparisonOnFixedBytes { index } => write!(
                f,
                "comparison {index}: order operator on bytes<N> — a digest's \
                 lexicographic order is an encoding artifact; identity only"
            ),
            Self::OrderComparisonOnString { index } => write!(
                f,
                "comparison {index}: order operator on String — strings are equality-only"
            ),
            Self::OrderComparisonOnBool { index } => write!(
                f,
                "comparison {index}: order operator on Bool — booleans are equality-only"
            ),
            Self::ConstantComparison { index } => {
                write!(f, "comparison {index}: neither side is a variable")
            }
            Self::SelfComparison { index } => {
                write!(f, "comparison {index}: a variable compared with itself")
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
                 write no condition"
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
            Self::MultiplePackTerms { find } => {
                write!(f, "find {find}: at most one Pack term per head")
            }
            Self::MixedPackAndFold { find } => {
                write!(f, "find {find}: Pack and fold aggregates may not mix")
            }
            Self::MixedPackAndArg { find } => {
                write!(f, "find {find}: Pack and Arg terms may not mix")
            }
            Self::PackInputType { find } => {
                write!(f, "find {find}: Pack folds an interval variable only")
            }
            Self::DurationInBinding { atom, field } => write!(
                f,
                "atom {atom}, field {}: Duration is a computation, not a bindable value",
                field.0
            ),
            Self::DurationOverNonInterval { var } => {
                write!(
                    f,
                    "Duration over variable {}, which is not an interval",
                    var.0
                )
            }
            Self::DurationAggregateOp { find } => {
                write!(f, "find {find}: Duration aggregates are Sum/Min/Max only")
            }
            Self::DurationComparisonOperator { index } => write!(
                f,
                "comparison {index}: Duration compares under order operators only"
            ),
            Self::DurationBothSides { index } => write!(
                f,
                "comparison {index}: Duration on both sides — one measure side \
                 against a u64 term or literal"
            ),
            Self::TooManyAtoms { count } => {
                write!(f, "{count} atom occurrences exceed the planner cap")
            }
            Self::TooManyVariables { count } => {
                write!(f, "{count} distinct variables exceed the 128-bit bitset")
            }
            Self::TooManyPredicates { count } => {
                write!(f, "{count} predicates exceed the program cap")
            }
            Self::UnknownOutputPredicate { pred } => {
                write!(f, "output predicate p{} is not in the program", pred.0)
            }
            Self::UnknownPredicate { atom, pred } => {
                write!(
                    f,
                    "atom {atom}: predicate p{} is not in the program",
                    pred.0
                )
            }
            Self::PredicateColumnOutOfRange { atom, field } => write!(
                f,
                "atom {atom}: head position {} is beyond the target predicate's arity",
                field.0
            ),
            Self::NegationThroughCycle { pred, via } => write!(
                f,
                "predicate p{} negates p{} inside its own recursive component — \
                 negation reads finished lower strata only",
                pred.0, via.0
            ),
            Self::AggregationThroughCycle { pred, via } => write!(
                f,
                "predicate p{}'s fold reads p{} inside its own recursive component — \
                 aggregation reads finished lower strata only",
                pred.0, via.0
            ),
            Self::MeasureInRecursiveHead { pred } => write!(
                f,
                "recursive predicate p{} projects a Duration — recursive heads \
                 project bound variables only",
                pred.0
            ),
            Self::UnresolvedPredicateSignature { pred } => write!(
                f,
                "predicate p{}'s signature never resolves — every rule's types \
                 depend on its own recursive component",
                pred.0
            ),
            Self::AggregateInteriorPredicate { pred } => write!(
                f,
                "predicate p{} folds below the output — a fold's answers \
                 materialize only at the output predicate's finalize; \
                 aggregate over the finished predicate from the output \
                 instead",
                pred.0
            ),
            Self::MeasureInteriorPredicate { pred } => write!(
                f,
                "predicate p{} projects a Duration below the output — the \
                 executable program class keeps interior heads to bound \
                 variables; project the interval and measure it at the \
                 output instead",
                pred.0
            ),
        }
    }
}

impl fmt::Display for Error {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // a rendering table: one arm per variant
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
            Self::StoreKindMismatch { found, expected } => {
                write!(
                    f,
                    "the store on disk is {found}, this constructor opens {expected} stores"
                )
            }
            Self::DescriptorMissing => {
                write!(
                    f,
                    "the store carries no schema descriptor (not yet adopted): \
                     open it once under its creating schema and the descriptor back-fills"
                )
            }
            Self::Io(err) => write!(f, "io: {err}"),
            Self::Lmdb(err) => write!(f, "lmdb: {err}"),
            Self::ReadersFull { max_readers } => {
                write!(f, "all {max_readers} reader slots hold open snapshots")
            }
            Self::Schema(err) => write!(f, "schema declaration: {err}"),
            Self::Validation(err) => write!(f, "query validation: {err}"),
            Self::FactShape(err) => write!(f, "dynamic fact: {err}"),
            Self::CommitRejected { violations } => {
                // Compiler-style: every violated statement, in
                // materialized statement order — the complete set.
                write!(f, "commit rejected: ")?;
                for (index, violation) in violations.as_slice().iter().enumerate() {
                    if index > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{violation}")?;
                }
                Ok(())
            }
            Self::FreshExhausted { relation, field } => write!(
                f,
                "fresh sequence exhausted (relation {}, field {})",
                relation.0, field.0
            ),
            Self::ClosedRelationWrite { relation } => write!(
                f,
                "relation {}: closed — its rows are ground axioms; changing them is a new theory",
                relation.0
            ),
            Self::GenerationMoved { witnessed, current } => write!(
                f,
                "the witnessed generation moved ({witnessed} \u{2192} {current}): \
                 a state-changing commit landed after the witness snapshot — \
                 re-run the query, re-compute, write_from again"
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
            Self::ForeignSnapshot => {
                write!(
                    f,
                    "a witness snapshot proves nothing about another database — \
                     write_from takes snapshots of the database being written"
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
                 write no condition",
                param.0
            ),
            Self::MeasureOfRay { start, end } => write!(
                f,
                "Duration of a ray: encoded interval [{start}, {end}) has no finite \
                 measure — exclude rays with an Allen predicate or a bounded-end filter"
            ),
            Self::FixpointBudgetExceeded {
                stratum,
                rounds,
                tuples,
            } => write!(
                f,
                "fixpoint budget exceeded: stratum {stratum} ran {rounds} rounds and \
                 derived {tuples} tuples — raise the budget \
                 (PreparedQuery::set_fixpoint_budget) or bound the closure"
            ),
            Self::Overflow(super::OverflowKind::Aggregate { find }) => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::Overflow(super::OverflowKind::OriginCapacity) => {
                write!(f, "origin capacity exceeded")
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
    /// Pairs the error with the schema it speaks about: a rejected
    /// commit (`CommitRejected`) `Display`s its complete violation set
    /// with every cited statement rendered back in the `schema!` algebra
    /// notation, in materialized statement order; every other variant
    /// renders as its plain `Display`. Formatting allocates — `Display`
    /// is never the hot path; the error payload itself stays ids and
    /// fact bytes.
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
            Error::CommitRejected { violations } => {
                write!(f, "commit rejected: ")?;
                for (index, violation) in violations.as_slice().iter().enumerate() {
                    if index > 0 {
                        write!(f, "; ")?;
                    }
                    let rendered = render::render(self.schema, violation.statement());
                    write!(
                        f,
                        "{} violated{}: `{rendered}` — ",
                        violation.law(),
                        violation.side()
                    )?;
                    violation.tail(f)?;
                }
                Ok(())
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
            | Self::FixedBytesWidthOutOfRange { .. }
            | Self::IntervalWidthOutOfRange { .. }
            | Self::FreshOnNonU64 { .. }
            | Self::RelationTooManyColumns { .. }
            | Self::TooManyStatements { .. }
            | Self::EmptyExtension { .. }
            | Self::ExtensionTooManyRows { .. }
            | Self::DuplicateExtensionHandle { .. }
            | Self::ExtensionArityMismatch { .. }
            | Self::ExtensionValueTypeMismatch { .. }
            | Self::ExtensionIntervalRay { .. }
            | Self::StrOnClosedRelation { .. }
            | Self::FreshOnClosedRelation { .. } => None,
            Self::StatementUnknownRelation { statement, .. }
            | Self::StatementUnknownField { statement, .. }
            | Self::EmptyProjection { statement, .. }
            | Self::DuplicateProjectionField { statement, .. }
            | Self::DuplicateSelectionField { statement, .. }
            | Self::FunctionalityMultipleIntervals { statement, .. }
            | Self::FunctionalityIntervalNotLast { statement, .. }
            | Self::DuplicateFunctionality { statement, .. }
            | Self::DeterminantKeyTooWide { statement, .. }
            | Self::ContainmentArityMismatch { statement, .. }
            | Self::ContainmentTypeMismatch { statement, .. }
            | Self::SelectedFieldProjected { statement, .. }
            | Self::SelectionLiteralTypeMismatch { statement, .. }
            | Self::SelectionLiteralNotUtf8 { statement, .. }
            | Self::NoMatchingTargetKey { statement, .. }
            | Self::NoPointwiseTargetKey { statement, .. }
            | Self::ClosedContainmentInterval { statement, .. }
            | Self::ClosedStatementRefuted { statement, .. }
            | Self::DuplicateStatement { statement, .. }
            | Self::DegenerateSelectionSet { statement, .. }
            | Self::DuplicateSelectionLiteral { statement, .. }
            | Self::CardinalityInvertedWindow { statement, .. }
            | Self::CardinalityVacuousWindow { statement, .. }
            | Self::CardinalityContainmentWindow { statement, .. }
            | Self::CardinalityIntervalPosition { statement, .. } => Some(*statement),
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
