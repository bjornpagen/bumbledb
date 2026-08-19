//! `Display` rendering for every error type — formatting runs lazily, only
//! when the host actually prints.
//!
//! Statements are anonymous (`docs/architecture/30-dependencies.md`), so
//! the plain `Display` impls cite them by id; the [`Error::display_with`]
//! and [`SchemaError::display_with`] adapters pair the error with the
//! schema it speaks about and render the statement back in the `schema!`
//! algebra notation (`crate::schema::render`).

use std::fmt;

use crate::encoding::InternId;
use crate::schema::{Schema, render};
use bumbledb_theory::schema::{SchemaDescriptor, StatementId};

use super::{
    CorruptionError, Direction, DynIdError, Error, FactShapeError, IoFailure, LmdbFailure,
    SchemaError, StatementErrorKind, TargetKeyCandidate, ValidationError, Violation, Violations,
};

impl fmt::Display for IoFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.raw_os {
            Some(code) => write!(f, "{}", std::io::Error::from_raw_os_error(code)),
            None => write!(f, "{}", std::io::Error::from(self.kind)),
        }
    }
}

impl fmt::Display for LmdbFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Mdb(error) => write!(f, "{error}"),
            Self::Encoding => write!(f, "error while encoding"),
            Self::Decoding => write!(f, "error while decoding"),
            Self::EnvAlreadyOpened => f.write_str(
                "environment already open in this program; \
                 close it to be able to open it again with different options",
            ),
        }
    }
}

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
    target: bumbledb_theory::schema::RelationId,
    projection: &[bumbledb_theory::schema::FieldId],
    available: &[TargetKeyCandidate],
    pointwise: bool,
) -> fmt::Result {
    write!(f, "target relation {} projection ", target.0)?;
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
            Self::Capacity { .. } => "capacity",
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
            Self::Functionality { .. } | Self::Capacity { .. } => "",
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
            Self::Capacity { measure, .. } => write!(
                f,
                "a parent's child-group measure ({measure}) falls outside the window"
            ),
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "statement {}: {} violated — ",
            self.statement(),
            self.law()
        )?;
        self.tail(f)
    }
}

impl fmt::Display for Violations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "admission rejected: ")?;
        for (index, violation) in self.iter().enumerate() {
            if index > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{violation}")?;
        }
        Ok(())
    }
}

