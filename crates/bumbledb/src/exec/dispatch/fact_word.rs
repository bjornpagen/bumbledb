use crate::encoding::{TypeDesc, field_bytes};
use crate::schema::{FieldId, RelationId, Schema};

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
pub(crate) fn fact_operand(
    schema: &Schema,
    relation: RelationId,
    fact: &[u8],
    field: FieldId,
) -> FactOperand {
    let layout = schema.relation(relation).layout();
    let bytes = field_bytes(fact, layout, usize::from(field.0));
    // The field's whole words, width carried by `as_chunks`'s type — a
    // scalar is one chunk, an interval two, a `bytes<N>` block `⌈N/8⌉`.
    let (word_bytes, _) = bytes.as_chunks::<8>();
    let word_at = |i: usize| u64::from_be_bytes(word_bytes[i]);
    match layout.field_type(usize::from(field.0)) {
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
        TypeDesc::Interval { .. } => FactOperand::Pair(word_at(0), word_at(1)),
    }
}

/// A scalar field's column word (the direct decode lane's reader).
///
/// # Panics
///
/// On a programmer-invariant violation: a multi-word field (its readers go
/// through [`fact_operand`]).
pub(crate) fn fact_word(schema: &Schema, relation: RelationId, fact: &[u8], field: FieldId) -> u64 {
    match fact_operand(schema, relation, fact, field) {
        FactOperand::Word(word) => word,
        FactOperand::Pair(..) | FactOperand::Block { .. } => {
            unreachable!("multi-word fields decode as pairs or blocks")
        }
    }
}
