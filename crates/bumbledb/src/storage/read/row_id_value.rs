use crate::error::Result;
use crate::storage::stored_u64;

pub(super) fn row_id_value(value: Option<&[u8]>) -> Result<Option<u64>> {
    value
        .map(|bytes| stored_u64(bytes, "M/U row id"))
        .transpose()
}
