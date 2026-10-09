//! Persistent directory/database entries in the runtime's one registry.
//! JavaScript capsules carry identities, never an independently owned engine.
use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use bumbledb::work::WorkContext;

use super::fence::{DirectoryLock, acquire_directory};
use super::{
    CloseReport, Operation, Output, Phase, Runtime, RuntimeError, State, WaitTarget, Waiter, Work,
    lock,
};

pub enum ManagedDbOutcome {
    Opened(ManagedDb),
    Rejected(Vec<crate::marshal::ViolationOut>),
    FingerprintMismatch { message: String },
    DestinationExists { message: String },
}

pub(super) struct DatabaseEntry {
    pub inner: Option<Arc<crate::DbInner>>,
    pub closing: bool,
    pub cleaning: bool,
    /// Live worker-affine sessions over this database. Each session also
    /// holds a registered [`DbLease`] operation, so database/directory
    /// cleanup cannot run until every session thread has exited.
    pub sessions: BTreeMap<u64, super::session::SessionSlot>,
}

impl DatabaseEntry {
    pub fn begin_close(&mut self) {
        self.closing = true;
        for session in self.sessions.values_mut() {
            session.closing = true;
        }
    }

    pub fn snapshot_caps(&self) -> Vec<super::registry::Capability> {
        self.sessions.values().map(|slot| slot.cap).collect()
    }
}

#[allow(
    clippy::struct_excessive_bools,
    reason = "one registry row carries the whole owner lifecycle: each flag \
              is an independent phase bit, not a disguised state machine"
)]
pub(super) struct OwnerEntry {
    pub opening: bool,
    pub closing: bool,
    pub cleaning: bool,
    pub failed: bool,
    pub remove: bool,
    pub lock: Option<DirectoryLock>,
    pub databases: BTreeMap<u64, DatabaseEntry>,
    /// Worker-table resources living in this directory (hosted machines);
    /// closing the owner closes them.
    pub resources: Vec<super::registry::Capability>,
}

impl OwnerEntry {
    pub fn begin_close(&mut self, remove: bool) {
        self.closing = true;
        self.remove |= remove;
        self.failed = false;
        for database in self.databases.values_mut() {
            database.begin_close();
        }
    }
}

pub struct DirectoryOwner {
    runtime: Arc<Runtime>,
    id: u64,
}

/// An internal worker reference. It cannot close ownership on drop; the
/// operation registry, not this reference, determines the directory lifetime.
#[derive(Clone)]
pub struct DirectoryReference {
    runtime: Arc<Runtime>,
    id: u64,
}

pub struct ManagedDb {
    runtime: Arc<Runtime>,
    owner: u64,
    id: u64,
}

/// Field order matters: the last transient Engine Arc drops before the
/// operation disappears from the registry and directory teardown can run.
///
/// Crate-internal: it hands out `crate::DbInner` (itself `pub(crate)`), so
/// it is not a public type and never escapes the addon boundary.
pub(crate) struct DbLease {
    inner: Arc<crate::DbInner>,
    operation: ExternalLease,
}

impl Deref for DbLease {
    type Target = crate::DbInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DbLease {
    /// The compiled schema, shareable to the owning thread.
    pub(crate) fn schema(&self) -> Arc<crate::schema::SchemaHandle> {
        Arc::clone(&self.inner.schema)
    }

    /// The engine borrowed for the lease's own lifetime. The lease holds a
    /// registered operation, so close cannot free the engine underneath it.
    pub(crate) fn db(&self) -> &crate::Engine {
        &self.inner.db
    }

    /// The shared inner Arc, for guards that must outlive a borrow of the
    /// lease (the write session's single-writer flag reset).
    pub(crate) fn inner_arc(&self) -> Arc<crate::DbInner> {
        Arc::clone(&self.inner)
    }

    /// The owning runtime (bounded diagnostics off the lease).
    pub(crate) fn runtime(&self) -> &Arc<Runtime> {
        &self.operation.runtime
    }
}

/// A registered hold on a directory owner: while it lives, the owner's
/// cleanup (and so its kernel lock release) waits.
pub(crate) struct ExternalLease {
    runtime: Arc<Runtime>,
    operation: Arc<Operation>,
}
impl Drop for ExternalLease {
    fn drop(&mut self) {
        self.runtime.end_external(&self.operation);
    }
}
impl Drop for DirectoryOwner {
    fn drop(&mut self) {
        self.begin_close();
    }
}
impl Drop for ManagedDb {
    fn drop(&mut self) {
        self.begin_close();
    }
}

pub(super) enum Cleanup {
    Database {
        owner: u64,
        id: u64,
        inner: Arc<crate::DbInner>,
    },
    Directory {
        owner: u64,
        lock: Option<DirectoryLock>,
        remove: bool,
    },
}

impl State {
    pub(super) fn require_owner(&self, owner: Option<u64>) -> Result<(), RuntimeError> {
        if let Some(id) = owner {
            let entry = self.owners.get(&id).ok_or(RuntimeError::ClosedHandle)?;
            if entry.closing {
                return Err(RuntimeError::ClosedHandle);
            }
        }
        Ok(())
    }

