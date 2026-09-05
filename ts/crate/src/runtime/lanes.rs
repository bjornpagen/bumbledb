//! Per-worker inbox (C7): routed jobs and coalesced close, never a
//! session-long reactor and never a thread-per-session host claim.

use std::sync::mpsc::{Receiver, Sender, channel};

use super::registry::Capability;
use super::session::Message;
use super::{Runtime, RuntimeError};

pub(crate) struct LaneEndpoints {
    pub senders: Vec<Sender<WorkerCommand>>,
    pub receivers: Vec<Receiver<WorkerCommand>>,
}

pub(crate) fn lane_channels(workers: usize) -> LaneEndpoints {
    let mut senders = Vec::with_capacity(workers);
    let mut receivers = Vec::with_capacity(workers);
    for _ in 0..workers {
        let (tx, rx) = channel();
        senders.push(tx);
        receivers.push(rx);
    }
    LaneEndpoints { senders, receivers }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct LaneId(pub usize);

/// One command for a configured worker's ordinary event loop.
pub(crate) enum WorkerCommand {
    /// One bounded job that borrows a live table entry.
    Resource {
        cap: Capability,
        message: Message,
    },
    /// Already-owned close. Coalesced on the route; this wakes the worker.
    Close(Capability),
    /// Wake a sleeping worker for any other source (shared queue, cleanup).
    Wake,
    /// Send payload install (result/cursor/draft/changes). Fire-and-forget:
    /// the capability is already reserved; the worker owns insert/rollback.
    InstallSend {
        cap: Capability,
        payload: super::registry::Payload,
    },
}

impl Runtime {
    pub(crate) fn lane_send(
        &self,
        lane: LaneId,
        command: WorkerCommand,
    ) -> Result<(), RuntimeError> {
        self.lane_senders
            .get(lane.0)
            .ok_or(RuntimeError::Internal)?
            .send(command)
            .map_err(|_| RuntimeError::ClosedHandle)?;
        // Hold the bookkeeping lock across notify so a worker that has
        // decided to sleep cannot miss this inbox item (lost-wakeup).
        // Caller must not already hold `runtime.state`.
        let _state = super::lock(&self.state);
        self.changed.notify_all();
        Ok(())
    }

    pub(crate) fn wake_worker(&self, worker: u32) {
        let _ = self.lane_send(LaneId(worker as usize), WorkerCommand::Wake);
    }

    /// Inbox Wake + lock-held notify. Caller must not hold `runtime.state`.
    pub(crate) fn wake_all_workers(&self) {
        for index in 0..self.lane_senders.len() {
            let _ = self.lane_send(LaneId(index), WorkerCommand::Wake);
        }
    }

    pub(crate) fn send_resource(
        &self,
        cap: Capability,
        message: Message,
    ) -> Result<(), RuntimeError> {
        self.lane_send(
            LaneId(cap.worker as usize),
            WorkerCommand::Resource { cap, message },
        )
    }

    pub(crate) fn send_close(&self, cap: Capability) -> Result<(), RuntimeError> {
        self.lane_send(LaneId(cap.worker as usize), WorkerCommand::Close(cap))
    }
}
