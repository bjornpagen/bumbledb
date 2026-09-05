//! The non-building cache probe.
use std::sync::Arc;

use crate::image::RelationImage;
use crate::image::ViewEpoch;
use bumbledb_theory::schema::RelationId;

use super::{ImageCache, RelationSlot};

impl ImageCache {
    pub(crate) fn peek_at(&self, rel: RelationId, epoch: ViewEpoch) -> Option<Arc<RelationImage>> {
        match (self.slot(rel), epoch) {
            (RelationSlot::Closed(slot), ViewEpoch::Closed) => slot.get().map(Arc::clone),
            (RelationSlot::Ordinary(cache), ViewEpoch::Store(version)) => cache
                .lock()
                .map
                .get(&version)
                .map(|cached| Arc::clone(&cached.image)),
            (RelationSlot::Ordinary(_), ViewEpoch::Heap(_)) => None,
            (RelationSlot::Closed(_), _) => {
                unreachable!("Closed slot carries no generation")
            }
            (RelationSlot::Ordinary(_), ViewEpoch::Closed) => {
                unreachable!("store generation on a closed image is unrepresentable")
            }
        }
    }
}
