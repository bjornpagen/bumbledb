//! Fresh-destination capability and snapshot adoption (CORE-015/CORE-016).

use heed::RoTxn;

use super::candidate::RowIndexer;
use super::error::{StoreCorruption, StoreError, StoreResult};
use super::format::{
    K_ATTACHMENT, K_GENERATION, K_HOST_RECORD_TAG, K_NEXT_ROW_ID, K_RELATION_VERSION_TAG,
};
use super::keys;
use super::rows;
use super::snapshot::OwnedSnapshot;
use super::store_env::Store;
use crate::storage::GenerationId;
use crate::work::WorkContext;
use bumbledb_theory::schema::RelationId;

/// Unforgeable proof that a store was freshly created for adoption (chapter
/// 61). Only [`Store::create`] mints this; snapshot adoption consumes it.
#[derive(Debug)]
pub struct FreshDestination(FreshDestinationToken);

#[derive(Debug)]
pub(crate) struct FreshDestinationToken;

impl FreshDestination {
    pub(crate) fn mint() -> Self {
        Self(FreshDestinationToken)
    }
}

impl Store {
    /// Copy every committed row, host record, attachment and generation of
    /// `source` into this store atomically. Requires a [`FreshDestination`]
    /// or complete metadata emptiness — zero rows alone is insufficient
    /// (CORE-015).
    /// # Errors
    /// `ForeignSchema`, `DestinationExists`, growth refusals, storage failure.
    pub fn adopt_snapshot(
        &self,
        source: &OwnedSnapshot,
        fresh: FreshDestination,
        indexer: &(impl RowIndexer + ?Sized),
        work: &WorkContext,
    ) -> StoreResult<()> {
        let _fresh = fresh;
        if source.schema_fingerprint() != self.inner.schema_fp {
            return Err(StoreError::ForeignSchema);
        }
        let owner = self.writer(work)?;
        loop {
            work.checkpoint()?;
            match copy_attempt(self, source, indexer, work) {
                Err(StoreError::MapFull { .. }) => {
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

    /// Adopt when the destination has no private create capability: every
    /// metadata family is checked under the writer. Zero facts is not enough.
    ///
    /// # Errors
    /// As [`Self::adopt_snapshot`].
    pub fn adopt_vacant_snapshot(
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
    refuse_nonempty_destination(&gated.txn, dest)?;
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
    {
        let source_txn = source.read_txn();
        let prefix = [K_RELATION_VERSION_TAG];
        let range = source
            .store_inner()
            .meta
            .prefix_iter(source_txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, value) = entry.map_err(StoreError::from_heed)?;
            inner
                .meta
                .put(&mut gated.txn, key, value)
                .map_err(super::store_env::map_txn_error)?;
        }
    }
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

/// Complete relevant metadata/data emptiness before adoption (CORE-015).
fn refuse_nonempty_destination(txn: &RoTxn<'_, heed::AnyTls>, dest: &Store) -> StoreResult<()> {
    refuse_any_rows(txn, dest)?;
    refuse_host_history(txn, dest)?;
    refuse_attachment(txn, dest)?;
    refuse_advanced_generation(txn, dest)?;
    refuse_relation_versions(txn, dest)?;
    Ok(())
}

fn refuse_any_rows(txn: &RoTxn<'_, heed::AnyTls>, dest: &Store) -> StoreResult<()> {
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

fn refuse_host_history(txn: &RoTxn<'_, heed::AnyTls>, dest: &Store) -> StoreResult<()> {
    let prefix = [K_HOST_RECORD_TAG];
    let mut range = dest
        .inner
        .meta
        .prefix_iter(txn, prefix.as_slice())
        .map_err(StoreError::from_heed)?;
    if range.next().is_some() {
        return Err(StoreError::DestinationExists {
            path: dest.path().to_path_buf(),
        });
    }
    Ok(())
}

fn refuse_attachment(txn: &RoTxn<'_, heed::AnyTls>, dest: &Store) -> StoreResult<()> {
    if dest
        .inner
        .meta
        .get(txn, K_ATTACHMENT)
        .map_err(StoreError::from_heed)?
        .is_some()
    {
        return Err(StoreError::DestinationExists {
            path: dest.path().to_path_buf(),
        });
    }
    Ok(())
}

fn refuse_advanced_generation(txn: &RoTxn<'_, heed::AnyTls>, dest: &Store) -> StoreResult<()> {
    let generation = super::store_env::read_generation(&dest.inner, txn)?;
    if generation != GenerationId::initial() {
        return Err(StoreError::DestinationExists {
            path: dest.path().to_path_buf(),
        });
    }
    Ok(())
}

fn refuse_relation_versions(txn: &RoTxn<'_, heed::AnyTls>, dest: &Store) -> StoreResult<()> {
    let prefix = [K_RELATION_VERSION_TAG];
    let mut range = dest
        .inner
        .meta
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
