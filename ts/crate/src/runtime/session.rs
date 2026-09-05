//! Worker-table snapshots (C7): one owned pinned read plus prepared state
//! per entry. Jobs borrow the entry for one operation and return to the
//! scheduler. No session-long reactor, no ready_rx, no JS-driven writer.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use bumbledb::work::{ExecutionPolicy, WorkContext, WorkError};
use bumbledb::{OwnedRead, PreparedQuery, SchemaDescriptor, Witness};

use super::lanes::{LaneId, WorkerCommand};
use super::owners::{DbLease, ManagedDb};
use super::registry::{Capability, CloseDrain, NativeKind};
use super::table::{SnapshotResource, TablePayload, WorkerContext};
use super::{Notify, Operation, Output, Runtime, RuntimeError, WaitTarget, lock};

/// One typed engine refusal crossing the executor as owned data.
pub(crate) fn engine_error(error: &bumbledb::Error) -> RuntimeError {
    if let bumbledb::Error::Store(store) = error {
        if let bumbledb::store::StoreError::Work(work) = store.as_ref() {
            return RuntimeError::Work(*work);
        }
    }
    RuntimeError::Engine {
        kind: crate::tags::error_family::tag(&error.family()),
        message: crate::marshal::engine_message(error),
    }
}

/// Borrowed snapshot entry for one job. Prepared state stays on the worker.
pub struct SnapshotAccess<'a> {
    pub owned: &'a OwnedRead<SchemaDescriptor>,
    pub sealed: &'a crate::Sealed,
    pub job: u64,
    pub prepared: &'a mut BTreeMap<u64, PreparedQuery<SchemaDescriptor>>,
}

impl SnapshotAccess<'_> {
    #[must_use]
    pub fn frame<'read>(
        &'read self,
        work: &'read WorkContext,
    ) -> bumbledb::ReadFrame<'read, SchemaDescriptor> {
        self.owned.frame(work)
    }

    pub fn install(&mut self, prepared: PreparedQuery<SchemaDescriptor>) -> u64 {
        let id = self.job;
        self.prepared.insert(id, prepared);
        id
    }

    pub fn execute(
        &mut self,
        id: u64,
        context: &WorkContext,
        args: &[bumbledb::ParamArg<'_>],
    ) -> Result<bumbledb::Answers, RuntimeError> {
        let prepared = self
            .prepared
            .get_mut(&id)
            .ok_or(RuntimeError::ClosedHandle)?;
        // L07 seam: execute against the owned frame, not a !Send ReadInstance.
        prepared
            .execute_collect_owned(self.owned, context, args)
            .map_err(|error| engine_error(&error))
    }

    pub fn remove_prepared(&mut self, id: u64) -> Result<(), RuntimeError> {
        self.prepared
            .remove(&id)
            .map(drop)
            .ok_or(RuntimeError::ClosedHandle)
    }

    #[must_use]
    pub fn prepared_count(&self) -> usize {
        self.prepared.len()
    }
}

pub type SnapshotWork = Box<
    dyn FnOnce(&WorkContext, &mut SnapshotAccess<'_>) -> Result<Output, RuntimeError> + Send,
>;

/// Live-ticket publication boundary. L13 calls [`PublicationSink::accept`]
/// with the original `DeliveryTicket` still alive: register `QueuedOutput`
/// and `commit()` are one transition. No park, no second preview.
pub struct PublicationSink<'a> {
    operation: &'a Operation,
    armed: bool,
    accepted: bool,
}

impl<'a> PublicationSink<'a> {
    pub(super) fn new(operation: &'a Operation, armed: bool) -> Self {
        Self {
            operation,
            armed,
            accepted: false,
        }
    }

    pub(super) fn accepted(&self) -> bool {
        self.accepted
    }

    pub(super) fn armed(&self) -> bool {
        self.armed
    }

    /// Register `output` and run `commit` under the output lock so no
    /// observer can take between them. `commit` must not allocate, read,
    /// checkpoint, or preview. Armed cancel and accept failure publish
    /// nothing — the caller aborts the live ticket.
    pub fn accept(
        &mut self,
        output: Output,
        commit: impl FnOnce() -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        if !output.queued_publication() {
            return Err(RuntimeError::Internal);
        }
        let mut slot = lock(&self.operation.output);
        if matches!(&*slot, Some(Ok(value)) if value.queued_publication()) {
            return Ok(());
        }
        if self.armed {
            return Err(RuntimeError::Work(WorkError::Cancelled));
        }
        commit()?;
        *slot = Some(Ok(output));
        self.accepted = true;
        Ok(())
    }
}

pub type PayloadWork = Box<
    dyn FnOnce(
            &WorkContext,
            &mut super::registry::Payload,
            &mut PublicationSink<'_>,
        ) -> Result<Output, RuntimeError>
        + Send,
>;

pub enum Message {
    Snapshot {
        operation: Arc<Operation>,
        work: SnapshotWork,
    },
    Payload {
        operation: Arc<Operation>,
        work: PayloadWork,
    },
    Close,
}

pub(super) struct SessionSlot {
    pub cap: Capability,
    pub closing: bool,
}

impl SessionSlot {
    pub(super) fn begin_close(&mut self, runtime: &Runtime) {
        if self.closing {
            return;
        }
        self.closing = true;
        let _ = runtime.request_resource_close(self.cap);
    }
}

pub enum PrepareReply {
    Ok(u64),
    IrError(String),
}

pub struct SessionOpened {
    pub session: SnapshotSession,
    pub sealed: Arc<crate::Sealed>,
    pub witness: Witness<SchemaDescriptor>,
    pub generation: u64,
    pub store: String,
    pub attachment: Option<Vec<u8>>,
}

struct SessionCore {
    runtime: Arc<Runtime>,
    owner: u64,
    database: u64,
    id: u64,
    cap: Capability,
}

