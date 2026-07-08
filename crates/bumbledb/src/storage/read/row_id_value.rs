use crate::error::{CorruptionError, Error, Result};

pub(super) fn row_id_value(value: Option<&[u8]>) -> Result<Option<u64>> {
    match value {
        None => Ok(None),
        Some(bytes) => Ok(Some(u64::from_le_bytes(bytes.try_into().map_err(
            |_| Error::Corruption(CorruptionError::MalformedValue("M/U row id")),
        )?))),
    }
}
