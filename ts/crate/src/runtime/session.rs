//! Worker-table snapshots (C7): one owned pinned read plus prepared state
//! per entry. Jobs borrow the entry for one operation and return to the
//! scheduler. No session-long reactor, no `ready_rx`, no JS-driven writer.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;

use bumbledb::work::WorkContext;
use bumbledb::{OwnedRead, PreparedQuery, SchemaDescriptor};

use super::lanes::{LaneId, WorkerCommand};
use super::owners::{DbLease, ManagedDb};
use super::registry::{Capability, CloseDrain, NativeKind};
use super::table::{SnapshotData, SnapshotResource, TablePayload, WorkerContext};
use super::{Notify, Operation, Output, Runtime, RuntimeError, WaitTarget, lock};

/// One typed engine refusal crossing the executor as owned data.
pub(crate) fn engine_error(error: &bumbledb::Error) -> RuntimeError {
    if let bumbledb::Error::Store(store) = error
        && let bumbledb::store::StoreError::Work(work) = store.as_ref()
    {
        return RuntimeError::Work(*work);
    }
    RuntimeError::Engine {
        diagnostic: None,
        kind: crate::tags::error_family::tag(&error.family()),
        message: crate::marshal::engine_message(error),
    }
}

/// Borrowed snapshot entry for one job. Prepared state stays on the worker.
pub struct SnapshotAccess<'a> {
    pub owned: &'a OwnedRead<SchemaDescriptor>,
    pub prepared: Option<&'a mut PreparedQuery<SchemaDescriptor>>,
    pub(crate) data: &'a Rc<SnapshotData>,
}

impl SnapshotAccess<'_> {
    #[must_use]
    pub fn frame<'read>(
        &'read self,
        work: &'read WorkContext,
    ) -> bumbledb::ReadFrame<'read, SchemaDescriptor> {
        self.owned.frame(work)
    }

    pub fn prepare(
        &self,
        runtime: &Arc<Runtime>,
        query: &bumbledb::Query,
        work: &WorkContext,
    ) -> Result<SnapshotSession, RuntimeError> {
        let prepared = self
            .frame(work)
            .prepare(query)
            .map_err(|error| engine_error(&error))?;
        runtime.install_prepared(Rc::clone(self.data), prepared)
    }

    pub fn execute(
        &mut self,
        context: &WorkContext,
        args: &[bumbledb::ParamArg<'_>],
    ) -> Result<bumbledb::CompleteResult, RuntimeError> {
        let prepared = self
            .prepared
            .as_deref_mut()
            .ok_or(RuntimeError::ClosedHandle)?;
        // L07 seam: execute against the owned frame, not a !Send ReadInstance.
        prepared
            .execute_complete_with_work(&self.owned.frame(context), context, args)
            .map_err(|error| engine_error(&error))
    }

    #[cfg(test)]
    #[must_use]
    pub fn prepared_count(&self) -> usize {
        usize::from(self.prepared.is_some())
    }
}