impl fmt::Display for DynIdError {
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

impl fmt::Display for FactShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(err) => write!(f, "{err}"),
            Self::ArityMismatch { relation, mismatch } => write!(
                f,
                "relation {}: {} values for {} fields",
                relation.0, mismatch.witnessed, mismatch.required
            ),
            Self::TypeMismatch { relation, field } => {
                write!(
                    f,
                    "relation {}, field {}: wrong value kind",
                    relation.0, field.0
                )
            }
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
            Self::MetaMissing => write!(f, "the _meta database or a required key is absent"),
            Self::StoreKindInvalid => write!(
                f,
                "the _meta store-kind marker is present but not a valid kind encoding"
            ),
            Self::DanglingInternId(id) => {
                write!(f, "intern id {} has no dictionary entry", id.raw())
            }
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
                mismatch,
            } => write!(
                f,
                "relation {}: row {row_id} is {} bytes, schema says {}",
                relation.0, mismatch.witnessed, mismatch.required
            ),
            Self::RowCountMismatch { relation, stored } => write!(
                f,
                "relation {}: stored row count {stored} desynced from the facts",
                relation.0
            ),
            Self::CounterDesync { relation, exceeded } => write!(
                f,
                "relation {}: stored row count {} exceeds the store's {}-entry witness",
                relation.0, exceeded.observed, exceeded.ceiling
            ),
            Self::MalformedValue(kind) => write!(f, "malformed stored value: {kind}"),
            Self::EphemeralDirtyArmed => write!(
                f,
                "ephemeral dirty marker armed — the store's last session never proved its sync"
            ),
            Self::DictReverseIdReuse => write!(f, "dict reverse id reuse"),
            Self::DescriptorRoundTrip => write!(f, "descriptor round trip"),
            Self::NonUtf8Intern(id) => write!(f, "intern id {id}: stored bytes are not UTF-8"),
            Self::NonzeroFixedBytesPad(tail) => write!(
                f,
                "bytes<N> trailing word {tail:02x?}: nonzero pad byte — the pad is encoding, not data"
            ),
            Self::DescriptorFingerprintDesync {
                fingerprint,
                descriptor_hash,
            } => write!(
                f,
                "the persisted schema descriptor hashes to {}, the stored fingerprint is {}",
                crate::schema::fingerprint::SchemaFingerprint(*descriptor_hash),
                crate::schema::fingerprint::SchemaFingerprint(*fingerprint)
            ),
            Self::FactWithoutMembership {
                relation, row_id, ..
            } => write!(
                f,
                "relation {}: row {row_id} has no membership entry",
                relation.0
            ),
            Self::MembershipWithoutFact {
                relation, row_id, ..
            } => write!(
                f,
                "relation {}: membership row {row_id} has no fact",
                relation.0
            ),
            Self::FactWithoutDeterminant {
                relation,
                statement,
                row_id,
                ..
            } => write!(
                f,
                "relation {}: row {row_id} has no determinant for statement {}",
                relation.0, statement.0
            ),
            Self::DeterminantWithoutFact {
                relation,
                statement,
                ..
            } => write!(
                f,
                "relation {}: determinant of statement {} has no fact",
                relation.0, statement.0
            ),
            Self::PointwiseOverlap {
                relation,
                statement,
                ..
            } => write!(
                f,
                "relation {}: pointwise overlap under statement {}",
                relation.0, statement.0
            ),
            Self::FactWithoutReverseEdge {
                statement,
                relation,
                row_id,
                ..
            } => write!(
                f,
                "relation {}: row {row_id} has no reverse edge for statement {}",
                relation.0, statement.0
            ),
            Self::ReverseEdgeWithoutFact { statement, .. } => {
                write!(
                    f,
                    "statement {}: reverse edge has no source fact",
                    statement.0
                )
            }
            Self::ReverseEdgeWeightDesync { statement, .. } => write!(
                f,
                "statement {}: reverse-edge weight slot disagrees with the live fact",
                statement.0
            ),
            Self::RowCountDesync {
                relation,
                stored,
                counted,
            } => write!(
                f,
                "relation {}: stored row count {stored} desynced from counted {counted}",
                relation.0
            ),
            Self::RowIdHighWaterLow {
                relation,
                stored,
                max_row_id,
            } => write!(
                f,
                "relation {}: stored high-water {stored} does not exceed row {max_row_id}",
                relation.0
            ),
            Self::FreshRowDesync {
                relation,
                row_id,
                fresh,
            } => write!(
                f,
                "relation {}: row {row_id} disagrees with fresh field {fresh}",
                relation.0
            ),
            Self::FreshNextValueLow {
                relation,
                field,
                stored,
                max_fresh,
            } => write!(
                f,
                "relation {}, field {}: stored next-value {stored} at or below committed {max_fresh}",
                relation.0, field.0
            ),
            Self::DictForwardDesync { intern_id, forward } => write!(
                f,
                "intern id {}: forward map holds {:?}",
                intern_id.raw(),
                forward.map(InternId::raw)
            ),
            Self::DictNextIdLow { stored, reverse_id } => write!(
                f,
                "dict next-id {} is at or below reverse id {}",
                stored.raw(),
                reverse_id.raw()
            ),
            Self::FreshRowDeterminantEntry {
                relation,
                statement,
                ..
            } => write!(
                f,
                "relation {}: fresh-row key {} has a U entry",
                relation.0, statement.0
            ),
            Self::InternBeyondNextId {
                relation,
                row_id,
                intern_id,
                next_id,
            } => write!(
                f,
                "relation {}: row {row_id} references intern {} at or beyond next-id {}",
                relation.0,
                intern_id.raw(),
                next_id.raw()
            ),
            Self::ClosedRelationEntry { relation, .. } => {
                write!(
                    f,
                    "relation {}: stored entry names a closed relation",
                    relation.0
                )
            }
            Self::Malformed { what, .. } => write!(f, "malformed stored entry: {what}"),
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
                mismatch,
            } => write!(
                f,
                "relation {}, row {row}: {} values for {} columns",
                r.0, mismatch.witnessed, mismatch.required
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
            Self::Statement { statement, kind } => {
                write!(f, "statement {}: {kind}", statement.0)
            }
        }
    }
}

