//! Wire tags for the data plane and engine errors: one exhaustive table per
//! mirrored engine enum, so a new engine variant fails to compile here.
use bumbledb::{ErrorFamily, Value};

use crate::marshal::OwnedParam;

macro_rules! wire_tags {
    ($(#[$doc:meta])* mod $mod_name:ident for $enum_ty:ty {
        $($const_name:ident : $pat:pat => $tag:literal),+ $(,)?
    }) => {
        $(#[$doc])*
        pub(crate) mod $mod_name {
            #[allow(unused_imports)]
            use super::*;

            $(pub(crate) const $const_name: &str = $tag;)+

            #[allow(dead_code)]
            pub(crate) fn tag(value: &$enum_ty) -> &'static str {
                match value {
                    $($pat => $const_name,)+
                }
            }
        }
    };
}

wire_tags! {
    /// `bumbledb::Value`: the `kind` of a tagged data-plane value (`CellIn`).
    mod value for Value {
        BOOL: Value::Bool(_) => "Bool",
        U64: Value::U64(_) => "U64",
        I64: Value::I64(_) => "I64",
        F64: Value::F64(_) => "F64",
        UUID: Value::Uuid(_) => "Uuid",
        STRING: Value::String(_) => "String",
        FIXED_BYTES: Value::FixedBytes(_) => "FixedBytes",
        INTERVAL_U64: Value::IntervalU64(_) => "IntervalU64",
        INTERVAL_I64: Value::IntervalI64(_) => "IntervalI64",
        INTERVAL_F64: Value::IntervalF64(_) => "IntervalF64",
    }
}

wire_tags! {
    /// The execute-param fork: a scalar param is a tagged value, a set is
    /// `{ kind: "Set", values }`.
    mod param for OwnedParam {
        SET: OwnedParam::Set(_) => "Set",
        SCALAR: OwnedParam::Scalar(_) => "Scalar",
    }
}

wire_tags! {
    /// `bumbledb::ErrorFamily` — the forced napi kind table. Engine errors
    /// cross as `{ kind, message }`; a new family arm breaks this crate.
    mod error_family for ErrorFamily {
        FORMAT_MISMATCH: ErrorFamily::FormatMismatch => "formatMismatch",
        SCHEMA_MISMATCH: ErrorFamily::SchemaMismatch => "schemaMismatch",
        ALREADY_INITIALIZED: ErrorFamily::AlreadyInitialized => "alreadyInitialized",
        DESTINATION_EXISTS: ErrorFamily::DestinationExists => "destinationExists",
        PUBLISHED_BUT_UNSYNCED: ErrorFamily::PublishedButUnsynced => "publishedButUnsynced",
        ENVIRONMENT_LOCKED: ErrorFamily::EnvironmentLocked => "environmentLocked",
        IO: ErrorFamily::Io => "io",
        LMDB: ErrorFamily::Lmdb => "lmdb",
        READERS_FULL: ErrorFamily::ReadersFull => "readersFull",
        SCHEMA: ErrorFamily::Schema => "schema",
        VALIDATION: ErrorFamily::Validation => "validation",
        FACT_SHAPE: ErrorFamily::FactShape => "factShape",
        CLOSED_RELATION_WRITE: ErrorFamily::ClosedRelationWrite => "closedRelationWrite",
        COMMIT_SYNC: ErrorFamily::CommitSync => "commitSync",
        TRANSACTION_POISONED: ErrorFamily::TransactionPoisoned => "transactionPoisoned",
        FOREIGN_PREPARED: ErrorFamily::ForeignPreparedQuery => "foreignPrepared",
        FOREIGN_WITNESS: ErrorFamily::ForeignWitness => "foreignWitness",
        PARAM: ErrorFamily::Param => "param",
        CAPACITY_RAY_MEASURE: ErrorFamily::CapacityRayMeasure => "capacityRayMeasure",
        OVERFLOW: ErrorFamily::Overflow => "overflow",
        SCALAR: ErrorFamily::Scalar => "scalar",
        RESULT_BYTES_OVERFLOW: ErrorFamily::ResultBytesOverflow => "resultBytesOverflow",
        CAPACITY: ErrorFamily::Capacity => "capacity",
        FULL: ErrorFamily::Full => "full",
        CORRUPTION: ErrorFamily::Corruption => "corruption",
        STORE: ErrorFamily::Store => "store",
    }
}