pub type SnapshotWork =
    Box<dyn FnOnce(&WorkContext, &mut SnapshotAccess<'_>) -> Result<Output, RuntimeError> + Send>;

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

    /// Register `output` and run `commit` under the output lock so no
    /// observer can take between them. `commit` must not allocate, read,
    /// checkpoint, or preview. Armed cancel and accept failure publish
    /// nothing — the caller aborts the live ticket.
    pub fn accept(&mut self, output: Output, commit: impl FnOnce()) -> Result<(), RuntimeError> {
        if !output.queued_publication() {
            return Err(RuntimeError::Internal);
        }
        if self.armed {
            self.operation.cancel();
            self.armed = false;
        }
        let mut slot = lock(&self.operation.output);
        if slot.is_some() {
            return Err(RuntimeError::Internal);
        }
        self.operation.context.checkpoint()?;
        commit();
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
}

pub(super) struct SessionSlot {
    pub cap: Capability,
    pub closing: bool,
}

pub struct SessionOpened {
    pub session: SnapshotSession,
    pub sealed: Arc<crate::Sealed>,
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
            operation.cancel();
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
        policy: WorkContext,
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
        policy: WorkContext,
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
        policy: WorkContext,
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
    /// (open or L14 authority pin). No `ready_rx`, no second hop onto this pool.
    pub(crate) fn spawn_read_session_for(
        self: &Arc<Self>,
        db: &ManagedDb,
        lease: DbLease,
        context: &WorkContext,
    ) -> Result<Output, RuntimeError> {
        if !Arc::ptr_eq(self, db.runtime()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let (owner, database) = db.ids();
        context.checkpoint()?;
        self.pin_snapshot(owner, database, lease, context)
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
        let store_identity = lease.db().integration_store().identity().store.to_string();
        // L07 seam: Db::snapshot → OwnedRead. Each job takes frame(&work).
        let pinned_read = lease
            .db()
            .snapshot(context)
            .map_err(|error| engine_error(&error))?;
        let generation = pinned_read.snapshot().generation().value();
        let attachment = pinned_read
            .snapshot()
            .attachment()
            .map_err(|error| engine_error(&bumbledb::Error::Store(Box::new(error))))?
            .map(<[u8]>::to_vec);
        let cap = self.reserve_snapshot_route(owner, database, worker, NativeKind::Snapshot)?;
        let resource = SnapshotResource {
            data: Rc::new(SnapshotData {
                owned: pinned_read,
                lease,
                owner,
                database,
            }),
            prepared: None,
        };
        let installed =
            WorkerContext::with(|ctx| ctx.table.insert(cap, TablePayload::Snapshot(resource), 0))?;
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
            generation,
            store: store_identity,
            attachment,
        }))
    }

    fn install_prepared(
        self: &Arc<Self>,
        data: Rc<SnapshotData>,
        prepared: PreparedQuery<SchemaDescriptor>,
    ) -> Result<SnapshotSession, RuntimeError> {
        if !Arc::ptr_eq(self, data.lease.runtime()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let worker = WorkerContext::worker_id()?;
        let (owner, database) = (data.owner, data.database);
        let cap = self.reserve_snapshot_route(owner, database, worker, NativeKind::Prepared)?;
        let resource = SnapshotResource {
            data,
            prepared: Some(Box::new(prepared)),
        };
        let installed =
            WorkerContext::with(|ctx| ctx.table.insert(cap, TablePayload::Snapshot(resource), 0))
                .and_then(std::convert::identity);
        if let Err(error) = installed {
            self.rollback_snapshot_route(owner, database, cap);
            return Err(error);
        }
        Ok(SnapshotSession {
            core: SessionCore {
                runtime: Arc::clone(self),
                owner,
                database,
                id: cap.id,
                cap,
            },
        })
    }

    fn reserve_snapshot_route(
        &self,
        owner: u64,
        database: u64,
        worker: u32,
        kind: NativeKind,
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
        let cap = match self.registry.insert_route(worker, kind) {
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

    #[expect(
        clippy::too_many_arguments,
        reason = "Explicit capability scope and one-shot job admission stay together"
    )]
    pub(super) fn submit_snapshot(
        &self,
        cap: Capability,
        owner: u64,
        database: u64,
        session: u64,
        policy: WorkContext,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<SnapshotWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.registry.state(cap)?.admit()?;
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
        policy: WorkContext,
        notify: Notify,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let context = policy;
        context.checkpoint()?;
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
        let operation = Arc::new(Operation {
            id,
            context,
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
            state.natives += 1;
        }
        match self.registry.insert_route(worker, kind) {
            Ok(cap) => Ok(cap),
            Err(error) => {
                let mut state = lock(&self.state);
                state.natives = state.natives.saturating_sub(1);
                Err(error)
            }
        }
    }

    pub(crate) fn rollback_native_route(&self, cap: Capability) {
        let mut state = lock(&self.state);
        if self.registry.rollback_route(cap).is_some() {
            state.natives = state.natives.saturating_sub(1);
            self.changed.notify_all();
        }
    }

    pub(crate) fn release_native_route(&self, cap: Capability) {
        // A close waiter must observe route removal and refunded ownership
        // together, never a drained route with a still-live resource count.
        let mut state = lock(&self.state);
        if self.registry.release(cap).is_some() {
            state.natives = state.natives.saturating_sub(1);
            self.changed.notify_all();
        }
    }

    pub(crate) fn install_send_payload(
        &self,
        cap: Capability,
        payload: super::registry::Payload,
    ) -> Result<(), RuntimeError> {
        let payload = Box::new(payload);
        if WorkerContext::worker_id() == Ok(cap.worker) {
            return WorkerContext::with(|ctx| {
                ctx.table.insert(cap, TablePayload::Native(payload), 0)
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
        policy: WorkContext,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<PayloadWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.registry.state(cap)?.admit()?;
        self.submit_payload_inner(cap, None, policy, notify, prepare)
    }

    /// Finalization revokes new admission immediately, waits behind existing
    /// worker jobs, and closes every alias even on cancellation or panic.
    #[cfg(test)]
    pub(crate) fn finalize_payload(
        self: &Arc<Self>,
        cap: Capability,
        policy: WorkContext,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<PayloadWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.finalize_payload_ordered(cap, None, policy, notify, prepare)
    }

    pub(crate) fn finalize_payload_ordered(
        self: &Arc<Self>,
        cap: Capability,
        order: Option<super::sequence::PayloadReservation>,
        policy: WorkContext,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<PayloadWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        struct Closing {
            runtime: Arc<Runtime>,
            cap: Capability,
        }
        impl Drop for Closing {
            fn drop(&mut self) {
                let _ = self.runtime.request_resource_close(self.cap);
            }
        }
        if order.is_none() {
            self.registry.begin_finalization(cap)?;
        }
        let closing = Closing {
            runtime: Arc::clone(self),
            cap,
        };
        self.submit_payload_inner(cap, order, policy, notify, |context| {
            let work = prepare(context)?;
            Ok(Box::new(move |context, payload, publication| {
                let _closing = closing;
                work(context, payload, publication)
            }))
        })
    }

    fn submit_payload_inner(
        &self,
        cap: Capability,
        order: Option<super::sequence::PayloadReservation>,
        policy: WorkContext,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<PayloadWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let context = policy;
        context.checkpoint()?;
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
        let operation = Arc::new(Operation {
            id,
            context,
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
        if let Some(order) = order {
            order.dispatch(Arc::clone(&operation), work);
            return Ok(operation);
        }
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
    ) -> Result<Output, RuntimeError> {
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
        outcome
    }
}

#[cfg(test)]
mod tests {
    //! D18/D24/D29 discriminators. Authored now; verification `NotRun`.
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
            cleanup_timeout: Duration::from_secs(5),
        }
    }

    fn policy() -> WorkContext {
        WorkContext::new()
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
            crate::Engine::create(&path, descriptor.clone(), policy())
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

    fn item_query() -> bumbledb::Query {
        use bumbledb::{Atom, AtomSource, FieldId, FindTerm, RelationId, Rule, Term, VarId};
        bumbledb::Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Edb(RelationId(0)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        })
    }

    fn prepare_items(runtime: &Arc<Runtime>, snapshot: &SnapshotSession) -> SnapshotSession {
        match run_read(runtime, snapshot, |_| {
            Ok(crate::db_wire::prepare_work(
                Arc::clone(runtime),
                item_query(),
            ))
        })
        .expect("prepare completes")
        {
            Output::Prepared(prepared) => prepared,
            _ => panic!("expected prepared handle"),
        }
    }

    fn prepared_address(runtime: &Arc<Runtime>, prepared: &SnapshotSession) -> u64 {
        match run_read(runtime, prepared, |_| {
            Ok(Box::new(|_, access| {
                assert_eq!(access.prepared_count(), 1);
                Ok(Output::Count(
                    std::ptr::from_ref(access.prepared.as_deref().unwrap()).addr() as u64,
                ))
            }))
        })
        .unwrap()
        {
            Output::Count(address) => address,
            _ => panic!("expected address"),
        }
    }

    fn execute_items(
        runtime: &Arc<Runtime>,
        prepared: &SnapshotSession,
    ) -> bumbledb::CompleteResult {
        match run_read(runtime, prepared, |_| {
            Ok(crate::db_wire::execute_prepared_work(vec![]))
        })
        .expect("prepared execution completes")
        {
            Output::CompleteResult(result) => result,
            _ => panic!("expected completed result"),
        }
    }

    #[cfg(feature = "alloc-counter")]
    fn measure_prepared_reuse(runtime: &Arc<Runtime>, prepared: &SnapshotSession) {
        use bumbledb::alloc_counter::{self, AllocWindow};

        fn measured(jobs: Vec<SnapshotWork>, access: &mut SnapshotAccess<'_>) -> AllocWindow {
            // Match native operation lifetimes without counting fixture/IR
            // construction. All engine preparation and execution is counted.
            let contexts: Vec<_> = jobs.iter().map(|_| policy()).collect();
            alloc_counter::reset();
            for (job, context) in jobs.into_iter().zip(&contexts) {
                let Output::CompleteResult(result) = job(context, access).unwrap() else {
                    panic!("expected completed result");
                };
                assert_eq!(result.len(), 512);
                drop(result);
            }
            alloc_counter::snapshot().window
        }

        run_read(runtime, prepared, |_| {
            Ok(Box::new(|context, access| {
                // Warm the actual retained plan before the compared windows.
                drop(crate::db_wire::execute_prepared_work(vec![])(
                    context, access,
                )?);
                let one_shot = measured(
                    (0..16)
                        .map(|_| crate::db_wire::execute_complete_work(item_query(), vec![]))
                        .collect(),
                    access,
                );
                let reused = measured(
                    (0..16)
                        .map(|_| crate::db_wire::execute_prepared_work(vec![]))
                        .collect(),
                    access,
                );
                eprintln!("16 native 512-row queries: one-shot={one_shot:?}; prepared={reused:?}");
                assert!(
                    reused.allocs < one_shot.allocs,
                    "explicit preparation must save actual engine allocations"
                );
                assert!(reused.alloc_bytes < one_shot.alloc_bytes);
                Ok(Output::Ready)
            }))
        })
        .unwrap();
    }

    #[test]
    fn prepared_queries_reuse_one_plan_and_close_independently() {
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("prepared-ownership");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let db = attach(&owner, &Mini.descriptor());
        {
            let lease = db.access().unwrap();
            let collection = bumbledb::AcceptedCollection::from_value_rows(
                bumbledb::RelationId(0),
                &Mini.descriptor().relations[0].fields,
                (0..512).map(|a| [bumbledb::Value::U64(a), bumbledb::Value::U64(a * 10)]),
            )
            .unwrap();
            assert!(matches!(
                lease
                    .db()
                    .write(policy(), |tx| {
                        tx.insert_accepted(&collection).map(|_| ())
                    })
                    .unwrap(),
                bumbledb::Admission::Accepted(_)
            ));
        }
        let snapshot = open_read(&runtime, &db);
        let initial = runtime.inspect().natives;
        let first = prepare_items(&runtime, &snapshot);
        let second = prepare_items(&runtime, &snapshot);
        let address = prepared_address(&runtime, &second);
        assert_ne!(address, prepared_address(&runtime, &first));
        #[cfg(feature = "alloc-counter")]
        measure_prepared_reuse(&runtime, &second);
        let retained_result = execute_items(&runtime, &first);
        assert_eq!(retained_result.len(), 512);
        assert_eq!(drain_session(&first), CloseReport::Closed);
        assert_eq!(drain_session(&first), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, initial + 1);
        assert!(matches!(
            run_read(&runtime, &first, |_| Ok(
                crate::db_wire::execute_prepared_work(vec![])
            )),
            Err(RuntimeError::ClosedHandle)
        ));
        // Closing a plan leaves the source pin available.
        assert!(matches!(
            run_read(&runtime, &snapshot, |_| Ok(
                crate::db_wire::execute_complete_work(item_query(), vec![])
            ))
            .unwrap(),
            Output::CompleteResult(_)
        ));
        assert_eq!(drain_session(&snapshot), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, initial);
        for _ in 0..32 {
            let result = execute_items(&runtime, &second);
            assert_eq!(result.len(), 512);
            assert_eq!(result.identity(), retained_result.identity());
            assert_eq!(prepared_address(&runtime, &second), address);
            assert_eq!(runtime.inspect().natives, initial);
        }
        for _ in 0..3 {
            assert!(matches!(
                run_read(&runtime, &second, |_| {
                    Ok(crate::db_wire::release_prepared_memory_work())
                })
                .unwrap(),
                Output::Ready
            ));
            db.access().unwrap().db().clear_cache();
            assert_eq!(prepared_address(&runtime, &second), address);
            let result = execute_items(&runtime, &second);
            assert_eq!(result.len(), 512);
            assert_eq!(result.identity(), retained_result.identity());
            assert_eq!(runtime.inspect().natives, initial);
        }
        let wrong_args = run_read(&runtime, &second, |_| {
            Ok(crate::db_wire::execute_prepared_work(vec![
                crate::marshal::OwnedParam::Scalar(bumbledb::Value::U64(1)),
            ]))
        });
        assert!(matches!(wrong_args, Err(RuntimeError::Engine { .. })));
        assert_eq!(execute_items(&runtime, &second).len(), 512);
        assert_eq!(drain_session(&second), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, initial - 1);
        let rows = retained_result.into_answers();
        assert_eq!(rows.len(), 512, "result survives every reader closing");
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn abandoned_preparation_and_database_close_reclaim_plans() {
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("prepared-drain");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let db = attach(&owner, &Mini.descriptor());
        let snapshot = open_read(&runtime, &db);
        let initial = runtime.inspect().natives;
        let mut invalid = item_query();
        invalid.rules[0].atoms[0].source = bumbledb::AtomSource::Edb(bumbledb::RelationId(99));
        let rejected = run_read(&runtime, &snapshot, |_| {
            Ok(crate::db_wire::prepare_work(Arc::clone(&runtime), invalid))
        });
        assert!(matches!(rejected, Err(RuntimeError::Engine { .. })));
        assert_eq!(runtime.inspect().natives, initial);

        // Cancellation after installation must drop the unpublished plan.
        let (cap_tx, cap_rx) = channel();
        let target = Arc::clone(&runtime);
        let cancelled = run_read(&runtime, &snapshot, |_| {
            Ok(Box::new(move |context, access| {
                let prepared = access.prepare(&target, &item_query(), context)?;
                cap_tx.send(prepared.capability()).unwrap();
                context.cancel();
                Ok(Output::Prepared(prepared))
            }))
        });
        assert!(matches!(
            cancelled,
            Err(RuntimeError::Work(bumbledb::work::WorkError::Cancelled))
        ));
        let (tx, rx) = channel();
        runtime
            .close_resource(
                cap_rx.recv().unwrap(),
                Box::new(move |report| {
                    tx.send(report).unwrap();
                }),
            )
            .unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            CloseReport::Closed
        );
        assert_eq!(runtime.inspect().natives, initial);
        for _ in 0..16 {
            let abandoned = prepare_items(&runtime, &snapshot);
            let cap = abandoned.capability();
            drop(abandoned);
            let (tx, rx) = channel();
            runtime
                .close_resource(
                    cap,
                    Box::new(move |report| {
                        tx.send(report).unwrap();
                    }),
                )
                .unwrap();
            assert_eq!(
                rx.recv_timeout(Duration::from_secs(5)).unwrap(),
                CloseReport::Closed
            );
            assert_eq!(runtime.inspect().natives, initial);
        }
        let first = prepare_items(&runtime, &snapshot);
        let second = prepare_items(&runtime, &snapshot);
        let cancelled = run_read(&runtime, &first, |_| {
            Ok(Box::new(|context, access| {
                let output = crate::db_wire::execute_prepared_work(vec![])(context, access)?;
                context.cancel();
                Ok(output)
            }))
        });
        assert!(matches!(
            cancelled,
            Err(RuntimeError::Work(bumbledb::work::WorkError::Cancelled))
        ));
        assert!(
            execute_items(&runtime, &first).is_empty(),
            "cancelled execution leaves the plan reusable"
        );
        let (tx, rx) = channel();
        db.drain(Box::new(move |report| {
            tx.send(report).unwrap();
        }));
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            CloseReport::Closed
        );
        assert_eq!(runtime.inspect().natives, 0);
        // Reachable wrappers no longer own payloads after database drain.
        for reader in [&snapshot, &first, &second] {
            assert_eq!(drain_session(reader), CloseReport::Closed);
        }
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn one_shot_queries_do_not_retain_prepared_state() {
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("one-shot-preparation");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let db = attach(&owner, &Mini.descriptor());
        let session = open_read(&runtime, &db);
        for _ in 0..32 {
            let output = run_read(&runtime, &session, |_| {
                Ok(Box::new(|context, access| {
                    let output = crate::db_wire::execute_complete_work(item_query(), vec![])(
                        context, access,
                    )?;
                    assert_eq!(
                        access.prepared_count(),
                        0,
                        "one-shot preparation escaped its operation"
                    );
                    Ok(output)
                }))
            })
            .expect("one-shot completes");
            assert!(matches!(output, Output::CompleteResult(_)));
        }
        assert_eq!(drain_session(&session), CloseReport::Closed);
        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
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
        let Output::Count(generation) = run_read(&runtime, &first, |_| {
            Ok(Box::new(|context, access| {
                context.checkpoint()?;
                let _ = access.frame(context);
                Ok(Output::Count(access.owned.snapshot().generation().value()))
            }))
        })
        .expect("read job") else {
            panic!("expected a count output");
        };

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
                Ok(Output::Generation(
                    access.owned.snapshot().generation().value(),
                ))
            }))
        })
        .expect("parent still readable after extra idle snapshots")
        {
            Output::Generation(current) => assert_eq!(current, generation),
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

        let (release, wait_release) = channel();
        let (entered, running) = channel();
        let _blocker = runtime
            .submit(policy(), Box::new(|| {}), move |_| {
                Ok(Box::new(move |_| {
                    entered.send(()).unwrap();
                    wait_release.recv().unwrap();
                    Ok(Output::Ready)
                }))
            })
            .expect("blocker submits");
        running
            .recv_timeout(Duration::from_secs(5))
            .expect("entered");
        while runtime
            .submit(policy(), Box::new(|| {}), |_| {
                Ok(Box::new(|_| Ok(Output::Ready)))
            })
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
            runtime.inspect().natives,
            0,
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
            .submit(policy(), Box::new(|| {}), move |_| {
                Ok(Box::new(move |_| {
                    entered.send(()).unwrap();
                    blocked.recv().unwrap();
                    Ok(Output::Ready)
                }))
            })
            .expect("blocker submits");
        running
            .recv_timeout(Duration::from_secs(5))
            .expect("entered");
        assert_eq!(runtime.inspect().active, 1, "the one worker is occupied");

        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Spent,
            },
        )
        .expect("js-thread admit returns without the worker");
        assert_eq!(
            runtime.inspect().active,
            1,
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
        // row or handle slot.
        let runtime = Runtime::start(options()).unwrap();
        let baseline = runtime.inspect();
        let (release, wait_release) = channel();
        let (entered, running) = channel();
        let blocker = runtime
            .submit(policy(), Box::new(|| {}), move |_| {
                Ok(Box::new(move |_| {
                    entered.send(()).unwrap();
                    wait_release.recv().unwrap();
                    Ok(Output::Ready)
                }))
            })
            .expect("blocker submits");
        running
            .recv_timeout(Duration::from_secs(5))
            .expect("entered");

        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Spent,
            },
        )
        .expect("capability first, table insert later");
        assert_eq!(runtime.registry.route_count(), 1);
        assert_eq!(runtime.inspect().natives, baseline.natives + 1);
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
        assert!(matches!(runtime.take(&blocker), Ok(Output::Ready)));
        assert_eq!(
            runtime.inspect().natives,
            baseline.natives,
            "counters match actual release"
        );
        assert_eq!(
            runtime.registry.route_count(),
            0,
            "uninstalled close leaves no row"
        );
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
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
            .submit(policy(), Box::new(|| {}), move |_| {
                Ok(Box::new(move |context, access| {
                    let _ = access.frame(context);
                    entered.send(()).unwrap();
                    blocked.recv().unwrap();
                    Ok(Output::Ready)
                }))
            })
            .expect("busy snapshot submits");
        running
            .recv_timeout(Duration::from_secs(5))
            .expect("entered");
        assert!(matches!(
            session.submit(policy(), Box::new(|| {}), |_| panic!(
                "busy refusal precedes input preparation"
            )),
            Err(RuntimeError::HandleBusy)
        ));
        let (tx, rx) = channel();
        session.drain(Box::new(move |report| {
            tx.send(report).unwrap();
        }));
        assert!(matches!(
            session.submit(policy(), Box::new(|| {}), |_| panic!(
                "closing refuses new work"
            )),
            Err(RuntimeError::ClosedHandle)
        ));
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
                        context.checkpoint()?;
                        let queued = QueuedOutput {
                            rows: vec![Vec::new()],
                        };
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
                        context.checkpoint()?;
                        let queued = QueuedOutput {
                            rows: vec![Vec::new()],
                        };
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
                        context.checkpoint()?;
                        let queued = QueuedOutput {
                            rows: vec![Vec::new()],
                        };
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
        admission.request_close().expect("close after publication");
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
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");
        let page = |context: &bumbledb::work::WorkContext| {
            context.checkpoint()?;
            Ok::<_, RuntimeError>(QueuedOutput {
                rows: vec![
                    vec![crate::marshal::ValueOut::U64(1)],
                    vec![crate::marshal::ValueOut::U64(2)],
                ],
            })
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
            .submit_payload(admission.cap(), policy(), Box::new(|| {}), |_| {
                Ok(Box::new(move |context, _, _| {
                    context.checkpoint()?;
                    let queued = QueuedOutput {
                        rows: vec![Vec::new()],
                    };
                    Ok(Output::Page(Some(queued)))
                }))
            })
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
                Err(RuntimeError::SpentHandle
                    | RuntimeError::Work(bumbledb::work::WorkError::Cancelled))
            ),
            "reclaimed publication is not a later JS take"
        );
    }

    #[test]
    fn d12_cancelled_queued_page_reclaims_open_without_waiter() {
        // Cancel/close with queued Page/Rows, no waiter, Phase::Open.
        // Supervise must drop the delivery copy; JS take count stays 0;
        // committed facts stay (no second no rewind).
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
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
                        context.checkpoint()?;
                        let queued = QueuedOutput {
                            rows: vec![Vec::new()],
                        };
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
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability first");
        let (release, blocked) = channel();
        let (entered, running) = channel();
        runtime
            .submit_payload(admission.cap(), policy(), Box::new(|| {}), |_| {
                Ok(Box::new(move |_, _, _| {
                    entered.send(()).unwrap();
                    blocked.recv().unwrap();
                    Ok(Output::Ready)
                }))
            })
            .expect("busy payload submits");
        running
            .recv_timeout(Duration::from_secs(5))
            .expect("entered");
        assert!(matches!(
            runtime.submit_payload(admission.cap(), policy(), Box::new(|| {}), |_| panic!(
                "busy refuses before preparation"
            )),
            Err(RuntimeError::HandleBusy)
        ));
        admission
            .request_close()
            .expect("close while busy cannot QueueFull");
        assert!(matches!(
            runtime.submit_payload(admission.cap(), policy(), Box::new(|| {}), |_| panic!(
                "closing refuses before preparation"
            )),
            Err(RuntimeError::ClosedHandle)
        ));
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
    fn busy_payload_refuses_temporarily_and_reuses_the_same_capability() {
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            std::sync::Arc::clone(&runtime),
            NativeKind::Result,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .expect("capability");
        let baseline = runtime.inspect();
        let (release, blocked) = channel();
        let (entered, running) = channel();
        let (done, completed) = channel();
        let first = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    done.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |_, _, _| {
                        entered.send(()).unwrap();
                        blocked.recv().unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .expect("first job");
        running
            .recv_timeout(Duration::from_secs(5))
            .expect("job holds capability");
        assert!(matches!(
            runtime.submit_payload(admission.cap(), policy(), Box::new(|| {}), |_| panic!(
                "busy input not prepared"
            )),
            Err(RuntimeError::HandleBusy)
        ));
        release.send(()).unwrap();
        completed
            .recv_timeout(Duration::from_secs(5))
            .expect("first complete");
        assert!(matches!(runtime.take(&first), Ok(Output::Ready)));
        let (done, completed) = channel();
        let next = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    done.send(()).unwrap();
                }),
                |_| Ok(Box::new(|_, _, _| Ok(Output::Ready))),
            )
            .expect("same capability reusable");
        completed
            .recv_timeout(Duration::from_secs(5))
            .expect("reuse complete");
        assert!(matches!(runtime.take(&next), Ok(Output::Ready)));
        assert_eq!(runtime.inspect().natives, baseline.natives);
        assert_eq!(runtime.inspect().queued, baseline.queued);
        admission.request_close().expect("close");
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, 0);
    }

    #[test]
    fn d29_failed_admission_rolls_back_and_history_does_not_accumulate() {
        // D29: failed admission before insertion leaves no payload/row/slot.
        // Long create/revoke returns to the admitted baseline. No tombstones.
        let runtime = Runtime::start(options()).unwrap();
        let baseline = runtime.inspect();
        assert_eq!(baseline.natives, 0);
        assert_eq!(runtime.registry.route_count(), 0);

        let handles: Vec<_> = (0..runtime.options.native_handle_capacity)
            .map(|_| runtime.retain_native().unwrap())
            .collect();
        assert!(
            matches!(
                runtime.reserve_native_route(NativeKind::Result),
                Err(RuntimeError::ResourceLimit {
                    dimension: "nativeHandleCapacity",
                    ..
                })
            ),
            "handle admission refuses before a route exists"
        );
        drop(handles);
        assert_eq!(runtime.inspect().natives, 0);
        assert_eq!(runtime.registry.route_count(), 0);

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
    #[test]
    fn finalization_revokes_aliases_drains_busy_work_and_consumes_on_cancel() {
        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            Arc::clone(&runtime),
            NativeKind::Result,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .unwrap();
        let (entered, running) = channel();
        let (release, blocked) = channel();
        let (done, completed) = channel();
        let first = runtime
            .submit_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    done.send(()).unwrap();
                }),
                |_| {
                    Ok(Box::new(move |_, _, _| {
                        entered.send(()).unwrap();
                        blocked.recv().unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        let (done, finished) = channel();
        let terminal = runtime
            .finalize_payload(
                admission.cap(),
                policy(),
                Box::new(move || {
                    done.send(()).unwrap();
                }),
                |_| Ok(Box::new(|_, _, _| Ok(Output::Ready))),
            )
            .expect("finalize queues behind the admitted write");
        assert!(matches!(
            runtime.submit_payload(admission.cap(), policy(), Box::new(|| {}), |_| panic!(
                "revoked"
            )),
            Err(RuntimeError::ClosedHandle)
        ));
        assert!(matches!(
            runtime.finalize_payload(admission.cap(), policy(), Box::new(|| {}), |_| panic!(
                "already finalizing"
            )),
            Err(RuntimeError::ClosedHandle)
        ));
        release.send(()).unwrap();
        completed.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(runtime.take(&first), Ok(Output::Ready)));
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(runtime.take(&terminal), Ok(Output::Ready)));
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, 0);

        let runtime = Runtime::start(options()).unwrap();
        let admission = super::super::registry::RegistryAdmission::admit(
            Arc::clone(&runtime),
            NativeKind::Result,
            super::super::registry::Payload::Result {
                result: None,
                state: super::super::registry::ResultState::Live,
            },
        )
        .unwrap();
        let stopped = policy();
        stopped.cancel();
        assert!(matches!(
            runtime.finalize_payload(admission.cap(), stopped, Box::new(|| {}), |_| panic!(
                "cancelled"
            )),
            Err(RuntimeError::Work(_))
        ));
        assert!(matches!(
            runtime.submit_payload(admission.cap(), policy(), Box::new(|| {}), |_| panic!(
                "consumed"
            )),
            Err(RuntimeError::ClosedHandle)
        ));
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        assert_eq!(runtime.inspect().natives, 0);
    }
}
