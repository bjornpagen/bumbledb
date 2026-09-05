use crate::encoding::{FactView, ValueType};
use crate::error::CorruptionError;
use bumbledb_theory::schema::FieldId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FactOperand {
    Word(u64),
    Pair(u64, u64),

    Block { words: [u64; 8], count: u8 },
}

/// # Errors
pub(crate) fn fact_operand(
    fact: FactView<'_, '_>,
    field: FieldId,
) -> Result<FactOperand, CorruptionError> {
    let bytes = crate::encoding::field_bytes(fact, usize::from(field.0));

    let (word_bytes, _) = bytes.as_chunks::<8>();
    let word_at = |i: usize| u64::from_be_bytes(word_bytes[i]);
    Ok(match fact.layout().field_type(usize::from(field.0)) {
        ValueType::Bool => FactOperand::Word(u64::from(bytes[0])),
        ValueType::U64 | ValueType::I64 | ValueType::String => FactOperand::Word(word_at(0)),
        ValueType::F64 => {
            FactOperand::Word(crate::encoding::decode_f64(word_bytes[0])?.to_order_key())
        }
        ValueType::FixedBytes { len } => {
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
        ValueType::Interval { .. } => FactOperand::Pair(word_at(0), word_at(1)),

        ValueType::FixedInterval { width: w, .. } => {
            let (start, end) = crate::encoding::decode_fixed_interval_start(word_bytes[0], w)
                .map_err(CorruptionError::from)?;
            FactOperand::Pair(start, end)
        }
    })
}

/// # Errors
/// # Panics
/// On a programmer-invariant violation: a multi-word field (its readers go
/// through [`fact_operand`]).
pub(crate) fn fact_word(fact: FactView<'_, '_>, field: FieldId) -> Result<u64, CorruptionError> {
    match fact_operand(fact, field)? {
        FactOperand::Word(word) => Ok(word),
        FactOperand::Pair(..) | FactOperand::Block { .. } => {
            unreachable!("multi-word fields decode as pairs or blocks")
        }
    }
}
