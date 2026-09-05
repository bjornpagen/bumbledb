//! Native operation ownership. No database semantics or JavaScript objects live here.
//!
//! Admission conservatively reserves an operation's complete byte allowance.
//! Reservations survive completion until acknowledgement (or actual reclamation).
//! A separate supervisor delivers finite drain reports even when every worker is busy.
use std::collections::{BTreeMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use bumbledb::work::{ByteReservation, ExecutionPolicy, WorkContext, WorkError};

pub mod owners;
pub mod fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    RuntimeAlreadyLive,
    ForeignRuntime,
    ClosedHandle,
    SpentHandle,
    QueueFull,
    InvalidArgument,
    Internal,
    DirectoryBusy,
    InvalidPath,
    Io { kind: std::io::ErrorKind, code: Option<i32> },
    ResourceLimit {
        dimension: &'static str,
        used: u64,
        requested: u64,
        limit: u64,
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
    Fs(fs::FsOutput),
}
pub type Work = Box<dyn FnOnce(&WorkContext) -> Result<Output, RuntimeError> + Send>;
type Notify = Box<dyn FnOnce() + Send>;
type Report = Box<dyn FnOnce(CloseReport) + Send>;

pub struct Operation {
    id: u64,
    context: WorkContext,
    bytes: [u64; 4],
    owner: Option<u64>,
    database: Option<u64>,
    external: bool,
    // Protected exclusively by Runtime.state. Output never escapes a live worker.
    completion: Mutex<Option<Notify>>,
    output: Mutex<Option<Result<Output, RuntimeError>>>,
}

struct Job {
    operation: Arc<Operation>,
    work: Work,
}
enum Action {
    Job(Job),
    Cleanup(owners::Cleanup),
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
}
struct State {
    phase: Phase,
    next_id: u64,
    workers: usize,
    active: usize,
    queue: VecDeque<Job>,
    operations: BTreeMap<u64, Arc<Operation>>,
    owners: BTreeMap<u64, owners::OwnerEntry>,
    reserved: [u64; 4],
    waiters: Vec<Waiter>,
}

pub struct Runtime {
    pub options: Options,
    state: Mutex<State>,
    changed: Condvar,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // No user/engine work runs with this bookkeeping lock held. Poison is still
    // recovered so cleanup cannot be disabled by an unrelated caught panic.
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl State {
    fn inspection(&self) -> Inspection {
        Inspection {
            phase: self.phase,
            queued: self.queue.len(),
            active: self.active,
            retained: self.operations.len(),
            owners: self.owners.len(),
            databases: self.owners.values().map(|owner| owner.databases.len()).sum(),
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
        let runtime = Arc::new(Self {
            options,
            state: Mutex::new(State {
                phase: Phase::Open,
                next_id: 1,
                workers: 0,
                active: 0,
                queue: VecDeque::new(),
                operations: BTreeMap::new(),
                owners: BTreeMap::new(),
                reserved: [0; 4],
                waiters: Vec::new(),
            }),
            changed: Condvar::new(),
        });
        let mut workers = Vec::new();
        for index in 0..options.workers {
            let owner = Arc::clone(&runtime);
            lock(&runtime.state).workers += 1;
            if let Ok(worker) = thread::Builder::new()
                .name(format!("bumbledb-{index}"))
                .spawn(move || owner.worker())
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
        for (index, requested) in bytes.iter().copied().enumerate() {
            let used = state.reserved[index];
            let limit = self.options.aggregate_bytes[index];
            if used.checked_add(requested).is_none_or(|next| next > limit) {
                return Err(RuntimeError::ResourceLimit {
                    dimension: ["inputBytes", "workingBytes", "scratchBytes", "resultBytes"][index],
                    used,
                    requested,
                    limit,
                });
            }
        }
        let id = state.next_id;
        state.next_id = id.checked_add(1).ok_or(RuntimeError::Internal)?;
        let operation = Arc::new(Operation {
            id,
            context,
            bytes,
            owner,
            database,
            external: false,
            completion: Mutex::new(Some(notify)),
            output: Mutex::new(None),
        });
        for (used, bytes) in state.reserved.iter_mut().zip(bytes) {
            *used += bytes;
        }
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
        if let Some(operation) = operation {
            operation.context.cancel();
        } else {
            Self::closing(&mut state);
        }
        if state.phase == Phase::Closed
            || operation.is_some_and(|op| !state.operations.contains_key(&op.id))
        {
            drop(state);
            report(CloseReport::Closed);
            return;
        }
        if state.waiters.len() >= self.options.cleanup_capacity {
            drop(state);
            report(CloseReport::Failed);
            return;
        }
        state.waiters.push(Waiter {
            target: operation.map_or(WaitTarget::Runtime, |value| WaitTarget::Operation(value.id)),
            deadline: Instant::now() + self.options.cleanup_timeout,
            report,
        });
        self.changed.notify_all();
    }

    pub fn begin_close(&self) {
        Self::closing(&mut lock(&self.state));
        self.changed.notify_all();
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

    fn worker(&self) {
        loop {
            let action = {
                let mut state = lock(&self.state);
                loop {
                    if let Some(cleanup) = state.cleanup() {
                        state.active += 1;
                        break Action::Cleanup(cleanup);
                    }
                    if let Some(job) = state.queue.pop_front() {
                        state.active += 1;
                        break Action::Job(job);
                    }
                    if state.phase != Phase::Open && state.owners.is_empty() {
                        state.workers -= 1;
                        self.changed.notify_all();
                        return;
                    }
                    state = self
                        .changed
                        .wait(state)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                }
            };
            let job = match action {
                Action::Job(job) => job,
                Action::Cleanup(cleanup) => {
                    self.run_cleanup(cleanup);
                    let mut state = lock(&self.state);
                    state.active -= 1;
                    self.changed.notify_all();
                    continue;
                }
            };
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                job.operation.context.checkpoint()?;
                (job.work)(&job.operation.context)
            }));
            let panic = outcome.is_err();
            let mut outcome = outcome.unwrap_or(Err(RuntimeError::Internal));
            // A late successful acquisition cannot survive cancellation. Dropping
            // the returned owner here reclaims it before the drain is completed.
            // A completed store mutation is evidence, not a cancellable
            // acquisition. Interruption may discard its delivery, never
            // rewrite a known mutation outcome into a rollback claim.
            if !matches!(&outcome, Ok(Output::Fs(value)) if value.mutating())
                && let Err(error) = job.operation.context.checkpoint() {
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
    }

    fn supervise(&self, workers: Vec<JoinHandle<()>>) {
        let mut workers = Some(workers);
        loop {
            let mut state = lock(&self.state);
            let completed: Vec<_> = state
                .operations
                .iter()
                .filter_map(|(&id, operation)| {
                    (lock(&operation.output).is_some()
                        && operation.context.checkpoint() == Err(WorkError::Cancelled))
                    .then_some(id)
                })
                .collect();
            let discarded: Vec<_> = completed.into_iter().filter_map(|id| state.remove(id)).collect();
            if !discarded.is_empty() {
                drop(state);
                drop(discarded);
                state = lock(&self.state);
            }
            if state.phase == Phase::Closing && state.workers == 0 && state.operations.is_empty() && state.owners.is_empty() {
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
                let done = state.target_done(waiter.target);
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

#[cfg(test)]
mod tests;
