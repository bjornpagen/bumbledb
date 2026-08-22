//! LMDB environment, keys, dictionary, delta write path, and reads.

use crate::error::{CorruptionError, Error, Result};

#[cfg_attr(not(test), allow(dead_code))]
pub mod catalog;
pub mod commit;
pub mod delta;
pub mod dict;
pub mod env;
pub mod keys;
pub mod read;

pub(crate) fn stored_u64(bytes: &[u8], what: &'static str) -> Result<u64> {
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
        Error::Corruption(CorruptionError::MalformedValue(what))
    })?))
}