impl fmt::Display for StatementErrorKind {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // a rendering table: one arm per roster line
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Short bindings: r = relation, fd = field. The carrier
        // (`SchemaError::Statement`) prints the `statement N:` prefix.
        match self {
            Self::UnknownRelation { relation: r } => write!(f, "unknown relation {}", r.0),
            Self::UnknownField {
                relation: r,
                field: fd,
            } => write!(f, "relation {} has no field {}", r.0, fd.0),
            Self::EmptyProjection { relation: r } => {
                write!(f, "empty projection on relation {}", r.0)
            }
            Self::DuplicateProjectionField {
                relation: r,
                field: fd,
            } => write!(f, "field {} projected twice on relation {}", fd.0, r.0),
            Self::DuplicateSelectionField {
                relation: r,
                field: fd,
            } => write!(f, "field {} selected twice on relation {}", fd.0, r.0),
            Self::FunctionalityMultipleIntervals {
                relation: r,
                field: fd,
            } => write!(
                f,
                "second interval field {} on relation {} — the ordered determinant answers one dimension",
                fd.0, r.0
            ),
            Self::FunctionalityIntervalNotLast {
                relation: r,
                field: fd,
            } => write!(
                f,
                "interval field {} on relation {} must be the final projection position",
                fd.0, r.0
            ),
            Self::DuplicateFunctionality { earlier } => {
                write!(f, "statement {} already keys this field set", earlier.0)
            }
            Self::DeterminantKeyTooWide { width } => write!(
                f,
                "{width}-byte determinant key exceeds the key-size ceiling"
            ),
            Self::ContainmentArityMismatch { mismatch } => write!(
                f,
                "{} source positions against {} target positions",
                mismatch.witnessed, mismatch.required
            ),
            Self::ContainmentTypeMismatch { position } => {
                write!(f, "structural type mismatch at position {position}")
            }
            Self::SelectedFieldProjected {
                relation: r,
                field: fd,
            } => write!(
                f,
                "field {} on relation {} is both selected and projected",
                fd.0, r.0
            ),
            Self::SelectionLiteralTypeMismatch {
                relation: r,
                field: fd,
            } => write!(
                f,
                "selection literal type mismatch at relation {}, field {}",
                r.0, fd.0
            ),
            Self::NoMatchingTargetKey {
                target,
                projection,
                available,
            } => target_key_rejection(f, *target, projection, available, false),
            Self::NoPointwiseTargetKey {
                target,
                projection,
                available,
            } => target_key_rejection(f, *target, projection, available, true),
            Self::ClosedContainmentInterval { relation: r } => write!(
                f,
                "interval position on a containment with closed relation {} — \
                 pointwise judgments against a virtual extension are refused",
                r.0
            ),
            Self::ClosedTargetNotHandle { target, projection } => {
                write!(
                    f,
                    "closed target relation {} is addressed by its synthetic id \
                     only — projection ",
                    target.0
                )?;
                field_set(f, projection)?;
                write!(
                    f,
                    " must be exactly {{0}} (rewrite the target side as `R(id)`)"
                )
            }
            Self::ClosedStatementRefuted { relation: r, row } => write!(
                f,
                "refuted by ground axiom {} of closed relation {} — \
                 a theory whose axioms refute its own statement has no model",
                row, r.0
            ),
            Self::DuplicateStatement { earlier } => {
                write!(f, "duplicates statement {} — write it once", earlier.0)
            }
            Self::DegenerateSelectionSet {
                relation: r,
                field: fd,
                len,
            } => write!(
                f,
                "literal set of {len} on relation {}, field {} — a set binding \
                 carries at least two literals (one literal is the equality spelling)",
                r.0, fd.0
            ),
            Self::DuplicateSelectionLiteral {
                relation: r,
                field: fd,
            } => write!(
                f,
                "duplicate literal in the set binding on relation {}, field {} — \
                 write it once",
                r.0, fd.0
            ),
            Self::CapacityInvertedWindow { lo, hi } => write!(
                f,
                "the window {lo}..{hi} is inverted — no measure satisfies \
                 hi < lo; the canonical bounds are lo < hi ({{lo..hi}}), an exact measure \
                 lo = hi (the {{n}} spelling)"
            ),
            Self::CapacityVacuousWindow => write!(
                f,
                "the 0..* window admits every measure — it provably says \
                 nothing (lean/Bumbledb/Capacity.lean: capacity_zero_star); delete \
                 the statement"
            ),
            Self::CapacityContainmentWindow => write!(
                f,
                "the unit 1..* window says only what the bare containment says — \
                 drop the annotation and declare `target <= source` (a WEIGHTED \
                 {{1..*}} — a positive total — is a different law and stays legal)"
            ),
            Self::CapacityIntervalPosition {
                relation: r,
                field: fd,
            } => write!(
                f,
                "interval field {} on relation {} in a capacity projection — \
                 the group key identifies facts per parent, and an interval position \
                 would make the group ambiguous between facts and points; the interval \
                 measure enters through the weight bracket (`[Duration(field)]`)",
                fd.0, r.0
            ),
            Self::CapacityWeightNotU64 {
                relation: r,
                field: fd,
            } => write!(
                f,
                "weight field {} on relation {} is not u64-encoded — a `[field]` \
                 weight measures a u64 SOURCE position; a signed encoding is refused \
                 by polarity (a negative weight would let an insert lower a sum)",
                fd.0, r.0
            ),
            Self::CapacityWeightNotDuration {
                relation: r,
                field: fd,
            } => write!(
                f,
                "weight field {} on relation {} is not interval-typed — \
                 `[Duration(field)]` reads an interval position's measure",
                fd.0, r.0
            ),
            Self::CapacityBoundNotU64 {
                relation: r,
                field: fd,
            } => write!(
                f,
                "bound field {} on relation {} is not u64-encoded — a dependent \
                 bound reads a u64 field of the TARGET's row (a signed encoding \
                 cannot bound a non-negative measure)",
                fd.0, r.0
            ),
            Self::CapacityBoundNotDuration {
                relation: r,
                field: fd,
            } => write!(
                f,
                "bound field {} on relation {} is not interval-typed — \
                 `{{..Duration(field)}}` bounds by a TARGET interval's measure",
                fd.0, r.0
            ),
            Self::CapacityDimensionMixing { field: fd } => write!(
                f,
                "a unit (count) window against the Duration bound on field {} — \
                 a count of facts bounded by a span of time is a dimension error \
                 (ruled 2026-07-24, C18): weigh the source with `[Duration(field)]`, \
                 or bound by a u64 field or literal",
                fd.0
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
            Self::DnfExceedsRules { exceeded } => write!(
                f,
                "DNF distribution produces {} rules against the cap of {}",
                exceeded.observed, exceeded.ceiling
            ),
            Self::ConditionNestingTooDeep { rule, exceeded } => write!(
                f,
                "rule {rule}: condition trees nest {} deep against the cap of {}",
                exceeded.observed, exceeded.ceiling
            ),
            Self::HeadArityMismatch { rule, mismatch } => write!(
                f,
                "rule {rule}: {} find terms against a head of arity {}",
                mismatch.witnessed, mismatch.required
            ),
            Self::HeadTypeMismatch { rule, position } => write!(
                f,
                "rule {rule}: find term {position} disagrees with the head's positional type"
            ),
            Self::HeadAggregateMismatch { rule, position } => write!(
                f,
                "rule {rule}: find term {position} disagrees with the head's shape at that position"
            ),
            Self::CountAcrossRules { rules } => write!(
                f,
                "nullary Count in a fold-free head of a hand-written {rules}-rule \
                 query: the head projection admits one row per group, so the Count \
                 is the constant 1 — write one Count query per disjunct and merge in \
                 the host"
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
            Self::OrderComparisonOnClosedReference { index } => write!(
                f,
                "comparison {index}: order operator on a closed reference — \
                 declaration-id order is an accident, not semantics"
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
                write!(
                    f,
                    "find {find}: aggregate input outside the fold's type roster"
                )
            }
            Self::AggregateOverClosedReference { find } => write!(
                f,
                "find {find}: ordering fold over a closed reference — \
                 declaration-id order is an accident, not semantics"
            ),
            Self::CountWithVariable { find } => {
                write!(f, "find {find}: Count is nullary")
            }
            Self::AggregateWithoutVariable { find } => {
                write!(f, "find {find}: this aggregate requires a variable")
            }
            Self::AggregateOverGroupKey { find } => {
                write!(f, "find {find}: aggregate over a group-key variable")
            }
            Self::MultiplePackTerms { find } => {
                write!(f, "find {find}: at most one Pack term per head")
            }
            Self::MixedPackAndFold { find } => {
                write!(f, "find {find}: Pack and fold aggregates may not mix")
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
            Self::InteriorIdOverflow { count } => {
                write!(f, "{count} derived tables overflow InteriorId")
            }
            Self::EmptyInterior { interior } => {
                write!(f, "interior {} has no rules", interior.0)
            }
            Self::EmptyRecursiveBase => {
                write!(f, "rec has no base arms — that lfp is empty")
            }
            Self::EmptyRecursiveStep => {
                write!(f, "rec has no rec arms — write an interior")
            }
            Self::SelfInBase => {
                write!(f, "a base arm names the rec")
            }
            Self::RecArmMissingSelf => {
                write!(f, "a rec arm does not name the rec")
            }
            Self::NonlinearRecArm => {
                write!(f, "a rec arm names the rec more than once")
            }
            Self::NegationInRec => {
                write!(f, "negation inside the rec")
            }
            Self::UnknownInterior { atom, interior } => {
                write!(
                    f,
                    "atom {atom}: interior {} is not in the query",
                    interior.0
                )
            }
            Self::InteriorColumnOutOfRange { atom, field } => write!(
                f,
                "atom {atom}: head position {} is beyond the target interior's arity",
                field.0
            ),
            Self::InteriorNotPrior { interior, at } => write!(
                f,
                "interior {} reads interior {} which is not a prior interior",
                at.0, interior.0
            ),
            Self::AggregateInInterior { interior } => write!(
                f,
                "interior {} folds — interior and rec heads project bound variables only",
                interior.0
            ),
            Self::MeasureInInterior { interior } => write!(
                f,
                "interior {} projects a Duration — interior and rec heads project bound variables only",
                interior.0
            ),
            Self::MeasureInRec => {
                write!(f, "a measure site inside the rec")
            }
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
            Self::FormatMismatch { mismatch } => {
                write!(
                    f,
                    "storage format version {}, this build expects {}; \
                     no migration read arm exists — ETL through the SDK is the story",
                    mismatch.witnessed, mismatch.required
                )
            }
            Self::SchemaMismatch { mismatch } => {
                write!(
                    f,
                    "stored schema fingerprint {}, this build's schema is {}",
                    mismatch.witnessed, mismatch.required
                )
            }
            Self::AlreadyInitialized => {
                write!(
                    f,
                    "the directory already holds an LMDB environment; open it instead"
                )
            }
            Self::DestinationExists { path } => {
                write!(
                    f,
                    "destination {} already exists — including as an empty directory",
                    path.display()
                )
            }
            Self::PublishedButUnsynced { path, source } => {
                write!(
                    f,
                    "published {} but the directory entry is unsynced: {source}",
                    path.display()
                )
            }
            Self::EnvironmentLocked => {
                write!(f, "another live handle holds this environment's lock")
            }
            Self::StoreKindMismatch { mismatch } => {
                write!(
                    f,
                    "the store on disk is {}, this constructor opens {} stores",
                    mismatch.witnessed, mismatch.required
                )
            }
            Self::DescriptorMissing => {
                write!(
                    f,
                    "the store carries no schema descriptor (format 8 requires it)"
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
            Self::ForeignWitness => {
                write!(
                    f,
                    "a witness proves nothing about another database — \
                     write_from takes witnesses of the database being written"
                )
            }
            Self::ParamCountMismatch { mismatch } => {
                write!(
                    f,
                    "{} parameters supplied, the query takes {}",
                    mismatch.witnessed, mismatch.required
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
            Self::MeasureOfRay { start, end } => write!(
                f,
                "Duration of a ray: encoded interval [{start}, {end}) has no finite \
                 measure — exclude rays with an Allen predicate or a bounded-end filter"
            ),
            Self::CapacityRayMeasure { statement, fact } => write!(
                f,
                "statement {}: capacity measure of a ray — a row's Duration weight or \
                 bound is [s, ∞), which has no finite measure; the commit refuses whole \
                 (offending row: {} bytes)",
                statement.0,
                fact.len()
            ),
            Self::DerivedBudgetExceeded { rounds, tuples } => write!(
                f,
                "derived-tuples budget exceeded: {rounds} rec rounds and \
                 {tuples} derived tuples — raise the budget \
                 (PreparedQuery::set_derived_budget) or bound the closure"
            ),
            Self::Overflow(super::OverflowKind::Aggregate { find }) => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::Overflow(super::OverflowKind::OriginCapacity) => {
                write!(f, "origin capacity exceeded")
            }
            Self::TransactionPoisoned { source } => {
                write!(f, "write transaction poisoned: {source}")
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

impl Violations {
    /// Pairs the rejection with the schema it speaks about: every cited
    /// statement renders back in the `schema!` algebra notation, in
    /// materialized statement order. Formatting allocates — `Display` is
    /// never the hot path; the payload itself stays ids and fact bytes.
    #[must_use]
    pub fn display_with<'a>(&'a self, schema: &'a Schema) -> impl fmt::Display + 'a {
        ViolationsDisplayWith {
            violations: self,
            schema,
        }
    }
}

/// [`Violations::display_with`]'s adapter.
struct ViolationsDisplayWith<'a> {
    violations: &'a Violations,
    schema: &'a Schema,
}

impl fmt::Display for ViolationsDisplayWith<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "admission rejected: ")?;
        for (index, violation) in self.violations.iter().enumerate() {
            if index > 0 {
                write!(f, "; ")?;
            }
            let rendered = render::render(self.schema, violation.statement_id(self.schema));
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
}

impl Error {
    /// Pairs the error with the schema it speaks about. Theory rejection
    /// is [`Violations::display_with`]; every `Error` variant renders as
    /// its plain `Display`.
    #[must_use]
    pub fn display_with<'a>(&'a self, schema: &'a Schema) -> impl fmt::Display + 'a {
        let _ = schema;
        DisplayWith { error: self }
    }
}

/// [`Error::display_with`]'s adapter.
struct DisplayWith<'a> {
    error: &'a Error,
}

impl fmt::Display for DisplayWith<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl SchemaError {
    /// The offending statement, for the roster arm that carries one —
    /// a field read off the typed partition, not a hand-sorted variant
    /// roster: a statement-scoped kind cannot exist without its id.
    fn statement(&self) -> Option<StatementId> {
        match self {
            Self::Statement { statement, .. } => Some(*statement),
            _ => None,
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
