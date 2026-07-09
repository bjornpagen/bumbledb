use crate::encoding::{encode_bool, encode_i64};
use crate::image::view::Const;
use crate::ir::Value;
use crate::storage::dict::{TAG_BYTES, TAG_STRING};

/// Lowers a literal into column-form constant representation. String/Bytes
/// stay raw bytes (`PendingIntern`) — resolution to intern-id words happens
/// per execution, where a dictionary miss means an empty result. Interval
/// literals lower to their two encoded column words (each half exactly as
/// the scalar of its element type, so u64 word order is value order —
/// `docs/architecture/50-storage.md`).
pub(crate) fn lower_literal(value: &Value) -> Const {
    match value {
        Value::Bool(b) => Const::Byte(encode_bool(*b)),
        Value::Enum(ordinal) => Const::Byte(*ordinal),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(i64_word(*v)),
        Value::String(bytes) => Const::PendingIntern {
            tag: TAG_STRING,
            bytes: bytes.clone(),
        },
        Value::Bytes(bytes) => Const::PendingIntern {
            tag: TAG_BYTES,
            bytes: bytes.clone(),
        },
        Value::IntervalU64(start, end) => Const::Interval {
            start: *start,
            end: *end,
        },
        Value::IntervalI64(start, end) => Const::Interval {
            start: i64_word(*start),
            end: i64_word(*end),
        },
    }
}

/// The column word of a point literal — the interval element domain: U64
/// raw, I64 sign-flip-biased (readers: the membership lowerings, which
/// need the bare word for [`crate::image::view::ResolvedWordSource`]).
///
/// # Panics
///
/// Only on programmer-invariant violations already excluded by validation
/// (a non-element literal in a point position).
pub(super) fn point_word(value: &Value) -> u64 {
    match value {
        Value::U64(v) => *v,
        Value::I64(v) => i64_word(*v),
        _ => unreachable!("validated: interval points are U64/I64"),
    }
}

/// The biased I64 column word (u64 word order equals i64 value order).
fn i64_word(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}
