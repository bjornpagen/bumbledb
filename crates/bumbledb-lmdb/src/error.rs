//! Minimal LMDB-layer error shell for the v5 rebuild.

/// Result type for LMDB-layer operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level Bumbledb LMDB-layer error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Schema descriptor validation failed.
    #[error(transparent)]
    Schema(#[from] bumbledb_core::schema::SchemaError),

    /// The requested operation was intentionally purged pending the v5 rebuild.
    #[error("operation {operation} is unavailable pending {prd}")]
    Unavailable {
        /// Operation name.
        operation: &'static str,
        /// PRD that must rebuild the operation.
        prd: &'static str,
    },

    /// Query IR or execution request is invalid.
    #[error("invalid query: {reason}")]
    InvalidQuery {
        /// Rejection reason.
        reason: String,
    },

    /// On-disk storage format marker is missing or incompatible.
    #[error("storage format mismatch: expected {expected}, found {found}")]
    StorageFormatMismatch {
        /// Expected storage format version.
        expected: u32,
        /// Found marker or absence reason.
        found: String,
    },

    /// Stored schema does not match the supplied schema.
    #[error("schema mismatch: expected {expected}, found {found}")]
    SchemaMismatch {
        /// Expected schema fingerprint.
        expected: String,
        /// Found schema fingerprint.
        found: String,
    },

    /// Fact did not match the schema or operation requirements.
    #[error("invalid fact: {reason}")]
    InvalidFact {
        /// Rejection reason.
        reason: String,
    },

    /// Unique constraint violation.
    #[error("unique constraint violation: {relation}.{constraint}")]
    UniqueViolation {
        /// Relation name.
        relation: String,
        /// Constraint name.
        constraint: String,
    },

    /// Foreign key constraint violation.
    #[error("foreign key violation: {relation}.{constraint}")]
    ForeignKeyViolation {
        /// Relation name.
        relation: String,
        /// Constraint name.
        constraint: String,
    },

    /// Restrict delete violation.
    #[error("restrict delete violation: {relation}.{constraint}")]
    RestrictViolation {
        /// Relation name.
        relation: String,
        /// Constraint name.
        constraint: String,
    },

    /// Durable bytes are corrupt or internally inconsistent.
    #[error("corrupt storage: {reason}")]
    Corrupt {
        /// Corruption reason.
        reason: String,
    },

    /// LMDB failure.
    #[error(transparent)]
    Lmdb(#[from] heed::Error),

    /// I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Creates an unavailable-operation error tied to a rebuild PRD.
    pub fn unavailable(operation: &'static str, prd: &'static str) -> Self {
        Self::Unavailable { operation, prd }
    }

    /// Creates an invalid-query error.
    pub fn invalid_query(reason: impl Into<String>) -> Self {
        Self::InvalidQuery {
            reason: reason.into(),
        }
    }

    /// Creates a storage-format mismatch error.
    pub fn storage_format_mismatch(expected: u32, found: impl Into<String>) -> Self {
        Self::StorageFormatMismatch {
            expected,
            found: found.into(),
        }
    }

    /// Creates a schema mismatch error.
    pub fn schema_mismatch(expected: impl Into<String>, found: impl Into<String>) -> Self {
        Self::SchemaMismatch {
            expected: expected.into(),
            found: found.into(),
        }
    }

    /// Creates an invalid-fact error.
    pub fn invalid_fact(reason: impl Into<String>) -> Self {
        Self::InvalidFact {
            reason: reason.into(),
        }
    }

    /// Creates a unique violation error.
    pub fn unique_violation(relation: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self::UniqueViolation {
            relation: relation.into(),
            constraint: constraint.into(),
        }
    }

    /// Creates a foreign-key violation error.
    pub fn foreign_key_violation(
        relation: impl Into<String>,
        constraint: impl Into<String>,
    ) -> Self {
        Self::ForeignKeyViolation {
            relation: relation.into(),
            constraint: constraint.into(),
        }
    }

    /// Creates a restrict violation error.
    pub fn restrict_violation(relation: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self::RestrictViolation {
            relation: relation.into(),
            constraint: constraint.into(),
        }
    }

    /// Creates a corrupt-storage error.
    pub fn corrupt(reason: impl Into<String>) -> Self {
        Self::Corrupt {
            reason: reason.into(),
        }
    }
}
