use crate::error::{Error, Result};

use super::read_meta::read_u64;
use super::{GenerationId, META_DICT_NEXT_ID, META_TX_ID, WriteTxn};

impl WriteTxn<'_> {
    /// Commits (fsync per LMDB defaults). The write path's one durability
    /// # Errors
    /// raw OS errno, `Lmdb` on any LMDB-coded failure; either way the
    pub fn commit(self) -> Result<()> {
        self.txn.commit().map_err(Error::from_commit)
    }

    pub(crate) fn put_generation(&mut self, generation: GenerationId) -> Result<()> {
        self.env.meta.put(
            &mut self.txn,
            META_TX_ID,
            generation.storage_word().to_le_bytes().as_slice(),
        )?;
        Ok(())
    }

    pub(crate) fn put_dict_next_id(&mut self, next: u64) -> Result<()> {
        self.env.meta.put(
            &mut self.txn,
            META_DICT_NEXT_ID,
            next.to_le_bytes().as_slice(),
        )?;
        Ok(())
    }

    /// # Errors
    pub fn generation(&self) -> Result<GenerationId> {
        read_u64(&self.env.meta, &self.txn, META_TX_ID, "tx id").map(GenerationId::from_storage)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn dict_next_id(&self) -> Result<u64> {
        super::read_meta::read_dict_next_id(&self.env.meta, &self.txn)
    }
}
