//! Image binding: epoch + lazy image, generic over frozen or store.

use std::sync::Arc;

use super::RelationImage;
use super::epoch::ViewEpoch;
use crate::error::Result;
use crate::image::cache::ImageCache;
use crate::schema::Schema;
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

/// Store image binding: generation-aware cache over one read transaction.
pub(crate) struct LmdbImages<'txn, 'env> {
    pub txn: &'txn ReadTxn<'env>,
    pub cache: &'txn ImageCache,
}

impl<'txn, 'env> LmdbImages<'txn, 'env> {
    pub(crate) fn new(txn: &'txn ReadTxn<'env>, cache: &'txn ImageCache) -> Self {
        Self { txn, cache }
    }
}

impl ImageBind for LmdbImages<'_, '_> {
    fn epoch(&self, schema: &Schema, relation: RelationId) -> Result<ViewEpoch> {
        if schema.relation(relation).body().closed_rows().is_some() {
            Ok(ViewEpoch::Closed)
        } else {
            Ok(ViewEpoch::Store(self.txn.generation()?))
        }
    }

    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>> {
        self.cache.get_or_build(self.txn, schema, relation)
    }

    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>> {
        self.cache.peek(self.txn, schema, relation)
    }
}
