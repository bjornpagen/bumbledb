//! Physical copy of one committed snapshot into a freshly created store —
//! the substrate under `Db::compact` and `Db::from_instance`-style
//! publication. The copy is a storage operation over already-admitted
//! content: every source row was judged at its own commit, the source view
//! is one coherent snapshot (ENG-003), and the destination adopts the whole
//! logical state in one durable transaction — a crash leaves the
//! destination either empty or complete, never half-copied.

use heed::RoTxn;

use super::candidate::RowIndexer;
use super::error::{StoreCorruption, StoreError, StoreResult};
use super::format::{K_ATTACHMENT, K_GENERATION, K_HOST_RECORD_TAG};
use super::keys;
use super::rows;
use super::snapshot::OwnedSnapshot;
use super::store_env::Store;
use crate::work::WorkContext;
use bumbledb_theory::schema::RelationId;

impl Store {
    /// Copy every committed row, host record, the attachment and the
    /// generation of `source` into this store, atomically. The destination
    /// must be freshly created for the same schema and still empty; a
    /// non-empty destination refuses before any write. Map exhaustion
    /// aborts, grows under the exclusive gate, and retries the whole copy —
    /// immutable native work, never a partial adoption.
    /// # Errors
    /// `ForeignSchema` on a schema mismatch, `DestinationExists` when the
    /// destination already holds rows, growth refusals, storage failure or
    /// stopped work.
    pub fn adopt_snapshot(
        &self,
        source: &OwnedSnapshot,
        indexer: &(impl RowIndexer + ?Sized),
        work: &WorkContext,
    ) -> StoreResult<()> {
        if source.schema_fingerprint() != self.inner.schema_fp {
            return Err(StoreError::ForeignSchema);
        }
        let owner = self.writer(work)?;
        loop {
            work.checkpoint()?;
            match copy_attempt(self, source, indexer, work) {
                Err(StoreError::MapFull { .. }) => {
                    // The failed transaction is dropped whole; grow strictly
                    // or return the typed refusal that bounds this loop.
                    self.grow(work, None)?;
                }
                other => {
                    let () = other?;
                    break;
                }
            }
        }
        drop(owner);
        Ok(())
    }
}

fn copy_attempt(
    dest: &Store,
    source: &OwnedSnapshot,
    indexer: &(impl RowIndexer + ?Sized),
    work: &WorkContext,
) -> StoreResult<()> {
    let inner = &dest.inner;
    let mut gated = dest.gated_write_txn(work)?;
    refuse_nonempty(&gated.txn, dest)?;
    // Rows, in the source's physical order; the destination mints fresh
    // local ids (row ids are storage surrogates, never logical identity).
    {
        let source_txn = source.read_txn();
        let prefix = [keys::TAG_ROW];
        let range = source
            .store_inner()
            .data
            .prefix_iter(source_txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, row) = entry.map_err(StoreError::from_heed)?;
            let relation = relation_of_row_key(key)?;
            rows::insert_row(inner, &mut gated.txn, relation, row, indexer, work)?;
        }
    }
    // Host records and the attachment, verbatim from the same source view.
    {
        let source_txn = source.read_txn();
        let prefix = [K_HOST_RECORD_TAG];
        let range = source
            .store_inner()
            .meta
            .prefix_iter(source_txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, value) = entry.map_err(StoreError::from_heed)?;
            work.input(value.len() as u64)?;
            inner
                .meta
                .put(&mut gated.txn, key, value)
                .map_err(super::store_env::map_txn_error)?;
        }
        if let Some(attachment) = source.attachment()? {
            work.input(attachment.len() as u64)?;
            inner
                .meta
                .put(&mut gated.txn, K_ATTACHMENT, attachment)
                .map_err(super::store_env::map_txn_error)?;
        }
    }
    // The copied state names the generation it was exported at.
    inner
        .meta
        .put(
            &mut gated.txn,
            K_GENERATION,
            &source.generation().storage_word().to_be_bytes(),
        )
        .map_err(super::store_env::map_txn_error)?;
    gated.commit()
}

fn refuse_nonempty(txn: &RoTxn<'_, heed::AnyTls>, dest: &Store) -> StoreResult<()> {
    let prefix = [keys::TAG_ROW];
    let mut range = dest
        .inner
        .data
        .prefix_iter(txn, prefix.as_slice())
        .map_err(StoreError::from_heed)?;
    if range.next().is_some() {
        return Err(StoreError::DestinationExists {
            path: dest.path().to_path_buf(),
        });
    }
    Ok(())
}

fn relation_of_row_key(key: &[u8]) -> StoreResult<RelationId> {
    if key.len() != keys::ROW_KEY_LEN {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "row key width",
        )));
    }
    let relation: [u8; 4] = key[1..5]
        .try_into()
        .map_err(|_| StoreError::Corruption(StoreCorruption::MalformedKey("row key relation")))?;
    Ok(RelationId(u32::from_be_bytes(relation)))
}
