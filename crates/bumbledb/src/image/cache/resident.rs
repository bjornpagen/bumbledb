//! Resident-image observability (feature `trace`).

use super::ImageCache;

impl ImageCache {
    /// Resident images and their total slab bytes, right now (feature
    /// `trace`; computed under the map lock).
    #[must_use]
    pub fn resident(&self) -> (u64, u64) {
        let inner = self.inner.lock().expect("cache mutex");
        let images = inner.map.len() as u64;
        let bytes = inner
            .map
            .values()
            .map(|image| image.byte_size() as u64)
            .sum();
        (images, bytes)
    }
}
