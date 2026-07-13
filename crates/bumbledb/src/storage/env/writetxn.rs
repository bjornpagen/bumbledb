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

    /// Aborts: drops the transaction, nothing persists. Test-only since
    /// the counters-only flush stopped opening a transaction it might
    /// discard (the live abort path is simply dropping the value).
    #[cfg(test)]
    pub(crate) fn abort(self) {
        drop(self.txn);
    }

    /// Advances the storage tx id (reader: the 40-storage doc's commit step 4; the id
    /// advances iff the delta changed logical state).
    pub(crate) fn put_generation(&mut self, generation: GenerationId) -> Result<()> {
        self.env.meta.put(
            &mut self.txn,
            META_TX_ID,
            generation.storage_word().to_le_bytes().as_slice(),
        )?;
        Ok(())
    }

    /// Reads the dictionary next-id counter (reader: `storage::dict`'s
    /// direct-write intern, test-only since the delta's pending-intern set
    /// re-homed the live path in the 40-storage doc), sentinel-checked
    /// ([`super::read_meta::read_dict_next_id`]).
    #[cfg(test)]
    pub(crate) fn dict_next_id(&self) -> Result<u64> {
        super::read_meta::read_dict_next_id(&self.env.meta, &self.txn)
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
    /// `Corruption(MetaMissing)` if the tx-id key is absent or malformed.
    pub fn generation(&self) -> Result<GenerationId> {
        read_u64(&self.env.meta, &self.txn, META_TX_ID).map(GenerationId::from_storage)
    }
}
