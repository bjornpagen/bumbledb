use crate::encoding::FactView;
use crate::error::{CorruptionError, Error, Mismatch, Result};
use crate::schema::Schema;
use bumbledb_theory::schema::RelationId;

pub(crate) fn check_width<'bytes, 'layout>(
    schema: &'layout Schema,
    rel: RelationId,
    row_id: u64,
    bytes: &'bytes [u8],
) -> Result<FactView<'bytes, 'layout>> {
    let layout = schema.relation(rel).layout();
    layout.view(bytes).ok_or_else(|| {
        Error::Corruption(CorruptionError::WrongFactWidth {
            relation: rel,
            row_id,
            mismatch: Mismatch {
                witnessed: bytes.len(),
                required: layout.fact_width(),
            },
        })
    })
}