    pub(super) fn target_done(&self, target: WaitTarget) -> bool {
        match target {
            WaitTarget::Runtime => self.phase == Phase::Closed,
            WaitTarget::Operation(id) => !self.operations.contains_key(&id),
            WaitTarget::Owner(id) => !self.owners.contains_key(&id),
            WaitTarget::Database(owner, id) => self
                .owners
                .get(&owner)
                .is_none_or(|entry| !entry.databases.contains_key(&id)),
            WaitTarget::Session(owner, database, id) => self
                .owners
                .get(&owner)
                .and_then(|entry| entry.databases.get(&database))
                .is_none_or(|entry| !entry.sessions.contains_key(&id)),
            WaitTarget::Resource(_) => false,
        }
    }

    pub(super) fn target_failed(&self, target: WaitTarget) -> bool {
        match target {
            WaitTarget::Runtime => self.owners.values().any(|entry| entry.failed),
            WaitTarget::Owner(id) | WaitTarget::Database(id, _) | WaitTarget::Session(id, _, _) => {
                self.owners.get(&id).is_some_and(|entry| entry.failed)
            }
            WaitTarget::Operation(_) | WaitTarget::Resource(_) => false,
        }
    }

    pub(super) fn cleanup(&mut self) -> Option<Cleanup> {
        for (&owner_id, owner) in &mut self.owners {
            if owner.opening || owner.cleaning || owner.failed {
                continue;
            }
            // A host workflow spans its awaited transport/sidecar operations.
            // Its native lease prevents runtime close from unlocking mid-await.
            let owner_busy = self
                .operations
                .values()
                .any(|operation| operation.owner == Some(owner_id));
            for (&id, database) in &mut owner.databases {
                if !database.closing || database.cleaning {
                    continue;
                }
                let busy = self.operations.values().any(|operation| {
                    operation.owner == Some(owner_id) && operation.database == Some(id)
                });
                if busy || (owner.closing && owner_busy) {
                    continue;
                }
                database.cleaning = true;
                if let Some(inner) = database.inner.take() {
                    return Some(Cleanup::Database {
                        owner: owner_id,
                        id,
                        inner,
                    });
                }
            }
            if owner.closing && !owner_busy && owner.databases.is_empty() {
                owner.cleaning = true;
                return Some(Cleanup::Directory {
                    owner: owner_id,
                    lock: owner.lock.take(),
                    remove: owner.remove,
                });
            }
        }
        None
    }
}

impl Runtime {
    pub fn submit_owned(
        self: &Arc<Self>,
        owner: &DirectoryOwner,
        policy: WorkContext,
        notify: Box<dyn FnOnce() + Send>,
        prepare: impl FnOnce(&WorkContext) -> Result<Work, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        if !Arc::ptr_eq(self, &owner.runtime) {
            return Err(RuntimeError::ForeignRuntime);
        }
        self.submit_at(Some(owner.id), None, policy, notify, prepare)
    }

