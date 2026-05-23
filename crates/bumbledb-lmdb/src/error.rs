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
}
