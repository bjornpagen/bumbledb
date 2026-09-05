//! Native operation ownership. No database semantics or JavaScript objects live here.
//!
//! Admission conservatively reserves an operation's complete byte allowance.
//! Reservations survive completion until acknowledgement (or actual reclamation).
//! A separate supervisor delivers finite drain reports even when every worker is busy.
//!
//! Each configured worker owns a resource table as ordinary event-loop state.
//! Capabilities route to `runtime/worker/kind/id/generation`. Jobs borrow one
//! entry and return to the scheduler. Workers wake for every inbox, queue,
//! close and cleanup source. JS-driven WriterSession/HostWrite ABI is deleted.
use std::collections::{BTreeMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static NEXT_RUNTIME: AtomicU64 = AtomicU64::new(1);

use bumbledb::work::{ByteReservation, ExecutionPolicy, WorkContext, WorkError};

pub mod lanes;
pub mod owners;
pub mod publication;
pub mod registry;
pub mod session;
pub mod table;

pub use registry::registry_draft::DraftLedger;
pub use registry::{
    Capability, CloseDrain, NativeKind, NativeRegistry, ResourceHeader, ResourceState,
};
pub use session::{
    PayloadWork, PublicationSink, SnapshotAccess, SnapshotSession, SnapshotWork,
};

/// Queued conversion owner (D01/C8). Charge stays with the page until JS
/// transfer (`runtime_rows_take`) or native drain. `Output::Page` / `Rows`
/// carry this — not a bare `Vec<Vec<ValueOut>>`.
pub struct QueuedOutput {
    pub rows: Vec<Vec<crate::marshal::ValueOut>>,
    pub charge: ByteReservation,
}

impl QueuedOutput {
    pub fn admit(
        work: &WorkContext,
        rows: Vec<Vec<crate::marshal::ValueOut>>,
        bytes: u64,
    ) -> Result<Self, RuntimeError> {
        Ok(Self {
            rows,
            charge: work.reserve(bumbledb::work::ByteKind::Result, bytes)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    RuntimeAlreadyLive,
    ForeignRuntime,
    ClosedHandle,
    SpentHandle,
    QueueFull,
    InvalidArgument,
    Internal,
    DirectoryBusy,
    /// The one LMDB writer for this database is already owned by a live
    /// write session; the engine ships the refusal, never a queued thread
    /// blocked on the writer mutex.
    WriterBusy,
    InvalidPath,
    Io {
        kind: std::io::ErrorKind,
        code: Option<i32>,
    },
    ResourceLimit {
        dimension: &'static str,
        used: u64,
        requested: u64,
        limit: u64,
    },
    /// A typed engine refusal crossing the executor as owned data: the
    /// core error family tag plus its rendered message. Never a raw
    /// pointer, handle or borrow.
    Engine {
        kind: &'static str,
        message: String,
    },
    Work(WorkError),
}

impl From<WorkError> for RuntimeError {
    fn from(value: WorkError) -> Self {
        Self::Work(value)
    }
}

#[derive(Clone, Copy)]
pub struct Options {
    pub workers: usize,
    pub queue_capacity: usize,
    pub cleanup_capacity: usize,
    pub owner_capacity: usize,
    pub native_handle_capacity: usize,
    pub aggregate_bytes: [u64; 4],
    pub chunk_bytes: u64,
    pub cleanup_timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inspection {
    pub phase: Phase,
    pub queued: usize,
    pub active: usize,
    pub retained: usize,
    pub owners: usize,
    pub databases: usize,
    pub natives: usize,
    pub reserved: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReport {
    Closed,
    Incomplete(Inspection),
    Failed,
}

pub enum Output {
    Ready,
    Hash([u8; 32], ByteReservation),
    Directory(owners::DirectoryOwner),
    Db(owners::ManagedDbOutcome),
    /// A worker-table snapshot opened over a managed database.
    Session(session::SessionOpened),
    /// Owned engine rows crossing back from a session/pool job. Charge
    /// stays with the page until JS transfer or native drain.
    Rows(QueuedOutput),
    Row(Option<Vec<crate::marshal::ValueOut>>),
    Contains(bool),
    Count(u64),
    Generation(u64),
    Prepared(session::PrepareReply),
    Mutation {
        submitted: u64,
        changed: u64,
    },
    Admitted(crate::AdmitOwned),
    /// A grammar-lane payload (log command/decision framing) computed on
    /// the executor: owned bytes plus optional owned metadata.
    Log(crate::log::LogOutput),
    /// A sealed detached schema descriptor (charged compile/admission).
    Descriptor(crate::marshal::DescriptorWire),
    /// A snapshot-bound execution session sharing a pinned read session.
    ExecSession(crate::db_wire::ExecSessionOpened),
    /// One sealed completed query result (C05), owned and independent.
    CompleteResult(bumbledb::CompleteResult),
    /// The one consuming cursor a spent result's backing moved into.
    ResultCursor(bumbledb::ResultCursor),
    /// One owned page off a cursor; `None` is the terminal EOF.
    Page(Option<QueuedOutput>),
    /// A database-free change draft (schema compiled on the executor).
    Draft(crate::db_wire::DraftOpened),
    /// A sealed immutable `ChangeSet` consumed out of a draft.
    Changes(crate::db_wire::ChangesOpened),
    /// One immutable final-state apply outcome (chapter 35 `Db.apply`).
    Apply(crate::db_wire::ApplyOutcomeOwned),
    /// Bounded database diagnostics (measurements, never rows).
    DbReport(crate::db_wire::DbInspectionOwned),
    /// Owned bounded byte payloads (row codec, migration codec responses).
    Bytes(Vec<u8>),
    /// A log-machine payload (histories, commands, caches, admin — C10's
    /// `LogNative` roster over the internal Rust machine).
    Machine(crate::log_wire::MachineOutput),
}

impl Output {
    /// Whether this outcome is EVIDENCE of a dispatched store mutation. A
    /// completed mutation is evidence, not a cancellable acquisition:
    /// interruption may discard its delivery, never rewrite a known
    /// mutation outcome into a rollback claim (the certainty model's one
    /// carve-out from post-work cancellation).
    fn mutation_evidence(&self) -> bool {
        match self {
            Self::Apply(_) => true,
            Self::Machine(output) => output.mutation_evidence(),
            _ => false,
        }
    }

    /// A page/rows owner whose charge and cursor consume are already
    /// committed. Later checkpoints must not replace this slot.
    fn queued_publication(&self) -> bool {
        matches!(self, Self::Page(_) | Self::Rows(_))
    }
}
pub type Work = Box<dyn FnOnce(&WorkContext) -> Result<Output, RuntimeError> + Send>;
pub(crate) type Notify = Box<dyn FnOnce() + Send>;
pub(crate) type Report = Box<dyn FnOnce(CloseReport) + Send>;

pub struct Operation {
    id: u64,
    context: WorkContext,
    bytes: [u64; 4],
    owner: Option<u64>,
    database: Option<u64>,
    session: Option<u64>,
    external: bool,
    // Protected exclusively by Runtime.state. Output never escapes a live worker.
    completion: Mutex<Option<Notify>>,
    output: Mutex<Option<Result<Output, RuntimeError>>>,
}

impl Operation {
    /// Whether this is an externally driven lease (a persistent
    /// owner/session hold), rather than a queued one-shot job. The
    /// `runtime_wire` sibling module cannot read the private field, so this
    /// accessor is the one authorized crossing.
    pub(crate) fn is_external(&self) -> bool {
        self.external
    }
}

struct Job {
    operation: Arc<Operation>,
    work: Work,
}
enum Action {
    Job(Job),
    Cleanup(owners::Cleanup),
    /// Teardown and heavy disposal — independent of ordinary queue
    /// saturation (C7 control drain).
    Control(ControlJob),
    /// Inbox or close flag arrived while waiting; return to the event loop.
    Recheck,
}

pub(crate) type ControlWork = Box<dyn FnOnce() + Send>;

struct ControlJob {
    work: ControlWork,
    report: Option<Report>,
}
struct Waiter {
    target: WaitTarget,
    deadline: Instant,
    report: Report,
}
#[derive(Clone, Copy)]
enum WaitTarget {
    Runtime,
    Operation(u64),
    Owner(u64),
    Database(u64, u64),
    Session(u64, u64, u64),
    Resource(Capability),
}
struct State {
    phase: Phase,
    next_id: u64,
    workers: usize,
    active: usize,
    queue: VecDeque<Job>,
    /// Control/teardown lane: bounded separately from `queue_capacity`.
    control: VecDeque<ControlJob>,
    operations: BTreeMap<u64, Arc<Operation>>,
    owners: BTreeMap<u64, owners::OwnerEntry>,
    reserved: [u64; 4],
    waiters: Vec<Waiter>,
    /// Retained bridge-owned native resources (results, cursors, drafts,
    /// change sets, log capabilities): counted against
    /// `native_handle_capacity` and byte-charged against the resultBytes
    /// aggregate while retained.
    natives: usize,
}

#[cfg(test)]
struct PublicationHold {
    entered: std::sync::mpsc::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
}

pub struct Runtime {
    pub options: Options,
    pub(crate) registry: Arc<NativeRegistry>,
    pub(crate) identity: u64,
    pub(crate) lane_senders: Vec<std::sync::mpsc::Sender<lanes::WorkerCommand>>,
    state: Mutex<State>,
    changed: Condvar,
    /// One-shot D12/D25 probe: next payload publication cancels after
    /// `work()` returns a page and before `operation.output` is written.
    publication_cancel: AtomicBool,
    #[cfg(test)]
    publication_hold: Mutex<Option<PublicationHold>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // No user/engine work runs with this bookkeeping lock held. Poison is still
    // recovered so cleanup cannot be disabled by an unrelated caught panic.
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl State {
    /// The one aggregate byte-admission gate: reserve `bytes` against the
    /// runtime allowance or refuse with the exact exhausted dimension.
    fn charge(&mut self, options: &Options, bytes: [u64; 4]) -> Result<(), RuntimeError> {
        for (index, requested) in bytes.iter().copied().enumerate() {
            let used = self.reserved[index];
            let limit = options.aggregate_bytes[index];
            if used.checked_add(requested).is_none_or(|next| next > limit) {
                return Err(RuntimeError::ResourceLimit {
                    dimension: ["inputBytes", "workingBytes", "scratchBytes", "resultBytes"][index],
                    used,
                    requested,
                    limit,
                });
            }
        }
        for (used, requested) in self.reserved.iter_mut().zip(bytes) {
            *used += requested;
        }
        Ok(())
    }

    fn inspection(&self) -> Inspection {
        Inspection {
            phase: self.phase,
            queued: self.queue.len(),
            active: self.active,
            retained: self.operations.len(),
            owners: self.owners.len(),
            databases: self
                .owners
                .values()
                .map(|owner| owner.databases.len())
                .sum(),
            natives: self.natives,
            reserved: self.reserved,
        }
    }
    fn remove(&mut self, id: u64) -> Option<Result<Output, RuntimeError>> {
        if let Some(operation) = self.operations.remove(&id) {
            for (used, bytes) in self.reserved.iter_mut().zip(operation.bytes) {
                *used -= bytes;
            }
            // Release owned output even if an inert JS operation wrapper is retained.
            let mut output = lock(&operation.output);
            if output.as_ref().is_some_and(Result::is_ok) {
                return output.take();
            }
        }
        None
    }
}

impl Runtime {
    pub fn start(options: Options) -> Result<Arc<Self>, RuntimeError> {
        if options.workers == 0
            || options.queue_capacity == 0
            || options.cleanup_capacity == 0
            || options.owner_capacity == 0
            || options.native_handle_capacity == 0
            || options.chunk_bytes == 0
            || options.cleanup_timeout.is_zero()
            || Instant::now()
                .checked_add(options.cleanup_timeout)
                .is_none()
        {
            return Err(RuntimeError::InvalidArgument);
        }
        let endpoints = lanes::lane_channels(options.workers);
        let lane_receivers = endpoints.receivers;
        let identity = NEXT_RUNTIME.fetch_add(1, Ordering::Relaxed);
        let runtime = Arc::new(Self {
            options,
            identity,
            registry: Arc::new(NativeRegistry::new(identity, options.workers as u32)),
            lane_senders: endpoints.senders,
            state: Mutex::new(State {
                phase: Phase::Open,
                next_id: 1,
                workers: 0,
                active: 0,
                queue: VecDeque::new(),
                control: VecDeque::new(),
                operations: BTreeMap::new(),
                owners: BTreeMap::new(),
                reserved: [0; 4],
                waiters: Vec::new(),
                natives: 0,
            }),
            changed: Condvar::new(),
            publication_cancel: AtomicBool::new(false),
            #[cfg(test)]
            publication_hold: Mutex::new(None),
        });
        let mut workers = Vec::new();
        for (index, lane_rx) in lane_receivers.into_iter().enumerate() {
            let owner = Arc::clone(&runtime);
            lock(&runtime.state).workers += 1;
            if let Ok(worker) = thread::Builder::new()
                .name(format!("bumbledb-{index}"))
                .spawn(move || owner.worker(index as u32, lane_rx))
            {
                workers.push(worker);
            } else {
                lock(&runtime.state).workers -= 1;
                runtime.begin_close();
                for worker in workers {
                    let _ = worker.join();
                }
                return Err(RuntimeError::Internal);
            }
        }
        let owner = Arc::clone(&runtime);
        let workers = Arc::new(Mutex::new(Some(workers)));
        let cleanup_workers = Arc::clone(&workers);
        if thread::Builder::new()
            .name("bumbledb-cleanup".into())
            .spawn(move || owner.supervise(lock(&cleanup_workers).take().unwrap_or_default()))
            .is_err()
        {
            runtime.begin_close();
            // Startup failed before accepting any job: these idle workers can
            // be joined now, so the failed acquisition cannot overlap a successor.
            for worker in lock(&workers).take().unwrap_or_default() {
                let _ = worker.join();
            }
            return Err(RuntimeError::Internal);
        }
        Ok(runtime)
    }

    pub fn inspect(&self) -> Inspection {
        lock(&self.state).inspection()
    }

    pub fn submit(
        &self,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<Work, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        self.submit_at(None, None, policy, notify, prepare)
    }

    fn submit_at(
        &self,
        owner: Option<u64>,
        database: Option<u64>,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<Work, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        // The one core clock/counter authority starts BEFORE admission/queue wait.
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
        state.require_owner(owner)?;
        if state.queue.len() >= self.options.queue_capacity
            || state.operations.len()
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
            owner,
            database,
            session: None,
            external: false,
            completion: Mutex::new(Some(notify)),
            output: Mutex::new(None),
        });
        state.operations.insert(id, Arc::clone(&operation));
        drop(state);
        // Bounded native input extraction is charged after registration, before
        // dispatch. No worker sees or borrows a host object.
        let preparation = catch_unwind(AssertUnwindSafe(|| prepare(&operation.context)));
        let mut state = lock(&self.state);
        let prepared = preparation.unwrap_or_else(|_| {
            Self::closing(&mut state);
            Err(RuntimeError::Internal)
        });
        let work = match prepared {
            Ok(work)
                if state.phase == Phase::Open
                    && state.queue.len() < self.options.queue_capacity =>
            {
                work
            }
            other => {
                let refusal = if state.phase == Phase::Open {
                    RuntimeError::QueueFull
                } else {
                    RuntimeError::ClosedHandle
                };
                let discarded = state.remove(id);
                self.changed.notify_all();
                drop(state);
                drop(discarded);
                return Err(other.err().unwrap_or(refusal));
            }
        };
        state.queue.push_back(Job {
            operation: Arc::clone(&operation),
            work,
        });
        self.changed.notify_all();
        Ok(operation)
    }

    pub fn take(&self, operation: &Operation) -> Result<Output, RuntimeError> {
        let mut state = lock(&self.state);
        if !state.operations.contains_key(&operation.id) {
            // Reclaimed failures retain only bounded diagnostic data on the
            // inert wrapper, never engine resources or a successful value.
            return lock(&operation.output)
                .take()
                .unwrap_or(Err(RuntimeError::SpentHandle));
        }
        let value = lock(&operation.output)
            .take()
            .ok_or(RuntimeError::InvalidArgument)?;
        let discarded = state.remove(operation.id);
        self.changed.notify_all();
        drop(state);
        drop(discarded);
        value
    }

    pub fn drain(&self, operation: Option<&Operation>, report: Report) {
        let mut state = lock(&self.state);
        let mut discarded = Vec::new();
        let wake_workers = if let Some(operation) = operation {
            operation.context.cancel();
            if lock(&operation.output).is_some() {
                if let Some(value) = state.remove(operation.id) {
                    discarded.push(value);
                }
            }
            false
        } else {
            self.closing_runtime(&mut state);
            let abandoned: Vec<u64> = state
                .operations
                .iter()
                .filter_map(|(&id, op)| lock(&op.output).is_some().then_some(id))
                .collect();
            for id in abandoned {
                if let Some(value) = state.remove(id) {
                    discarded.push(value);
                }
            }
            true
        };
        if state.phase == Phase::Closed
            || operation.is_some_and(|op| !state.operations.contains_key(&op.id))
        {
            drop(state);
            drop(discarded);
            if wake_workers {
                self.wake_all_workers();
            } else {
                self.changed.notify_all();
            }
            report(CloseReport::Closed);
            return;
        }
        if state.waiters.len() >= self.options.cleanup_capacity {
            drop(state);
            drop(discarded);
            if wake_workers {
                self.wake_all_workers();
            }
            report(CloseReport::Failed);
            return;
        }
        state.waiters.push(Waiter {
            target: operation.map_or(WaitTarget::Runtime, |value| WaitTarget::Operation(value.id)),
            deadline: Instant::now() + self.options.cleanup_timeout,
            report,
        });
        if wake_workers {
            drop(state);
            drop(discarded);
            self.wake_all_workers();
        } else {
            self.changed.notify_all();
            drop(state);
            drop(discarded);
        }
    }

    pub fn begin_close(&self) {
        let mut state = lock(&self.state);
        self.closing_runtime(&mut state);
        drop(state);
        self.wake_all_workers();
    }

    fn closing(state: &mut State) {
        if state.phase == Phase::Open {
            state.phase = Phase::Closing;
        }
        for operation in state.operations.values() {
            operation.context.cancel();
        }
        for owner in state.owners.values_mut() {
            owner.begin_close(false);
        }
    }

    fn closing_runtime(&self, state: &mut State) {
        Self::closing(state);
        let _ = self.registry.close_all();
    }

    /// Marks every route closing. Callers that hold `state` must drop it
    /// before [`Self::wake_all_workers`] — `lane_send` re-acquires the lock.
    pub(crate) fn revoke_registry(&self) {
        let _ = self.registry.close_all();
    }

    /// Arm the next payload publication gap (one-shot). L16
    /// `runtimeArmPublicationCancel`.
    pub(crate) fn arm_publication_cancel(&self) {
        self.publication_cancel.store(true, Ordering::Release);
    }

    fn take_publication_cancel(&self) -> bool {
        self.publication_cancel.swap(false, Ordering::AcqRel)
    }

    /// Test-only: cancel a live operation without installing a drain
    /// waiter. Supervise reclaims abandoned queued output. Not a public
    /// debug API.
    #[cfg(test)]
    pub(crate) fn cancel_without_waiter(&self, operation: &Operation) {
        operation.context.cancel();
        self.changed.notify_all();
    }

    /// Test-only pause after native publication and before the JS
    /// completion callback. Not a public debug API.
    #[cfg(test)]
    pub(crate) fn arm_publication_hold(
        &self,
        entered: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
    ) {
        *lock(&self.publication_hold) = Some(PublicationHold { entered, release });
    }

    fn wait_publication_hold(&self) {
        #[cfg(test)]
        {
            if let Some(hold) = lock(&self.publication_hold).take() {
                let _ = hold.entered.send(());
                let _ = hold.release.recv();
            }
        }
    }

    /// Notify waiters after a page/rows owner is already registered.
    /// Does not write `Ready` over the publication.
    fn complete_published(&self, operation: &Arc<Operation>) {
        let notify = lock(&operation.completion).take();
        self.wait_publication_hold();
        self.changed.notify_all();
        if let Some(notify) = notify
            && catch_unwind(AssertUnwindSafe(notify)).is_err()
        {
            self.begin_close();
        }
    }

    /// Deliver one session job's terminal outcome: publish the output,
    /// wake waiters and fire the completion callback exactly once. The
    /// session thread is not a pool worker, so `active` is untouched;
    /// the operation stays retained (reservation held) until taken or
    /// reclaimed, exactly like pool work.
    pub(crate) fn complete_operation(
        &self,
        operation: &Arc<Operation>,
        outcome: Result<Output, RuntimeError>,
    ) {
        let notify = lock(&operation.completion).take();
        {
            let _state = lock(&self.state);
            let mut slot = lock(&operation.output);
            // A registered page/rows owner is the publication. Do not
            // replace it with a later checkpoint refusal.
            if !matches!(&*slot, Some(Ok(output)) if output.queued_publication()) {
                *slot = Some(outcome);
            }
        }
        self.changed.notify_all();
        if let Some(notify) = notify
            && catch_unwind(AssertUnwindSafe(notify)).is_err()
        {
            self.begin_close();
        }
    }

    /// Schedules heavy teardown on the control lane. Progresses even when
    /// the ordinary work queue is saturated (C7 independent control drain).
    pub(crate) fn submit_control(
        &self,
        work: ControlWork,
        report: Option<Report>,
    ) -> Result<(), RuntimeError> {
        let mut state = lock(&self.state);
        if state.phase == Phase::Closed {
            if let Some(report) = report {
                drop(state);
                report(CloseReport::Closed);
            }
            return Err(RuntimeError::ClosedHandle);
        }
        if state.control.len() >= self.options.cleanup_capacity {
            if let Some(report) = report {
                drop(state);
                report(CloseReport::Failed);
            }
            return Err(RuntimeError::QueueFull);
        }
        state.control.push_back(ControlJob { work, report });
        self.changed.notify_all();
        Ok(())
    }

    fn worker(&self, index: u32, lane_rx: std::sync::mpsc::Receiver<lanes::WorkerCommand>) {
        table::WorkerContext::attach(index);
        loop {
            while let Ok(command) = lane_rx.try_recv() {
                self.run_worker_command(command);
            }
            self.drain_closing_on_worker(index);
            let action = {
                let mut state = lock(&self.state);
                loop {
                    if let Some(job) = state.control.pop_front() {
                        state.active += 1;
                        break Action::Control(job);
                    }
                    if let Some(cleanup) = state.cleanup() {
                        state.active += 1;
                        break Action::Cleanup(cleanup);
                    }
                    if let Some(job) = state.queue.pop_front() {
                        state.active += 1;
                        break Action::Job(job);
                    }
                    if let Ok(command) = lane_rx.try_recv() {
                        drop(state);
                        self.run_worker_command(command);
                        break Action::Recheck;
                    }
                    if state.phase != Phase::Open && state.owners.is_empty() {
                        let table_empty = table::WorkerContext::with(|ctx| ctx.table.is_empty())
                            .unwrap_or(true);
                        if table_empty {
                            state.workers -= 1;
                            self.changed.notify_all();
                            drop(state);
                            if let Some(ctx) = table::WorkerContext::take() {
                                drop(ctx);
                            }
                            return;
                        }
                    }
                    state = self
                        .changed
                        .wait(state)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    break Action::Recheck;
                }
            };
            match action {
                Action::Recheck => continue,
                Action::Control(job) => {
                    let done = catch_unwind(AssertUnwindSafe(job.work)).is_ok();
                    if let Some(report) = job.report {
                        report(if done {
                            CloseReport::Closed
                        } else {
                            CloseReport::Failed
                        });
                    }
                    let mut state = lock(&self.state);
                    state.active -= 1;
                    self.changed.notify_all();
                }
                Action::Cleanup(cleanup) => {
                    self.run_cleanup(cleanup);
                    let mut state = lock(&self.state);
                    state.active -= 1;
                    self.changed.notify_all();
                }
                Action::Job(job) => self.run_pool_job(job),
            }
        }
    }

    fn run_pool_job(&self, job: Job) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            job.operation.context.checkpoint()?;
            (job.work)(&job.operation.context)
        }));
        let panic = outcome.is_err();
        let mut outcome = outcome.unwrap_or(Err(RuntimeError::Internal));
        if !matches!(&outcome, Ok(output) if output.mutation_evidence())
            && let Err(error) = job.operation.context.checkpoint()
        {
            outcome = Err(error.into());
        }
        let notify = lock(&job.operation.completion).take();
        {
            let mut state = lock(&self.state);
            state.active -= 1;
            if panic {
                Self::closing(&mut state);
            }
            *lock(&job.operation.output) = Some(outcome);
            self.changed.notify_all();
        }
        if let Some(notify) = notify
            && catch_unwind(AssertUnwindSafe(notify)).is_err()
        {
            self.begin_close();
        }
    }

    fn run_worker_command(&self, command: lanes::WorkerCommand) {
        use lanes::WorkerCommand;
        match command {
            WorkerCommand::Wake => {}
            WorkerCommand::Close(cap) => {
                self.drop_closing_entry(cap);
            }
            WorkerCommand::InstallSend { cap, payload } => {
                self.install_send_on_worker(cap, payload);
            }
            WorkerCommand::Resource { cap, message } => match message {
                session::Message::Close => {
                    self.drop_closing_entry(cap);
                }
                session::Message::Snapshot { operation, work } => {
                    self.dispatch_snapshot_message(cap, operation, work);
                }
                session::Message::Payload { operation, work } => {
                    self.dispatch_payload_message(cap, operation, work);
                }
            },
        }
    }

    fn dispatch_snapshot_message(
        &self,
        cap: Capability,
        operation: Arc<Operation>,
        work: session::SnapshotWork,
    ) {
        if self.registry.begin_job(cap).is_err() {
            operation.context.cancel();
            self.complete_operation(&operation, Err(RuntimeError::ClosedHandle));
            return;
        }
        let close_after = table::WorkerContext::with(|ctx| {
            let (payload, _) = match ctx.table.borrow_mut(cap) {
                Ok(borrowed) => borrowed,
                Err(error) => {
                    self.registry.end_job(cap);
                    self.complete_operation(&operation, Err(error));
                    return false;
                }
            };
            let table::TablePayload::Snapshot(resource) = payload else {
                ctx.table.mark_live(cap);
                self.registry.end_job(cap);
                self.complete_operation(&operation, Err(RuntimeError::InvalidArgument));
                return false;
            };
            let mut access = session::SnapshotAccess {
                owned: &resource.owned,
                sealed: resource.sealed.as_ref(),
                job: operation.id,
                prepared: &mut resource.prepared,
            };
            self.run_snapshot_job(&operation, work, &mut access);
            ctx.table.mark_live(cap);
            self.registry.end_job(cap);
            matches!(
                self.registry.state(cap),
                Ok(registry::ResourceState::Closing)
            )
        });
        match close_after {
            Ok(true) => self.drop_closing_entry(cap),
            Ok(false) => {}
            Err(error) => {
                self.registry.end_job(cap);
                self.complete_operation(&operation, Err(error));
            }
        }
    }

    /// One publication transition. L13 either calls
    /// [`session::PublicationSink::accept`] with the live ticket `commit`,
    /// or returns [`crate::db_wire::PullOutcome::committed_output`] and
    /// L12 commits via [`crate::db_wire::accept_publication`] under the
    /// same output lock. After acceptance: no allocate, read, checkpoint,
    /// or preview. Predelivery `Err` publishes nothing. `None` means the
    /// page is already registered — notify only.
    fn run_payload_publication(
        &self,
        operation: &Operation,
        native: &mut registry::Payload,
        work: session::PayloadWork,
    ) -> Result<Option<Output>, RuntimeError> {
        operation.context.checkpoint()?;
        let mut sink = session::PublicationSink::new(operation, self.take_publication_cancel());
        let value = match work(&operation.context, native, &mut sink) {
            Ok(value) => value,
            Err(_) if sink.accepted() => return Ok(None),
            Err(error) => {
                crate::db_wire::reject_publication(native);
                return Err(error);
            }
        };
        if sink.accepted() {
            return Ok(None);
        }
        if !value.queued_publication() {
            crate::db_wire::reject_publication(native);
            operation.context.checkpoint()?;
            return Ok(Some(value));
        }
        let mut slot = lock(&operation.output);
        if matches!(&*slot, Some(Ok(output)) if output.queued_publication()) {
            crate::db_wire::reject_publication(native);
            drop(value);
            return Ok(None);
        }
        if sink.armed() {
            crate::db_wire::reject_publication(native);
            drop(value);
            return Err(RuntimeError::Work(WorkError::Cancelled));
        }
        crate::db_wire::accept_publication(native);
        *slot = Some(Ok(value));
        Ok(None)
    }

    fn dispatch_payload_message(
        &self,
        cap: Capability,
        operation: Arc<Operation>,
        work: session::PayloadWork,
    ) {
        if self.registry.begin_job(cap).is_err() {
            operation.context.cancel();
            self.complete_operation(&operation, Err(RuntimeError::ClosedHandle));
            return;
        }
        let close_after = table::WorkerContext::with(|ctx| {
            let (payload, _) = match ctx.table.borrow_mut(cap) {
                Ok(borrowed) => borrowed,
                Err(error) => {
                    self.registry.end_job(cap);
                    self.complete_operation(&operation, Err(error));
                    return false;
                }
            };
            let table::TablePayload::Native(native) = payload else {
                ctx.table.mark_live(cap);
                self.registry.end_job(cap);
                self.complete_operation(&operation, Err(RuntimeError::InvalidArgument));
                return false;
            };
            let result = catch_unwind(AssertUnwindSafe(|| {
                self.run_payload_publication(&operation, native, work)
            }));
            ctx.table.mark_live(cap);
            self.registry.end_job(cap);
            let panicked = result.is_err();
            let result = result.unwrap_or(Err(RuntimeError::Internal));
            let close = matches!(
                self.registry.state(cap),
                Ok(registry::ResourceState::Closing)
            );
            (panicked, close, result)
        });
        match close_after {
            Ok((panicked, close, result)) => {
                if panicked {
                    self.begin_close();
                }
                match result {
                    Ok(None) => self.complete_published(&operation),
                    Ok(Some(value)) => self.complete_operation(&operation, Ok(value)),
                    Err(error) => self.complete_operation(&operation, Err(error)),
                }
                if close {
                    self.drop_closing_entry(cap);
                }
            }
            Err(error) => {
                self.registry.end_job(cap);
                self.complete_operation(&operation, Err(error));
            }
        }
    }

    fn install_send_on_worker(&self, cap: Capability, payload: registry::Payload) {
        match self.registry.state(cap) {
            Ok(registry::ResourceState::Closing) | Err(_) => {
                drop(payload);
                self.release_native_route(cap);
                return;
            }
            Ok(_) => {}
        }
        let inserted = table::WorkerContext::with(|ctx| {
            ctx.table
                .insert(cap, table::TablePayload::Native(payload), 0)
        });
        match inserted {
            Ok(Ok(())) => {
                if matches!(
                    self.registry.state(cap),
                    Ok(registry::ResourceState::Closing) | Err(_)
                ) {
                    self.drop_closing_entry(cap);
                }
            }
            Ok(Err(_)) | Err(_) => {
                self.rollback_native_route(cap);
            }
        }
    }

    fn snapshot_route_owner(&self, cap: Capability) -> Option<(u64, u64)> {
        let state = lock(&self.state);
        state.owners.iter().find_map(|(&owner, entry)| {
            entry.databases.iter().find_map(|(&database, db)| {
                db.sessions.get(&cap.id).and_then(|slot| {
                    (slot.cap == cap).then_some((owner, database))
                })
            })
        })
    }

    fn drain_closing_on_worker(&self, worker: u32) {
        for cap in self.registry.closing_for_worker(worker) {
            self.drop_closing_entry(cap);
        }
    }

    fn drop_closing_entry(&self, cap: Capability) {
        if cap.id == 0 {
            return;
        }
        let taken = table::WorkerContext::with(|ctx| ctx.table.take(cap));
        match taken {
            Ok(Some((_, table::TablePayload::Snapshot(resource)))) => {
                let owner = resource.owner;
                let database = resource.database;
                drop(resource);
                self.release_snapshot_route(owner, database, cap);
            }
            Ok(Some((_, table::TablePayload::Native(payload)))) => {
                drop(payload);
                self.release_native_route(cap);
            }
            Ok(None) | Err(_) => {
                // Close won the race with async install, or the payload
                // never arrived. Snapshot slots still need the owner map
                // cleared; native routes just drop. Both releases are
                // idempotent if the row is already gone.
                if cap.kind == NativeKind::Snapshot {
                    if let Some((owner, database)) = self.snapshot_route_owner(cap) {
                        self.release_snapshot_route(owner, database, cap);
                        return;
                    }
                }
                self.release_native_route(cap);
            }
        }
    }

    /// Registers one bounded operation targeted at a managed database
    /// (busy/cleanup accounting counts it against the owner AND the child,
    /// so database/directory teardown drains it). Mirrors `submit`.
    pub(crate) fn submit_db(
        self: &Arc<Self>,
        db: &owners::ManagedDb,
        policy: ExecutionPolicy,
        notify: Notify,
        prepare: impl FnOnce(&WorkContext) -> Result<Work, RuntimeError>,
    ) -> Result<Arc<Operation>, RuntimeError> {
        if !Arc::ptr_eq(self, db.runtime()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let (owner, database) = db.ids();
        self.submit_at(Some(owner), Some(database), policy, notify, prepare)
    }

    /// Admits one retained bridge-owned native resource: counted against
    /// `native_handle_capacity`, its payload bytes charged against the
    /// resultBytes aggregate until the returned guard drops. Never a
    /// silent admission: capacity and byte refusals are typed.
    pub(crate) fn retain_native(
        self: &Arc<Self>,
        bytes: u64,
    ) -> Result<RetainedNative, RuntimeError> {
        let mut state = lock(&self.state);
        if state.phase != Phase::Open {
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
        drop(state);
        Ok(RetainedNative {
            runtime: Arc::clone(self),
            bytes,
        })
    }

    /// Operations currently bound to one managed database (queued, active
    /// or retained) — a bounded diagnostic for `Db.inspect`.
    pub(crate) fn database_operations(&self, owner: u64, database: u64) -> u64 {
        let state = lock(&self.state);
        state
            .operations
            .values()
            .filter(|operation| {
                operation.owner == Some(owner) && operation.database == Some(database)
            })
            .count() as u64
    }

    /// Cancelled work with output is abandoned: reclaim Page/Rows even
    /// when `Phase::Open` and no waiter. Dropping the delivery copy does
    /// not rewind committed cursor or store facts. JS take is not required.
    fn reclaim_cancelled_operation(operation: &Operation) -> bool {
        if lock(&operation.output).is_none() {
            return false;
        }
        operation.context.checkpoint() == Err(WorkError::Cancelled)
    }

    fn supervise(&self, workers: Vec<JoinHandle<()>>) {
        let mut workers = Some(workers);
        loop {
            let mut state = lock(&self.state);
            let completed: Vec<_> = state
                .operations
                .iter()
                .filter_map(|(&id, operation)| {
                    Self::reclaim_cancelled_operation(operation).then_some(id)
                })
                .collect();
            let discarded: Vec<_> = completed
                .into_iter()
                .filter_map(|id| state.remove(id))
                .collect();
            if !discarded.is_empty() {
                drop(state);
                drop(discarded);
                state = lock(&self.state);
            }
            if state.phase == Phase::Closing
                && state.workers == 0
                && state.operations.is_empty()
                && state.owners.is_empty()
                && state.natives == 0
            {
                drop(state);
                for worker in workers.take().unwrap_or_default() {
                    let _ = worker.join();
                }
                state = lock(&self.state);
                state.phase = Phase::Closed;
            }
            let now = Instant::now();
            let mut ready = Vec::new();
            let mut pending = Vec::new();
            for waiter in std::mem::take(&mut state.waiters) {
                let done = match waiter.target {
                    WaitTarget::Resource(cap) => self.registry.join(cap),
                    other => state.target_done(other),
                };
                if done {
                    ready.push((waiter.report, CloseReport::Closed));
                } else if state.target_failed(waiter.target) {
                    ready.push((waiter.report, CloseReport::Failed));
                } else if now >= waiter.deadline {
                    ready.push((waiter.report, CloseReport::Incomplete(state.inspection())));
                } else {
                    pending.push(waiter);
                }
            }
            state.waiters = pending;
            if !ready.is_empty() {
                drop(state);
                for (report, value) in ready {
                    report(value);
                }
                continue;
            }
            // Remain available for idempotent close on retained wrappers. No OS
            // thread is needed after Closed; drain handles that state inline.
            if state.phase == Phase::Closed {
                return;
            }
            if let Some(deadline) = state.waiters.iter().map(|value| value.deadline).min() {
                let _guard = self
                    .changed
                    .wait_timeout(state, deadline.saturating_duration_since(Instant::now()))
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            } else {
                let _guard = self
                    .changed
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
    }
}

/// The retained-native accounting guard: releases its handle-count slot and
/// resultBytes charge when the resource actually leaves the registry —
/// however it leaves (explicit close, take failure, or GC of the wrapper).
pub(crate) struct RetainedNative {
    runtime: Arc<Runtime>,
    bytes: u64,
}

impl RetainedNative {
    /// Grows this resource's retained byte charge (draft chunks accumulate;
    /// chunks never reset the aggregate). Refuses typed at the aggregate
    /// resultBytes cap before any growth is recorded.
    pub(crate) fn grow(&mut self, bytes: u64) -> Result<(), RuntimeError> {
        let mut state = lock(&self.runtime.state);
        let used = state.reserved[3];
        let limit = self.runtime.options.aggregate_bytes[3];
        if used.checked_add(bytes).is_none_or(|next| next > limit) {
            return Err(RuntimeError::ResourceLimit {
                dimension: "resultBytes",
                used,
                requested: bytes,
                limit,
            });
        }
        state.reserved[3] += bytes;
        drop(state);
        self.bytes += bytes;
        Ok(())
    }
}

impl Drop for RetainedNative {
    fn drop(&mut self) {
        let mut state = lock(&self.runtime.state);
        state.reserved[3] = state.reserved[3].saturating_sub(self.bytes);
        state.natives = state.natives.saturating_sub(1);
        drop(state);
        self.runtime.changed.notify_all();
    }
}

#[cfg(test)]
mod tests;