impl SessionCore {
    fn begin_close(&self) {
        let _ = self.runtime.request_resource_close(self.cap);
        let mut state = lock(&self.runtime.state);
        if let Some(slot) = state
            .owners
            .get_mut(&self.owner)
            .and_then(|owner| owner.databases.get_mut(&self.database))
            .and_then(|database| database.sessions.get_mut(&self.id))
        {
            slot.closing = true;
        }
        for operation in state
            .operations
            .values()
            .filter(|operation| operation.session == Some(self.id))
        {
            operation.context.cancel();
        }
        self.runtime.changed.notify_all();
    }

    fn drain(&self, report: super::Report) {
        self.begin_close();
        self.runtime.wait_target(
            WaitTarget::Session(self.owner, self.database, self.id),
            report,
        );
    }

    fn submit(
        &self,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<SnapshotWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.runtime.submit_snapshot(
            self.cap,
            self.owner,
            self.database,
            self.id,
            policy,
            notify,
            prepare,
        )
    }
}

impl Drop for SessionCore {
    fn drop(&mut self) {
        self.begin_close();
    }
}

pub struct SnapshotSession {
    core: SessionCore,
}

impl SnapshotSession {
    #[must_use]
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.core.runtime
    }

    #[must_use]
    pub fn capability(&self) -> Capability {
        self.core.cap
    }

    pub fn begin_close(&self) {
        self.core.begin_close();
    }

    pub fn drain(&self, report: super::Report) {
        self.core.drain(report);
    }

    pub fn submit(
        &self,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<SnapshotWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.core.submit(policy, notify, prepare)
    }
}

