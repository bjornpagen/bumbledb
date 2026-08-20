//! Resident-image observability (feature `trace`).

use super::{ImageCache, RelationSlot};

impl ImageCache {
    /// Resident images and their total slab bytes, right now (feature
    /// `trace`; computed under each ordinary slot's lock). Synthesized
    /// closed-relation images count once each from first touch — they
    /// live in [`RelationSlot::Closed`] and never leave.
    #[must_use]
    pub fn resident(&self) -> (u64, u64) {
        let mut images = 0;
        let mut bytes = 0;
        for slot in &self.slots {
            match slot {
                RelationSlot::Closed(slot) | RelationSlot::Frozen(slot) => {
                    if let Some(image) = slot.get() {
                        images += 1;
                        bytes += image.byte_size() as u64;
                    }
                }
                RelationSlot::Ordinary(cache) => {
                    let inner = cache.lock();
                    images += inner.map.len() as u64;
                    bytes += inner
                        .map
                        .values()
                        .map(|cached| cached.image.byte_size() as u64)
                        .sum::<u64>();
                }
            }
        }
        (images, bytes)
    }
}
