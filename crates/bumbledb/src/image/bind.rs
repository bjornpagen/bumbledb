//! Image binding: epoch + lazy image, generic over frozen or store.
use std::sync::Arc;

use super::RelationImage;
use super::epoch::ViewEpoch;
use crate::error::Result;
use crate::image::cache::{ImageCache, RelationSlot};
use crate::schema::Schema;
use crate::storage::catalog::LmdbReadCatalog;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::RelationId;

pub(crate) trait ImageBind {
    fn epoch(&self, schema: &Schema, relation: RelationId) -> Result<ViewEpoch>;
    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>>;
    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>>;
}

pub(crate) struct LmdbSource<'txn, T = ReadTxn<'txn>> {
    txn: T,
    cache: &'txn ImageCache,
}

pub(crate) trait AsReadTxn {
    fn as_read_txn(&self) -> &ReadTxn<'_>;
}

impl AsReadTxn for ReadTxn<'_> {
    fn as_read_txn(&self) -> &ReadTxn<'_> {
        self
    }
}

impl AsReadTxn for &ReadTxn<'_> {
    fn as_read_txn(&self) -> &ReadTxn<'_> {
        self
    }
}

impl<'txn> LmdbSource<'txn, ReadTxn<'txn>> {
    pub(crate) fn new(txn: ReadTxn<'txn>, cache: &'txn ImageCache) -> Self {
        Self { txn, cache }
    }

    pub(crate) fn into_txn(self) -> ReadTxn<'txn> {
        self.txn
    }
}

impl<'txn> LmdbSource<'txn, &'txn ReadTxn<'txn>> {
    pub(crate) fn bind(txn: &'txn ReadTxn<'txn>, cache: &'txn ImageCache) -> Self {
        Self { txn, cache }
    }
}

impl<'txn, T: AsReadTxn> LmdbSource<'txn, T> {
    pub(crate) fn txn(&self) -> &ReadTxn<'_> {
        self.txn.as_read_txn()
    }

    pub(crate) fn cache(&self) -> &'txn ImageCache {
        self.cache
    }

    pub(crate) fn catalog(&self) -> LmdbReadCatalog<'_, '_> {
        LmdbReadCatalog::new(self.txn())
    }
}

impl<T: AsReadTxn> ImageBind for LmdbSource<'_, T> {
    fn epoch(&self, _schema: &Schema, relation: RelationId) -> Result<ViewEpoch> {
        match self.cache.slot(relation) {
            RelationSlot::Closed(_) => Ok(ViewEpoch::Closed),
            RelationSlot::Ordinary(_) => Ok(ViewEpoch::Store(self.txn().generation()?)),
            RelationSlot::Frozen(_) => {
                unreachable!("store ImageCache never constructs Frozen slots")
            }
        }
    }

    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>> {
        let epoch = self.epoch(schema, relation)?;
        self.cache
            .get_or_build_at(self.txn(), schema, relation, epoch)
    }

    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>> {
        let epoch = self.epoch(schema, relation)?;
        Ok(self.cache.peek_at(relation, epoch))
    }
}
