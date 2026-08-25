//! `MemStore`: the five verbs over one in-process map. Etags are the
//! blake3 of the object bytes — the same opaque-token contract
//! `FsStore` speaks — and create-only and CAS are atomic under the
//! mutex that is this process. Every read returns a fresh buffer, so
//! a caller cannot alias the map. `put_create` and `put_swap` speak
//! the trait's sums (`Created | Exists | Ambiguous`,
//! `Swapped | Moved | Ambiguous`); the mutex proves every outcome, so
//! `Ambiguous` is unrepresentable here — there is no transport and no
//! pid. The fencing token on a [`Fenced`] write is the generation the
//! CAS can lose to: a lower token is `Moved`. That is the honest
//! scope: tests and ephemeral dev inside one process, no persistence,
//! no cross-process claim, no configuration.

use std::collections::HashMap;
use std::io;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::fs::content_etag;
use super::{Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result, StoreError, StoreKey, Swap};

/// One object plus the fencing generation the next swap can lose to.
struct Object {
    fetched: Fetched,
    token: u64,
}

/// The five verbs over one `HashMap`. Single-process only. Third
/// `Etag` producer beside `FsStore` and `S3Store`: blake3 of the
/// bytes — `FsStore`'s mint — carried as the same opaque brand.
pub struct MemStore {
    objects: Mutex<HashMap<StoreKey, Object>>,
}

impl MemStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<StoreKey, Object>> {
        self.objects.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn fresh(object: &Object) -> Fetched {
        Fetched {
            bytes: object.fetched.bytes.clone(),
            etag: object.fetched.etag.clone(),
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
            Some(object) if object.fetched.etag == *etag => Ok(Poll::Unchanged),
            Some(object) => Ok(Poll::Changed(Self::fresh(object))),
            None => Err(StoreError {
                op: "get_if_changed",
                key: key.to_string(),
                source: io::Error::from(io::ErrorKind::NotFound),
            }),
        }
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> Result<Create> {
        let body = body.into();
        let mut objects = self.lock();
        if objects.contains_key(key) {
            return Ok(Create::Exists);
        }
        let etag = content_etag(body.bytes);
        objects.insert(
            key.clone(),
            Object {
                fetched: Fetched {
                    bytes: body.bytes.to_vec(),
                    etag: etag.clone(),
                },
                token: body.token,
            },
        );
        Ok(Create::Created(etag))
    }

    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> Result<Swap> {
        let body = body.into();
        let mut objects = self.lock();
        match objects.get(key) {
            Some(current) if current.fetched.etag == *etag && body.token >= current.token => {}
            Some(_) | None => return Ok(Swap::Moved),
        }
        let next = content_etag(body.bytes);
        objects.insert(
            key.clone(),
            Object {
                fetched: Fetched {
                    bytes: body.bytes.to_vec(),
                    etag: next.clone(),
                },
                token: body.token,
            },
        );
        Ok(Swap::Swapped(next))
    }

    fn delete(&self, key: &StoreKey) -> Result<()> {
        self.lock().remove(key);
        Ok(())
    }
}
