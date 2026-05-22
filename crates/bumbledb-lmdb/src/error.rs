//! Layered error taxonomy for the LMDB-backed engine.

use std::path::PathBuf;

/// Result type for storage operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level Bumbledb LMDB-layer error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Environment open/setup failure.
    #[error(transparent)]
    Open(#[from] OpenError),

    /// Schema/fingerprint/layout failure.
    #[error(transparent)]
    Schema(#[from] SchemaError),

    /// Storage/dictionary/metadata failure.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// Transaction lifecycle failure.
    #[error(transparent)]
    Transaction(#[from] TransactionError),

    /// Constraint or write validation failure.
    #[error(transparent)]
    Constraint(#[from] ConstraintError),

    /// Query planning/execution failure.
    #[error(transparent)]
    Query(#[from] QueryError),

    /// Backup or compact-copy failure.
    #[error(transparent)]
    Backup(#[from] BackupError),

    /// Persisted state is malformed or inconsistent.
    #[error(transparent)]
    Corruption(#[from] CorruptionError),

    /// Engine invariant failure.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// Test-only injected failure.
    #[cfg(feature = "test-failpoints")]
    #[error(transparent)]
    Test(#[from] TestError),
}

impl Error {
    pub(crate) fn lmdb(operation: &'static str, source: heed::Error) -> Self {
        StorageError::Lmdb { operation, source }.into()
    }

    pub(crate) fn io(operation: &'static str, source: std::io::Error) -> Self {
        StorageError::Io { operation, source }.into()
    }

    pub(crate) fn corrupt(message: &'static str) -> Self {
        CorruptionError::Message(message).into()
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        InternalError::Invariant {
            message: message.into(),
        }
        .into()
    }

    pub(crate) fn unknown_relation(relation: impl Into<String>) -> Self {
        SchemaError::UnknownRelation {
            relation: relation.into(),
        }
        .into()
    }

    pub(crate) fn unknown_field(relation: impl Into<String>, field: impl Into<String>) -> Self {
        ConstraintError::UnknownField {
            relation: relation.into(),
            field: field.into(),
        }
        .into()
    }

    pub(crate) fn missing_field(relation: impl Into<String>, field: impl Into<String>) -> Self {
        ConstraintError::MissingField {
            relation: relation.into(),
            field: field.into(),
        }
        .into()
    }

    pub(crate) fn unknown_index(relation: impl Into<String>, index: impl Into<String>) -> Self {
        QueryError::Plan(PlanError::UnknownIndex {
            relation: relation.into(),
            index: index.into(),
        })
        .into()
    }

    pub(crate) fn storage_format_mismatch(expected: u32, found: u32) -> Self {
        OpenError::StorageFormatMismatch { expected, found }.into()
    }

    pub(crate) fn missing_storage_format_version() -> Self {
        OpenError::MissingStorageFormatVersion.into()
    }

    pub(crate) fn schema_mismatch(expected: String, found: String) -> Self {
        SchemaError::SchemaMismatch { expected, found }.into()
    }

    pub(crate) fn bulk_load_target_exists(path: impl Into<PathBuf>) -> Self {
        StorageError::BulkLoadTargetExists { path: path.into() }.into()
    }

    pub(crate) fn dictionary_value_not_found(kind: &'static str) -> Self {
        StorageError::DictionaryValueNotFound { kind }.into()
    }

    pub(crate) fn hash_collision(kind: &'static str) -> Self {
        StorageError::HashCollision { kind }.into()
    }

    pub(crate) fn unique_violation(
        relation: impl Into<String>,
        constraint: impl Into<String>,
    ) -> Self {
        ConstraintError::UniqueViolation {
            relation: relation.into(),
            constraint: constraint.into(),
        }
        .into()
    }

    pub(crate) fn foreign_key_violation(
        relation: impl Into<String>,
        constraint: impl Into<String>,
        target_relation: impl Into<String>,
    ) -> Self {
        ConstraintError::ForeignKeyViolation {
            relation: relation.into(),
            constraint: constraint.into(),
            target_relation: target_relation.into(),
        }
        .into()
    }

    pub(crate) fn restrict_violation(
        relation: impl Into<String>,
        referenced_by: impl Into<String>,
        constraint: impl Into<String>,
    ) -> Self {
        ConstraintError::RestrictViolation {
            relation: relation.into(),
            referenced_by: referenced_by.into(),
            constraint: constraint.into(),
        }
        .into()
    }

    pub(crate) fn type_mismatch(
        relation: impl Into<String>,
        field: impl Into<String>,
        expected: impl Into<String>,
        actual: &'static str,
    ) -> Self {
        ConstraintError::TypeMismatch {
            relation: relation.into(),
            field: field.into(),
            expected: expected.into(),
            actual,
        }
        .into()
    }

    pub(crate) fn missing_input(input: impl Into<String>) -> Self {
        QueryError::Execute(ExecuteError::MissingInput {
            input: input.into(),
        })
        .into()
    }

    pub(crate) fn query_input_type_mismatch(
        input: impl Into<String>,
        expected: impl Into<String>,
        actual: &'static str,
    ) -> Self {
        QueryError::Execute(ExecuteError::InputTypeMismatch {
            input: input.into(),
            expected: expected.into(),
            actual,
        })
        .into()
    }

    pub(crate) fn integer_overflow(operation: &'static str) -> Self {
        QueryError::Aggregate(AggregateError::IntegerOverflow { operation }).into()
    }

    pub(crate) fn decimal_overflow(operation: &'static str) -> Self {
        QueryError::Aggregate(AggregateError::DecimalOverflow { operation }).into()
    }

    pub(crate) fn aggregate_type_mismatch(function: &'static str, actual: &'static str) -> Self {
        QueryError::Aggregate(AggregateError::TypeMismatch { function, actual }).into()
    }

    pub(crate) fn invalid_utf8_dictionary_string() -> Self {
        CorruptionError::InvalidUtf8DictionaryString.into()
    }
}

impl From<heed::Error> for Error {
    fn from(source: heed::Error) -> Self {
        Error::lmdb("lmdb", source)
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::io("io", source)
    }
}

impl From<bumbledb_core::schema::SchemaError> for Error {
    fn from(source: bumbledb_core::schema::SchemaError) -> Self {
        SchemaError::Descriptor { source }.into()
    }
}

/// Open/setup failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenError {
    /// Storage format version is missing.
    #[error("storage format version metadata is missing")]
    MissingStorageFormatVersion,

    /// Storage format version mismatch.
    #[error("storage format version mismatch: expected {expected}, found {found}")]
    StorageFormatMismatch { expected: u32, found: u32 },
}

/// Schema and descriptor failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SchemaError {
    /// Schema descriptor failure.
    #[error(transparent)]
    Descriptor {
        #[from]
        source: bumbledb_core::schema::SchemaError,
    },

    /// Relation is not present in the schema.
    #[error("unknown relation {relation}")]
    UnknownRelation { relation: String },

    /// Existing database was opened with a different schema fingerprint.
    #[error("schema fingerprint mismatch: expected {expected}, found {found}")]
    SchemaMismatch { expected: String, found: String },
}

/// Storage/dictionary/metadata failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StorageError {
    /// LMDB operation failure.
    #[error("LMDB operation {operation} failed")]
    Lmdb {
        operation: &'static str,
        #[source]
        source: heed::Error,
    },

    /// Filesystem IO failure.
    #[error("IO operation {operation} failed")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },

    /// Bulk-load target already exists.
    #[error("bulk load target already contains a database: {path}")]
    BulkLoadTargetExists { path: PathBuf },

    /// Dictionary value not found.
    #[error("dictionary value not found for {kind}")]
    DictionaryValueNotFound { kind: &'static str },

    /// Dictionary hash collision.
    #[error("dictionary hash collision for {kind}")]
    HashCollision { kind: &'static str },

    /// Metadata counter overflow.
    #[error("metadata counter {name} overflowed")]
    CounterOverflow { name: &'static str },

    /// Metadata counter underflow.
    #[error("metadata counter {name} underflowed")]
    CounterUnderflow { name: &'static str },
}

/// Transaction lifecycle failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransactionError {
    /// Failed to begin read transaction.
    #[error("failed to begin read transaction")]
    BeginRead {
        #[source]
        source: heed::Error,
    },

    /// Failed to begin write transaction.
    #[error("failed to begin write transaction")]
    BeginWrite {
        #[source]
        source: heed::Error,
    },

    /// Failed to commit write transaction.
    #[error("failed to commit write transaction")]
    Commit {
        #[source]
        source: heed::Error,
    },

    /// Reader cleanup failed.
    #[error("reader cleanup failed")]
    ReaderCleanup {
        #[source]
        source: heed::Error,
    },
}

/// Constraint/write validation failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConstraintError {
    /// Unique constraint violation.
    #[error("unique constraint {relation}.{constraint} violated")]
    UniqueViolation {
        relation: String,
        constraint: String,
    },

    /// Foreign key violation.
    #[error("foreign key {relation}.{constraint} references missing {target_relation}")]
    ForeignKeyViolation {
        relation: String,
        constraint: String,
        target_relation: String,
    },

    /// Restrict-delete violation.
    #[error("cannot delete {relation}; referenced by {referenced_by}.{constraint}")]
    RestrictViolation {
        relation: String,
        referenced_by: String,
        constraint: String,
    },

    /// Missing fact field.
    #[error("missing field {relation}.{field}")]
    MissingField { relation: String, field: String },

    /// Unknown fact field.
    #[error("unknown field {relation}.{field}")]
    UnknownField { relation: String, field: String },

    /// Value type mismatch.
    #[error("type mismatch for {relation}.{field}: expected {expected}, got {actual}")]
    TypeMismatch {
        relation: String,
        field: String,
        expected: String,
        actual: &'static str,
    },
}

/// Query failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueryError {
    /// Planning failure.
    #[error(transparent)]
    Plan(#[from] PlanError),

    /// Execution failure.
    #[error(transparent)]
    Execute(#[from] ExecuteError),

    /// Aggregation failure.
    #[error(transparent)]
    Aggregate(#[from] AggregateError),
}

/// Query planning failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlanError {
    /// No access path for relation.
    #[error("no access path for relation {relation}")]
    NoAccessPath { relation: String },

    /// Unknown index.
    #[error("unknown index {relation}.{index}")]
    UnknownIndex { relation: String, index: String },

    /// Invalid range index.
    #[error("range index {relation}.{index} has no leading field")]
    InvalidRangeIndex { relation: String, index: String },

    /// Non-contiguous prefix.
    #[error("index prefix for {relation}.{index} is not contiguous")]
    NonContiguousPrefix { relation: String, index: String },
}

/// Query execution failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExecuteError {
    /// Missing input.
    #[error("missing query input ${input}")]
    MissingInput { input: String },

    /// Input type mismatch.
    #[error("query input ${input} expected {expected}, got {actual}")]
    InputTypeMismatch {
        input: String,
        expected: String,
        actual: &'static str,
    },

    /// Unbound projection variable.
    #[error("variable {variable} is unbound at projection")]
    UnboundProjectionVariable { variable: usize },

    /// Literal mismatch.
    #[error("typed literal does not match literal value")]
    LiteralMismatch,
}

/// Aggregation failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AggregateError {
    /// Integer overflow.
    #[error("integer overflow during {operation}")]
    IntegerOverflow { operation: &'static str },

    /// Decimal overflow.
    #[error("decimal overflow during {operation}")]
    DecimalOverflow { operation: &'static str },

    /// Aggregate type mismatch.
    #[error("aggregate {function} received unexpected value kind {actual}")]
    TypeMismatch {
        function: &'static str,
        actual: &'static str,
    },
}

/// Backup/compact-copy failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BackupError {
    /// Failed to copy environment.
    #[error("failed to copy LMDB environment to {path}")]
    Copy {
        path: PathBuf,
        compact: bool,
        #[source]
        source: heed::Error,
    },
}

/// Corruption/malformed persisted state failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CorruptionError {
    /// Generic corruption message.
    #[error("storage metadata is corrupt: {0}")]
    Message(&'static str),

    /// Stored string dictionary bytes are not valid UTF-8.
    #[error("dictionary string is not valid UTF-8")]
    InvalidUtf8DictionaryString,
}

/// Internal engine invariant failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InternalError {
    /// Internal invariant failure.
    #[error("internal invariant failed: {message}")]
    Invariant { message: String },
}

/// Test-only injected failure.
#[cfg(feature = "test-failpoints")]
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TestError {
    /// Injected failpoint.
    #[error("injected failpoint: {name}")]
    InjectedFailpoint { name: &'static str },
}
