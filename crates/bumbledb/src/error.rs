//! The workspace error taxonomy (PRD 04; skeleton per `docs/architecture/60-api.md`).
//!
//! Everything reachable from user input or disk returns these typed errors;
//! panics are reserved for programmer-invariant violations. Payloads carry
//! ids, not formatted strings — `Display` (PRD 28) formats lazily.

use crate::schema::fingerprint::SchemaFingerprint;
use crate::schema::{ConstraintId, FieldId, RelationId};

/// Corruption detected while decoding stored bytes — a hard error, never a
/// skip, never a default (`docs/architecture/40-storage.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorruptionError {
    /// A Bool byte other than `0x00`/`0x01` — there is no distinct "true".
    InvalidBool(u8),
    /// An Enum ordinal at or beyond the declared variant count.
    EnumOrdinalOutOfRange { ordinal: u8, variant_count: u16 },
    /// The `_meta` database or one of its required keys is absent or
    /// malformed: the environment is not a usable bumbledb database.
    MetaMissing,
    /// An intern id with no reverse dictionary entry — a fact referencing it
    /// is corrupt.
    DanglingInternId(u64),
}

/// A schema declaration error (PRD 02's validation boundary). Every illegal
/// schema shape has a distinct variant; an invalid schema is
/// unconstructible, not flagged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    DuplicateRelationName {
        name: Box<str>,
    },
    DuplicateFieldName {
        relation: RelationId,
        name: Box<str>,
    },
    DuplicateConstraintName {
        relation: RelationId,
        name: Box<str>,
    },
    EnumWithoutVariants {
        relation: RelationId,
        field: FieldId,
    },
    EnumTooManyVariants {
        relation: RelationId,
        field: FieldId,
        count: usize,
    },
    DuplicateEnumVariant {
        relation: RelationId,
        field: FieldId,
        variant: Box<str>,
    },
    SerialOnNonU64 {
        relation: RelationId,
        field: FieldId,
    },
    UnknownConstraintField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    UniqueWithoutFields {
        relation: RelationId,
        constraint: ConstraintId,
    },
    UniqueDuplicateField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    /// The constraint's guard key would exceed LMDB's key ceiling
    /// (`storage::keys::MAX_GUARD_WIDTH`) once embedded in a Restrict key.
    GuardKeyTooWide {
        relation: RelationId,
        constraint: ConstraintId,
        width: usize,
    },
    UnknownFkTargetRelation {
        relation: RelationId,
        constraint: ConstraintId,
        target: RelationId,
    },
    UnknownFkTargetConstraint {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkTargetNotUnique {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkArityMismatch {
        relation: RelationId,
        constraint: ConstraintId,
    },
    FkFieldTypeMismatch {
        relation: RelationId,
        constraint: ConstraintId,
        position: usize,
    },
}

/// The one workspace error type, categorized per `docs/architecture/60-api.md`.
///
/// The Validation (IR boundary, PRD 14) and Write (PRDs 07-08) categories
/// gain their variants in the PRDs that raise them.
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
    Io(std::io::Error),
    Lmdb(heed::Error),

    // --- Declaration / validation errors ---
    Schema(SchemaError),

    // --- Runtime errors ---
    /// Hard corruption error, never a skip.
    Corruption(CorruptionError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<heed::Error> for Error {
    fn from(err: heed::Error) -> Self {
        Self::Lmdb(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<SchemaError> for Error {
    fn from(err: SchemaError) -> Self {
        Self::Schema(err)
    }
}

impl From<CorruptionError> for Error {
    fn from(err: CorruptionError) -> Self {
        Self::Corruption(err)
    }
}
