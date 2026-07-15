use crate::encoding::{encode_bool, encode_i64};
use crate::image::view::Const;
use crate::ir::Value;

/// Lowers a literal into column-form constant representation. String
/// stays raw bytes (`PendingIntern`) — resolution to an intern-id word
/// happens per execution, where a dictionary miss means an empty result.
/// A `bytes<N>` literal is self-encoding: its padded canonical bytes read
/// as big-endian column words — one `Word` for N ≤ 8, a `Words` span
/// otherwise — with zero dictionary traffic ever. Interval literals lower
/// to their two encoded column words (each half exactly as the scalar of
/// its element type, so u64 word order is value order —
/// `docs/architecture/50-storage.md`).
pub(crate) fn lower_literal(value: &Value) -> Const {
    match value {
        Value::Bool(b) => Const::Byte(encode_bool(*b)),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(i64_word(*v)),
        Value::String(bytes) => Const::PendingIntern {
            bytes: bytes.clone(),
        },
        Value::FixedBytes(raw) => fixed_bytes_const(raw),
        Value::IntervalU64(interval) => Const::Interval {
            start: interval.start(),
            end: interval.end(),
        },
        Value::IntervalI64(interval) => Const::Interval {
            start: i64_word(interval.start()),
            end: i64_word(interval.end()),
        },
        // A mask literal is only ever legal inside `CmpOp::Allen`'s mask
        // position, which lowers through `MaskConst`, never through here.
        Value::AllenMask(_) => unreachable!("validated: mask values are not terms"),
    }
}

/// A `bytes<N>` value's column-form constant: the padded words (readers:
/// this lowering — prepare-time, allocation sanctioned). The bind path
/// resolves through [`fixed_bytes_word_buf`] instead, writing into
/// pooled slots (the allocation contract's steady-state clause).
pub(crate) fn fixed_bytes_const(raw: &[u8]) -> Const {
    let (words, count) = fixed_bytes_word_buf(raw);
    match count {
        1 => Const::Word(words[0]),
        n => Const::Words(words[..n].into()),
    }
}

/// A `bytes<N>` value's `⌈N/8⌉` column words in a fixed buffer — the
/// padded canonical bytes as big-endian words, exactly what the image's
/// word columns hold, with zero heap traffic (8 words is the validated
/// 64-byte ceiling). Returns the buffer and the span's word count.
pub(crate) fn fixed_bytes_word_buf(raw: &[u8]) -> ([u64; 8], usize) {
    debug_assert!(
        raw.len() <= crate::encoding::MAX_FIXED_BYTES,
        "bytes<N> widths are 1..=64"
    );
    let mut words = [0u64; 8];
    let mut count = 0;
    for (word, chunk) in words.iter_mut().zip(raw.chunks(8)) {
        let mut padded = [0u8; 8];
        padded[..chunk.len()].copy_from_slice(chunk);
        *word = u64::from_be_bytes(padded);
        count += 1;
    }
    (words, count)
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
