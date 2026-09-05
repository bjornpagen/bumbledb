//! Worker-local resource table (C7): ordinary event-loop state, not a
//! parked stack and not a runtime-global payload map.
//!
//! Each configured worker owns one table. Snapshot entries hold L07's
//! [`OwnedRead`] from `Db::snapshot`, plus worker-affine prepared state.
//! Jobs borrow the entry and take `frame(&work)`. Send payloads (results,
//! cursors, drafts with [`super::DraftLedger`], changes, repository locks)
//! live here so no consumer can run conversion/I/O under the shared route
//! lock. `NativeKind::RepositoryLock` is stamped on minted lock handles.

use std::cell::RefCell;
use std::collections::BTreeMap;

use bumbledb::{OwnedRead, PreparedQuery, SchemaDescriptor};

use super::owners::DbLease;
use super::registry::{Capability, NativeKind, Payload, ResourceState};
use super::RuntimeError;

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
    Native(Payload),
}

/// Owned pinned read (`Db::snapshot`) plus worker-local prepared ids.
/// Each job takes `frame(&work)`. Nothing here is a callback stack borrow.
pub(crate) struct SnapshotResource {
    pub owned: OwnedRead<SchemaDescriptor>,
    pub prepared: BTreeMap<u64, PreparedQuery<SchemaDescriptor>>,
    pub sealed: std::sync::Arc<crate::Sealed>,
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

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
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

    pub(crate) fn mark_closing(&mut self, cap: Capability) {
        if let Some(entry) = self.entries.get_mut(&(cap.kind, cap.id))
            && entry.generation == cap.generation
        {
            entry.state = ResourceState::Closing;
        }
    }

    pub(crate) fn take(&mut self, cap: Capability) -> Option<(u64, TablePayload)> {
        let entry = self.entries.remove(&(cap.kind, cap.id))?;
        if entry.generation != cap.generation {
            self.entries.insert((cap.kind, cap.id), entry);
            return None;
        }
        Some((entry.bytes, entry.payload?))
    }

    pub(crate) fn drain_all(&mut self) -> Vec<(NativeKind, u64, u64, TablePayload)> {
        let mut out = Vec::new();
        for ((kind, id), entry) in std::mem::take(&mut self.entries) {
            if let Some(payload) = entry.payload {
                out.push((kind, id, entry.bytes, payload));
            }
        }
        out
    }
}

fn payload_kind(payload: &TablePayload) -> NativeKind {
    match payload {
        TablePayload::Snapshot(_) => NativeKind::Snapshot,
        TablePayload::Native(Payload::Result { .. }) => NativeKind::Result,
        TablePayload::Native(Payload::Cursor { .. }) => NativeKind::Cursor,
        TablePayload::Native(Payload::Draft(_)) => NativeKind::Draft,
        TablePayload::Native(Payload::Changes { .. }) => NativeKind::Changes,
        TablePayload::Native(Payload::RepositoryLock { .. }) => NativeKind::RepositoryLock,
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
