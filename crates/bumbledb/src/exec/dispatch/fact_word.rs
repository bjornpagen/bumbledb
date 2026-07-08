use super::GuardPlan;
use crate::encoding::field_bytes;
use crate::schema::{FieldId, Schema};

/// One field's column word sliced straight out of canonical fact bytes.
pub(crate) fn fact_word(schema: &Schema, plan: &GuardPlan, fact: &[u8], field: FieldId) -> u64 {
    let layout = schema.relation(plan.relation).layout();
    let bytes = field_bytes(fact, layout, usize::from(field.0));
    match bytes.len() {
        1 => u64::from(bytes[0]),
        _ => u64::from_be_bytes(bytes.try_into().expect("8-byte field")),
    }
}
