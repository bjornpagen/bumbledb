//! Shared routing/admission metadata (C7).
//!
//! The runtime-wide lock covers only capability routes, resource state and
//! native handle counts. Payloads live in the owning worker table. Absence or
//! generation mismatch refuses — IDs are not kept as revoked tombstones.
//!
//! Consumer contract (L13/L14/L16):
//! - `Capability { runtime, worker, kind, id, generation }` is the only token.
//! - `RegistryAdmission::admit` reserves a handle slot then installs on a worker.
//! - `Runtime::submit_payload` / `SnapshotSession::submit` borrow one entry.
//! - `Runtime::close_resource` is the joined close; it cannot `QueueFull`.
//! - Failed drafts release their pending rows and stay terminal.
//! - `Output::Page` / `Rows` carry [`super::QueuedOutput`]. No `Cursor.pending`.
//! - Lock handles mint with `NativeKind::RepositoryLock`.
//! - `with_payload`, `RetainedGuard`, and JS-driven `WriterSession` are gone.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use bumbledb::{ChangeSet, CompleteResult, ResultCursor};

use super::{Runtime, RuntimeError, lock};

/// Discriminator for one resource row. JS handles validate the full
/// [`Capability`] before every access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativeKind {
    Snapshot,
    Prepared,
    Result,
    Cursor,
    Draft,
    Changes,
    RepositoryLock,
}

/// Live / in-use / draining. Busy and closing refuse new work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    Live,
    Busy,
    Closing,
}

/// Worker-routed capability header (C7). Binds runtime identity, worker
/// route, kind, ID and generation. Never holds payload bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceHeader {
    pub runtime: u64,
    pub worker: u32,
    pub kind: NativeKind,
    pub id: u64,
    pub generation: u64,
}

/// Guaranteed close-drain: already-owned obligation, not QueueFull-prone
/// new work (C7). Repeated close joins one drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseDrain {
    pub header: ResourceHeader,
}

/// A checked capability crossing into the registry. Never holds payload
/// bytes — only coordinates and generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub runtime: u64,
    pub worker: u32,
    pub kind: NativeKind,
    pub id: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultState {
    Live,
    Spent,
}

/// Send payloads installed onto a worker table. Snapshot entries are
/// constructed on the owning worker and never travel through this enum.
pub(crate) enum Payload {
    Result {
        result: Option<CompleteResult>,
        state: ResultState,
    },
    Cursor {
        cursor: ResultCursor,
        drained: bool,
    },
    Draft(registry_draft::DraftPayload),
    Changes {
        changes: ChangeSet,
        schema: Arc<bumbledb::schema::Schema>,
        fingerprint: String,
    },
    /// Kernel repository exclusion. Capability.kind is `RepositoryLock`.
    RepositoryLock {
        _lock: bumbledb_log::store::fence::RepositoryLock,
    },
}

struct Route {
    worker: u32,
    generation: u64,
    state: ResourceState,
    close: bool,
}

pub(crate) struct NativeRegistry {
    runtime: u64,
    workers: u32,
    next_id: AtomicU64,
    next_worker: AtomicU32,
    routes: Mutex<BTreeMap<(NativeKind, u64), Route>>,
}

impl NativeRegistry {
    pub(crate) fn new(runtime: u64, workers: u32) -> Self {
        Self {
            runtime,
            workers: workers.max(1),
            next_id: AtomicU64::new(1),
            next_worker: AtomicU32::new(0),
            routes: Mutex::new(BTreeMap::new()),
        }
    }

