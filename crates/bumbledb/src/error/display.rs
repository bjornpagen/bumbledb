//! `Display` rendering for every error type — formatting runs lazily, only
//! when the host actually prints.
//! Statements are anonymous, so
//! the plain `Display` impls cite them by id; the [`Error::display_with`]
//! and [`SchemaError::display_with`] adapters pair the error with the
//! schema it speaks about and render the statement back in the `schema!`
use std::fmt;

use crate::schema::{Schema, render};
use bumbledb_theory::schema::{SchemaDescriptor, StatementId};

use super::{
    CorruptionError, Direction, DynIdError, Error, FactShapeError, IoFailure, LmdbFailure,
    SchemaError, StatementErrorKind, TargetKeyCandidate, Violation, Violations,
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

/// `names` pairs `projection` positionwise (the variants' construction
/// invariant).
fn field_set(
    f: &mut fmt::Formatter<'_>,
    projection: &[bumbledb_theory::schema::FieldId],
    names: &[Box<str>],
) -> fmt::Result {
    let mut fields: Vec<_> = projection.iter().copied().zip(names.iter()).collect();
    fields.sort_unstable_by_key(|(field, _)| *field);
    write!(f, "{{")?;
    for (index, (field, name)) in fields.iter().enumerate() {
        if index > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{name} ({})", field.0)?;
    }
    write!(f, "}}")
}

fn target_key_rejection(
    f: &mut fmt::Formatter<'_>,
    target: bumbledb_theory::schema::RelationId,
    target_name: &str,
    projection: &[bumbledb_theory::schema::FieldId],
    projection_names: &[Box<str>],
    available: &[TargetKeyCandidate],
    pointwise: bool,
) -> fmt::Result {
    write!(
        f,
        "target relation {target_name} ({}) projection ",
        target.0
    )?;
    field_set(f, projection, projection_names)?;
    write!(f, " matches no declared key; available keys: ")?;
    if available.is_empty() {
        write!(f, "none")?;
    } else {
        for (index, candidate) in available.iter().enumerate() {
            if index > 0 {
                write!(f, "; ")?;
            }
            write!(f, "key {} ", candidate.key.0)?;
            field_set(f, &candidate.projection, &candidate.projection_names)?;
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

impl Violation {
    fn law(&self) -> &'static str {
        match self {
            Self::Functionality { .. } => "functionality",
            Self::Containment { .. } => "containment",
            Self::Capacity { .. } => "capacity",
        }
    }

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
            Self::PayloadBound { relation } => write!(
                f,
                "relation {}: variable-width payloads exceed the 4 GiB transport bound of one collection",
                relation.0
            ),
        }
    }
}

impl fmt::Display for CorruptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonicalF64(bytes) => {
                write!(f, "noncanonical stored F64 order bytes: {bytes:02x?}")
            }
            Self::InvalidBool(byte) => write!(f, "invalid Bool byte {byte:#04x}"),
            Self::InvalidInterval(bytes) => {
                write!(f, "interval bytes {bytes:02x?}: start >= end")
            }
            Self::InvalidFixedIntervalStart(bytes) => write!(
                f,
                "fixed-width interval start {bytes:02x?}: start + w at or past the domain ceiling"
            ),
            Self::MetaMissing => write!(f, "the _meta database or a required key is absent"),
            Self::DanglingInternId(id) => {
                write!(
                    f,
                    "text token {} was never minted by the query image's interner",
                    id.raw()
                )
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
            Self::NonzeroFixedBytesPad(tail) => write!(
                f,
                "bytes<N> trailing word {tail:02x?}: nonzero pad byte — the pad is encoding, not data"
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRelationName { name } => write!(f, "duplicate relation name `{name}`"),
            Self::DuplicateFieldName { relation: r, name } => {
                write!(f, "relation {}: duplicate field name `{name}`", r.0)
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
            Self::Statement { statement, kind } => {
                write!(f, "statement {}: {kind}", statement.0)
            }
        }
    }
}

impl fmt::Display for StatementErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
                target_name,
                projection,
                projection_names,
                available,
            } => target_key_rejection(
                f,
                *target,
                target_name,
                projection,
                projection_names,
                available,
                false,
            ),
            Self::NoPointwiseTargetKey {
                target,
                target_name,
                projection,
                projection_names,
                available,
            } => target_key_rejection(
                f,
                *target,
                target_name,
                projection,
                projection_names,
                available,
                true,
            ),
            Self::ClosedContainmentInterval { relation: r } => write!(
                f,
                "interval position on a containment with closed relation {} — \
                 pointwise judgments against a virtual extension are refused",
                r.0
            ),
            Self::ClosedTargetNotHandle {
                target,
                target_name,
                projection,
                projection_names,
            } => {
                write!(
                    f,
                    "closed target relation {target_name} ({}) is addressed by its \
                     synthetic id only — projection ",
                    target.0
                )?;
                field_set(f, projection, projection_names)?;
                write!(
                    f,
                    " must be exactly {{id (0)}} (rewrite the target side as `R(id)`)"
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

impl fmt::Display for super::Capacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ResidentRows => "a relation image exceeds u32 row positions",
            Self::DistinctRows => "a distinct-row index is full",
            Self::Groups => "a group-by table is full",
            Self::ResultBytes => "a result byte heap exceeds u32 offsets",
        })
    }
}

