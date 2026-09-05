//! Image binding: epoch + lazy image over one execution's [`QuerySource`].
use std::sync::Arc;

use super::RelationImage;
use super::epoch::{TextGeneration, ViewEpoch};
use crate::api::prepared::source::QuerySource;
use crate::error::Result;
use crate::image::cache::{ImageCache, RelationSlot};
use crate::image::intern::InternerHandle;
use crate::image::ResidentAdmit;
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
        Self {
            source,
            cache,
            generation: cache.acquire(),
        }
    }

    pub(crate) fn source(&self) -> &'a QuerySource<'a> {
        self.source
    }

    pub(crate) fn cache(&self) -> &'a ImageCache {
        self.cache
    }

    /// The execution's generation owner. Tokens interned here cannot
    /// outlive this handle (or an image that clones it).
    pub(crate) fn generation(&self) -> &GenerationHandle {
        &self.generation
    }

    pub(crate) fn text_generation(&self) -> TextGeneration {
        TextGeneration::of(self.generation.identity())
    }

    /// Resolver bound to this execution's generation.
    pub(crate) fn interner(&self) -> InternerHandle<'_> {
        InternerHandle::new(&self.generation, self.source.work())
    }

    /// Production intern seam. Cache/allocation refusal is
    /// [`ResidentAdmit::BeyondMemory`] — call
    /// [`crate::image::ResidentTextExhausted::open_nonresident`].
    /// Compare tokens with [`crate::image::TextEq`], not raw `u64 ==`.
    pub(crate) fn intern_or_spill(&self, text: &str) -> Result<ResidentAdmit<u64>> {
        self.interner().intern_or_spill(text)
    }

    /// The one production text equality for this execution's generation.
    #[must_use]
    pub(crate) fn text_eq<'b>(
        &'b self,
        scratch: Option<&'b crate::image::NonresidentTextStore>,
    ) -> crate::image::TextEq<'b> {
        crate::image::TextEq::bind(&self.generation, scratch)
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
        Ok(self.cache.peek_at(relation, epoch))
    }
}
