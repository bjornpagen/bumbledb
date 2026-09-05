//! Image binding: epoch + lazy image over one execution's [`QuerySource`].
use std::sync::Arc;

use super::RelationImage;
use super::epoch::ViewEpoch;
use crate::api::prepared::source::QuerySource;
use crate::error::Result;
use crate::image::cache::{ImageCache, RelationSlot};
use crate::schema::Schema;
use bumbledb_theory::schema::RelationId;

pub(crate) trait ImageBind {
    fn epoch(&self, schema: &Schema, relation: RelationId) -> Result<ViewEpoch>;
    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>>;
    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>>;
}

/// One execution's image access: the prepared query's cache (relation
/// slots + text interner) bound to the execution's row source. Store
/// epochs memoize per generation; heap epochs are per-execution ticks that
/// can never hit a memo (heap instances carry no durable identity).
pub(crate) struct SourceImages<'a> {
    source: &'a QuerySource<'a>,
    cache: &'a ImageCache,
}

impl<'a> SourceImages<'a> {
    pub(crate) fn bind(source: &'a QuerySource<'a>, cache: &'a ImageCache) -> Self {
        Self { source, cache }
    }

    pub(crate) fn source(&self) -> &'a QuerySource<'a> {
        self.source
    }

    pub(crate) fn cache(&self) -> &'a ImageCache {
        self.cache
    }
}

impl ImageBind for SourceImages<'_> {
    fn epoch(&self, _schema: &Schema, relation: RelationId) -> Result<ViewEpoch> {
        match self.cache.slot(relation) {
            RelationSlot::Closed(_) => Ok(ViewEpoch::Closed),
            RelationSlot::Ordinary(_) => Ok(self.source.epoch()),
        }
    }

    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>> {
        let epoch = self.epoch(schema, relation)?;
        self.cache
            .get_or_build_at(self.source, schema, relation, epoch)
    }

    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>> {
        let epoch = self.epoch(schema, relation)?;
        Ok(self.cache.peek_at(relation, epoch))
    }
}
