use crate::encoding::{TypeDesc, field_bytes};
use crate::error::CorruptionError;
use crate::schema::Schema;
use bumbledb_theory::schema::{FieldId, RelationId};

/// One field's value sliced straight out of canonical fact bytes, in
/// column-word form: a scalar's byte-order-normalized word (1-byte columns
/// widen — bool/enum ordinals compare faithfully as words), an interval
/// field's `(start, end)` word pair, or a `bytes<N > 8>` field's padded
/// word block (a bytes<N ≤ 8> field is one word, like every scalar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FactOperand {
    Word(u64),
    Pair(u64, u64),
    /// A multi-word `bytes<N>` value: `count` padded words in byte order.
    Block {
        words: [u64; 8],
        count: u8,
    },
}

/// Reads a field's [`FactOperand`] from canonical fact bytes. Dispatch is
/// on the field's *type*, never its byte width — a bytes<16> field and an
/// interval field are both 16 bytes with different shapes.
///
/// # Errors
///
/// [`CorruptionError::InvalidFixedIntervalStart`] on a fixed-width
/// interval field whose stored start sits at or past the Q2 bound — the
/// derived end would reach the ceiling (the ray sentinel, unconstructible
/// in the fixed family) or overflow. Hard error, never a skip, never a
/// classification: the same conviction the image lane's
/// [`crate::encoding::decode_fixed_interval_start`] routing delivers.
pub(crate) fn fact_operand(
    schema: &Schema,
    relation: RelationId,
    fact: &[u8],
    field: FieldId,
) -> Result<FactOperand, CorruptionError> {
    let layout = schema.relation(relation).layout();
    let bytes = field_bytes(fact, layout, usize::from(field.0));
    // The field's whole words, width carried by `as_chunks`'s type — a
    // scalar is one chunk, an interval two, a `bytes<N>` block `⌈N/8⌉`.
    let (word_bytes, _) = bytes.as_chunks::<8>();
    let word_at = |i: usize| u64::from_be_bytes(word_bytes[i]);
    Ok(match layout.field_type(usize::from(field.0)) {
        TypeDesc::Bool => FactOperand::Word(u64::from(bytes[0])),
        TypeDesc::U64 | TypeDesc::I64 | TypeDesc::String => FactOperand::Word(word_at(0)),
        TypeDesc::FixedBytes { len } => {
            let count = crate::encoding::fixed_bytes_words(len);
            if count == 1 {
                FactOperand::Word(word_at(0))
            } else {
                let mut words = [0u64; 8];
                for (slot, &chunk) in words[..count].iter_mut().zip(word_bytes) {
                    *slot = u64::from_be_bytes(chunk);
                }
                FactOperand::Block {
                    words,
                    count: u8::try_from(count).expect("at most 8 words"),
                }
            }
        }
        TypeDesc::Interval { width: None, .. } => FactOperand::Pair(word_at(0), word_at(1)),
        // A fixed-width field stores one word; the end re-derives from the
        // TYPE's width through the one shared decoder, which convicts the
        // at-bound AND overflow starts as corruption (Q2's bound holds at
        // rest too — corrupt stored bytes never reach classification).
        TypeDesc::Interval { width: Some(w), .. } => {
            let (start, end) = crate::encoding::decode_fixed_interval_start(word_bytes[0], w)?;
            FactOperand::Pair(start, end)
        }
    })
}

/// A scalar field's column word (the direct decode lane's reader).
///
/// # Errors
///
/// [`CorruptionError`] as [`fact_operand`] (unreachable for the scalar
/// fields this reader serves, but the conviction stays in the type).
///
/// # Panics
///
/// On a programmer-invariant violation: a multi-word field (its readers go
/// through [`fact_operand`]).
pub(crate) fn fact_word(
    schema: &Schema,
    relation: RelationId,
    fact: &[u8],
    field: FieldId,
) -> Result<u64, CorruptionError> {
    match fact_operand(schema, relation, fact, field)? {
        FactOperand::Word(word) => Ok(word),
        FactOperand::Pair(..) | FactOperand::Block { .. } => {
            unreachable!("multi-word fields decode as pairs or blocks")
        }
    }
}
