//! Per-tenant replicas: an LRU of replicas keyed by tenant id. A tenant
//! is a prefix (`<root>/t/<tenant>`); eviction closes the replica and
//! deletes its directory — the disposable law — and the `_shared`
//! control-plane tenant is pinned, never evicted. Braids shard within a
//! tenant; tenants shard the world. Cross-tenant queries belong to the
//! heap arm, and this layer refuses to pretend otherwise.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::Theory;

use crate::replica::{Fault, OpenRefusal, Opened, Replica};
use crate::store::{
    Create, Etag, Fetched, ObjectStore, Poll, Result as StoreResult, StoreKey, Swap,
};

/// The pinned control-plane tenant.
pub const SHARED_TENANT: &str = "_shared";

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
    /// Tenant ids are single path segments: nonempty, no separators, no
    /// dot segments.
    Id,
    Open(OpenRefusal),
}

/// Outcome of a tenant lookup.
pub enum Tenant<'lru, T: Theory + Clone, S: ObjectStore> {
    Ready(&'lru mut Replica<T, Shared<S>>),
    Refused(TenantRefusal),
}

/// The LRU itself. Recency order lives in `open`: the back is the most
/// recently used.
pub struct Tenants<T: Theory + Clone, S: ObjectStore> {
    store: Shared<S>,
    root: String,
    dir: PathBuf,
    theory: T,
    options: TenantOptions,
    open: Vec<(String, Replica<T, Shared<S>>)>,
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

fn well_formed(id: &str) -> bool {
    !id.is_empty() && !id.contains('/') && id != "." && id != ".."
}

impl<T: Theory + Clone, S: ObjectStore> Tenants<T, S> {
    fn prefix(&self, id: &str) -> String {
        if self.root.is_empty() {
            format!("t/{id}")
        } else {
            format!("{}/t/{id}", self.root)
        }
    }

    /// The tenant's replica, opening it on a miss and evicting the
    /// least-recent unpinned replicas past the budget. The returned
    /// handle is the most recent by definition.
    pub fn tenant(&mut self, id: &str) -> Result<Tenant<'_, T, S>, Fault> {
        if !well_formed(id) {
            return Ok(Tenant::Refused(TenantRefusal::Id));
        }
        if let Some(index) = self.open.iter().position(|(name, _)| name == id) {
            let entry = self.open.remove(index);
            self.open.push(entry);
        } else {
            let replica = match Replica::open(
                self.store.clone(),
                &self.prefix(id),
                &self.dir.join(id),
                self.theory.clone(),
            )? {
                Opened::Ready(replica) => *replica,
                Opened::Refused(refusal) => {
                    return Ok(Tenant::Refused(TenantRefusal::Open(refusal)));
                }
            };
            self.open.push((id.to_string(), replica));
            self.enforce_budget()?;
        }
        let (_, replica) = self.open.last_mut().expect("pushed above");
        Ok(Tenant::Ready(replica))
    }

    /// Evicts one tenant by id: closes the replica and deletes its
    /// directory. Pinned `_shared` refuses by doing nothing.
    pub fn evict(&mut self, id: &str) -> Result<(), Fault> {
        if id == SHARED_TENANT {
            return Ok(());
        }
        if let Some(index) = self.open.iter().position(|(name, _)| name == id) {
            let (_, replica) = self.open.remove(index);
            replica.dispose().map_err(Fault::Io)?;
        }
        Ok(())
    }

    /// Open replica count.
    #[must_use]
    pub fn open_count(&self) -> usize {
        self.open.len()
    }

    /// Currently open tenant ids, least recent first.
    #[must_use]
    pub fn open_ids(&self) -> Vec<&str> {
        self.open.iter().map(|(name, _)| name.as_str()).collect()
    }

    fn total_bytes(&self) -> Result<u64, Fault> {
        let mut total: u64 = 0;
        for (_, replica) in &self.open {
            total = total.saturating_add(replica.db().disk_size()?);
        }
        Ok(total)
    }

    /// Least-recent-first eviction, skipping the pinned tenant and the
    /// most recent entry (the one the caller is about to use), until
    /// both budgets hold or nothing evictable remains.
    fn enforce_budget(&mut self) -> Result<(), Fault> {
        loop {
            let over_count = self.open.len() > self.options.max_open;
            let over_bytes = self.total_bytes()? > self.options.budget_bytes;
            if !over_count && !over_bytes {
                return Ok(());
            }
            let last = self.open.len().saturating_sub(1);
            let Some(index) = self
                .open
                .iter()
                .enumerate()
                .position(|(index, (name, _))| name != SHARED_TENANT && index != last)
            else {
                return Ok(());
            };
            let (_, replica) = self.open.remove(index);
            replica.dispose().map_err(Fault::Io)?;
        }
    }
}
