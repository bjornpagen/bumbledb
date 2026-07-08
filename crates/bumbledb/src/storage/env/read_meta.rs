use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn};

use crate::error::{CorruptionError, Error, Result};

pub(super) fn read_u64(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
    key: &[u8],
) -> Result<u64> {
    let bytes: [u8; 8] = meta
        .get(rtxn, key)?
        .and_then(|b| b.try_into().ok())
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
    Ok(u64::from_le_bytes(bytes))
}