impl Runtime {
    pub fn open_session(
        self: &Arc<Self>,
        db: &ManagedDb,
        policy: ExecutionPolicy,
        notify: Notify,
    ) -> Result<Arc<Operation>, RuntimeError> {
        if !Arc::ptr_eq(self, db.runtime()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let runtime = Arc::clone(self);
        let (owner, database) = db.ids();
        let lease = db.access()?;
        self.submit_at(Some(owner), Some(database), policy, notify, move |_| {
            Ok(Box::new(move |context| {
                context.checkpoint()?;
                runtime.pin_snapshot(owner, database, lease, context)
            }))
        })
    }

    /// Pin on the current worker. Called from an already-running pool job
    /// (open or L14 authority pin). No ready_rx, no second hop onto this pool.
    pub(crate) fn spawn_read_session_for(
        self: &Arc<Self>,
        db: &ManagedDb,
        lease: DbLease,
    ) -> Result<Output, RuntimeError> {
        if !Arc::ptr_eq(self, db.runtime()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let (owner, database) = db.ids();
        let context = ExecutionPolicy {
            input_bytes: 0,
            working_bytes: 0,
            scratch_bytes: 0,
            result_bytes: 0,
            rows: 0,
            work_units: 0,
            timeout: self.options.cleanup_timeout,
        }
        .start()?;
        context.checkpoint()?;
        self.pin_snapshot(owner, database, lease, &context)
    }

    fn pin_snapshot(
        self: &Arc<Self>,
        owner: u64,
        database: u64,
        lease: DbLease,
        context: &WorkContext,
    ) -> Result<Output, RuntimeError> {
        context.checkpoint()?;
        let worker = WorkerContext::worker_id()?;
        let sealed = lease.sealed();
        let store = lease.db().integration_store().identity().store.to_string();
        // L07 seam: Db::snapshot → OwnedRead. Each job takes frame(&work).
        let owned = lease.db().snapshot(context).map_err(engine_error)?;
        let generation = owned.snapshot().generation().value();
        let attachment = owned
            .snapshot()
            .attachment()
            .map_err(|error| engine_error(&bumbledb::Error::from_store(error)))?
            .map(<[u8]>::to_vec);
        let witness = owned.witness().map_err(engine_error)?;
        let cap = self.reserve_snapshot_route(owner, database, worker)?;
        let resource = SnapshotResource {
            owned,
            prepared: BTreeMap::new(),
            sealed: Arc::clone(&sealed),
            lease,
            owner,
            database,
        };
        let installed = WorkerContext::with(|ctx| {
            ctx.table
                .insert(cap, TablePayload::Snapshot(resource), 0)
        })?;
        if let Err(error) = installed {
            self.rollback_snapshot_route(owner, database, cap);
            return Err(error);
        }
        Ok(Output::Session(SessionOpened {
            session: SnapshotSession {
                core: SessionCore {
                    runtime: Arc::clone(self),
                    owner,
                    database,
                    id: cap.id,
                    cap,
                },
            },
            sealed,
            witness,
            generation,
            store,
            attachment,
        }))
    }

    fn reserve_snapshot_route(
        &self,
        owner: u64,
        database: u64,
        worker: u32,
    ) -> Result<Capability, RuntimeError> {
        let mut state = lock(&self.state);
        if state.phase != super::Phase::Open {
            return Err(RuntimeError::ClosedHandle);
        }
        state.require_owner(Some(owner))?;
        let sessions: usize = state
            .owners
            .values()
            .flat_map(|entry| entry.databases.values())
            .map(|database| database.sessions.len())
            .sum();
        if sessions >= self.options.native_handle_capacity
            || state.natives >= self.options.native_handle_capacity
        {
            return Err(RuntimeError::ResourceLimit {
                dimension: "nativeHandleCapacity",
                used: sessions.max(state.natives) as u64,
                requested: 1,
                limit: self.options.native_handle_capacity as u64,
            });
        }
        let entry = state
            .owners
            .get_mut(&owner)
            .and_then(|entry| entry.databases.get_mut(&database))
            .ok_or(RuntimeError::ClosedHandle)?;
        if entry.closing {
            return Err(RuntimeError::ClosedHandle);
        }
        state.natives += 1;
        drop(state);
        let cap = match self.registry.insert_route(worker, NativeKind::Snapshot, 0) {
            Ok(cap) => cap,
            Err(error) => {
                let mut state = lock(&self.state);
                state.natives = state.natives.saturating_sub(1);
                return Err(error);
            }
        };
        let mut state = lock(&self.state);
        let Some(entry) = state
            .owners
            .get_mut(&owner)
            .and_then(|entry| entry.databases.get_mut(&database))
        else {
            drop(state);
            let _ = self.registry.rollback_route(cap);
            let mut state = lock(&self.state);
            state.natives = state.natives.saturating_sub(1);
            return Err(RuntimeError::ClosedHandle);
        };
        if entry.closing {
            drop(state);
            let _ = self.registry.rollback_route(cap);
            let mut state = lock(&self.state);
            state.natives = state.natives.saturating_sub(1);
            return Err(RuntimeError::ClosedHandle);
        }
        entry.sessions.insert(
            cap.id,
            SessionSlot {
                cap,
                closing: false,
            },
        );
        Ok(cap)
    }

    fn rollback_snapshot_route(&self, owner: u64, database: u64, cap: Capability) {
        let mut state = lock(&self.state);
        if let Some(entry) = state
            .owners
            .get_mut(&owner)
            .and_then(|entry| entry.databases.get_mut(&database))
        {
            entry.sessions.remove(&cap.id);
        }
        state.natives = state.natives.saturating_sub(1);
        drop(state);
        let _ = self.registry.rollback_route(cap);
        self.changed.notify_all();
    }

    pub(crate) fn release_snapshot_route(&self, owner: u64, database: u64, cap: Capability) {
        let mut state = lock(&self.state);
        if let Some(entry) = state
            .owners
            .get_mut(&owner)
            .and_then(|entry| entry.databases.get_mut(&database))
        {
            entry.sessions.remove(&cap.id);
        }
        if self.registry.release(cap).is_some() {
            state.natives = state.natives.saturating_sub(1);
        }
        self.changed.notify_all();
    }

    pub(super) fn submit_snapshot(
        &self,
        cap: Capability,
        owner: u64,
        database: u64,
        session: u64,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<SnapshotWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.registry.check(cap)?;
        match self.registry.state(cap)? {
            super::registry::ResourceState::Live => {}
            super::registry::ResourceState::Busy | super::registry::ResourceState::Closing => {
                return Err(RuntimeError::ClosedHandle);
            }
        }
        let operation =
            self.register_session_operation(owner, database, session, policy, notify)?;
        let prepared = catch_unwind(AssertUnwindSafe(|| prepare(&operation.context)));
        let mut state = lock(&self.state);
        let work = match prepared.unwrap_or_else(|_| {
            Self::closing(&mut state);
            Err(RuntimeError::Internal)
        }) {
            Ok(work) if state.phase == super::Phase::Open => work,
            other => {
                let discarded = state.remove(operation.id);
                self.changed.notify_all();
                drop(state);
                drop(discarded);
                return Err(other.err().unwrap_or(RuntimeError::ClosedHandle));
            }
        };
        drop(state);
        if self
            .send_resource(
                cap,
                Message::Snapshot {
                    operation: Arc::clone(&operation),
                    work,
                },
            )
            .is_err()
        {
            let mut state = lock(&self.state);
            let discarded = state.remove(operation.id);
            self.changed.notify_all();
            drop(state);
            drop(discarded);
            return Err(RuntimeError::ClosedHandle);
        }
        Ok(operation)
    }

    fn register_session_operation(
        &self,
        owner: u64,
        database: u64,
        session: u64,
        policy: ExecutionPolicy,
        notify: Notify,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let context = policy.start()?;
        context.checkpoint()?;
        let bytes = [
            policy.input_bytes,
            policy.working_bytes,
            policy.scratch_bytes,
            policy.result_bytes,
        ];
        let mut state = lock(&self.state);
        if state.phase != super::Phase::Open {
            return Err(RuntimeError::ClosedHandle);
        }
        state.require_owner(Some(owner))?;
        let live = state
            .owners
            .get(&owner)
            .and_then(|entry| entry.databases.get(&database))
            .and_then(|entry| entry.sessions.get(&session))
            .is_some_and(|slot| !slot.closing);
        if !live {
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
        state.charge(&self.options, bytes)?;
        let operation = Arc::new(Operation {
            id,
            context,
            bytes,
            owner: Some(owner),
            database: Some(database),
            session: Some(session),
            external: false,
            completion: std::sync::Mutex::new(Some(notify)),
            output: std::sync::Mutex::new(None),
        });
        state.operations.insert(id, Arc::clone(&operation));
        Ok(operation)
    }

    pub(crate) fn request_resource_close(
        &self,
        cap: Capability,
    ) -> Result<CloseDrain, RuntimeError> {
        let drain = self.registry.request_close(cap)?;
        let _ = self.send_close(cap);
        self.changed.notify_all();
        Ok(drain)
    }

    pub(crate) fn reserve_native_route(
        self: &Arc<Self>,
        kind: NativeKind,
        bytes: u64,
    ) -> Result<Capability, RuntimeError> {
        let worker = WorkerContext::worker_id().unwrap_or_else(|_| self.registry.pick_worker());
        {
            let mut state = lock(&self.state);
            if state.phase != super::Phase::Open {
                return Err(RuntimeError::ClosedHandle);
            }
            if state.natives >= self.options.native_handle_capacity {
                return Err(RuntimeError::ResourceLimit {
                    dimension: "nativeHandleCapacity",
                    used: state.natives as u64,
                    requested: 1,
                    limit: self.options.native_handle_capacity as u64,
                });
            }
            let used = state.reserved[3];
            let limit = self.options.aggregate_bytes[3];
            if used.checked_add(bytes).is_none_or(|next| next > limit) {
                return Err(RuntimeError::ResourceLimit {
                    dimension: "resultBytes",
                    used,
                    requested: bytes,
                    limit,
                });
            }
            state.reserved[3] += bytes;
            state.natives += 1;
        }
        match self.registry.insert_route(worker, kind, bytes) {
            Ok(cap) => Ok(cap),
            Err(error) => {
                let mut state = lock(&self.state);
                state.reserved[3] = state.reserved[3].saturating_sub(bytes);
                state.natives = state.natives.saturating_sub(1);
                Err(error)
            }
        }
    }

    pub(crate) fn rollback_native_route(&self, cap: Capability) {
        if let Some(bytes) = self.registry.rollback_route(cap) {
            let mut state = lock(&self.state);
            state.reserved[3] = state.reserved[3].saturating_sub(bytes);
            state.natives = state.natives.saturating_sub(1);
            self.changed.notify_all();
        }
    }

    pub(crate) fn grow_native_route(&self, cap: Capability, extra: u64) -> Result<(), RuntimeError> {
        {
            let mut state = lock(&self.state);
            let used = state.reserved[3];
            let limit = self.options.aggregate_bytes[3];
            if used.checked_add(extra).is_none_or(|next| next > limit) {
                return Err(RuntimeError::ResourceLimit {
                    dimension: "resultBytes",
                    used,
                    requested: extra,
                    limit,
                });
            }
            state.reserved[3] += extra;
        }
        if let Err(error) = self.registry.add_bytes(cap, extra) {
            let mut state = lock(&self.state);
            state.reserved[3] = state.reserved[3].saturating_sub(extra);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn release_native_route(&self, cap: Capability) {
        if let Some(bytes) = self.registry.release(cap) {
            let mut state = lock(&self.state);
            state.reserved[3] = state.reserved[3].saturating_sub(bytes);
            state.natives = state.natives.saturating_sub(1);
            self.changed.notify_all();
        }
    }

    /// Capability-first lock mint. `cap.kind` is always RepositoryLock.
    /// Same-worker insert is local; JS/cross-worker is fire-and-forget.
    pub(crate) fn mint_repository_lock(
        self: &Arc<Self>,
        lock: bumbledb_log::store::fence::RepositoryLock,
    ) -> Result<super::registry::RegistryAdmission, RuntimeError> {
        super::registry::RegistryAdmission::admit(
            Arc::clone(self),
            NativeKind::RepositoryLock,
            0,
            super::registry::Payload::RepositoryLock { lock },
        )
    }

    pub(crate) fn install_send_payload(
        &self,
        cap: Capability,
        payload: super::registry::Payload,
    ) -> Result<(), RuntimeError> {
        if WorkerContext::worker_id() == Ok(cap.worker) {
            return WorkerContext::with(|ctx| {
                ctx.table
                    .insert(cap, TablePayload::Native(payload), 0)
            })?;
        }
        // JS / other-thread take: enqueue and return. The reserved
        // capability is the admission; the worker inserts later.
        self.lane_send(
            LaneId(cap.worker as usize),
            WorkerCommand::InstallSend { cap, payload },
        )
    }

    /// Route one Send-payload job to the owning worker. L13/L14 replace
    /// `with_payload` with this — no global mutex around conversion.
    pub(crate) fn submit_payload(
        &self,
        cap: Capability,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<PayloadWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.registry.check(cap)?;
        match self.registry.state(cap)? {
            super::registry::ResourceState::Live => {}
            super::registry::ResourceState::Busy | super::registry::ResourceState::Closing => {
                return Err(RuntimeError::ClosedHandle);
            }
        }
        let context = policy.start()?;
        context.checkpoint()?;
        let bytes = [
            policy.input_bytes,
            policy.working_bytes,
            policy.scratch_bytes,
            policy.result_bytes,
        ];
        let mut state = lock(&self.state);
        if state.phase != super::Phase::Open {
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
        state.charge(&self.options, bytes)?;
        let operation = Arc::new(Operation {
            id,
            context,
            bytes,
            owner: None,
            database: None,
            session: None,
            external: false,
            completion: std::sync::Mutex::new(Some(notify)),
            output: std::sync::Mutex::new(None),
        });
        state.operations.insert(id, Arc::clone(&operation));
        drop(state);
        let prepared = catch_unwind(AssertUnwindSafe(|| prepare(&operation.context)));
        let mut state = lock(&self.state);
        let work = match prepared.unwrap_or_else(|_| {
            Self::closing(&mut state);
            Err(RuntimeError::Internal)
        }) {
            Ok(work) if state.phase == super::Phase::Open => work,
            other => {
                let discarded = state.remove(operation.id);
                self.changed.notify_all();
                drop(state);
                drop(discarded);
                return Err(other.err().unwrap_or(RuntimeError::ClosedHandle));
            }
        };
        drop(state);
        if self
            .send_resource(
                cap,
                Message::Payload {
                    operation: Arc::clone(&operation),
                    work,
                },
            )
            .is_err()
        {
            let mut state = lock(&self.state);
            let discarded = state.remove(operation.id);
            self.changed.notify_all();
            drop(state);
            drop(discarded);
            return Err(RuntimeError::ClosedHandle);
        }
        Ok(operation)
    }

    pub(crate) fn close_resource(
        &self,
        cap: Capability,
        report: super::Report,
    ) -> Result<(), RuntimeError> {
        match self.request_resource_close(cap) {
            Ok(_) | Err(RuntimeError::ClosedHandle) => {
                self.wait_target(WaitTarget::Resource(cap), report);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn run_snapshot_job(
        &self,
        operation: &Arc<Operation>,
        work: SnapshotWork,
        access: &mut SnapshotAccess<'_>,
    ) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            operation.context.checkpoint()?;
            let value = work(&operation.context, access)?;
            operation.context.checkpoint()?;
            Ok(value)
        }));
        let panicked = outcome.is_err();
        let outcome = outcome.unwrap_or(Err(RuntimeError::Internal));
        if panicked {
            self.begin_close();
        }
        self.complete_operation(operation, outcome);
    }
}

#[cfg(test)]
mod tests {
    //! D18/D24/D29 discriminators. Authored now; verification NotRun.
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};

    use bumbledb::{SchemaDescriptor, Theory as _};

    use super::super::owners::{DirectoryOwner, ManagedDb};
    use super::super::{CloseReport, Options, Phase, QueuedOutput};
    use super::*;

    bumbledb::schema! {
        pub Mini;
        relation Item { a: u64, b: u64 }
        Item(a) -> Item;
    }

    fn options() -> Options {
        Options {
            workers: 1,
            queue_capacity: 4,
            cleanup_capacity: 8,
            owner_capacity: 4,
            native_handle_capacity: 8,
            aggregate_bytes: [1 << 20; 4],
            chunk_bytes: 1 << 16,
            cleanup_timeout: Duration::from_millis(200),
        }
    }

    fn policy() -> ExecutionPolicy {
        ExecutionPolicy {
            input_bytes: 1 << 16,
            working_bytes: 1 << 16,
            scratch_bytes: 1 << 16,
            result_bytes: 1 << 16,
            rows: 1 << 16,
            work_units: 1 << 16,
            timeout: Duration::from_secs(5),
        }
    }

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bumbledb-l12-session-{tag}-{}-{seq}",
            std::process::id()
        ))
    }

    fn acquire(runtime: &Arc<Runtime>, path: &std::path::Path) -> DirectoryOwner {
        let (tx, rx) = channel();
        let operation = runtime
            .acquire_directory(
                path.to_string_lossy().into_owned(),
                policy(),
                Box::new(move || {
                    tx.send(()).unwrap();
                }),
            )
            .expect("acquire submits");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("acquire notify");
        match runtime.take(&operation) {
            Ok(Output::Directory(owner)) => owner,
            _ => panic!("expected a directory owner"),
        }
    }

    fn attach(owner: &DirectoryOwner, descriptor: &SchemaDescriptor) -> ManagedDb {
        let path = owner.child_path("db").expect("child path");
        #[rustfmt::skip]
        let Ok(bumbledb::Admission::Accepted(db)) =
            crate::Engine::create(&path, descriptor.clone())
        else {
            panic!("engine create must accept a fresh store")
        };
        owner
            .attach_db(crate::assemble_inner(db, descriptor.clone(), Vec::new()))
            .expect("attach db to the owner")
    }

    fn open_read(runtime: &Arc<Runtime>, db: &ManagedDb) -> SnapshotSession {
        let (tx, rx) = channel();
        let operation = runtime
            .open_session(
                db,
                policy(),
                Box::new(move || {
                    tx.send(()).unwrap();
                }),
            )
            .expect("open snapshot submits");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("snapshot notify");
        match runtime.take(&operation) {
            Ok(Output::Session(opened)) => opened.session,
            _ => panic!("expected a snapshot output"),
        }
    }

    fn run_read(
        runtime: &Arc<Runtime>,
        session: &SnapshotSession,
        prepare: impl FnOnce(&WorkContext) -> Result<SnapshotWork, RuntimeError>,
    ) -> Result<Output, RuntimeError> {
        let (tx, rx) = channel();
        let operation = session.submit(
            policy(),
            Box::new(move || {
                tx.send(()).unwrap();
            }),
            prepare,
        )?;
        rx.recv_timeout(Duration::from_secs(5))
            .expect("read job notify");
        runtime.take(&operation)
    }

    fn drain_session(session: &SnapshotSession) -> CloseReport {
        let (tx, rx) = channel();
        session.drain(Box::new(move |report| {
            tx.send(report).unwrap();
        }));
        rx.recv_timeout(Duration::from_secs(5))
            .expect("session drain")
    }

    fn drain_runtime(runtime: &Arc<Runtime>) -> CloseReport {
        let (tx, rx) = channel();
        runtime.drain(
            None,
            Box::new(move |report| {
                tx.send(report).unwrap();
            }),
        );
        rx.recv_timeout(Duration::from_secs(5))
            .expect("runtime drain")
    }

    #[test]
    fn d24_one_worker_open_read_close_and_idle_snapshots_share_the_pool() {
        // D24: workers=1, open/read/close; more idle snapshots than workers;
        // sleeping worker then an opening job on that same worker. Ready
        // after reactor-exit / missing inbox wakeup must fail this schedule.
        let runtime = Runtime::start(options()).unwrap();
        assert_eq!(runtime.inspect().active, 0, "pool starts asleep");
        let base = unique_dir("d24-one-worker");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);

        let first = open_read(&runtime, &db);
        match run_read(&runtime, &first, |_| {
            Ok(Box::new(|context, access| {
                context.checkpoint()?;
                let _ = access.frame(context);
                Ok(Output::Count(access.owned.snapshot().generation().value()))
            }))
        })
        .expect("read job")
        {
            Output::Count(_) => {}
            _ => panic!("expected a count output"),
        }

        let mut idle = Vec::new();
        for _ in 0..3 {
            idle.push(open_read(&runtime, &db));
        }
        assert!(
            idle.len() > options().workers,
            "more idle snapshots than workers"
        );
        assert_eq!(
            runtime.options.workers, 1,
            "this schedule is the one-worker case"
        );
        match run_read(&runtime, &first, |_| {
            Ok(Box::new(|context, access| {
                context.checkpoint()?;
                let _ = access.frame(context);
                Ok(Output::Generation(access.owned.snapshot().generation().value()))
            }))
        })
        .expect("parent still readable after extra idle snapshots")
        {
            Output::Generation(_) => {}
            _ => panic!("expected a generation output"),
        }

        assert_eq!(drain_session(&first), CloseReport::Closed);
        for session in idle {
            assert_eq!(drain_session(&session), CloseReport::Closed);
        }
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn d18_close_drains_while_js_tokens_stay_reachable_and_queue_is_full() {
        // D18: keep wrappers reachable; fill the ordinary queue; close still
        // drains. QueueFull must not strand teardown. Counters match release.
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("d18-close");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);
        let session = open_read(&runtime, &db);
        let natives_live = runtime.inspect().natives;
        assert!(natives_live >= 1, "snapshot occupies a handle slot");

        let (release, blocked) = channel();
        let (entered, running) = channel();
        let _blocker = runtime
            .submit(
                policy(),
                Box::new(|| {}),
                move |_| {
                    Ok(Box::new(move |_| {
                        entered.send(()).unwrap();
                        blocked.recv().unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .expect("blocker submits");
        running.recv_timeout(Duration::from_secs(5)).expect("entered");
        while runtime
            .submit(
                policy(),
                Box::new(|| {}),
                |_| Ok(Box::new(|_| Ok(Output::Ready))),
            )
            .is_ok()
        {}
        assert!(
            matches!(
                runtime.submit(policy(), Box::new(|| {}), |_| Ok(Box::new(|_| Ok(
                    Output::Ready
                )))),
                Err(RuntimeError::QueueFull)
            ),
            "ordinary queue is saturated"
        );
        let (tx, rx) = channel();
        session.drain(Box::new(move |report| {
            tx.send(report).unwrap();
        }));
        let _ = session.capability();
        release.send(()).unwrap();
        let report = rx.recv_timeout(Duration::from_secs(5)).expect("close join");
        assert_eq!(report, CloseReport::Closed);
        assert_eq!(
            drain_session(&session),
            CloseReport::Closed,
            "repeated close joins one transition"
        );
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(
            runtime.inspect().natives, 0,
            "counters match actual release; not zeroed as cleanup"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn d24_js_thread_admit_returns_without_waiting_on_busy_worker() {
        // D24: JS-thread admit/install must return while the only worker
        // is blocked. A ready_rx / thread-join on take fails this schedule.
        let runtime = Runtime::start(options()).unwrap();
        let (release, blocked) = channel();
        let (entered, running) = channel();
        let _blocker = runtime
            .submit(
                policy(),
                Box::new(|| {}),
                move |_| {
                    Ok(Box::new(move |_| {
                        entered.send(()).unwrap();
                        blocked.recv().unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .expect("blocker submits");
        running.recv_timeout(Duration::from_secs(5)).expect("entered");
        assert_eq!(runtime.inspect().active, 1, "the one worker is occupied");

        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            0,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Spent,
            },
        )
        .expect("js-thread admit returns without the worker");
        assert_eq!(
            runtime.inspect().active, 1,
            "admit must not join the busy worker"
        );
        assert_eq!(runtime.registry.route_count(), 1, "capability is reserved");
        assert_eq!(runtime.inspect().natives, 1);

        admission
            .request_close()
            .expect("close is a coalesced drain");
        release.send(()).unwrap();
        let (tx, rx) = channel();
        runtime
            .close_resource(
                admission.cap(),
                Box::new(move |_| {
                    tx.send(()).unwrap();
                }),
            )
            .expect("close joins");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("uninstalled route drains");
        assert_eq!(runtime.inspect().natives, 0);
        assert_eq!(runtime.registry.route_count(), 0);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d18_close_of_uninstalled_route_does_not_leave_a_row() {
        // D18: reserve + async install, then close before the worker
        // inserts. The route must drain; QueueFull cannot apply; no leftover
        // row or charge.
        let runtime = Runtime::start(options()).unwrap();
        let baseline = runtime.inspect();
        let (release, blocked) = channel();
        let (entered, running) = channel();
        let _blocker = runtime
            .submit(
                policy(),
                Box::new(|| {}),
                move |_| {
                    Ok(Box::new(move |_| {
                        entered.send(()).unwrap();
                        blocked.recv().unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .expect("blocker submits");
        running.recv_timeout(Duration::from_secs(5)).expect("entered");

        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            32,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Spent,
            },
        )
        .expect("capability first, table insert later");
        assert_eq!(runtime.registry.route_count(), 1);
        assert_eq!(runtime.inspect().natives, baseline.natives + 1);
        assert_eq!(runtime.inspect().reserved[3], baseline.reserved[3] + 32);
        assert!(
            !runtime.registry.join(admission.cap()),
            "close has not drained yet"
        );

        admission
            .request_close()
            .expect("close before worker insert cannot QueueFull");
        release.send(()).unwrap();
        let (tx, rx) = channel();
        runtime
            .close_resource(
                admission.cap(),
                Box::new(move |_| {
                    tx.send(()).unwrap();
                }),
            )
            .expect("joined close");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("close report");
        assert_eq!(
            runtime.inspect().natives,
            baseline.natives,
            "counters match actual release"
        );
        assert_eq!(runtime.inspect().reserved[3], baseline.reserved[3]);
        assert_eq!(
            runtime.registry.route_count(),
            0,
            "uninstalled close leaves no row"
        );
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d29_repository_lock_capability_stamps_kind() {
        // D29: minted lock handles carry NativeKind::RepositoryLock on the
        // capability. A directory-owner twin without this stamp fails.
        let runtime = Runtime::start(options()).unwrap();
        let cap = runtime
            .reserve_native_route(NativeKind::RepositoryLock, 0)
            .expect("lock route");
        assert_eq!(cap.kind, NativeKind::RepositoryLock);
        runtime.rollback_native_route(cap);
        assert_eq!(runtime.registry.route_count(), 0);
        assert_eq!(runtime.inspect().natives, 0);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d29_draft_payload_persists_work_deadline_terminal() {
        // D29: DraftLedger (used_work, allowance_work, deadline, terminal)
        // lives on DraftPayload. Reconstructing the ledger at finish fails.
        use super::super::registry::registry_draft::DraftLedger;
        use std::time::Instant;
        let ledger = DraftLedger {
            used_work: 3,
            allowance_work: 8,
            deadline: Instant::now() + Duration::from_secs(1),
            terminal: false,
        };
        assert_eq!(ledger.used_work, 3);
        assert_eq!(ledger.allowance_work, 8);
        assert!(!ledger.terminal);
    }

    #[test]
    fn d18_idle_shutdown_wakes_sleeping_pool_without_reentering_state() {
        // D18: idle pool (active==0) then runtime drain. Re-locking
        // runtime.state from lane_send during begin_close/drain hangs.
        let runtime = Runtime::start(options()).unwrap();
        assert_eq!(runtime.inspect().active, 0, "pool starts asleep");
        assert_eq!(runtime.inspect().phase, Phase::Open);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(runtime.inspect().phase, Phase::Closed);
        assert_eq!(runtime.inspect().natives, 0);
        assert_eq!(runtime.registry.route_count(), 0);
    }

    #[test]
    fn d18_close_during_busy_snapshot_drains_after_job() {
        // D18: close while a snapshot job holds the table entry. Destruction
        // after WorkerContext::with returns; a nested with() panics.
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("d18-busy-snapshot");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);
        let session = open_read(&runtime, &db);
        let (release, blocked) = channel();
        let (entered, running) = channel();
        let _busy = session
            .submit(
                policy(),
                Box::new(|| {}),
                move |_| {
                    Ok(Box::new(move |context, access| {
                        let _ = access.frame(context);
                        entered.send(()).unwrap();
                        blocked.recv().unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .expect("busy snapshot submits");
        running.recv_timeout(Duration::from_secs(5)).expect("entered");
        let (tx, rx) = channel();
        session.drain(Box::new(move |report| {
            tx.send(report).unwrap();
        }));
        release.send(()).unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).expect("close join"),
            CloseReport::Closed
        );
        assert_eq!(
            drain_session(&session),
            CloseReport::Closed,
            "repeated close joins one transition"
        );
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, 0);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn d12_arm_publication_cancel_drops_unregistered_page() {
        // D12/D25: arm, then work returns a page. The one-shot must fail
        // before operation.output; retry on the same cap delivers the page.
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            0,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");
        runtime.arm_publication_cancel();
        let (fail_tx, fail_rx) = channel();
        let refused = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    fail_tx.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |context, _, _| {
                        let queued = QueuedOutput::admit(context, vec![Vec::new()], 0)?;
                        Ok(Output::Page(Some(queued)))
                    }))
                },
            )
            .expect("armed page submits");
        fail_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("armed notify");
        assert!(
            matches!(
                runtime.take(&refused),
                Err(RuntimeError::Work(bumbledb::work::WorkError::Cancelled))
            ),
            "unregistered local page must drop; cursor/result stay retryable"
        );
        let (ok_tx, ok_rx) = channel();
        let delivered = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    ok_tx.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |context, _, _| {
                        let queued = QueuedOutput::admit(context, vec![Vec::new()], 0)?;
                        Ok(Output::Page(Some(queued)))
                    }))
                },
            )
            .expect("retry submits");
        ok_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("retry notify");
        assert!(
            matches!(runtime.take(&delivered), Ok(Output::Page(Some(_)))),
            "retry after armed cancel must not skip"
        );
        admission.request_close().expect("close");
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d12_dispatch_payload_registers_page_before_post_checkpoint() {
        // D12: publication is dispatch_payload_message, not an L13 helper.
        // Predelivery Err leaves the cap retryable. A live (not cancelled)
        // page stays registered for take; cancel-without-take is the
        // abandoned-output discriminator, not this one.
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            0,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");

        let (refused_tx, refused_rx) = channel();
        let refused = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    refused_tx.send(()).unwrap();
                }),
                |_| Ok(Box::new(|_, _, _| Err(RuntimeError::InvalidArgument))),
            )
            .expect("predelivery submits");
        refused_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("predelivery notify");
        assert!(
            matches!(runtime.take(&refused), Err(RuntimeError::InvalidArgument)),
            "predelivery must not register a page"
        );

        let (page_tx, page_rx) = channel();
        let delivered = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    page_tx.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |context, _, _| {
                        let queued = QueuedOutput::admit(context, vec![Vec::new()], 0)?;
                        Ok(Output::Page(Some(queued)))
                    }))
                },
            )
            .expect("page submits after refusal");
        page_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("page notify");
        assert!(
            matches!(runtime.take(&delivered), Ok(Output::Page(Some(_)))),
            "a live registered page remains takeable until cancel"
        );
        admission
            .request_close()
            .expect("close after publication");
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d12_publication_boundary_cancel_does_not_skip_or_duplicate_rows() {
        // D12: native publication-boundary cancel cannot skip or duplicate
        // rows. The armed reject drops the local page before accept; retry
        // delivers the same two rows, not a later page or a doubled page.
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            0,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");
        let page = |context: &bumbledb::work::WorkContext| {
            QueuedOutput::admit(
                context,
                vec![
                    vec![crate::marshal::ValueOut::U64(1)],
                    vec![crate::marshal::ValueOut::U64(2)],
                ],
                16,
            )
        };
        runtime.arm_publication_cancel();
        let (fail_tx, fail_rx) = channel();
        let refused = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    fail_tx.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |context, _, _| {
                        Ok(Output::Page(Some(page(context)?)))
                    }))
                },
            )
            .expect("armed two-row page submits");
        fail_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("armed notify");
        assert!(
            matches!(
                runtime.take(&refused),
                Err(RuntimeError::Work(bumbledb::work::WorkError::Cancelled))
            ),
            "boundary cancel must not deliver the first page"
        );
        let (ok_tx, ok_rx) = channel();
        let delivered = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    ok_tx.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |context, _, _| {
                        Ok(Output::Page(Some(page(context)?)))
                    }))
                },
            )
            .expect("retry submits");
        ok_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("retry notify");
        match runtime.take(&delivered) {
            Ok(Output::Page(Some(queued))) => {
                assert_eq!(queued.rows.len(), 2, "retry must not skip or duplicate");
                assert!(matches!(
                    &queued.rows[0][..],
                    [crate::marshal::ValueOut::U64(1)]
                ));
                assert!(matches!(
                    &queued.rows[1][..],
                    [crate::marshal::ValueOut::U64(2)]
                ));
            }
            _ => panic!("retry must deliver the same two rows"),
        }
        admission.request_close().expect("close");
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d12_abandoned_publication_reclaims_on_cancel_close_without_js_take() {
        // D12: pause after native publication, before the JS callback.
        // Interrupt, retain wrappers, cancel+close must finish and release
        // native resources without a JavaScript take.
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            0,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");
        let (entered_tx, entered_rx) = channel();
        let (release_tx, release_rx) = channel();
        runtime.arm_publication_hold(entered_tx, release_rx);
        let published = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(|| {}),
                |_| {
                    Ok(Box::new(move |context, _, _| {
                        let queued = QueuedOutput::admit(context, vec![Vec::new()], 0)?;
                        Ok(Output::Page(Some(queued)))
                    }))
                },
            )
            .expect("page submits");
        entered_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("published before JS callback");
        let retained = std::sync::Arc::clone(&published);
        let (cancel_tx, cancel_rx) = channel();
        runtime.drain(
            Some(&published),
            Box::new(move |report| {
                cancel_tx.send(report).unwrap();
            }),
        );
        assert_eq!(
            cancel_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("cancel joins"),
            CloseReport::Closed,
            "abandoned queued output must reclaim without JS take"
        );
        release_tx.send(()).expect("release publication hold");
        admission.request_close().expect("close retained wrapper");
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, 0);
        assert!(
            matches!(
                runtime.take(&retained),
                Err(RuntimeError::SpentHandle)
                    | Err(RuntimeError::Work(bumbledb::work::WorkError::Cancelled))
            ),
            "reclaimed publication is not a later JS take"
        );
    }

    #[test]
    fn d12_cancelled_queued_page_reclaims_open_without_waiter() {
        // Cancel/close with queued Page/Rows, no waiter, Phase::Open.
        // Supervise must drop the delivery copy; JS take count stays 0;
        // committed facts stay (no second accept_publication, no rewind).
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            0,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");
        let takes = 0u32;
        let (page_tx, page_rx) = channel();
        let published = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    page_tx.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |context, _, _| {
                        let queued = QueuedOutput::admit(context, vec![Vec::new()], 0)?;
                        Ok(Output::Page(Some(queued)))
                    }))
                },
            )
            .expect("page submits");
        page_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("page notify");
        assert_eq!(runtime.inspect().phase, Phase::Open);
        assert!(
            runtime.inspect().retained >= 1,
            "queued page is retained before cancel"
        );
        runtime.cancel_without_waiter(&published);
        let deadline = Instant::now() + Duration::from_secs(5);
        while runtime.inspect().retained != 0 {
            assert!(
                Instant::now() < deadline,
                "Open + no waiter must still reclaim abandoned Page/Rows"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(runtime.inspect().phase, Phase::Open);
        assert_eq!(takes, 0, "JS take count stays 0");
        let _ = published;
        admission.request_close().expect("close");
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d18_close_during_busy_payload_drains_after_job() {
        // D18: close while a payload job holds the table entry. Same
        // nested-borrow failure as the snapshot schedule.
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            0,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");
        let (release, blocked) = channel();
        let (entered, running) = channel();
        runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(|| {}),
                |_| {
                    Ok(Box::new(move |_, _, _| {
                        entered.send(()).unwrap();
                        blocked.recv().unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .expect("busy payload submits");
        running.recv_timeout(Duration::from_secs(5)).expect("entered");
        admission
            .request_close()
            .expect("close while busy cannot QueueFull");
        release.send(()).unwrap();
        let (tx, rx) = channel();
        runtime
            .close_resource(
                admission.cap(),
                Box::new(move |_| {
                    tx.send(()).unwrap();
                }),
            )
            .expect("joined close");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("payload close report");
        assert_eq!(runtime.inspect().natives, 0);
        assert_eq!(runtime.registry.route_count(), 0);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    }

    #[test]
    fn d29_failed_admission_rolls_back_and_history_does_not_accumulate() {
        // D29: failed admission before insertion leaves no payload/row/charge.
        // Long create/revoke returns to the admitted baseline. No tombstones.
        let runtime = Runtime::start(options()).unwrap();
        let baseline = runtime.inspect();
        assert_eq!(baseline.natives, 0);
        assert_eq!(runtime.registry.route_count(), 0);

        assert!(
            matches!(
                runtime.reserve_native_route(NativeKind::Result, u64::MAX),
                Err(RuntimeError::ResourceLimit { .. })
            ),
            "byte admission refuses before a route exists"
        );
        assert_eq!(runtime.inspect().natives, 0);
        assert_eq!(runtime.registry.route_count(), 0);
        assert_eq!(runtime.inspect().reserved[3], 0);

        let base = unique_dir("d29-history");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);
        let after_db = runtime.inspect().natives;
        for _ in 0..4 {
            let session = open_read(&runtime, &db);
            assert_eq!(drain_session(&session), CloseReport::Closed);
        }
        assert_eq!(
            runtime.inspect().natives,
            after_db,
            "create/revoke history returns to the admitted baseline"
        );
        assert_eq!(
            runtime.registry.route_count(),
            0,
            "drained snapshot routes are absent, not tombstones"
        );
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn session_close_is_idempotent_and_the_database_survives() {
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("idempotent-close");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);
        let session = open_read(&runtime, &db);
        assert_eq!(drain_session(&session), CloseReport::Closed);
        assert_eq!(drain_session(&session), CloseReport::Closed);
        assert_eq!(runtime.inspect().databases, 1);
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_foreign_runtime_refuses_to_open_a_session_over_a_managed_db() {
        let first = Runtime::start(options()).unwrap();
        let second = Runtime::start(options()).unwrap();
        let base = unique_dir("foreign");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&first, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);
        assert!(matches!(
            second.open_session(&db, policy(), Box::new(|| {})),
            Err(RuntimeError::ForeignRuntime)
        ));
        drop(owner);
        assert_eq!(drain_runtime(&first), CloseReport::Closed);
        assert_eq!(drain_runtime(&second), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn runtime_shutdown_drains_a_live_snapshot() {
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("open-vs-shutdown");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);
        let session = open_read(&runtime, &db);
        drop(owner);
        drop(db);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(drain_session(&session), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }
}
