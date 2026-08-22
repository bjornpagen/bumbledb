//! The non-building cache probe.

use std::sync::Arc;

use crate::image::RelationImage;
use crate::image::ViewEpoch;
use bumbledb_theory::schema::RelationId;

#[cfg(test)]
use crate::error::Result;
#[cfg(test)]
use crate::image::bind::{ImageBind, LmdbSource};
#[cfg(test)]
use crate::schema::Schema;
#[cfg(test)]
use crate::storage::env::ReadTxn;

use super::{ImageCache, RelationSlot};

impl ImageCache {
    /// # Errors

    /// # Panics
    #[cfg(test)]
    pub fn peek(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
    ) -> Result<Option<Arc<RelationImage>>> {
        LmdbSource::bind(txn, self).peek(schema, rel)
    }

    pub(crate) fn peek_at(&self, rel: RelationId, epoch: ViewEpoch) -> Option<Arc<RelationImage>> {
        match (self.slot(rel), epoch) {
            (RelationSlot::Closed(slot), ViewEpoch::Closed) => slot.get().map(Arc::clone),
            (RelationSlot::Ordinary(cache), ViewEpoch::Store(generation)) => cache
                .lock()
                .map
                .get(&generation)
                .map(|cached| Arc::clone(&cached.image)),
            (RelationSlot::Closed(_), _) => {
                unreachable!("Closed slot carries no generation")
            }
            (RelationSlot::Frozen(_), _) => {
                unreachable!("store ImageCache never constructs Frozen slots")
            }
            (RelationSlot::Ordinary(_), _) => {
                unreachable!("store generation on a closed image is unrepresentable")
            }
        }
    }
}
