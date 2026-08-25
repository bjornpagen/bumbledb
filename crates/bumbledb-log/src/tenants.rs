//! Per-tenant replicas: an LRU of replicas keyed by tenant id. A tenant
//! is a prefix (`<root>/t/<tenant>`); eviction closes the replica and
//! deletes its directory — the disposable law — and the `_shared`
//! control-plane tenant is pinned, never evicted. A replica directory
//! has one owner: a fenced CAS lease. `tenant` returns a live handle
//! whose refcount pins eviction off until drop; a disposed handle is a
//! distinct type with no replica, so every verb on it is a compile-time
//! refusal.

use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bumbledb::Theory;

use crate::replica::{Fault, OpenRefusal, Opened, Replica};
use crate::store::fence::{DIR_TTL_MS, HeldLease, LeaseBusy, acquire_dir};
use crate::store::{
    Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result as StoreResult, StoreKey, Swap,
    WriterId, segment_ok,
};

/// The pinned control-plane tenant.
pub const SHARED_TENANT: &str = "_shared";

/// A live tenant handle. The pin is held until this value is dropped,
/// so the LRU cannot evict or dispose the replica for the borrow.
/// Replica verbs are reachable by deref; a [`Disposed`] handle has none.
pub struct Live<'lru, T: Theory + Clone, S: ObjectStore> {
    replica: &'lru mut Replica<T, Shared<S>>,
    pin: Arc<AtomicUsize>,
}

impl<T: Theory + Clone, S: ObjectStore> Drop for Live<'_, T, S> {
    fn drop(&mut self) {
        self.pin.fetch_sub(1, Ordering::Release);
    }
}

impl<T: Theory + Clone, S: ObjectStore> Deref for Live<'_, T, S> {
    type Target = Replica<T, Shared<S>>;

    fn deref(&self) -> &Self::Target {
        self.replica
    }
}

impl<T: Theory + Clone, S: ObjectStore> DerefMut for Live<'_, T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.replica
    }
}

/// A handle whose replica is gone. Every verb on this type is a
/// compile-time refusal — there is no replica field to call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disposed;

/// One store handle fanned out to every tenant replica. Delegation is
/// total: the tenant layer adds no verb.
pub struct Shared<S>(Arc<S>);

impl<S> Clone for Shared<S> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<S: ObjectStore> ObjectStore for Shared<S> {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.0.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.0.get_if_changed(key, etag)
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        self.0.put_create(key, body)
    }

    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> StoreResult<Swap> {
        self.0.put_swap(key, body, etag)
    }

    fn delete(&self, key: &StoreKey) -> StoreResult<()> {
        self.0.delete(key)
    }
}

/// The LRU's budget knobs: total on-disk bytes across open replicas and
/// the open-handle count.
#[derive(Debug, Clone, Copy)]
pub struct TenantOptions {
    pub budget_bytes: u64,
    pub max_open: usize,
}

/// Why a tenant handle refused.
#[derive(Debug)]
pub enum TenantRefusal {
    /// Tenant ids are one [`segment_ok`] path segment.
    Id,
    Open(OpenRefusal),
    /// Another process holds the directory lease.
    Exclusive,
}

/// Outcome of a tenant lookup. `Live` is the handle; a [`Disposed`]
/// handle is a different type and cannot appear here.
pub enum Tenant<'lru, T: Theory + Clone, S: ObjectStore> {
    Live(Live<'lru, T, S>),
    Refused(TenantRefusal),
}

struct Entry<T: Theory + Clone, S: ObjectStore> {
    id: String,
    replica: Replica<T, Shared<S>>,
    lease: HeldLease,
    pins: Arc<AtomicUsize>,
}

/// The LRU itself. Recency order lives in `open`: the back is the most
/// recently used.
pub struct Tenants<T: Theory + Clone, S: ObjectStore> {
    store: Shared<S>,
    root: String,
    dir: PathBuf,
    theory: T,
    options: TenantOptions,
    open: Vec<Entry<T, S>>,
}

/// Opens the tenant layer over `store`, with object prefixes under
/// `root` and local directories under `dir`.
pub fn open_tenants<T: Theory + Clone, S: ObjectStore>(
    store: S,
    root: &str,
    dir: &Path,
    theory: T,
    options: TenantOptions,
) -> Tenants<T, S> {
    Tenants {
        store: Shared(Arc::new(store)),
        root: root.to_string(),
        dir: dir.to_path_buf(),
        theory,
        options,
        open: Vec::new(),
    }
}

impl<T: Theory + Clone, S: ObjectStore> Tenants<T, S> {
    fn prefix(&self, id: &str) -> String {
        if self.root.is_empty() {
            format!("t/{id}")
        } else {
            format!("{}/t/{id}", self.root)
        }
    }

    fn holder() -> WriterId {
        WriterId(u64::from(std::process::id()))
    }

    fn add_pin(&mut self) {
        self.open
            .last_mut()
            .expect("pinned entry")
            .pins
            .fetch_add(1, Ordering::Acquire);
    }

