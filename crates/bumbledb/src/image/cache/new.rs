//! Construction of an empty [`ImageCache`].

use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(feature = "trace")]
use super::stats;
use super::{CacheInner, ImageCache};

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                newest: 0,
            }),
            #[cfg(feature = "trace")]
            counters: stats::CacheCounters::default(),
        }
    }
}
