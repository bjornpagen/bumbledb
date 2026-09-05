//! Worker-affine persistent read and write sessions.
//!
//! The engine's `ReadInstance`, `WriteTx` and `PreparedQuery` are `!Send`
//! (`Rc`-backed pipe tables, borrow-scoped LMDB transactions, writer
//! thread identity). A session therefore owns ONE dedicated OS thread
//! that enters a single `Db::read`/`Db::write` lease and keeps every
//! `!Send` resource inside that stack frame for the session's whole
//! lifetime. Jobs cross as `Send` closures over owned data; the resources
//! they touch never leave the owning thread. This is real affinity —
//! never `unsafe Send`, raw-pointer smuggling, or lifetime erasure. It
//! replaces the deleted raw-pointer `InstanceHandle`/`TxHandle`
//! scoped-borrow surface and the JS-callback-inside-a-native-transaction
//! shape chapters 31/32/35 forbid: no JavaScript executes while an engine
//! transaction frame is on this thread's stack.
//!
//! Every session job is a registered runtime [`Operation`]: it is
//! charged, cancellable and drainable exactly like pool work, so close
//! reports account for session work and shutdown joins it. The session
//! itself holds a [`DbLease`] (a registered external operation), so the
//! managed database cannot be reclaimed while the session's transaction
//! is live, and a database/owner/runtime close drains the session before
//! directory teardown.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use bumbledb::work::{ExecutionPolicy, WorkContext, WorkError};
use bumbledb::{PreparedQuery, ReadInstance, SchemaDescriptor, Witness, WriteTx};

use super::owners::{DbLease, ManagedDb};
use super::{Notify, Operation, Output, Phase, Runtime, RuntimeError, WaitTarget, lock};
use crate::marshal::ViolationWire;

/// One typed engine refusal crossing the executor as owned data.
pub(crate) fn engine_error(error: &bumbledb::Error) -> RuntimeError {
    RuntimeError::Engine {
        kind: crate::tags::error_family::tag(&error.family()),
        message: crate::marshal::engine_message(error),
    }
}

/// The `!Send` state a READ session's owning thread holds. Borrowed by
/// every job on that thread; nothing in here ever crosses threads.
pub struct ReadFrame<'a, 'txn> {
    pub instance: &'a ReadInstance<'txn, SchemaDescriptor>,
    pub sealed: &'a crate::Sealed,
    /// The running job's own operation id — the runtime's one counter,
    /// never reused. A prepare job installs its result under this id.
    pub job: u64,
    prepared: &'a mut BTreeMap<u64, PreparedQuery<SchemaDescriptor>>,
}

impl ReadFrame<'_, '_> {
    /// Installs the job's prepared query under its own operation id and
    /// returns that id. Ids are never reused, so a retained stale id can
    /// only miss — it cannot alias a successor prepared query.
    pub fn install(&mut self, prepared: PreparedQuery<SchemaDescriptor>) -> u64 {
        let id = self.job;
        self.prepared.insert(id, prepared);
        id
    }

