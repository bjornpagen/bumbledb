//! Image binding: epoch + lazy image, generic over frozen or store.

use std::sync::Arc;

use super::RelationImage;
use super::epoch::ViewEpoch;
use crate::error::Result;
use crate::image::cache::ImageCache;
use crate::schema::Schema;
use crate::storage::catalog::LmdbReadCatalog;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::RelationId;

/// Image-binding layer above the join kernel. The kernel sees only
/// `Arc<RelationImage>`. This trait is the only stratum that touches a
/// catalog or a cache.
pub(crate) trait ImageBind {
    fn epoch(&self, schema: &Schema, relation: RelationId) -> Result<ViewEpoch>;
    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>>;
    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>>;
}

/// Store coordinate: one read transaction plus the environment image
/// cache. `T` is the ownership: [`ReadTxn`] inside [`crate::ReadInstance`],
/// or `&ReadTxn` for prepare/execute helpers that already hold a lease.
pub(crate) struct LmdbSource<'txn, T = ReadTxn<'txn>> {
    txn: T,
    cache: &'txn ImageCache,
}

/// Either an owned or borrowed LMDB read transaction.
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
    fn epoch(&self, schema: &Schema, relation: RelationId) -> Result<ViewEpoch> {
        if schema.relation(relation).body().closed_rows().is_some() {
            Ok(ViewEpoch::Closed)
        } else {
            Ok(ViewEpoch::Store(self.txn().generation()?))
        }
    }

    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>> {
        self.cache.get_or_build(self.txn(), schema, relation)
    }

    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>> {
        self.cache.peek(self.txn(), schema, relation)
    }
}
