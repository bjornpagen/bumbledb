use crate::encoding::{encode_bool, encode_i64};
use crate::image::view::Const;
use crate::ir::Value;

pub(crate) fn lower_literal(value: &Value) -> Const {
    match value {
        Value::Bool(b) => Const::Byte(encode_bool(*b)),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(i64_word(*v)),
        Value::String(text) => Const::PendingIntern {
            bytes: Box::from(text.as_bytes()),
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
    }
}

pub(crate) fn fixed_bytes_const(raw: &[u8]) -> Const {
    let (words, count) = fixed_bytes_word_buf(raw);
    match count {
        1 => Const::Word(words[0]),
        n => Const::Words(words[..n].into()),
    }
}

/// A `bytes<N>` value's `⌈N/8⌉` column words in a fixed buffer — the padded
/// canonical bytes as big-endian words, exactly what the image's word columns
/// hold, with zero heap traffic (8 words is the validated 64-byte ceiling;
/// [`crate::encoding::FixedBytesValue`] is a stack `Copy` type, and its
/// `padded` is the zero-pad law's one owner — every chunk is exactly 8 bytes by
/// the padded-length invariant).
pub(crate) fn fixed_bytes_word_buf(raw: &[u8]) -> ([u64; 8], usize) {
    let value = crate::encoding::FixedBytesValue::new(raw);
    let mut words = [0u64; 8];
    let mut count = 0;
    for (word, chunk) in words.iter_mut().zip(value.padded().as_chunks::<8>().0) {
        *word = u64::from_be_bytes(*chunk);
        count += 1;
    }
    (words, count)
}

/// # Panics
/// Only on programmer-invariant violations already excluded by validation (a
/// non-element literal in a point position).
pub(super) fn point_word(value: &Value) -> u64 {
    match value {
        Value::U64(v) => *v,
        Value::I64(v) => i64_word(*v),
        _ => unreachable!("validated: interval points are U64/I64"),
    }
}

fn i64_word(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

#[cfg(test)]
mod tests {
    use super::fixed_bytes_word_buf;

    #[test]
    fn word_buf_matches_the_padded_chunking() {
        for len in [1usize, 7, 8, 9, 16, 63, 64] {
            let raw: Vec<u8> = (0..len)
                .map(|i| u8::try_from(i % 251).expect("small").wrapping_add(1))
                .collect();
            let mut expected = Vec::new();
            for chunk in raw.chunks(8) {
                let mut padded = [0u8; 8];
                padded[..chunk.len()].copy_from_slice(chunk);
                expected.push(u64::from_be_bytes(padded));
            }
            let (words, count) = fixed_bytes_word_buf(&raw);
            assert_eq!(count, expected.len(), "len {len}");
            assert_eq!(&words[..count], &expected[..], "len {len}");
        }
    }
}