    /// Executes an installed prepared query against the pinned instance.
    /// The split borrow lives here: the map entry and the instance are
    /// disjoint fields of the frame.
    pub fn execute(
        &mut self,
        id: u64,
        args: &[bumbledb::ParamArg<'_>],
    ) -> Result<bumbledb::Answers, RuntimeError> {
        let instance = self.instance;
        let prepared = self
            .prepared
            .get_mut(&id)
            .ok_or(RuntimeError::ClosedHandle)?;
        instance
            .execute_collect(prepared, args)
            .map_err(|error| engine_error(&error))
    }

    /// Removes (closes) a prepared query. A second remove of the same id
    /// refuses as a closed handle — the double-release attack shape.
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

/// The `!Send` state a WRITE session's owning thread holds: the one
/// exclusive engine write transaction plus the sealed roster datum.
pub struct WriteFrame<'a, 'txn> {
    pub tx: &'a mut WriteTx<'txn, SchemaDescriptor>,
    pub sealed: &'a crate::Sealed,
}

/// One read-session job: a `Send` closure over the thread-owned frame.
pub type ReadWork = Box<
    dyn for<'a, 'txn> FnOnce(&WorkContext, &mut ReadFrame<'a, 'txn>) -> Result<Output, RuntimeError>
        + Send,
>;

/// One write-session job: a `Send` closure over the thread-owned frame.
pub type WriteWork = Box<
    dyn for<'a, 'txn> FnOnce(
            &WorkContext,
            &mut WriteFrame<'a, 'txn>,
        ) -> Result<Output, RuntimeError>
        + Send,
>;

/// A submitted session job, typed by the frame it needs. Submission
/// refuses a job whose kind disagrees with the slot's kind, so a retained
/// read capability can never reach a write frame and vice versa.
pub enum SessionJob {
    Read(ReadWork),
    Write(WriteWork),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionKind {
    Read,
    Write,
}

pub(super) enum Message {
    Read {
        operation: Arc<Operation>,
        work: ReadWork,
    },
    Write {
        operation: Arc<Operation>,
        work: WriteWork,
    },
    /// Terminal write-session verb: end the transaction. `commit: false`
    /// is an explicit abort. The operation's output is the owned
    /// [`WriteConclusion`].
    Finish {
        operation: Arc<Operation>,
        commit: bool,
    },
    Close,
}

/// The registry's per-session record. The sender is the only channel to
/// the owning thread; `closing` stops new admission before the thread
/// observes `Close`.
pub(super) struct SessionSlot {
    pub sender: Sender<Message>,
    pub kind: SessionKind,
    pub closing: bool,
}

impl SessionSlot {
    pub(super) fn begin_close(&mut self) {
        if !self.closing {
            self.closing = true;
            let _ = self.sender.send(Message::Close);
        }
    }
}

/// The write transaction's terminal outcome, fully owned: the rejection
/// evidence is rendered to wire rows on the owning thread, so no borrow
/// of the descriptor or transaction survives the frame.
pub enum WriteConclusion {
    Accepted(u64),
    Rejected(Vec<ViolationWire>),
    Moved { witnessed: u64, current: u64 },
    Aborted,
}

/// A prepare job's reply: the installed prepared-query id, or the typed
/// IR refusal (a domain outcome, not a thrown error).
pub enum PrepareReply {
    Ok(u64),
    IrError(String),
}

/// The opened read-session output crossing back from the opening job.
/// Carries the sealed roster datum so the JS thread can marshal rows and
/// keys before dispatch without touching the owning thread.
pub struct SessionOpened {
    pub session: SnapshotSession,
    pub sealed: Arc<crate::Sealed>,
    pub witness: Witness<SchemaDescriptor>,
    pub generation: u64,
    /// The persisted store identity (lowercase hex) — the `CoreWitness`
    /// wire's `store` half (chapter 35: catalog/store identity plus
    /// generation, never a log `StateStamp`).
    pub store: String,
    /// The committed host attachment (the log's authority control), read
    /// INSIDE the pinned frame so a published snapshot's provenance is
    /// exactly the pinned data's — never a racing later commit's.
    pub attachment: Option<Vec<u8>>,
}

/// The opened write-session output crossing back from the opening job.
pub struct WriterOpened {
    pub session: WriteSession,
    pub sealed: Arc<crate::Sealed>,
}

/// The identity core of a JS-held session capability. The runtime's
/// registry, not this wrapper, owns the thread, the lease and the `!Send`
/// resources. Dropping the wrapper begins close (GC is a backstop; the
/// explicit close path is the contract).
struct SessionCore {
    runtime: Arc<Runtime>,
    owner: u64,
    database: u64,
    id: u64,
}

impl SessionCore {
    fn begin_close(&self) {
        let mut state = lock(&self.runtime.state);
        if let Some(slot) = state
            .owners
            .get_mut(&self.owner)
            .and_then(|owner| owner.databases.get_mut(&self.database))
            .and_then(|database| database.sessions.get_mut(&self.id))
        {
            slot.begin_close();
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
        kind: SessionKind,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<SessionJob, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.runtime.submit_session(
            self.owner,
            self.database,
            self.id,
            kind,
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

/// A JS-held READ session capability: one pinned coherent generation.
pub struct SnapshotSession {
    core: SessionCore,
}

impl SnapshotSession {
    #[must_use]
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.core.runtime
    }

    /// Marks the slot closing, cancels this session's queued/active jobs
    /// and sends the terminal `Close`. Idempotent; joining the actual
    /// drain is [`Self::drain`].
    pub fn begin_close(&self) {
        self.core.begin_close();
    }

    /// Begin close and report when the slot is actually gone (thread
    /// exited, lease released) — or Incomplete/Failed per the runtime's
    /// finite cleanup policy.
    pub fn drain(&self, report: super::Report) {
        self.core.drain(report);
    }

    /// Submits one bounded read job to the owning thread. Registration
    /// reserves bytes and admission before the message is sent; a closing
    /// slot refuses. The job observes the operation's own `WorkContext`.
    pub fn submit(
        &self,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<ReadWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.core
            .submit(SessionKind::Read, policy, notify, |context| {
                prepare(context).map(SessionJob::Read)
            })
    }
}

/// A JS-held WRITE session capability: the one exclusive engine writer,
/// pinned to its owning thread across the whole (possibly remote-awaited)
/// host workflow. Finish commits or aborts; close without finish aborts.
pub struct WriteSession {
    core: SessionCore,
}

impl WriteSession {
    #[must_use]
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.core.runtime
    }

    pub fn begin_close(&self) {
        self.core.begin_close();
    }

    pub fn drain(&self, report: super::Report) {
        self.core.drain(report);
    }

    /// Submits one bounded write job to the owning thread.
    pub fn submit(
        &self,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<WriteWork, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.core
            .submit(SessionKind::Write, policy, notify, |context| {
                prepare(context).map(SessionJob::Write)
            })
    }

    /// Ends the transaction: commit (`true`) or explicit abort (`false`).
    /// The returned operation's output is the owned [`WriteConclusion`];
    /// the slot is closing from this point, so no further job can enter
    /// the frame behind the terminal message.
    pub fn finish(
        &self,
        commit: bool,
        policy: ExecutionPolicy,
        notify: Notify,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.core.runtime.finish_session(
            self.core.owner,
            self.core.database,
            self.core.id,
            commit,
            policy,
            notify,
        )
    }
}

impl Runtime {
    /// Opens a worker-affine READ session over a managed database: a pool
    /// job takes a [`DbLease`], reserves the session slot, spawns the
    /// owning thread and waits for its read lease to pin a coherent
    /// generation. The output is [`Output::Session`].
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
                runtime.spawn_read_session(owner, database, lease)
            }))
        })
    }

    /// Opens a worker-affine WRITE session. The engine's single-writer
    /// rule is enforced by refusal (`WriterBusy`), never by parking a
    /// thread on the writer mutex behind a JS-driven session. With a
    /// witness, the commit is conditional (`write_from`) and a moved
    /// generation is a domain outcome.
    pub fn open_writer(
        self: &Arc<Self>,
        db: &ManagedDb,
        witness: Option<Witness<SchemaDescriptor>>,
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
                if lease.writing.swap(true, Ordering::AcqRel) {
                    return Err(RuntimeError::WriterBusy);
                }
                let flag = WriterFlag(lease.inner_arc());
                runtime.spawn_write_session(owner, database, lease, witness, flag)
            }))
        })
    }

    /// Reserves the slot under the registry lock. Runs on a pool worker.
    fn reserve_session_slot(
        &self,
        owner: u64,
        database: u64,
        kind: SessionKind,
        sender: Sender<Message>,
    ) -> Result<u64, RuntimeError> {
        let mut state = lock(&self.state);
        if state.phase != Phase::Open {
            return Err(RuntimeError::ClosedHandle);
        }
        state.require_owner(Some(owner))?;
        let sessions: usize = state
            .owners
            .values()
            .flat_map(|entry| entry.databases.values())
            .map(|database| database.sessions.len())
            .sum();
        if sessions >= self.options.native_handle_capacity {
            return Err(RuntimeError::ResourceLimit {
                dimension: "nativeHandleCapacity",
                used: sessions as u64,
                requested: 1,
                limit: self.options.native_handle_capacity as u64,
            });
        }
        let id = state.next_id;
        state.next_id = id.checked_add(1).ok_or(RuntimeError::Internal)?;
        let entry = state
            .owners
            .get_mut(&owner)
            .and_then(|entry| entry.databases.get_mut(&database))
            .ok_or(RuntimeError::ClosedHandle)?;
        if entry.closing {
            return Err(RuntimeError::ClosedHandle);
        }
        entry.sessions.insert(
            id,
            SessionSlot {
                sender,
                kind,
                closing: false,
            },
        );
        Ok(id)
    }

    fn spawn_read_session(
        self: &Arc<Self>,
        owner: u64,
        database: u64,
        lease: DbLease,
    ) -> Result<Output, RuntimeError> {
        let sealed = lease.sealed();
        let store = lease.db().integration_store().identity().store.to_string();
        let (sender, receiver) = channel::<Message>();
        let id = self.reserve_session_slot(owner, database, SessionKind::Read, sender)?;
        let (ready, opened) =
            channel::<Result<(Witness<SchemaDescriptor>, u64, Option<Vec<u8>>), RuntimeError>>();
        let runtime = Arc::clone(self);
        let spawned = thread::Builder::new()
            .name(format!("bumbledb-read-{id}"))
            .spawn(move || {
                runtime.read_session_body(owner, database, id, &lease, &ready, &receiver);
            });
        if spawned.is_err() {
            self.remove_session(owner, database, id);
            return Err(RuntimeError::Internal);
        }
        match opened.recv() {
            Ok(Ok((witness, generation, attachment))) => Ok(Output::Session(SessionOpened {
                session: SnapshotSession {
                    core: SessionCore {
                        runtime: Arc::clone(self),
                        owner,
                        database,
                        id,
                    },
                },
                sealed,
                witness,
                generation,
                store,
                attachment,
            })),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(RuntimeError::Internal),
        }
    }

    /// The db-bridge/log entry to the reactor from INSIDE a registered job:
    /// spawn one pinned read session over an already-held lease (the caller
    /// acquired it on the JS thread, so close cannot race the acquisition).
    pub(crate) fn spawn_read_session_for(
        self: &Arc<Self>,
        db: &ManagedDb,
        lease: DbLease,
    ) -> Result<Output, RuntimeError> {
        let (owner, database) = db.ids();
        self.spawn_read_session(owner, database, lease)
    }

    fn spawn_write_session(
        self: &Arc<Self>,
        owner: u64,
        database: u64,
        lease: DbLease,
        witness: Option<Witness<SchemaDescriptor>>,
        flag: WriterFlag,
    ) -> Result<Output, RuntimeError> {
        let sealed = lease.sealed();
        let (sender, receiver) = channel::<Message>();
        let id = self.reserve_session_slot(owner, database, SessionKind::Write, sender)?;
        let (ready, opened) = channel::<Result<(), RuntimeError>>();
        let runtime = Arc::clone(self);
        let spawned = thread::Builder::new()
            .name(format!("bumbledb-write-{id}"))
            .spawn(move || {
                runtime.write_session_body(owner, database, id, &lease, witness, &ready, &receiver);
                drop(flag);
                drop(lease);
            });
        if spawned.is_err() {
            self.remove_session(owner, database, id);
            return Err(RuntimeError::Internal);
        }
        match opened.recv() {
            Ok(Ok(())) => Ok(Output::Writer(WriterOpened {
                session: WriteSession {
                    core: SessionCore {
                        runtime: Arc::clone(self),
                        owner,
                        database,
                        id,
                    },
                },
                sealed,
            })),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(RuntimeError::Internal),
        }
    }

    /// The READ owning thread: one `Db::read` lease for the session
    /// lifetime; jobs run against the borrowed frame; teardown removes
    /// the slot and releases the lease LAST, so waiters observe reality.
    #[allow(clippy::type_complexity)]
    fn read_session_body(
        self: Arc<Self>,
        owner: u64,
        database: u64,
        id: u64,
        lease: &DbLease,
        ready: &Sender<Result<(Witness<SchemaDescriptor>, u64, Option<Vec<u8>>), RuntimeError>>,
        receiver: &Receiver<Message>,
    ) {
        let sealed = lease.sealed();
        let runtime = Arc::clone(&self);
        let entered = lease.db().read(|instance| {
            let witness = instance.witness()?;
            let generation = instance.generation()?.value();
            let attachment = instance.integration_host_attachment()?.map(<[u8]>::to_vec);
            if ready.send(Ok((witness, generation, attachment))).is_err() {
                // The opener vanished (cancelled open). Tear down.
                return Ok(());
            }
            let mut prepared = BTreeMap::new();
            loop {
                let Ok(message) = receiver.recv() else { break };
                match message {
                    Message::Close => break,
                    Message::Read { operation, work } => {
                        let mut frame = ReadFrame {
                            instance,
                            sealed: sealed.as_ref(),
                            job: operation.id,
                            prepared: &mut prepared,
                        };
                        runtime.run_read_job(&operation, work, &mut frame);
                    }
                    Message::Write { operation, .. } | Message::Finish { operation, .. } => {
                        runtime.complete_operation(&operation, Err(RuntimeError::InvalidArgument));
                    }
                }
            }
            Ok(())
        });
        if let Err(error) = &entered {
            // The read lease itself refused; the opener still waits.
            let _ = ready.send(Err(engine_error(error)));
        }
        self.teardown_session(owner, database, id, receiver);
    }

    /// The WRITE owning thread: one `Db::write`/`write_from` frame for
    /// the session lifetime. The terminal `Finish` message ends the frame
    /// and its operation carries the owned conclusion; close without
    /// finish aborts. No JS runs while this frame is on the stack.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::needless_pass_by_value
    )]
    fn write_session_body(
        self: &Arc<Self>,
        owner: u64,
        database: u64,
        id: u64,
        lease: &DbLease,
        witness: Option<Witness<SchemaDescriptor>>,
        ready: &Sender<Result<(), RuntimeError>>,
        receiver: &Receiver<Message>,
    ) {
        let sealed = lease.sealed();
        let runtime = Arc::clone(self);
        let mut conclusion: Option<(Arc<Operation>, bool)> = None;
        let conclusion_slot = &mut conclusion;
        let body = |tx: &mut WriteTx<'_, SchemaDescriptor>| -> bumbledb::Result<()> {
            if ready.send(Ok(())).is_err() {
                // The opener vanished (cancelled open). Abort the frame.
                return Err(crate::abort_sentinel());
            }
            loop {
                let Ok(message) = receiver.recv() else {
                    return Err(crate::abort_sentinel());
                };
                match message {
                    Message::Close => return Err(crate::abort_sentinel()),
                    Message::Finish { operation, commit } => {
                        *conclusion_slot = Some((operation, commit));
                        return if commit {
                            Ok(())
                        } else {
                            Err(crate::abort_sentinel())
                        };
                    }
                    Message::Write { operation, work } => {
                        let mut frame = WriteFrame {
                            tx,
                            sealed: sealed.as_ref(),
                        };
                        runtime.run_write_job(&operation, work, &mut frame);
                    }
                    Message::Read { operation, .. } => {
                        runtime.complete_operation(&operation, Err(RuntimeError::InvalidArgument));
                    }
                }
            }
        };
        let result = match &witness {
            None => match lease.db().write(body) {
                Ok(bumbledb::Admission::Accepted(committed)) => {
                    Ok(bumbledb::ConditionalWrite::Accepted(committed))
                }
                Ok(bumbledb::Admission::Rejected(violations)) => {
                    Ok(bumbledb::ConditionalWrite::Rejected(violations))
                }
                Err(error) => Err(error),
            },
            Some(witness) => lease.db().write_from(witness, body),
        };
        match conclusion {
            Some((operation, commit)) => {
                let outcome = match result {
                    Ok(bumbledb::ConditionalWrite::Accepted(committed)) => Ok(Output::Write(
                        WriteConclusion::Accepted(committed.generation.value()),
                    )),
                    Ok(bumbledb::ConditionalWrite::Rejected(violations)) => {
                        Ok(Output::Write(WriteConclusion::Rejected(
                            crate::violations_wire(&sealed.descriptor, &violations),
                        )))
                    }
                    Ok(bumbledb::ConditionalWrite::Moved { witnessed, current }) => {
                        Ok(Output::Write(WriteConclusion::Moved {
                            witnessed: witnessed.value(),
                            current: current.value(),
                        }))
                    }
                    Err(error) => {
                        if commit {
                            Err(engine_error(&error))
                        } else {
                            // The explicit abort's own sentinel: a domain
                            // outcome, not a failure.
                            Ok(Output::Write(WriteConclusion::Aborted))
                        }
                    }
                };
                self.complete_operation(&operation, outcome);
            }
            None => {
                // Closed/cancelled without finish, or the write lease
                // itself refused before the frame entered. If the opener
                // still waits, deliver the refusal.
                if let Err(error) = &result {
                    let _ = ready.send(Err(engine_error(error)));
                }
            }
        }
        self.teardown_session(owner, database, id, receiver);
    }

    /// Cancel every job still queued behind the terminal message, remove
    /// the slot, then wake waiters. The caller drops the lease AFTER this
    /// returns (write) or when the read closure frame unwinds.
    fn teardown_session(&self, owner: u64, database: u64, id: u64, receiver: &Receiver<Message>) {
        while let Ok(message) = receiver.try_recv() {
            match message {
                Message::Read { operation, .. }
                | Message::Write { operation, .. }
                | Message::Finish { operation, .. } => {
                    operation.context.cancel();
                    self.complete_operation(
                        &operation,
                        Err(RuntimeError::Work(WorkError::Cancelled)),
                    );
                }
                Message::Close => {}
            }
        }
        self.remove_session(owner, database, id);
    }

    fn remove_session(&self, owner: u64, database: u64, id: u64) {
        let mut state = lock(&self.state);
        if let Some(entry) = state
            .owners
            .get_mut(&owner)
            .and_then(|entry| entry.databases.get_mut(&database))
        {
            entry.sessions.remove(&id);
        }
        self.changed.notify_all();
    }

    fn run_read_job(
        &self,
        operation: &Arc<Operation>,
        work: ReadWork,
        frame: &mut ReadFrame<'_, '_>,
    ) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            operation.context.checkpoint()?;
            let value = work(&operation.context, frame)?;
            operation.context.checkpoint()?;
            Ok(value)
        }));
        self.settle_session_job(operation, outcome);
    }

    fn run_write_job(
        &self,
        operation: &Arc<Operation>,
        work: WriteWork,
        frame: &mut WriteFrame<'_, '_>,
    ) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            operation.context.checkpoint()?;
            let value = work(&operation.context, frame)?;
            operation.context.checkpoint()?;
            Ok(value)
        }));
        self.settle_session_job(operation, outcome);
    }

    fn settle_session_job(
        &self,
        operation: &Arc<Operation>,
        outcome: Result<Result<Output, RuntimeError>, Box<dyn std::any::Any + Send>>,
    ) {
        let panicked = outcome.is_err();
        let outcome = outcome.unwrap_or(Err(RuntimeError::Internal));
        if panicked {
            // A panicked session job faults the runtime, as pool panics
            // do: the frame may hold damaged state; never keep serving it.
            self.begin_close();
        }
        self.complete_operation(operation, outcome);
    }

    /// Registers and dispatches one session job. Mirrors `submit_at`, but
    /// the work goes to the session's channel, never the pool queue, and
    /// the slot's kind must match the job's frame.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn submit_session(
        &self,
        owner: u64,
        database: u64,
        session: u64,
        kind: SessionKind,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<SessionJob, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let operation =
            self.register_session_operation(owner, database, session, kind, policy, notify)?;
        // Bounded input extraction on the calling (JS) thread, after
        // registration — exactly the pool-submit discipline.
        let prepared = catch_unwind(AssertUnwindSafe(|| prepare(&operation.context)));
        self.dispatch_session_job(owner, database, session, &operation, prepared)
    }

    /// Registers and dispatches the terminal write-session finish.
    pub(super) fn finish_session(
        &self,
        owner: u64,
        database: u64,
        session: u64,
        commit: bool,
        policy: ExecutionPolicy,
        notify: Notify,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let operation = self.register_session_operation(
            owner,
            database,
            session,
            SessionKind::Write,
            policy,
            notify,
        )?;
        let mut state = lock(&self.state);
        let slot = state
            .owners
            .get_mut(&owner)
            .and_then(|entry| entry.databases.get_mut(&database))
            .and_then(|entry| entry.sessions.get_mut(&session))
            .filter(|slot| !slot.closing);
        let sent = slot.is_some_and(|slot| {
            // The frame ends behind this message; stop admission first so
            // no later job can race in behind the terminal verb.
            slot.closing = true;
            slot.sender
                .send(Message::Finish {
                    operation: Arc::clone(&operation),
                    commit,
                })
                .is_ok()
        });
        if !sent {
            let discarded = state.remove(operation.id);
            self.changed.notify_all();
            drop(state);
            drop(discarded);
            return Err(RuntimeError::ClosedHandle);
        }
        drop(state);
        Ok(operation)
    }

    fn register_session_operation(
        &self,
        owner: u64,
        database: u64,
        session: u64,
        kind: SessionKind,
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
        if state.phase != Phase::Open {
            return Err(RuntimeError::ClosedHandle);
        }
        state.require_owner(Some(owner))?;
        let live = state
            .owners
            .get(&owner)
            .and_then(|entry| entry.databases.get(&database))
            .and_then(|entry| entry.sessions.get(&session))
            .is_some_and(|slot| !slot.closing && slot.kind == kind);
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

    fn dispatch_session_job(
        &self,
        owner: u64,
        database: u64,
        session: u64,
        operation: &Arc<Operation>,
        prepared: Result<Result<SessionJob, RuntimeError>, Box<dyn std::any::Any + Send>>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        let mut state = lock(&self.state);
        let work = match prepared.unwrap_or_else(|_| {
            Self::closing(&mut state);
            Err(RuntimeError::Internal)
        }) {
            Ok(work) if state.phase == Phase::Open => work,
            other => {
                let discarded = state.remove(operation.id);
                self.changed.notify_all();
                drop(state);
                drop(discarded);
                return Err(other.err().unwrap_or(RuntimeError::ClosedHandle));
            }
        };
        let message = match work {
            SessionJob::Read(work) => Message::Read {
                operation: Arc::clone(operation),
                work,
            },
            SessionJob::Write(work) => Message::Write {
                operation: Arc::clone(operation),
                work,
            },
        };
        let sent = state
            .owners
            .get(&owner)
            .and_then(|entry| entry.databases.get(&database))
            .and_then(|entry| entry.sessions.get(&session))
            .filter(|slot| !slot.closing)
            .map(|slot| slot.sender.send(message).is_ok());
        if sent != Some(true) {
            let discarded = state.remove(operation.id);
            self.changed.notify_all();
            drop(state);
            drop(discarded);
            return Err(RuntimeError::ClosedHandle);
        }
        drop(state);
        Ok(Arc::clone(operation))
    }
}

