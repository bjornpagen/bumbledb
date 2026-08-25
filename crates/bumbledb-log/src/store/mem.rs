//! `MemStore`: the five verbs over one in-process map. Etags are the
//! blake3 of the object bytes — the same opaque-token contract
//! `FsStore` speaks — and create-only and CAS are atomic under the
//! mutex that is this process. Every read returns a fresh buffer, so
//! a caller cannot alias the map. `put_create` and `put_swap` speak
//! the trait's sums (`Created | Exists | Ambiguous`,
//! `Swapped | Moved | Ambiguous`); the mutex proves every outcome, so
//! `Ambiguous` is unrepresentable here — there is no transport and no
//! pid. That is the honest scope: tests and ephemeral dev inside one
//! process, no persistence, no cross-process claim, no configuration.

use std::collections::HashMap;
use std::io;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::fs::content_etag;
use super::{Create, Etag, Fetched, ObjectStore, Poll, Result, StoreError, StoreKey, Swap};

/// The five verbs over one `HashMap`. Single-process only. Third
/// `Etag` producer beside `FsStore` and `S3Store`: blake3 of the
/// bytes — `FsStore`'s mint — carried as the same opaque brand.
pub struct MemStore {
    objects: Mutex<HashMap<StoreKey, Fetched>>,
}

impl MemStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<StoreKey, Fetched>> {
        self.objects.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn fresh(fetched: &Fetched) -> Fetched {
        Fetched {
            bytes: fetched.bytes.clone(),
            etag: fetched.etag.clone(),
        }
    }
}

impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectStore for MemStore {
    fn get(&self, key: &StoreKey) -> Result<Option<Fetched>> {
        Ok(self.lock().get(key).map(Self::fresh))
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> Result<Poll> {
        match self.lock().get(key) {
            Some(fetched) if fetched.etag == *etag => Ok(Poll::Unchanged),
            Some(fetched) => Ok(Poll::Changed(Self::fresh(fetched))),
            None => Err(StoreError {
                op: "get_if_changed",
                key: key.to_string(),
                source: io::Error::from(io::ErrorKind::NotFound),
            }),
        }
    }

    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> Result<Create> {
        let mut objects = self.lock();
        if objects.contains_key(key) {
            return Ok(Create::Exists);
        }
        let etag = content_etag(bytes);
        objects.insert(
            key.clone(),
            Fetched {
                bytes: bytes.to_vec(),
                etag: etag.clone(),
            },
        );
        Ok(Create::Created(etag))
    }

    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> Result<Swap> {
        let mut objects = self.lock();
        match objects.get(key) {
            Some(current) if current.etag == *etag => {}
            Some(_) | None => return Ok(Swap::Moved),
        }
        let next = content_etag(bytes);
        objects.insert(
            key.clone(),
            Fetched {
                bytes: bytes.to_vec(),
                etag: next.clone(),
            },
        );
        Ok(Swap::Swapped(next))
    }

    fn delete(&self, key: &StoreKey) -> Result<()> {
        self.lock().remove(key);
        Ok(())
    }
}
