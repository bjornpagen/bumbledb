use crate::encoding::{encode_bool, encode_i64};
use crate::image::view::Const;
use crate::ir::Value;
use crate::storage::dict::{TAG_BYTES, TAG_STRING};

/// Lowers a literal into column-form constant representation. String/Bytes
/// stay raw bytes (`PendingIntern`) — resolution to intern-id words happens
/// per execution, where a dictionary miss means an empty result.
pub(super) fn lower_literal(value: &Value) -> Const {
    match value {
        Value::Bool(b) => Const::Byte(encode_bool(*b)),
        Value::Enum(ordinal) => Const::Byte(*ordinal),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(u64::from_be_bytes(encode_i64(*v))),
        Value::String(bytes) => Const::PendingIntern {
            tag: TAG_STRING,
            bytes: bytes.clone(),
        },
        Value::Bytes(bytes) => Const::PendingIntern {
            tag: TAG_BYTES,
            bytes: bytes.clone(),
        },
    }
}