    pub fn acquire_directory(
        self: &Arc<Self>,
        path: String,
        policy: WorkContext,
        notify: Box<dyn FnOnce() + Send>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let id = self.reserve_owner_slot()?;
        let pending = PendingDirectory {
            runtime: Arc::clone(self),
            id,
            complete: false,
        };
        let runtime = Arc::clone(self);
        self.submit_at(Some(id), None, policy, notify, move |context| {
            context.checkpoint()?;
            Ok(Box::new(move |context| {
                let mut pending = pending;
                context.checkpoint()?;
                let acquired = acquire_directory(Path::new(&path)).map_err(io_error);
                let mut state = lock(&runtime.state);
                let entry = state
                    .owners
                    .get_mut(&id)
                    .ok_or(RuntimeError::ClosedHandle)?;
                entry.opening = false;
                pending.complete = true;
                match acquired {
                    Ok(held) => entry.lock = Some(held),
                    Err(error) => {
                        entry.begin_close(false);
                        runtime.changed.notify_all();
                        return Err(error);
                    }
                }
                if entry.closing || context.checkpoint().is_err() {
                    entry.begin_close(false);
                    runtime.changed.notify_all();
                    return Err(RuntimeError::ClosedHandle);
                }
                drop(state);
                Ok(Output::Directory(DirectoryOwner { runtime, id }))
            }))
        })
    }

    /// Reserves one directory-owner slot before its kernel lock exists.
    fn reserve_owner_slot(&self) -> Result<u64, RuntimeError> {
        let mut state = lock(&self.state);
        if state.phase != Phase::Open {
            return Err(RuntimeError::ClosedHandle);
        }
        if state.owners.len() >= self.options.owner_capacity {
            return Err(RuntimeError::resource_limit(
                "ownerCapacity",
                state.owners.len(),
                self.options.owner_capacity,
            ));
        }
        let id = state.next_id;
        state.next_id = id.checked_add(1).ok_or(RuntimeError::Internal)?;
        state.owners.insert(
            id,
            OwnerEntry {
                opening: true,
                closing: false,
                cleaning: false,
                failed: false,
                remove: false,
                lock: None,
                databases: BTreeMap::new(),
                resources: Vec::new(),
            },
        );
        Ok(id)
    }

    fn begin_external(
        self: &Arc<Self>,
        owner: u64,
        database: Option<u64>,
        policy: WorkContext,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let context = policy;
        context.checkpoint()?;
        let mut state = lock(&self.state);
        if state.phase != Phase::Open {
            return Err(RuntimeError::ClosedHandle);
        }
        state.require_owner(Some(owner))?;
        let entry = state.owners.get(&owner).ok_or(RuntimeError::ClosedHandle)?;
        if entry.opening {
            return Err(RuntimeError::ClosedHandle);
        }
        if let Some(id) = database
            && entry
                .databases
                .get(&id)
                .is_none_or(|db| db.closing || db.inner.is_none())
        {
            return Err(RuntimeError::ClosedHandle);
        }
        if state.operations.len()
            >= self
                .options
                .workers
                .saturating_add(self.options.queue_capacity)
        {
            return Err(RuntimeError::QueueFull);
        }
        let id = state.next_id;
        state.next_id = id.checked_add(1).ok_or(RuntimeError::Internal)?;
        let operation = Arc::new(Operation {
            id,
            context,
            owner: Some(owner),
            database,
            session: None,
            external: true,
            completion: std::sync::Mutex::new(None),
            output: std::sync::Mutex::new(None),
        });
        state.active += 1;
        state.operations.insert(id, Arc::clone(&operation));
        Ok(operation)
    }

    pub fn end_external(&self, operation: &Operation) {
        let discarded = {
            let mut state = lock(&self.state);
            if !operation.external || !state.operations.contains_key(&operation.id) {
                return;
            }
            state.active -= 1;
            let value = state.remove(operation.id);
            self.changed.notify_all();
            value
        };
        drop(discarded);
    }

    pub fn checkpoint_external(&self, operation: &Operation) -> Result<(), RuntimeError> {
        let state = lock(&self.state);
        if !operation.external || !state.operations.contains_key(&operation.id) {
            return Err(RuntimeError::ClosedHandle);
        }
        operation.context.checkpoint().map_err(Into::into)
    }

    pub(super) fn wait_target(
        &self,
        target: WaitTarget,
        report: Box<dyn FnOnce(CloseReport) + Send>,
    ) {
        let mut state = lock(&self.state);
        let resource_done = match target {
            WaitTarget::Resource(cap) => self.registry.join(cap),
            _ => false,
        };
        let immediate = if resource_done || state.target_done(target) {
            Some(CloseReport::Closed)
        } else if state.target_failed(target)
            || state.waiters.len() >= self.options.cleanup_capacity
        {
            Some(CloseReport::Failed)
        } else {
            None
        };
        if let Some(value) = immediate {
            drop(state);
            report(value);
            return;
        }
        state.waiters.push(Waiter {
            target,
            deadline: Instant::now() + self.options.cleanup_timeout,
            report,
        });
        self.changed.notify_all();
    }

