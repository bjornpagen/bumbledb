//! Worker-local resource table (C7): ordinary event-loop state, not a
//! parked stack and not a runtime-global payload map.
//!
//! Each configured worker owns one table. Snapshot entries hold L07's
//! [`OwnedRead`] from `Db::snapshot`, plus worker-affine prepared state.
//! Jobs borrow the entry and take `frame(&work)`. Send payloads (results,
//! cursors, drafts, changes, repository locks)
//! live here so no consumer can run conversion/I/O under the shared route
//! lock. `NativeKind::RepositoryLock` is stamped on minted lock handles.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use bumbledb::{OwnedRead, PreparedQuery, SchemaDescriptor};

use super::RuntimeError;
use super::owners::DbLease;
use super::registry::{Capability, NativeKind, Payload, ResourceState};

thread_local! {
    static CURRENT: RefCell<Option<WorkerContext>> = const { RefCell::new(None) };
}

pub(crate) struct WorkerContext {
    pub id: u32,
    pub table: WorkerTable,
}

pub(crate) struct WorkerTable {
    entries: BTreeMap<(NativeKind, u64), TableEntry>,
}

struct TableEntry {
    generation: u64,
    state: ResourceState,
    bytes: u64,
    payload: Option<TablePayload>,
}

pub(crate) enum TablePayload {
    Snapshot(SnapshotResource),
    Native(Box<Payload>),
}

/// Each explicit preparation owns one plan, not a map of past operations.
/// The snapshot pin is shared only on its owning worker. Closing either
/// handle releases its own state without invalidating the other owner.
pub(crate) struct SnapshotResource {
    pub prepared: Option<Box<PreparedQuery<SchemaDescriptor>>>,
    pub data: Rc<SnapshotData>,
}

pub(crate) struct SnapshotData {
    pub owned: OwnedRead<SchemaDescriptor>,
    pub lease: DbLease,
    pub owner: u64,
    pub database: u64,
}

impl WorkerTable {
    pub(crate) fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn insert(
        &mut self,
        cap: Capability,
        payload: TablePayload,
        bytes: u64,
    ) -> Result<(), RuntimeError> {
        if cap.kind != payload_kind(&payload) {
            return Err(RuntimeError::Internal);
        }
        let key = (cap.kind, cap.id);
        if self.entries.contains_key(&key) {
            return Err(RuntimeError::Internal);
        }
        self.entries.insert(
            key,
            TableEntry {
                generation: cap.generation,
                state: ResourceState::Live,
                bytes,
                payload: Some(payload),
            },
        );
        Ok(())
    }

    pub(crate) fn borrow_mut(
        &mut self,
        cap: Capability,
    ) -> Result<(&mut TablePayload, &mut ResourceState), RuntimeError> {
        let entry = self
            .entries
            .get_mut(&(cap.kind, cap.id))
            .ok_or(RuntimeError::ClosedHandle)?;
        if entry.generation != cap.generation {
            return Err(RuntimeError::ClosedHandle);
        }
        match entry.state {
            ResourceState::Live => {}
            ResourceState::Busy | ResourceState::Closing => {
                return Err(RuntimeError::ClosedHandle);
            }
        }
        let payload = entry.payload.as_mut().ok_or(RuntimeError::ClosedHandle)?;
        entry.state = ResourceState::Busy;
        Ok((payload, &mut entry.state))
    }

    pub(crate) fn mark_live(&mut self, cap: Capability) {
        if let Some(entry) = self.entries.get_mut(&(cap.kind, cap.id))
            && entry.generation == cap.generation
            && entry.state == ResourceState::Busy
        {
            entry.state = ResourceState::Live;
        }
    }

    /// Detach a busy payload while its job runs. The route remains busy;
    /// the worker can install a newly prepared child without recursively
    /// borrowing this table's thread-local `RefCell`.
    pub(crate) fn checkout(&mut self, cap: Capability) -> Result<TablePayload, RuntimeError> {
        self.borrow_mut(cap)?;
        self.entries
            .get_mut(&(cap.kind, cap.id))
            .and_then(|entry| entry.payload.take())
            .ok_or(RuntimeError::Internal)
    }

    pub(crate) fn checkin(
        &mut self,
        cap: Capability,
        payload: TablePayload,
    ) -> Result<(), RuntimeError> {
        let entry = self
            .entries
            .get_mut(&(cap.kind, cap.id))
            .ok_or(RuntimeError::Internal)?;
        if entry.generation != cap.generation
            || entry.state != ResourceState::Busy
            || entry.payload.is_some()
        {
            return Err(RuntimeError::Internal);
        }
        entry.payload = Some(payload);
        entry.state = ResourceState::Live;
        Ok(())
    }

    pub(crate) fn take(&mut self, cap: Capability) -> Option<(u64, TablePayload)> {
        let entry = self.entries.remove(&(cap.kind, cap.id))?;
        if entry.generation != cap.generation {
            self.entries.insert((cap.kind, cap.id), entry);
            return None;
        }
        Some((entry.bytes, entry.payload?))
    }
}

fn payload_kind(payload: &TablePayload) -> NativeKind {
    match payload {
        TablePayload::Snapshot(resource) => {
            if resource.prepared.is_some() {
                NativeKind::Prepared
            } else {
                NativeKind::Snapshot
            }
        }
        TablePayload::Native(native) => match native.as_ref() {
            Payload::Result { .. } => NativeKind::Result,
            Payload::Cursor { .. } => NativeKind::Cursor,
            Payload::Draft(_) => NativeKind::Draft,
            Payload::Changes { .. } => NativeKind::Changes,
            Payload::ChangesCursor(_) => NativeKind::ChangesCursor,
            Payload::RepositoryLock { .. } => NativeKind::RepositoryLock,
        },
    }
}

impl WorkerContext {
    pub(crate) fn attach(id: u32) {
        CURRENT.with(|slot| {
            *slot.borrow_mut() = Some(Self {
                id,
                table: WorkerTable::new(),
            });
        });
    }

    pub(crate) fn take() -> Option<Self> {
        CURRENT.with(|slot| slot.borrow_mut().take())
    }

    pub(crate) fn with<T>(f: impl FnOnce(&mut Self) -> T) -> Result<T, RuntimeError> {
        CURRENT.with(|slot| {
            let mut guard = slot.borrow_mut();
            let ctx = guard.as_mut().ok_or(RuntimeError::Internal)?;
            Ok(f(ctx))
        })
    }

    pub(crate) fn worker_id() -> Result<u32, RuntimeError> {
        CURRENT.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|ctx| ctx.id)
                .ok_or(RuntimeError::Internal)
        })
    }
}
