//! `Display` for every error type. Statements are anonymous, so plain
//! `Display` cites them by id; [`Violations::display_with`] and
//! [`crate::schema::render::schema_error`] render them back as declared.

use std::fmt;

use crate::schema::{Schema, render};

use super::{
    CorruptionError, Direction, DynIdError, Error, FactShapeError, IoFailure, LmdbFailure,
    Violation, Violations,
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
            Self::DanglingInternId(id) => write!(
                f,
                "text token {} was never minted by the query image's interner",
                id.raw()
            ),
            Self::WrongFactWidth {
                relation,
                row_id,
                mismatch,
            } => write!(
                f,
                "relation {}: row {row_id} is {} bytes, the schema says {}",
                relation.0, mismatch.witnessed, mismatch.required
            ),
            Self::RowCountMismatch { relation, stored } => write!(
                f,
                "relation {}: stored row count {stored} disagrees with the rows",
                relation.0
            ),
            Self::MalformedValue(kind) => write!(f, "malformed stored value: {kind}"),
            Self::NonzeroFixedBytesPad(tail) => {
                write!(
                    f,
                    "bytes<N> trailing word {tail:02x?} has a nonzero pad byte"
                )
            }
            Self::MetaMissing(what) => write!(f, "meta entry `{what}` is absent or malformed"),
            Self::MalformedKey(what) => write!(f, "malformed physical key: {what}"),
            Self::DanglingIndexEntry => f.write_str("an index entry names no row"),
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

impl fmt::Display for super::HostKeyFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { actual } => write!(
                f,
                "host key has {actual} bytes; the limit is {}",
                crate::storage::store::keys::HOST_KEY_MAX
            ),
            Self::NotStrictlyOrdered => f.write_str("host keys must be strictly increasing"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotABumbleDb { path } => {
                write!(
                    f,
                    "{} is not a bumbledb store of this format",
                    path.display()
                )
            }
            Self::SchemaMismatch => f.write_str("the store was created with another schema"),
            Self::Locked { path } => {
                write!(f, "another live owner holds the lock on {}", path.display())
            }
            Self::DestinationExists { path } => {
                write!(f, "destination {} already exists", path.display())
            }
            Self::Closed => f.write_str("the database is closing or closed"),
            Self::ReentrantWriter => f.write_str("this thread already holds the writer"),
            Self::ReadersFull => f.write_str("every reader slot holds an open snapshot"),
            Self::Full { ceiling } => {
                write!(f, "the store is full at its {ceiling}-byte map ceiling")
            }
            Self::Io(err) => write!(f, "io: {err}"),
            Self::Lmdb(err) => write!(f, "lmdb: {err}"),
            Self::Corruption(err) => write!(f, "corruption: {err}"),
            Self::ForeignSchema => f.write_str("the change set was sealed for another schema"),
            Self::ForeignWitness => {
                f.write_str("the witness belongs to another database environment")
            }
            Self::ForeignPreparedQuery => {
                f.write_str("the prepared query belongs to another database")
            }
            Self::ClosedRelationWrite { relation } => write!(
                f,
                "relation {} is closed: its rows are ground axioms of the schema",
                relation.0
            ),
            Self::TransactionPoisoned { source } => {
                write!(f, "write transaction poisoned: {source}")
            }
            Self::Changes(err) => write!(f, "change set: {err}"),
            Self::HostKey(fault) => write!(f, "host record: {fault}"),
            Self::Exhausted(super::Counter::RowIds) => f.write_str("row ordinals exhausted"),
            Self::Exhausted(super::Counter::Generations) => {
                f.write_str("generation counter exhausted")
            }
            Self::CapacityRayMeasure { statement } => write!(
                f,
                "statement {}: a capacity weight or bound is a ray, which has no finite measure",
                statement.0
            ),
            Self::MeasureOverflow { statement } => write!(
                f,
                "statement {}: a capacity group's measure overflows",
                statement.0
            ),
            Self::Schema(err) => write!(f, "schema declaration: {err}"),
            Self::Compile(err) => write!(f, "schema compile: {err}"),
            Self::Validation(err) => write!(f, "query validation: {err}"),
            Self::FactShape(err) => write!(f, "dynamic fact: {err}"),
            Self::ParamCountMismatch { mismatch } => write!(
                f,
                "{} parameters supplied, the query takes {}",
                mismatch.witnessed, mismatch.required
            ),
            Self::ParamTypeMismatch { param, expected } => {
                write!(f, "parameter {}: expected {expected:?}", param.0)
            }
            Self::ParamSetExpected { param } => {
                write!(f, "parameter {}: the query binds a set", param.0)
            }
            Self::ParamScalarExpected { param } => {
                write!(f, "parameter {}: the query binds a scalar", param.0)
            }
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
                "parameter {}: a point is MIN..=MAX-1; MAX is a ray's open end",
                param.0
            ),
            Self::Overflow(super::OverflowKind::Aggregate { find }) => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::Overflow(super::OverflowKind::OriginCapacity) => {
                f.write_str("origin capacity exceeded")
            }
            Self::Overflow(super::OverflowKind::Cardinality) => {
                f.write_str("aggregate binding cardinality exceeds u64::MAX")
            }
            Self::Scalar { find, source } => write!(f, "find {find}: {source}"),
            Self::Capacity(capacity) => write!(f, "capacity reached: {capacity}"),
            Self::ResultBytesOverflow => f.write_str("the result byte heap exceeds u32 offsets"),
            Self::Cancelled => f.write_str("operation cancelled"),
            Self::Allocation => f.write_str("allocation refused"),
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