/// Clears the database's single-writer flag when the write session's
/// owning thread exits, however it exits.
struct WriterFlag(Arc<crate::DbInner>);

impl Drop for WriterFlag {
    fn drop(&mut self) {
        self.0.writing.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    //! Engine-backed worker-affine session tests (C09 / RUN / FFI).
    //!
    //! These drive the real reactor: a directory owner in the one registry,
    //! an actual `bumbledb::Db` attached as a `ManagedDb`, and read/write
    //! sessions whose `!Send` `ReadInstance`/`WriteTx` never leave the
    //! owning thread — only owned `Value`/report data crosses the channel.
    //! Written, NEVER run in this phase (F1); F3 executes them.
    use std::sync::mpsc::channel;
    use std::time::Duration;

    use bumbledb::{SchemaDescriptor, Theory as _, Value};

    use super::super::owners::{DirectoryOwner, ManagedDb};
    use super::super::{CloseReport, Options};
    use super::*;

    bumbledb::schema! {
        pub Mini;
        relation Item { a: u64, b: u64 }
        Item(a) -> Item;
    }

    fn options() -> Options {
        Options {
            workers: 2,
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
            "bumbledb-p06-session-{tag}-{}-{seq}",
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
            .expect("open read session submits");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("session notify");
        match runtime.take(&operation) {
            Ok(Output::Session(opened)) => opened.session,
            _ => panic!("expected a read session output"),
        }
    }

    fn open_write(runtime: &Arc<Runtime>, db: &ManagedDb) -> Result<WriteSession, RuntimeError> {
        let (tx, rx) = channel();
        let operation = runtime.open_writer(
            db,
            None,
            policy(),
            Box::new(move || {
                tx.send(()).unwrap();
            }),
        )?;
        rx.recv_timeout(Duration::from_secs(5))
            .expect("writer notify");
        match runtime.take(&operation) {
            Ok(Output::Writer(opened)) => Ok(opened.session),
            Ok(_) => panic!("expected a write session output"),
            Err(error) => Err(error),
        }
    }

    /// Runs one session job and returns its taken output.
    fn run_read(
        runtime: &Arc<Runtime>,
        session: &SnapshotSession,
        prepare: impl FnOnce(&WorkContext) -> Result<ReadWork, RuntimeError>,
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

    fn insert_two(
        runtime: &Arc<Runtime>,
        writer: &WriteSession,
        fields: Vec<bumbledb::schema::FieldDescriptor>,
    ) {
        let (tx, rx) = channel();
        let operation = writer
            .submit(
                policy(),
                Box::new(move || {
                    tx.send(()).unwrap();
                }),
                move |_| {
                    let rows = [
                        [Value::U64(1), Value::U64(10)],
                        [Value::U64(2), Value::U64(20)],
                    ];
                    let collection = bumbledb::AcceptedCollection::from_value_rows(
                        bumbledb::RelationId(0),
                        &fields,
                        rows,
                    )
                    .map_err(|_| RuntimeError::InvalidArgument)?;
                    Ok(Box::new(move |_context, frame| {
                        let report = frame
                            .tx
                            .insert_accepted(&collection)
                            .map_err(|error| engine_error(&error))?;
                        Ok(Output::Mutation {
                            submitted: report.submitted(),
                            changed: report.changed(),
                        })
                    }))
                },
            )
            .expect("write job submits");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("write job notify");
        match runtime.take(&operation) {
            Ok(Output::Mutation { submitted, changed }) => {
                assert_eq!(submitted, 2, "two rows submitted");
                assert!(changed <= submitted, "changed never exceeds submitted");
            }
            _ => panic!("expected a mutation report"),
        }
    }

    fn finish(runtime: &Arc<Runtime>, writer: &WriteSession, commit: bool) -> Output {
        let (tx, rx) = channel();
        let operation = writer
            .finish(
                commit,
                policy(),
                Box::new(move || {
                    tx.send(()).unwrap();
                }),
            )
            .expect("finish submits");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("finish notify");
        runtime.take(&operation).expect("finish output")
    }

    fn drain_session(session: &SnapshotSession) -> CloseReport {
        let (tx, rx) = channel();
        session.drain(Box::new(move |report| {
            tx.send(report).unwrap();
        }));
        rx.recv_timeout(Duration::from_secs(5))
            .expect("session drain")
    }

    fn drain_writer(session: &WriteSession) -> CloseReport {
        let (tx, rx) = channel();
        session.drain(Box::new(move |report| {
            tx.send(report).unwrap();
        }));
        rx.recv_timeout(Duration::from_secs(5))
            .expect("writer drain")
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
    fn read_and_write_sessions_pin_engine_state_and_cross_only_owned_data() {
        // RUN-01/RUN-05 held-session affinity + FFI: the write session's
        // exclusive `WriteTx` and the read session's `ReadInstance` never
        // leave their owning threads; only owned `Value`/report data
        // crosses the channel. A committed write is visible to a later
        // read session's pinned snapshot.
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("round-trip");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);

        let writer = open_write(&runtime, &db).expect("writer opens");
        insert_two(&runtime, &writer, descriptor.relations[0].fields.clone());
        match finish(&runtime, &writer, true) {
            Output::Write(WriteConclusion::Accepted(_)) => {}
            _ => panic!("commit must accept two distinct keyed rows"),
        }
        assert_eq!(drain_writer(&writer), CloseReport::Closed);

        let session = open_read(&runtime, &db);
        match run_read(&runtime, &session, |_| {
            Ok(Box::new(|context, frame| {
                context.checkpoint()?;
                frame
                    .instance
                    .count(bumbledb::RelationId(0))
                    .map(Output::Count)
                    .map_err(|error| engine_error(&error))
            }))
        })
        .expect("count job")
        {
            Output::Count(count) => assert_eq!(count, 2, "committed rows are visible"),
            _ => panic!("expected a count output"),
        }
        match run_read(&runtime, &session, |_| {
            Ok(Box::new(|context, frame| {
                context.checkpoint()?;
                frame
                    .instance
                    .contains_dyn(bumbledb::RelationId(0), &[Value::U64(1), Value::U64(10)])
                    .map(Output::Contains)
                    .map_err(|error| engine_error(&error))
            }))
        })
        .expect("contains job")
        {
            Output::Contains(found) => assert!(found, "the exact committed row is present"),
            _ => panic!("expected a contains output"),
        }
        assert_eq!(drain_session(&session), CloseReport::Closed);

        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_second_writer_refuses_while_one_write_session_is_live() {
        // RUN-05/FFI: the engine's single-writer rule is enforced by
        // refusal (`WriterBusy`), never by parking a thread on the writer
        // mutex. Releasing the first writer lets the next one in.
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("single-writer");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);

        let first = open_write(&runtime, &db).expect("first writer opens");
        assert!(
            matches!(open_write(&runtime, &db), Err(RuntimeError::WriterBusy)),
            "a live write session fences a second writer"
        );
        assert_eq!(drain_writer(&first), CloseReport::Closed);

        let second = open_write(&runtime, &db).expect("writer after release");
        assert_eq!(drain_writer(&second), CloseReport::Closed);

        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_foreign_runtime_refuses_to_open_a_session_over_a_managed_db() {
        // FFI foreign-addon/kind: a `ManagedDb` belongs to exactly one
        // runtime; a second runtime refuses it before any work registers.
        let first = Runtime::start(options()).unwrap();
        let second = Runtime::start(options()).unwrap();
        let base = unique_dir("foreign");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&first, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);

        assert!(
            matches!(
                second.open_session(&db, policy(), Box::new(|| {})),
                Err(RuntimeError::ForeignRuntime)
            ),
            "the wrong runtime cannot open a session over a foreign db"
        );
        assert!(
            matches!(
                second.open_writer(&db, None, policy(), Box::new(|| {})),
                Err(RuntimeError::ForeignRuntime)
            ),
            "the wrong runtime cannot open a writer over a foreign db"
        );

        drop(owner);
        assert_eq!(drain_runtime(&first), CloseReport::Closed);
        assert_eq!(drain_runtime(&second), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn session_close_is_idempotent_and_the_database_survives() {
        // RUN close: draining a session reports the real terminal outcome;
        // a second drain on the same inert session joins it (double
        // release is harmless). The database entry survives an individual
        // session close.
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("idempotent-close");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);

        let session = open_read(&runtime, &db);
        assert_eq!(drain_session(&session), CloseReport::Closed);
        assert_eq!(
            drain_session(&session),
            CloseReport::Closed,
            "a second close joins the first"
        );
        // The database is still registered after the session closed.
        assert_eq!(runtime.inspect().databases, 1);

        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_stale_prepared_id_misses_and_never_aliases() {
        // FFI generation/stale-scope + double-release shape: prepared ids
        // are minted from the runtime's one never-reused counter, so a
        // retained id that names no installed query can only miss
        // (`ClosedHandle`) — it can never alias a successor prepared query.
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("stale-prepared");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);

        let session = open_read(&runtime, &db);
        // Removing an id that was never installed is the double-release /
        // stale-handle shape: a typed closed-handle miss, never a panic
        // and never a touch of another job's state.
        assert!(
            matches!(
                run_read(&runtime, &session, |_| {
                    Ok(Box::new(|context, frame| {
                        context.checkpoint()?;
                        frame.remove_prepared(9_999)?;
                        Ok(Output::Ready)
                    }))
                }),
                Err(RuntimeError::ClosedHandle)
            ),
            "a stale/never-installed prepared id misses as ClosedHandle"
        );
        assert_eq!(drain_session(&session), CloseReport::Closed);

        drop(owner);
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn runtime_shutdown_drains_a_live_read_session() {
        // RUN open-vs-shutdown: a runtime close with a live session stops
        // admission, drains the session thread and reports the terminal
        // outcome; the session's own close then joins the completed drain.
        let runtime = Runtime::start(options()).unwrap();
        let base = unique_dir("open-vs-shutdown");
        std::fs::create_dir_all(&base).unwrap();
        let owner = acquire(&runtime, &base.join("tenant"));
        let descriptor = Mini.descriptor();
        let db = attach(&owner, &descriptor);

        let session = open_read(&runtime, &db);
        // Drop the JS-held handles; the registry still owns the resources.
        drop(owner);
        drop(db);
        // Runtime shutdown must join the session thread and the owner.
        assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
        // The session wrapper's own close now joins the finished drain.
        assert_eq!(drain_session(&session), CloseReport::Closed);
        let _ = std::fs::remove_dir_all(&base);
    }
}
