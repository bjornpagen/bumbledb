//! LMDB environment, keys, dictionary, delta write path, and reads (docs/architecture).

use crate::error::{CorruptionError, Error, Result};

pub mod commit;
pub mod delta;
pub mod dict;
pub mod env;
pub mod keys;
pub mod read;

/// Decodes one stored little-endian u64 value — row ids (`M`/`U`),
/// counters (`S`), serial next-values (`Q`). Any other width is typed
/// corruption; `what` names the failing shape,
/// [`CorruptionError::MalformedValue`]-style.
pub(crate) fn stored_u64(bytes: &[u8], what: &'static str) -> Result<u64> {
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
        Error::Corruption(CorruptionError::MalformedValue(what))
    })?))
}