impl fmt::Display for Error {
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
            Self::Io(err) => write!(f, "io: {err}"),
            Self::Hatch(_) => write!(f, "io: bridge decline"),
            Self::Lmdb(err) => write!(f, "lmdb: {err}"),
            Self::ReadersFull { max_readers } => {
                write!(f, "all {max_readers} reader slots hold open snapshots")
            }
            Self::Schema(err) => write!(f, "schema declaration: {err}"),
            Self::Validation(err) => write!(f, "query validation: {err}"),
            Self::FactShape(err) => write!(f, "dynamic fact: {err}"),
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
            Self::CapacityRayMeasure { statement } => write!(
                f,
                "statement {}: capacity measure of a ray — a row's Duration weight or \
                 bound is [s, ∞), which has no finite measure; the commit refuses whole",
                statement.0
            ),
            Self::Overflow(super::OverflowKind::Aggregate { find }) => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::Overflow(super::OverflowKind::OriginCapacity) => {
                write!(f, "origin capacity exceeded")
            }
            Self::Overflow(super::OverflowKind::Cardinality) => {
                write!(f, "aggregate binding cardinality exceeds u64::MAX")
            }
            Self::Scalar { find, source } => write!(f, "find {find}: {source}"),
            Self::TransactionPoisoned { source } => {
                write!(f, "write transaction poisoned: {source}")
            }
            Self::ResultBytesOverflow => {
                write!(
                    f,
                    "the result buffer's byte heap exceeds u32 offsets (4 GiB)"
                )
            }
            Self::Capacity(capacity) => write!(f, "capacity reached: {capacity}"),
            Self::Full { ceiling } => write!(
                f,
                "the store is full at its {ceiling}-byte map ceiling; reopen with a larger ceiling"
            ),
            Self::Corruption(err) => write!(f, "corruption: {err}"),
            Self::Store(err) => err.fmt(f),
        }
    }
}

impl Violations {
    #[must_use]
    pub fn display_with<'a>(&'a self, schema: &'a Schema) -> impl fmt::Display + 'a {
        ViolationsDisplayWith {
            violations: self,
            schema,
        }
    }
}

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
    #[must_use]
    pub fn display_with<'a>(&'a self, schema: &'a Schema) -> impl fmt::Display + 'a {
        let _ = schema;
        DisplayWith { error: self }
    }
}

struct DisplayWith<'a> {
    error: &'a Error,
}

impl fmt::Display for DisplayWith<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl SchemaError {
    fn statement(&self) -> Option<StatementId> {
        match self {
            Self::Statement { statement, .. } => Some(*statement),
            _ => None,
        }
    }

    #[must_use]
    pub fn display_with<'a>(&'a self, descriptor: &'a SchemaDescriptor) -> impl fmt::Display + 'a {
        SchemaDisplayWith {
            error: self,
            descriptor,
        }
    }
}

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
