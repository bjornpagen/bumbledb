use crate::error::{Error, Result};

use super::read_meta::read_u64;
use super::{GenerationId, META_DICT_NEXT_ID, META_TX_ID, WriteTxn};

impl WriteTxn<'_> {
    /// Commits (fsync per LMDB defaults). The write path's one durability
    /// boundary: the errno is parsed here, once ([`Error::from_commit`]).
    ///
    /// # Errors
    ///
    /// [`Error::CommitSync`] when the commit's write/sync path surfaced a
    /// raw OS errno, `Lmdb` on any LMDB-coded failure; either way the
    /// transaction aborted — nothing persists.
    pub fn commit(self) -> Result<()> {
        self.txn.commit().map_err(Error::from_commit)
    }

    /// Advances the storage tx id (reader: the 50-storage doc's commit step 4; the id
    /// advances iff the delta changed logical state).
    pub(crate) fn put_generation(&mut self, generation: GenerationId) -> Result<()> {
        self.env.meta.put(
            &mut self.txn,
            META_TX_ID,
            generation.storage_word().to_le_bytes().as_slice(),
        )?;
        Ok(())
    }

    /// Writes the dictionary next-id counter.
    pub(crate) fn put_dict_next_id(&mut self, next: u64) -> Result<()> {
        self.env.meta.put(
            &mut self.txn,
            META_DICT_NEXT_ID,
            next.to_le_bytes().as_slice(),
        )?;
        Ok(())
    }

    /// The current committed generation as seen by this write transaction.
    ///
    /// # Errors
    ///
    /// `Corruption(MetaMissing)` if the tx-id key is absent,
    /// `Corruption(MalformedValue)` if its value is mis-sized.
    pub fn generation(&self) -> Result<GenerationId> {
        read_u64(&self.env.meta, &self.txn, META_TX_ID, "tx id").map(GenerationId::from_storage)
    }
}
