use crate::error::{CorruptionError, Error, Result};

use super::read_meta::read_u64;
use super::{ReadTxn, META_DICT_NEXT_ID, META_TX_ID};

impl ReadTxn<'_> {
    /// The reader's generation: the storage tx id read from `_meta` *inside
    /// this snapshot* — never an in-process counter. This is the
    /// race-closing rule of `docs/architecture/40-storage.md`; the 40-storage doc keys
    /// the image cache on it.
    ///
    /// # Errors
    ///
    /// `Corruption(MetaMissing)` if the tx-id key is absent or malformed.
    pub fn generation(&self) -> Result<u64> {
        if let Some(g) = self.generation.get() {
            return Ok(*g);
        }
        let g = read_u64(&self.env.meta, &self.txn, META_TX_ID)?;
        Ok(*self.generation.get_or_init(|| g))
    }

    /// The committed dictionary next-id as of this snapshot (reader: the
    /// delta's lazy pending-intern counter). A stored `u64::MAX` — the
    /// miss sentinel, never mintable — is corrupt data, typed.
    pub(crate) fn dict_next_id(&self) -> Result<u64> {
        let next = read_u64(&self.env.meta, &self.txn, META_DICT_NEXT_ID)?;
        if next == u64::MAX {
            return Err(Error::Corruption(CorruptionError::MalformedValue(
                "dict next id",
            )));
        }
        Ok(next)
    }
}