    fn unpin_last(&mut self) {
        self.open
            .last_mut()
            .expect("pinned entry")
            .pins
            .fetch_sub(1, Ordering::Release);
    }

    fn take_live(&mut self) -> Live<'_, T, S> {
        let last = self.open.last_mut().expect("pinned entry");
        Live {
            replica: &mut last.replica,
            pin: Arc::clone(&last.pins),
        }
    }

    fn finish_live(&mut self) -> Result<Tenant<'_, T, S>, Fault> {
        self.add_pin();
        if let Err(err) = self.enforce_budget() {
            self.unpin_last();
            return Err(err);
        }
        Ok(Tenant::Live(self.take_live()))
    }

    /// The tenant's replica, opening it on a miss and evicting the
    /// least-recent unpinned replicas past the budget. The returned
    /// handle is live: its pin holds eviction off until drop.
    ///
    /// # Errors
    /// # Panics
    pub fn tenant(&mut self, id: &str) -> Result<Tenant<'_, T, S>, Fault> {
        if !segment_ok(id) {
            return Ok(Tenant::Refused(TenantRefusal::Id));
        }
        if let Some(index) = self.open.iter().position(|entry| entry.id == id) {
            let entry = self.open.remove(index);
            self.open.push(entry);
            let current = self
                .open
                .last()
                .expect("pushed above")
                .lease
                .still_current()?;
            if !current {
                // Another process may own the directory. Close our
                // replica and expire our token; do not delete the dir.
                drop(self.open.pop().expect("pushed above"));
                return Ok(Tenant::Refused(TenantRefusal::Exclusive));
            }
            self.open
                .last()
                .expect("pushed above")
                .lease
                .refresh(DIR_TTL_MS)?;
            return self.finish_live();
        }
        let local = self.dir.join(id);
        fs::create_dir_all(&local)?;
        let lease = match acquire_dir(&local, Self::holder()) {
            Ok(lease) => lease,
            Err(LeaseBusy::Live) => return Ok(Tenant::Refused(TenantRefusal::Exclusive)),
            Err(LeaseBusy::Io(err)) => return Err(Fault::Io(err)),
        };
        let replica = match Replica::open(
            self.store.clone(),
            &self.prefix(id),
            &local,
            self.theory.clone(),
        )? {
            Opened::Ready(replica) => *replica,
            Opened::Refused(refusal) => {
                drop(lease);
                return Ok(Tenant::Refused(TenantRefusal::Open(refusal)));
            }
        };
        self.open.push(Entry {
            id: id.to_string(),
            replica,
            lease,
            pins: Arc::new(AtomicUsize::new(0)),
        });
        self.finish_live()
    }

    /// Evicts one tenant by id: closes the replica and deletes its
    /// directory. Pinned `_shared` refuses by doing nothing. A live
    /// pin refuses by doing nothing. The returned [`Disposed`] is the
    /// handle type after eviction.
    ///
    /// # Errors
    pub fn evict(&mut self, id: &str) -> Result<Option<Disposed>, Fault> {
        if id == SHARED_TENANT {
            return Ok(None);
        }
        if let Some(index) = self.open.iter().position(|entry| entry.id == id) {
            if self.open[index].pins.load(Ordering::Acquire) > 0 {
                return Ok(None);
            }
            let entry = self.open.remove(index);
            drop(entry.lease);
            entry.replica.dispose()?;
            return Ok(Some(Disposed));
        }
        Ok(None)
    }

    /// Open replica count.
    #[must_use]
    pub fn open_count(&self) -> usize {
        self.open.len()
    }

    /// Currently open tenant ids, least recent first.
    #[must_use]
    pub fn open_ids(&self) -> Vec<&str> {
        self.open.iter().map(|entry| entry.id.as_str()).collect()
    }

    fn total_bytes(&self) -> Result<u64, Fault> {
        let mut total: u64 = 0;
        for entry in &self.open {
            // Mounted: the engine store. Unmounted: no store, so no bytes.
            let bytes = match entry.replica.db() {
                Ok(db) => db.disk_size()?,
                Err(_) => 0,
            };
            total = total.saturating_add(bytes);
        }
        Ok(total)
    }

    /// Least-recent-first eviction, skipping the pinned tenant and any
    /// live pin, until both budgets hold or nothing evictable remains.
    fn enforce_budget(&mut self) -> Result<(), Fault> {
        loop {
            let over_count = self.open.len() > self.options.max_open;
            let over_bytes = self.total_bytes()? > self.options.budget_bytes;
            if !over_count && !over_bytes {
                return Ok(());
            }
            let Some(index) = self.open.iter().position(|entry| {
                entry.id != SHARED_TENANT && entry.pins.load(Ordering::Acquire) == 0
            }) else {
                return Ok(());
            };
            let entry = self.open.remove(index);
            drop(entry.lease);
            entry.replica.dispose()?;
        }
    }
}
