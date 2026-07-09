use crate::encoding::field_bytes;
use crate::schema::{FieldId, RelationId, Schema};

/// One field's value sliced straight out of canonical fact bytes, in
/// column-word form: a scalar's byte-order-normalized word (1-byte columns
/// widen — bool/enum ordinals compare faithfully as words), or an interval
/// field's `(start, end)` word pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FactOperand {
    Word(u64),
    Pair(u64, u64),
}

/// Reads a field's [`FactOperand`] from canonical fact bytes.
pub(crate) fn fact_operand(
    schema: &Schema,
    relation: RelationId,
    fact: &[u8],
    field: FieldId,
) -> FactOperand {
    let layout = schema.relation(relation).layout();
    let bytes = field_bytes(fact, layout, usize::from(field.0));
    match bytes.len() {
        1 => FactOperand::Word(u64::from(bytes[0])),
        8 => FactOperand::Word(u64::from_be_bytes(bytes.try_into().expect("8-byte field"))),
        _ => FactOperand::Pair(
            u64::from_be_bytes(bytes[..8].try_into().expect("16-byte field")),
            u64::from_be_bytes(bytes[8..].try_into().expect("16-byte field")),
        ),
    }
}

/// A scalar field's column word (the direct decode lane's reader).
///
/// # Panics
///
/// On a programmer-invariant violation: an interval field (its readers go
/// through [`fact_operand`] for the word pair).
pub(crate) fn fact_word(schema: &Schema, relation: RelationId, fact: &[u8], field: FieldId) -> u64 {
    match fact_operand(schema, relation, fact, field) {
        FactOperand::Word(word) => word,
        FactOperand::Pair(..) => unreachable!("interval fields decode as word pairs"),
    }
}
