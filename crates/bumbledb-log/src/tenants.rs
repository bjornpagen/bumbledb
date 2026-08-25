//! Per-tenant replicas: an LRU of replicas keyed by tenant id. A tenant
//! is a prefix (`<root>/t/<tenant>`); eviction closes the replica and
//! deletes its directory — the disposable law — and the `_shared`
//! control-plane tenant is pinned, never evicted. A replica directory
//! has one owner: a fenced CAS lease. `tenant` returns a live handle
//! whose pin holds eviction off; a disposed handle is a distinct type.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::Theory;

use crate::replica::{Fault, OpenRefusal, Opened, Replica};
use crate::store::fence::{DIR_TTL_MS, HeldLease, LeaseBusy, acquire_dir};
use crate::store::{
    Create, Etag, Fetched, ObjectStore, Poll, Result as StoreResult, StoreKey, Swap, WriterId,
    segment_ok,
};

/// The pinned control-plane tenant.
pub const SHARED_TENANT: &str = "_shared";

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

    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> StoreResult<Create> {
        self.0.put_create(key, bytes)
    }

    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> StoreResult<Swap> {
        self.0.put_swap(key, bytes, etag)
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

/// Outcome of a tenant lookup. `Ready` is a live replica; a [`Disposed`]
/// handle is a different type and cannot appear here.
pub enum Tenant<'lru, T: Theory + Clone, S: ObjectStore> {
    Ready(&'lru mut Replica<T, Shared<S>>),
    Refused(TenantRefusal),
}

struct Entry<T: Theory + Clone, S: ObjectStore> {
    id: String,
    replica: Replica<T, Shared<S>>,
    lease: HeldLease,
    pins: usize,
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

    /// The tenant's replica, opening it on a miss and evicting the
    /// least-recent unpinned replicas past the budget. The returned
    /// handle is live: its pin holds eviction off for the borrow.
    pub fn tenant(&mut self, id: &str) -> Result<Tenant<'_, T, S>, Fault> {
        if !segment_ok(id) {
            return Ok(Tenant::Refused(TenantRefusal::Id));
        }
        if let Some(index) = self.open.iter().position(|entry| entry.id == id) {
            let entry = self.open.remove(index);
            self.open.push(entry);
            let last = self.open.last_mut().expect("pushed above");
            last.pins = last.pins.saturating_add(1);
            let _ = last.lease.refresh(DIR_TTL_MS);
            last.pins = last.pins.saturating_sub(1);
        } else {
            let local = self.dir.join(id);
            if let Err(err) = fs::create_dir_all(&local) {
                return Err(Fault::Io(err));
            }
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
                pins: 1,
            });
            self.enforce_budget()?;
            if let Some(last) = self.open.last_mut() {
                last.pins = last.pins.saturating_sub(1);
            }
        }
        let replica = &mut self.open.last_mut().expect("pushed above").replica;
        Ok(Tenant::Ready(replica))
    }

    /// Evicts one tenant by id: closes the replica and deletes its
    /// directory. Pinned `_shared` refuses by doing nothing. The
    /// returned [`Disposed`] is the handle type after eviction.
    pub fn evict(&mut self, id: &str) -> Result<Option<Disposed>, Fault> {
        if id == SHARED_TENANT {
            return Ok(None);
        }
        if let Some(index) = self.open.iter().position(|entry| entry.id == id) {
            if self.open[index].pins > 0 {
                return Ok(None);
            }
            let entry = self.open.remove(index);
            drop(entry.lease);
            entry.replica.dispose().map_err(Fault::Io)?;
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
            total = total.saturating_add(entry.replica.db().disk_size()?);
        }
        Ok(total)
    }

    /// Least-recent-first eviction, skipping the pinned tenant, any
    /// pinned borrow, and the most recent entry, until both budgets
    /// hold or nothing evictable remains.
    fn enforce_budget(&mut self) -> Result<(), Fault> {
        loop {
            let over_count = self.open.len() > self.options.max_open;
            let over_bytes = self.total_bytes()? > self.options.budget_bytes;
            if !over_count && !over_bytes {
                return Ok(());
            }
            let last = self.open.len().saturating_sub(1);
            let Some(index) = self.open.iter().enumerate().position(|(index, entry)| {
                entry.id != SHARED_TENANT && index != last && entry.pins == 0
            }) else {
                return Ok(());
            };
            let entry = self.open.remove(index);
            drop(entry.lease);
            entry.replica.dispose().map_err(Fault::Io)?;
        }
    }
}
