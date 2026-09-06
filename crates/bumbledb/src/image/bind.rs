//! Image binding: epoch + lazy image over one execution's [`QuerySource`].
use std::sync::Arc;

use super::RelationImage;
use super::epoch::ViewEpoch;
use crate::api::prepared::source::QuerySource;
use crate::error::Result;
use crate::image::ResidentAdmit;
use crate::image::cache::{ImageCache, RelationSlot};
use crate::image::intern::InternerHandle;
use crate::schema::Schema;
use crate::work::GenerationHandle;
use bumbledb_theory::schema::RelationId;

pub(crate) trait ImageBind {
    fn epoch(&self, schema: &Schema, relation: RelationId) -> Result<ViewEpoch>;
    /// Resident image or [`ResidentAdmit::BeyondMemory`]. L05 execute
    /// must match and open scratch via
    /// [`crate::image::ResidentTextExhausted::open_nonresident`].
    fn image(
        &self,
        schema: &Schema,
        relation: RelationId,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>>;
    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>>;
    /// A query-local superset of the selected rows, or None when no supported
    /// index is bound. Never publish this image as a full relation.
    fn selection_image(
        &self,
        schema: &Schema,
        relation: RelationId,
        selections: &[crate::plan::fj::Selection],
        keys: &[Vec<u64>],
    ) -> Result<Option<ResidentAdmit<Arc<RelationImage>>>>;
}

/// One execution's image access: the prepared query's cache bound to the
/// execution's row source, plus the acquired generation that owns every
/// token this execution interprets.
pub(crate) struct SourceImages<'a> {
    source: &'a QuerySource<'a>,
    cache: &'a ImageCache,
    generation: GenerationHandle,
}

impl<'a> SourceImages<'a> {
    pub(crate) fn bind(source: &'a QuerySource<'a>, cache: &'a ImageCache) -> Self {
        Self::with_generation(source, cache, cache.acquire())
    }

    pub(crate) fn with_generation(
        source: &'a QuerySource<'a>,
        cache: &'a ImageCache,
        generation: GenerationHandle,
    ) -> Self {
        Self {
            source,
            cache,
            generation,
        }
    }

    pub(crate) fn source(&self) -> &'a QuerySource<'a> {
        self.source
    }

    /// The execution's generation owner. Tokens interned here cannot
    /// outlive this handle (or an image that clones it).
    pub(crate) fn generation(&self) -> &GenerationHandle {
        &self.generation
    }

    /// Resolver bound to this execution's generation.
    pub(crate) fn interner(&self) -> InternerHandle<'_> {
        InternerHandle::new(&self.generation, self.source.work())
    }
}

impl ImageBind for SourceImages<'_> {
    fn epoch(&self, _schema: &Schema, relation: RelationId) -> Result<ViewEpoch> {
        match self.cache.slot(relation) {
            RelationSlot::Closed(_) => Ok(ViewEpoch::Closed),
            RelationSlot::Ordinary(_) => self.source.relation_epoch(relation),
        }
    }

    fn image(
        &self,
        schema: &Schema,
        relation: RelationId,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>> {
        let epoch = self.epoch(schema, relation)?;
        self.cache
            .get_or_build_with(self.source, schema, relation, epoch, &self.generation)
    }

    fn peek(&self, schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>> {
        let epoch = self.epoch(schema, relation)?;
        Ok(self.cache.peek_at(relation, epoch, &self.generation))
    }

    fn selection_image(
        &self,
        schema: &Schema,
        relation: RelationId,
        selections: &[crate::plan::fj::Selection],
        keys: &[Vec<u64>],
    ) -> Result<Option<ResidentAdmit<Arc<RelationImage>>>> {
        super::selection::build(self, schema, relation, selections, keys)
    }
}
