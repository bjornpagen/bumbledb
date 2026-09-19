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
    Resource { cap: Capability, message: Message },
    /// Already-owned close. Coalesced on the route; this wakes the worker.
    Close(Capability),
    /// Wake a sleeping worker for any other source (shared queue, cleanup).
    Wake,
    /// Send payload install (result/cursor/draft/changes). Fire-and-forget:
    /// the capability is already reserved; the worker owns insert/rollback.
    InstallSend {
        cap: Capability,
        payload: Box<super::registry::Payload>,
    },
}

impl Runtime {
    pub(crate) fn lane_send(
        &self,
        lane: LaneId,
        command: WorkerCommand,
    ) -> Result<(), RuntimeError> {
        let sender = self
            .lane_senders
            .get(lane.0)
            .ok_or(RuntimeError::Internal)?;
        // Serialize enqueue with the worker's last inbox check/exit decision.
        // A channel send can otherwise succeed after that decision but before
        // its receiver drops, leaving an owned operation with no consumer.
        // Caller must not already hold runtime.state.
        let state = super::lock(&self.state);
        if state.phase != super::Phase::Open
            && matches!(
                &command,
                WorkerCommand::Resource { .. } | WorkerCommand::InstallSend { .. }
            )
        {
            // Work can own reservations whose Drop re-enters this runtime.
            drop(state);
            return Err(RuntimeError::ClosedHandle);
        }
        let result = sender.send(command);
        self.changed.notify_all();
        drop(state);
        // A failed send still owns its command; drop that outside bookkeeping.
        result.map_err(|_| RuntimeError::ClosedHandle)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::registry::{NativeKind, Payload, RegistryAdmission, ResultState};
    use crate::runtime::{CloseReport, Options, Output};
    use bumbledb::WorkContext;
    use std::sync::Arc;
    use std::time::Duration;

    struct InspectOnDrop(Arc<Runtime>);
    impl Drop for InspectOnDrop {
        fn drop(&mut self) {
            // Rejected work may own a reservation or close guard whose drop
            // re-enters runtime bookkeeping. Never drop it under that lock.
            self.0.inspect();
        }
    }

    #[test]
    fn closing_refuses_new_owned_inbox_work_even_while_the_receiver_is_alive() {
        let runtime = Runtime::start(Options {
            workers: 1,
            queue_capacity: 4,
            cleanup_capacity: 4,
            owner_capacity: 4,
            native_handle_capacity: 4,
            cleanup_timeout: Duration::from_secs(2),
        })
        .unwrap();
        let native = RegistryAdmission::admit(
            Arc::clone(&runtime),
            NativeKind::Result,
            Payload::Result {
                result: None,
                state: ResultState::Live,
            },
        )
        .unwrap();
        let (entered, running) = channel();
        let (release, blocked) = channel();
        let (notify, done) = channel();
        let operation = runtime
            .submit_payload(
                native.cap(),
                WorkContext::new(),
                Box::new(move || {
                    let _ = notify.send(());
                }),
                |_| {
                    Ok(Box::new(move |_, _, _| {
                        entered.send(()).unwrap();
                        blocked.recv_timeout(Duration::from_secs(5)).unwrap();
                        Ok(Output::Ready)
                    }))
                },
            )
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        runtime.begin_close();
        let reentrant = InspectOnDrop(Arc::clone(&runtime));
        let routed = runtime.send_resource(
            native.cap(),
            Message::Payload {
                operation: Arc::clone(&operation),
                work: Box::new(move |_, _, _| {
                    drop(reentrant);
                    panic!("closing work cannot execute")
                }),
            },
        );
        let installed = runtime.install_send_payload(
            native.cap(),
            Payload::Result {
                result: None,
                state: ResultState::Live,
            },
        );
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_secs(5)).unwrap();
        let (notify, done) = channel();
        runtime.drain(
            None,
            Box::new(move |report| {
                notify.send(report).unwrap();
            }),
        );
        let closed = done.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(routed, Err(RuntimeError::ClosedHandle));
        assert_eq!(installed, Err(RuntimeError::ClosedHandle));
        assert_eq!(closed, CloseReport::Closed);
        assert_eq!(runtime.inspect().retained, 0);
        assert_eq!(runtime.inspect().natives, 0);
    }
}
