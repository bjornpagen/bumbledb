//! Lazy frozen image slots: one [`OnceLock`] per relation, armed on first
//! query that needs the image. Admission does not build images.

use std::sync::{Arc, OnceLock};

use super::bind::ImageBind;
use super::epoch::ViewEpoch;
use super::{RelationImage, synthesize_closed};
use crate::error::Result;
use crate::schema::{RelationBody, Schema};
use crate::storage::catalog::FrozenCatalog;
use bumbledb_theory::schema::RelationId;

/// One relation's frozen image slot. Arms mirror [`ViewEpoch`] on a
/// heap instance: store generations are unrepresentable here.
enum FrozenSlot {
    Closed(OnceLock<Arc<RelationImage>>),
    Ordinary(OnceLock<Arc<RelationImage>>),
}

/// Per-relation slots for an admitted heap catalog.
pub(crate) struct FrozenImages {
    slots: Box<[FrozenSlot]>,
}

impl FrozenImages {
    pub(crate) fn new(schema: &Schema) -> Self {
        Self {
            slots: schema
                .relations()
                .iter()
                .map(|relation| match relation.body() {
                    RelationBody::Closed { .. } => FrozenSlot::Closed(OnceLock::new()),
                    RelationBody::Ordinary { .. } => FrozenSlot::Ordinary(OnceLock::new()),
                })
                .collect(),
        }
    }

    fn slot(&self, relation: RelationId) -> &FrozenSlot {
        &self.slots[relation.0 as usize]
    }

    pub(crate) fn epoch(&self, relation: RelationId) -> ViewEpoch {
        match self.slot(relation) {
            FrozenSlot::Closed(_) => ViewEpoch::Closed,
            FrozenSlot::Ordinary(_) => ViewEpoch::Frozen,
        }
    }

    pub(crate) fn peek(&self, relation: RelationId) -> Option<Arc<RelationImage>> {
        match self.slot(relation) {
            FrozenSlot::Closed(slot) | FrozenSlot::Ordinary(slot) => slot.get().map(Arc::clone),
        }
    }

    pub(crate) fn get_or_build(
        &self,
        catalog: &FrozenCatalog,
        schema: &Schema,
        relation: RelationId,
    ) -> Result<Arc<RelationImage>> {
        match self.slot(relation) {
            FrozenSlot::Closed(slot) => {
                Ok(Arc::clone(slot.get_or_init(|| {
                    synthesize_closed(relation, schema.relation(relation))
                })))
            }
            FrozenSlot::Ordinary(slot) => {
                if let Some(image) = slot.get() {
                    return Ok(Arc::clone(image));
                }
                let built = super::build(catalog, schema, relation)?;
                Ok(Arc::clone(slot.get_or_init(|| built)))
            }
        }
    }
}

/// Admitted heap catalog plus its lazy image slots.
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

    #[cfg(test)]
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
