use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::encoding::{
    DecimalRaw, InternId, TimestampMicros, decode_bool, decode_decimal, decode_enum, decode_i64,
    decode_intern_id, decode_timestamp, decode_u64, encode_bool, encode_decimal, encode_enum,
    encode_i64, encode_intern_id, encode_timestamp, encode_u64,
};
use bumbledb_core::schema::{
    AccessComponent, AccessLayout, ConstraintDescriptor, FieldDescriptor, RelationDescriptor,
    SchemaDescriptor, ValueType,
};

#[cfg(test)]
use crate::storage_schema::FACT_SET_ACCESS_NAME;
use crate::{Error, ReadTxn, RelationId, Result, StorageSchema, WriteTxn};

mod cursor;
mod encoding;
mod keys;
mod read;
mod types;
mod write;

pub(crate) use cursor::EncodedFactCursor;
#[cfg(test)]
use cursor::FactCursor;
pub(crate) use types::EncodedAccessItem;
pub use types::{DeleteOutcome, Fact, InsertOutcome, Value};
#[cfg(test)]
use types::{EncodedComponent, EncodedRange, FactCursorRecord, FieldValues};

use encoding::*;
use keys::*;
use types::{EncodedFact, InternMode};

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
