//! Resident-image observability (feature `trace`).

use super::ImageCache;

impl ImageCache {
    /// Resident images and their total slab bytes, right now (feature
    /// `trace`; computed under the map lock). Synthesized closed-relation
    /// images count once each from first touch — they live outside the
    /// generation map and never leave.
    #[must_use]
    pub fn resident(&self) -> (u64, u64) {
        let inner = self.inner.lock().expect("cache mutex");
        let mut images = inner.map.len() as u64;
        let mut bytes: u64 = inner
            .map
            .values()
            .map(|cached| cached.image.byte_size() as u64)
            .sum();
        drop(inner);
        for image in self.closed.iter().filter_map(std::sync::OnceLock::get) {
            images += 1;
            bytes += image.byte_size() as u64;
        }
        (images, bytes)
    }
}