    fn routes(&self) -> MutexGuard<'_, BTreeMap<(NativeKind, u64), Route>> {
        lock(&self.routes)
    }

    fn mint_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Least-recently-assigned worker. Idle snapshots share workers;
    /// assignment is not an exclusive lane claim.
    pub(crate) fn pick_worker(&self) -> u32 {
        let next = self.next_worker.fetch_add(1, Ordering::Relaxed);
        next % self.workers
    }

    pub(crate) fn check(&self, cap: Capability) -> Result<(), RuntimeError> {
        if cap.runtime != self.runtime {
            return Err(RuntimeError::ForeignRuntime);
        }
        Ok(())
    }

    /// Reserves a route row. Caller must already have reserved handle
    /// count. No payload is stored here.
    pub(crate) fn insert_route(
        &self,
        worker: u32,
        kind: NativeKind,
    ) -> Result<Capability, RuntimeError> {
        if worker >= self.workers {
            return Err(RuntimeError::Internal);
        }
        let id = self.mint_id();
        let generation = self.mint_id();
        let mut routes = self.routes();
        routes.insert(
            (kind, id),
            Route {
                worker,
                generation,
                state: ResourceState::Live,
                close: false,
            },
        );
        Ok(Capability {
            runtime: self.runtime,
            worker,
            kind,
            id,
            generation,
        })
    }

    /// Drops a route that was reserved but never installed. No tombstone.
    pub(crate) fn rollback_route(&self, cap: Capability) -> Option<()> {
        let mut routes = self.routes();
        let route = routes.remove(&(cap.kind, cap.id))?;
        if route.generation != cap.generation {
            routes.insert((cap.kind, cap.id), route);
            return None;
        }
        Some(())
    }

    pub(crate) fn state(&self, cap: Capability) -> Result<ResourceState, RuntimeError> {
        self.check(cap)?;
        let routes = self.routes();
        let route = routes
            .get(&(cap.kind, cap.id))
            .ok_or(RuntimeError::ClosedHandle)?;
        if route.generation != cap.generation {
            return Err(RuntimeError::ClosedHandle);
        }
        Ok(route.state)
    }

    /// Admits one job against a live resource. Busy/closing refuse.
    pub(crate) fn begin_job(&self, cap: Capability) -> Result<u32, RuntimeError> {
        self.check(cap)?;
        let mut routes = self.routes();
        let route = routes
            .get_mut(&(cap.kind, cap.id))
            .ok_or(RuntimeError::ClosedHandle)?;
        if route.generation != cap.generation {
            return Err(RuntimeError::ClosedHandle);
        }
        match route.state {
            ResourceState::Live => {
                route.state = ResourceState::Busy;
                Ok(route.worker)
            }
            ResourceState::Busy | ResourceState::Closing => Err(RuntimeError::ClosedHandle),
        }
    }

    pub(crate) fn end_job(&self, cap: Capability) {
        let mut routes = self.routes();
        if let Some(route) = routes.get_mut(&(cap.kind, cap.id))
            && route.generation == cap.generation
            && route.state == ResourceState::Busy
        {
            route.state = if route.close {
                ResourceState::Closing
            } else {
                ResourceState::Live
            };
        }
    }

    /// Revokes admission and records exactly one drain. Repeated calls
    /// join the existing flag — they do not enqueue rejectable work.
    pub(crate) fn request_close(&self, cap: Capability) -> Result<CloseDrain, RuntimeError> {
        self.check(cap)?;
        let mut routes = self.routes();
        let route = routes
            .get_mut(&(cap.kind, cap.id))
            .ok_or(RuntimeError::ClosedHandle)?;
        if route.generation != cap.generation {
            return Err(RuntimeError::ClosedHandle);
        }
        route.close = true;
        if route.state == ResourceState::Live {
            route.state = ResourceState::Closing;
        }
        Ok(CloseDrain {
            header: ResourceHeader {
                runtime: self.runtime,
                worker: route.worker,
                kind: cap.kind,
                id: cap.id,
                generation: route.generation,
            },
        })
    }

    /// Removes a drained route. No tombstone remains.
    pub(crate) fn release(&self, cap: Capability) -> Option<()> {
        let mut routes = self.routes();
        let route = routes.remove(&(cap.kind, cap.id))?;
        if route.generation != cap.generation {
            routes.insert((cap.kind, cap.id), route);
            return None;
        }
        Some(())
    }

    /// Idempotent join: the row is gone (drained) or already closing with
    /// no further admission.
    pub(crate) fn join(&self, cap: Capability) -> bool {
        if cap.runtime != self.runtime {
            return true;
        }
        let routes = self.routes();
        routes
            .get(&(cap.kind, cap.id))
            .is_none_or(|route| route.generation != cap.generation)
    }

    #[cfg(test)]
    pub(crate) fn route_count(&self) -> usize {
        self.routes().len()
    }

    /// Marks every live route closing. Workers drain their tables; rows
    /// disappear as payloads are released — they are not left as tombstones.
    pub(crate) fn close_all(&self) -> Vec<Capability> {
        let mut routes = self.routes();
        let mut caps = Vec::new();
        for (&(kind, id), route) in routes.iter_mut() {
            route.close = true;
            if route.state == ResourceState::Live {
                route.state = ResourceState::Closing;
            }
            caps.push(Capability {
                runtime: self.runtime,
                worker: route.worker,
                kind,
                id,
                generation: route.generation,
            });
        }
        caps
    }

    pub(crate) fn closing_for_worker(&self, worker: u32) -> Vec<Capability> {
        self.routes()
            .iter()
            .filter_map(|(&(kind, id), route)| {
                (route.worker == worker && route.close && route.state != ResourceState::Busy)
                    .then_some(Capability {
                        runtime: self.runtime,
                        worker,
                        kind,
                        id,
                        generation: route.generation,
                    })
            })
            .collect()
    }
}

/// Draft payload body lives here so `registry.rs` stays the one owner
/// table and draft ingestion can mutate through capabilities.
pub(crate) mod registry_draft {
    use std::sync::Arc;

    use bumbledb::{RelationId, Value};

    pub(crate) struct PendingChange {
        pub relation: RelationId,
        pub insert: bool,
        pub values: Vec<Value>,
    }

    pub(crate) struct DraftPayload {
        pub schema: Arc<bumbledb::schema::Schema>,
        pub pending: Vec<PendingChange>,
        pub terminal: bool,
    }
}

/// Reserve-then-install admission. The JS wrapper holds a capability
/// token only — it does not own the payload.
pub(crate) struct RegistryAdmission {
    pub runtime: Arc<Runtime>,
    pub cap: Capability,
}

impl RegistryAdmission {
    /// Reserves a handle slot and returns the capability immediately.
    /// Same-worker install is local; JS/cross-worker install is async
    /// (capability first, table insert later). A failed enqueue rolls the
    /// route back — no `ready_rx`, no thread-join.
    pub(crate) fn admit(
        runtime: Arc<Runtime>,
        kind: NativeKind,
        payload: Payload,
    ) -> Result<Self, RuntimeError> {
        let cap = runtime.reserve_native_route(kind)?;
        if let Err(error) = runtime.install_send_payload(cap, payload) {
            runtime.rollback_native_route(cap);
            return Err(error);
        }
        Ok(Self { runtime, cap })
    }

    pub(crate) fn cap(&self) -> Capability {
        self.cap
    }

    /// Coalesced close: existing admitted obligation, not a new job.
    #[cfg(test)]
    pub(crate) fn request_close(&self) -> Result<CloseDrain, RuntimeError> {
        self.runtime.request_resource_close(self.cap)
    }
}
