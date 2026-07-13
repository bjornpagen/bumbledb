use crate::error::Result;

use super::read_meta::{read_dict_next_id, read_u64};
use super::{META_TX_ID, ReadTxn};

impl ReadTxn<'_> {
    /// The reader's generation: the storage tx id read from `_meta` *inside
    /// this snapshot* — never an in-process counter. This is the
    /// race-closing rule of `docs/architecture/50-storage.md`; the 40-storage doc keys
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
    /// delta's lazy pending-intern counter), sentinel-checked
    /// ([`read_dict_next_id`]).
    pub(crate) fn dict_next_id(&self) -> Result<u64> {
        read_dict_next_id(&self.env.meta, &self.txn)
    }
}
