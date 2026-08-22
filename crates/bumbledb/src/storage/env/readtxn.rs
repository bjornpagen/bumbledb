use crate::error::Result;

#[cfg(test)]
use super::read_meta::read_fingerprint;
use super::read_meta::{read_dict_next_id, read_u64};
use super::{GenerationId, META_TX_ID, ReadTxn};

impl ReadTxn<'_> {

    /// # Errors

    pub fn generation(&self) -> Result<GenerationId> {
        if let Some(g) = self.generation.get() {
            return Ok(*g);
        }
        let g =
            GenerationId::from_storage(read_u64(&self.env.meta, &self.txn, META_TX_ID, "tx id")?);
        Ok(*self.generation.get_or_init(|| g))
    }

    pub(crate) fn dict_next_id(&self) -> Result<u64> {
        read_dict_next_id(&self.env.meta, &self.txn)
    }

    #[cfg(test)]
    pub(crate) fn stored_fingerprint(&self) -> Result<[u8; 32]> {
        read_fingerprint(&self.env.meta, &self.txn)
    }
}
