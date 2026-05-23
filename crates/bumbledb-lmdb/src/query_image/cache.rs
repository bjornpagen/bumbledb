use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::query_image::{QueryImage, QueryImageBuilder, QueryImageKey, QueryImageScope};
use crate::{Error, ReadTxn, Result, StorageSchema};

const QUERY_IMAGE_CACHE_MAX_IMAGES: usize = 32;

/// Cache of immutable query images by schema fingerprint and storage tx id.
#[derive(Default)]
pub struct QueryImageCache {
    images: RwLock<BTreeMap<QueryImageKey, Arc<QueryImage>>>,
    hits: AtomicU64,
    misses: AtomicU64,
    builds: AtomicU64,
    build_micros: AtomicU64,
}

/// Query image cache diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryImageCacheDiagnostics {
    /// Number of cached image entries.
    pub cached_images: usize,
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Images built and inserted.
    pub builds: u64,
    /// Total image build time in microseconds.
    pub build_micros: u64,
}

impl QueryImageCache {
    /// Returns an existing image for the read snapshot, or builds and caches one.
    #[cfg(test)]
    pub fn get_or_build(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
    ) -> Result<Arc<QueryImage>> {
        self.get_or_build_scoped(txn, schema, QueryImageScope::full(schema))
    }

    /// Returns an existing scoped image for the read snapshot, or builds and caches one.
    pub fn get_or_build_scoped(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
        scope: QueryImageScope,
    ) -> Result<Arc<QueryImage>> {
        let key = QueryImageKey {
            schema: schema.descriptor().fingerprint(),
            tx_id: txn.last_committed_tx_id()?,
            scope: scope.key(),
        };
        if let Some(image) = self
            .images
            .read()
            .map_err(|_| Error::internal("query image cache read lock poisoned"))?
            .get(&key)
            .cloned()
        {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(image);
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();
        let image = Arc::new(QueryImageBuilder::new(txn, schema, scope).build()?);
        let elapsed = start.elapsed().as_micros() as u64;
        let mut images = self
            .images
            .write()
            .map_err(|_| Error::internal("query image cache write lock poisoned"))?;
        if let Some(existing) = images.get(&key).cloned() {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(existing);
        }
        prune_query_image_cache_for_insert(&mut images, &key);
        images.insert(key, image.clone());
        self.builds.fetch_add(1, Ordering::Relaxed);
        self.build_micros.fetch_add(elapsed, Ordering::Relaxed);
        Ok(image)
    }

    /// Returns current query image cache diagnostics.
    pub fn diagnostics(&self) -> QueryImageCacheDiagnostics {
        QueryImageCacheDiagnostics {
            cached_images: self.images.read().map_or(0, |images| images.len()),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            builds: self.builds.load(Ordering::Relaxed),
            build_micros: self.build_micros.load(Ordering::Relaxed),
        }
    }
}

fn prune_query_image_cache_for_insert(
    images: &mut BTreeMap<QueryImageKey, Arc<QueryImage>>,
    incoming: &QueryImageKey,
) {
    images.retain(|key, _| key.schema == incoming.schema && key.tx_id == incoming.tx_id);
    while images.len() >= QUERY_IMAGE_CACHE_MAX_IMAGES {
        let Some(oldest) = images.keys().next().cloned() else {
            break;
        };
        images.remove(&oldest);
    }
}
