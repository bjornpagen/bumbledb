//! Fresh-destination capability and snapshot adoption (CORE-015/CORE-016).

use heed::types::Bytes;
use heed::{Database, PutFlags, RoTxn, RwTxn};

use super::candidate::RowIndexer;
use super::error::{StoreCorruption, StoreError, StoreResult};
use super::format::{
    K_ATTACHMENT, K_GENERATION, K_HOST_RECORD_TAG, K_RELATION_VERSION_TAG, K_STORE_ID,
};
use super::keys;
use super::rows;
use super::snapshot::OwnedSnapshot;
use super::store_env::Store;
use crate::storage::GenerationId;
use crate::work::WorkContext;

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
    /// Same-format maintenance copy, not logical snapshot adoption. Preserve
    /// row surrogates and every physical index, packing each tree in ascending
    /// key order. Rebuilding indexes row-by-row interleaves ordered insertions
    /// before other namespaces and leaves half-full LMDB pages.
    #[cfg_attr(
        not(any(test, feature = "collision-probe")),
        expect(
            clippy::needless_pass_by_value,
            reason = "The fresh-destination capability must be consumed, not borrowed"
        )
    )]
    pub(crate) fn compact_snapshot(
        &self,
        source: &OwnedSnapshot,
        fresh: FreshDestination,
        work: &WorkContext,
    ) -> StoreResult<()> {
        if source.schema_fingerprint() != self.inner.schema_fp {
            return Err(StoreError::ForeignSchema);
        }
        // Production has one format-fixed fingerprint policy. Forced-collision
        // stores are test-only and need logical reindexing when policies differ.
        #[cfg(any(test, feature = "collision-probe"))]
        if !same_fingerprinter(source.store_inner().fingerprinter, self.inner.fingerprinter) {
            return self.adopt_snapshot(source, fresh, &super::UnindexedRows, work);
        }
        let FreshDestination(FreshDestinationToken) = fresh;
        let owner = self.writer(work)?;
        loop {
            work.checkpoint()?;
            match compact_attempt(self, source, work) {
                Err(StoreError::MapFull { .. }) => {
                    self.grow(work, None)?;
                }
                result => {
                    result?;
                    break;
                }
            }
        }
        drop(owner);
        Ok(())
    }

    /// Copy every committed row, host record, attachment and generation of
    /// `source` into this store atomically. Requires a [`FreshDestination`]
    /// or complete metadata emptiness — zero rows alone is insufficient
    /// (CORE-015).
    /// # Errors
    /// `ForeignSchema`, `DestinationExists`, growth refusals, storage failure.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "The fresh-destination capability must be consumed, not borrowed"
    )]
    pub fn adopt_snapshot(
        &self,
        source: &OwnedSnapshot,
        fresh: FreshDestination,
        indexer: &(impl RowIndexer + ?Sized),
        work: &WorkContext,
    ) -> StoreResult<()> {
        let FreshDestination(FreshDestinationToken) = fresh;
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

#[cfg(any(test, feature = "collision-probe"))]
fn same_fingerprinter(left: super::Fingerprinter, right: super::Fingerprinter) -> bool {
    match (left, right) {
        (super::Fingerprinter::Blake3, super::Fingerprinter::Blake3) => true,
        (super::Fingerprinter::Constant(left), super::Fingerprinter::Constant(right)) => {
            left == right
        }
        _ => false,
    }
}

fn compact_attempt(dest: &Store, source: &OwnedSnapshot, work: &WorkContext) -> StoreResult<()> {
    let inner = &dest.inner;
    let mut gated = dest.gated_write_txn(work)?;
    refuse_nonempty_destination(&gated.txn, dest)?;
    // APPEND requires the entire target tree to be empty, not merely its row
    // namespace. Refuse orphan indexes too, even with a fresh-create token.
    if !inner
        .data
        .is_empty(&gated.txn)
        .map_err(StoreError::from_heed)?
    {
        return Err(StoreError::DestinationExists {
            path: dest.path().to_path_buf(),
        });
    }
    let source_inner = source.store_inner();
    let source_txn = source.read_txn();
    for entry in source_inner
        .data
        .iter(source_txn)
        .map_err(StoreError::from_heed)?
    {
        let (key, value) = entry.map_err(StoreError::from_heed)?;
        if key.first() == Some(&keys::TAG_ROW) {
            work.rows(1)?;
        }
        append_entry(*inner.data, &mut gated.txn, key, value, work)?;
    }
    // Repack metadata as well. Source and destination were opened/created with
    // the same format and schema. Copy all state, including future metadata
    // families, but keep the destination's newly minted store identity.
    inner
        .meta
        .clear(&mut gated.txn)
        .map_err(super::store_env::map_txn_error)?;
    for entry in source_inner
        .meta
        .iter(source_txn)
        .map_err(StoreError::from_heed)?
    {
        let (key, value) = entry.map_err(StoreError::from_heed)?;
        let value = if key == K_STORE_ID {
            inner.identity.store.0.as_slice()
        } else {
            value
        };
        append_entry(inner.meta, &mut gated.txn, key, value, work)?;
    }
    work.checkpoint()?;
    gated.commit()
}

/// No owned row buffers: small entries copy directly, overflow-sized values
/// fill LMDB's reserved space in bounded chunks with typed cancellation.
fn append_entry<C>(
    db: Database<Bytes, Bytes, C>,
    txn: &mut RwTxn<'_>,
    key: &[u8],
    value: &[u8],
    work: &WorkContext,
) -> StoreResult<()> {
    use std::io::Write as _;
    work.step(1)?;
    work.input(key.len() as u64)?;
    work.input(value.len() as u64)?;
    if value.len() <= rows::BYTE_QUANTUM {
        work.step(value.len() as u64)?;
        return db
            .put_with_flags(txn, PutFlags::APPEND, key, value)
            .map_err(super::store_env::map_txn_error);
    }
    let mut stopped = None;
    let result =
        db.get_or_put_reserved_with_flags(txn, PutFlags::APPEND, key, value.len(), |space| {
            for chunk in value.chunks(rows::BYTE_QUANTUM) {
                work.step(chunk.len() as u64).map_err(|error| {
                    stopped = Some(error);
                    std::io::Error::from(std::io::ErrorKind::Interrupted)
                })?;
                space.write_all(chunk)?;
            }
            Ok(())
        });
    if let Some(error) = stopped {
        return Err(StoreError::Work(error));
    }
    if result.map_err(super::store_env::map_txn_error)?.is_some() {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "compaction entries are not strictly ordered",
        )));
    }
    Ok(())
}

fn copy_attempt(
    dest: &Store,
    source: &OwnedSnapshot,
    indexer: &(impl RowIndexer + ?Sized),
    work: &WorkContext,
) -> StoreResult<()> {
    let inner = &dest.inner;
    let gated = dest.gated_write_txn(work)?;
    refuse_nonempty_destination(&gated.txn, dest)?;
    let mut writer = rows::RowWriter::new(inner, gated, work);
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
            let (relation, locator) = source.store_inner().keys.decode_row(key)?;
            if locator.home().len() != source.store_inner().det.home_width(relation) {
                return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                    "copy row home width",
                )));
            }
            writer.insert(relation, row, indexer)?;
        }
    }
    let mut gated = writer.finish()?;
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

/// Complete relevant metadata/data emptiness before adoption.
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
