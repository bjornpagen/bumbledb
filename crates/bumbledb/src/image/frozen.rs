//! Lazy frozen image slots: one [`OnceLock`] per relation, armed on first
//! query that needs the image. Admission does not build images.
use std::sync::Arc;

use super::bind::ImageBind;
use super::cache::RelationSlot;
use super::epoch::ViewEpoch;
use super::{RelationImage, synthesize_closed};
use crate::error::Result;
use crate::schema::Schema;
use crate::storage::catalog::FrozenCatalog;
use bumbledb_theory::schema::RelationId;

pub(crate) struct FrozenImages {
    slots: Box<[RelationSlot]>,
}

impl FrozenImages {
    pub(crate) fn new(schema: &Schema) -> Self {
        Self {
            slots: schema
                .relations()
                .iter()
                .map(|relation| RelationSlot::for_frozen(relation.body()))
                .collect(),
        }
    }

    fn slot(&self, relation: RelationId) -> &RelationSlot {
        &self.slots[relation.0 as usize]
    }

    pub(crate) fn epoch(&self, relation: RelationId) -> ViewEpoch {
        match self.slot(relation) {
            RelationSlot::Closed(_) => ViewEpoch::Closed,
            RelationSlot::Frozen(_) => ViewEpoch::Frozen,
            RelationSlot::Ordinary(_) => {
                unreachable!("frozen source does not hold Ordinary(GenerationCache)")
            }
        }
    }

    pub(crate) fn peek(&self, relation: RelationId) -> Option<Arc<RelationImage>> {
        match self.slot(relation) {
            RelationSlot::Closed(slot) | RelationSlot::Frozen(slot) => slot.get().map(Arc::clone),
            RelationSlot::Ordinary(_) => {
                unreachable!("frozen source does not hold Ordinary(GenerationCache)")
            }
        }
    }

    pub(crate) fn get_or_build(
        &self,
        catalog: &FrozenCatalog,
        schema: &Schema,
        relation: RelationId,
    ) -> Result<Arc<RelationImage>> {
        match self.slot(relation) {
            RelationSlot::Closed(slot) => {
                Ok(Arc::clone(slot.get_or_init(|| {
                    synthesize_closed(relation, schema.relation(relation))
                })))
            }
            RelationSlot::Frozen(slot) => {
                if let Some(image) = slot.get() {
                    return Ok(Arc::clone(image));
                }
                let built = super::build(catalog, schema, relation)?;
                Ok(Arc::clone(slot.get_or_init(|| built)))
            }
            RelationSlot::Ordinary(_) => {
                unreachable!("frozen source does not hold Ordinary(GenerationCache)")
            }
        }
    }
}

pub(crate) struct FrozenSource {
    pub(crate) catalog: FrozenCatalog,
    images: FrozenImages,
}

impl FrozenSource {
    pub(crate) fn new(schema: &Schema, catalog: FrozenCatalog) -> Self {
        Self {
            catalog,
            images: FrozenImages::new(schema),
        }
    }

    pub(crate) fn peek_image(&self, relation: RelationId) -> Option<Arc<RelationImage>> {
        self.images.peek(relation)
    }
}

impl ImageBind for FrozenSource {
    fn epoch(&self, _schema: &Schema, relation: RelationId) -> Result<ViewEpoch> {
        Ok(self.images.epoch(relation))
    }

    fn image(&self, schema: &Schema, relation: RelationId) -> Result<Arc<RelationImage>> {
        self.images.get_or_build(&self.catalog, schema, relation)
    }

    fn peek(&self, _schema: &Schema, relation: RelationId) -> Result<Option<Arc<RelationImage>>> {
        Ok(self.images.peek(relation))
    }
}
