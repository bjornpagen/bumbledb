use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Schema};

pub(super) fn check_width(
    schema: &Schema,
    rel: RelationId,
    row_id: u64,
    bytes: &[u8],
) -> Result<()> {
    let expected = schema.relation(rel).layout().fact_width();
    if bytes.len() == expected {
        Ok(())
    } else {
        Err(Error::Corruption(CorruptionError::WrongFactWidth {
            relation: rel,
            row_id,
            expected,
            actual: bytes.len(),
        }))
    }
}