    pub(super) fn run_cleanup(&self, cleanup: Cleanup) {
        match cleanup {
            Cleanup::Database { owner, id, inner } => {
                drop(inner);
                let mut state = lock(&self.state);
                if let Some(entry) = state.owners.get_mut(&owner) {
                    entry.databases.remove(&id);
                }
                self.changed.notify_all();
            }
            Cleanup::Directory {
                owner,
                lock: held,
                remove,
            } => {
                let failed = remove
                    && held.as_ref().is_some_and(|held| {
                        std::fs::remove_dir_all(held.directory())
                            .is_err_and(|error| error.kind() != std::io::ErrorKind::NotFound)
                    });
                if failed {
                    let mut state = lock(&self.state);
                    if let Some(entry) = state.owners.get_mut(&owner) {
                        entry.lock = held;
                        entry.cleaning = false;
                        entry.failed = true;
                    }
                } else {
                    drop(held);
                    let mut state = lock(&self.state);
                    state.owners.remove(&owner);
                }
                self.changed.notify_all();
            }
        }
    }
}

impl DirectoryOwner {
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }
    pub fn reference(&self) -> DirectoryReference {
        DirectoryReference {
            runtime: Arc::clone(&self.runtime),
            id: self.id,
        }
    }
    pub fn begin_close(&self) {
        self.close_with(false);
    }
    pub fn close_with(&self, remove: bool) {
        let caps = {
            let mut state = lock(&self.runtime.state);
            let caps = if let Some(entry) = state.owners.get_mut(&self.id) {
                entry.begin_close(remove);
                entry
                    .databases
                    .values()
                    .flat_map(DatabaseEntry::snapshot_caps)
                    .chain(entry.resources.iter().copied())
                    .collect()
            } else {
                Vec::new()
            };
            for operation in state
                .operations
                .values()
                .filter(|operation| operation.owner == Some(self.id))
            {
                operation.context.cancel();
            }
            self.runtime.changed.notify_all();
            caps
        };
        for cap in caps {
            let _ = self.runtime.request_resource_close(cap);
        }
    }
    pub fn drain(&self, report: Box<dyn FnOnce(CloseReport) + Send>) {
        self.begin_close();
        self.runtime.wait_target(WaitTarget::Owner(self.id), report);
    }
    pub fn begin_work(&self, policy: WorkContext) -> Result<Arc<Operation>, RuntimeError> {
        self.runtime.begin_external(self.id, None, policy)
    }
    #[cfg(test)]
    pub fn child_path(&self, name: &str) -> Result<std::path::PathBuf, RuntimeError> {
        self.reference().child_path(name)
    }
    #[cfg(test)]
    pub(crate) fn attach_db(&self, inner: crate::DbInner) -> Result<ManagedDb, RuntimeError> {
        self.reference().attach_db(inner)
    }
}

impl DirectoryReference {
    pub fn child_path(&self, name: &str) -> Result<std::path::PathBuf, RuntimeError> {
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.starts_with('~')
            || name.contains(['/', '\\', '\0'])
        {
            return Err(RuntimeError::InvalidPath);
        }
        let state = lock(&self.runtime.state);
        state.require_owner(Some(self.id))?;
        let entry = state
            .owners
            .get(&self.id)
            .ok_or(RuntimeError::ClosedHandle)?;
        let held = entry.lock.as_ref().ok_or(RuntimeError::ClosedHandle)?;
        let path = held.directory().join(name);
        drop(state);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(RuntimeError::InvalidPath),
            Ok(_) => Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
            Err(error) => Err(io_error(error)),
        }
    }
    pub(crate) fn lease(&self) -> Result<ExternalLease, RuntimeError> {
        let operation = self
            .runtime
            .begin_external(self.id, None, WorkContext::new())?;
        Ok(ExternalLease {
            runtime: Arc::clone(&self.runtime),
            operation,
        })
    }

    /// Close `cap` whenever this owner closes; refuses if it already is.
    pub(crate) fn adopt(&self, cap: super::registry::Capability) -> Result<(), RuntimeError> {
        let mut state = lock(&self.runtime.state);
        state.require_owner(Some(self.id))?;
        let entry = state
            .owners
            .get_mut(&self.id)
            .ok_or(RuntimeError::ClosedHandle)?;
        entry
            .resources
            .retain(|held| self.runtime.registry.state(*held).is_ok());
        entry.resources.push(cap);
        Ok(())
    }

    pub(crate) fn attach_db(&self, inner: crate::DbInner) -> Result<ManagedDb, RuntimeError> {
        let mut state = lock(&self.runtime.state);
        if state.phase != Phase::Open {
            return Err(RuntimeError::ClosedHandle);
        }
        state.require_owner(Some(self.id))?;
        let used: usize = state
            .owners
            .values()
            .map(|entry| entry.databases.len())
            .sum();
        if used >= self.runtime.options.native_handle_capacity {
            return Err(RuntimeError::resource_limit(
                "nativeHandleCapacity",
                used,
                self.runtime.options.native_handle_capacity,
            ));
        }
        let id = state.next_id;
        state.next_id = id.checked_add(1).ok_or(RuntimeError::Internal)?;
        state
            .owners
            .get_mut(&self.id)
            .ok_or(RuntimeError::ClosedHandle)?
            .databases
            .insert(
                id,
                DatabaseEntry {
                    inner: Some(Arc::new(inner)),
                    closing: false,
                    cleaning: false,
                    sessions: BTreeMap::new(),
                },
            );
        Ok(ManagedDb {
            runtime: Arc::clone(&self.runtime),
            owner: self.id,
            id,
        })
    }
}

impl ManagedDb {
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }
    pub(crate) fn ids(&self) -> (u64, u64) {
        (self.owner, self.id)
    }
    pub(crate) fn access(&self) -> Result<DbLease, RuntimeError> {
        let policy = WorkContext::new();
        let operation = self
            .runtime
            .begin_external(self.owner, Some(self.id), policy)?;
        let lease = ExternalLease {
            runtime: Arc::clone(&self.runtime),
            operation,
        };
        let state = lock(&self.runtime.state);
        // The registered operation prevents the resource being taken by close.
        let inner = state
            .owners
            .get(&self.owner)
            .and_then(|entry| entry.databases.get(&self.id))
            .and_then(|entry| entry.inner.as_ref())
            .cloned()
            .ok_or(RuntimeError::ClosedHandle)?;
        drop(state);
        Ok(DbLease {
            inner,
            operation: lease,
        })
    }
    pub fn begin_close(&self) {
        let caps = {
            let mut state = lock(&self.runtime.state);
            let caps = state
                .owners
                .get_mut(&self.owner)
                .and_then(|entry| entry.databases.get_mut(&self.id))
                .map_or_default(|entry| {
                    entry.begin_close();
                    entry.snapshot_caps()
                });
            for operation in state.operations.values().filter(|operation| {
                operation.owner == Some(self.owner) && operation.database == Some(self.id)
            }) {
                operation.context.cancel();
            }
            self.runtime.changed.notify_all();
            caps
        };
        for cap in caps {
            let _ = self.runtime.request_resource_close(cap);
        }
    }
    pub fn drain(&self, report: Box<dyn FnOnce(CloseReport) + Send>) {
        self.begin_close();
        self.runtime
            .wait_target(WaitTarget::Database(self.owner, self.id), report);
    }
}

struct PendingDirectory {
    runtime: Arc<Runtime>,
    id: u64,
    complete: bool,
}
impl Drop for PendingDirectory {
    fn drop(&mut self) {
        if !self.complete {
            let mut state = lock(&self.runtime.state);
            if let Some(entry) = state.owners.get_mut(&self.id) {
                entry.opening = false;
                entry.begin_close(false);
            }
            self.runtime.changed.notify_all();
        }
    }
}

pub(crate) fn io_error(error: std::io::Error) -> RuntimeError {
    match error.kind() {
        std::io::ErrorKind::WouldBlock => RuntimeError::DirectoryBusy,
        std::io::ErrorKind::InvalidInput => RuntimeError::InvalidPath,
        kind => RuntimeError::Io {
            kind: format!("{kind:?}"),
            os_code: error.raw_os_error(),
        },
    }
}
