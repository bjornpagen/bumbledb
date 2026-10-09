//! Admission order for a consuming resource whose inputs cross workers.
//! A reservation is one admitted call, never a row or a durable receipt.
//! Ready calls are dispatched in reservation order without blocking workers.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::session::{Message, PayloadWork};
use super::{Capability, Operation, Runtime, RuntimeError, lock};

enum Slot {
    Waiting,
    Ready(Arc<Operation>, PayloadWork),
    Abandoned,
}

struct State {
    first: u64,
    slots: VecDeque<Slot>,
}

pub struct PayloadSequence {
    runtime: Arc<Runtime>,
    cap: Capability,
    state: Mutex<State>,
}

pub struct PayloadReservation {
    sequence: Arc<PayloadSequence>,
    ticket: Option<u64>,
}

impl PayloadSequence {
    pub(crate) fn new(runtime: Arc<Runtime>, cap: Capability) -> Arc<Self> {
        Arc::new(Self {
            runtime,
            cap,
            state: Mutex::new(State {
                first: 0,
                slots: VecDeque::new(),
            }),
        })
    }

    pub(crate) fn reserve(
        self: &Arc<Self>,
        finalizing: bool,
    ) -> Result<PayloadReservation, RuntimeError> {
        let mut state = lock(&self.state);
        if finalizing {
            self.runtime.registry.begin_finalization(self.cap)?;
        } else {
            self.runtime.registry.state(self.cap)?.admit()?;
        }
        let ticket = state
            .first
            .checked_add(state.slots.len() as u64)
            .ok_or(RuntimeError::Internal)?;
        state.slots.push_back(Slot::Waiting);
        Ok(PayloadReservation {
            sequence: Arc::clone(self),
            ticket: Some(ticket),
        })
    }

    fn settle(&self, ticket: u64, value: Slot) {
        // Sending under this lock preserves ordering even when two workers
        // settle simultaneously. Failure completion happens outside it: it
        // may drop another reservation and re-enter this sequence.
        let mut failures = Vec::new();
        {
            let mut state = lock(&self.state);
            let offset = usize::try_from(ticket - state.first).expect("reserved sequence offset");
            state.slots[offset] = value;
            while matches!(state.slots.front(), Some(Slot::Ready(..) | Slot::Abandoned)) {
                let slot = state.slots.pop_front().expect("ready sequence slot");
                state.first += 1;
                if let Slot::Ready(operation, work) = slot
                    && let Err(error) = self.runtime.send_resource(
                        self.cap,
                        Message::Payload {
                            operation: Arc::clone(&operation),
                            work,
                        },
                    )
                {
                    failures.push((operation, error));
                }
            }
        }
        for (operation, error) in failures {
            self.runtime.complete_operation(&operation, Err(error));
        }
    }
}

impl PayloadReservation {
    pub(crate) fn dispatch(mut self, operation: Arc<Operation>, work: PayloadWork) {
        let ticket = self.ticket.take().expect("unsettled reservation");
        self.sequence.settle(ticket, Slot::Ready(operation, work));
    }
}

impl Drop for PayloadReservation {
    fn drop(&mut self) {
        if let Some(ticket) = self.ticket.take() {
            self.sequence.settle(ticket, Slot::Abandoned);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::registry::{NativeKind, Payload, RegistryAdmission, ResultState};
    use crate::runtime::{CloseReport, Options, Output};
    use bumbledb::WorkContext;
    use std::sync::mpsc::{Receiver, channel};
    use std::time::Duration;

    fn runtime(workers: usize) -> Arc<Runtime> {
        Runtime::start(Options {
            workers,
            queue_capacity: 16,
            cleanup_capacity: 16,
            owner_capacity: 4,
            native_handle_capacity: 16,
            cleanup_timeout: Duration::from_secs(2),
        })
        .unwrap()
    }
    fn admit(runtime: &Arc<Runtime>) -> RegistryAdmission {
        RegistryAdmission::admit(
            Arc::clone(runtime),
            NativeKind::Result,
            Payload::Result {
                result: None,
                state: ResultState::Live,
            },
        )
        .unwrap()
    }
    fn take(
        runtime: &Runtime,
        operation: &Arc<Operation>,
        done: &Receiver<()>,
    ) -> Result<Output, RuntimeError> {
        done.recv_timeout(Duration::from_secs(5)).unwrap();
        runtime.take(operation)
    }
    fn drain(runtime: &Runtime) {
        let (notify, done) = channel();
        runtime.drain(
            None,
            Box::new(move |report| {
                notify.send(report).unwrap();
            }),
        );
        assert_eq!(
            done.recv_timeout(Duration::from_secs(5)).unwrap(),
            CloseReport::Closed
        );
        assert_eq!(runtime.inspect().natives, 0);
    }

    #[test]
    fn finalization_waits_for_cross_worker_inputs_and_preserves_admission_order() {
        for workers in [1, 3] {
            for cancel in [false, true] {
                let runtime = runtime(workers);
                let target = admit(&runtime);
                let source = admit(&runtime);
                let sequence = PayloadSequence::new(Arc::clone(&runtime), target.cap());
                let events = Arc::new(Mutex::new(Vec::new()));
                let first = sequence.reserve(false).unwrap();
                let abandoned = sequence.reserve(false).unwrap();
                let (entered, running) = channel();
                let (release, blocked) = channel();
                let (notify, done) = channel();
                let observed = Arc::clone(&events);
                let context = WorkContext::new();
                let input = runtime
                    .submit_payload(
                        source.cap(),
                        context.clone(),
                        Box::new(move || {
                            notify.send(()).unwrap();
                        }),
                        |_| {
                            Ok(Box::new(move |_, _, _| {
                                entered.send(()).unwrap();
                                blocked.recv_timeout(Duration::from_secs(5)).unwrap();
                                Ok(Output::OrderedPayloadContinuation {
                                    reservation: first,
                                    work: Box::new(move |_, _, _| {
                                        lock(&observed).push(1);
                                        Ok(Output::Ready)
                                    }),
                                })
                            }))
                        },
                    )
                    .unwrap();
                running.recv_timeout(Duration::from_secs(5)).unwrap();
                let terminal = sequence.reserve(true).unwrap();
                assert!(matches!(
                    sequence.reserve(false),
                    Err(RuntimeError::ClosedHandle)
                ));
                assert!(matches!(
                    sequence.reserve(true),
                    Err(RuntimeError::ClosedHandle)
                ));
                let observed = Arc::clone(&events);
                let (notify, finished) = channel();
                let finish = runtime
                    .finalize_payload_ordered(
                        target.cap(),
                        Some(terminal),
                        WorkContext::new(),
                        Box::new(move || {
                            notify.send(()).unwrap();
                        }),
                        |_| {
                            Ok(Box::new(move |_, _, _| {
                                lock(&observed).push(2);
                                Ok(Output::Ready)
                            }))
                        },
                    )
                    .unwrap();
                assert!(
                    finished.try_recv().is_err(),
                    "finalizer must not pass a pending input"
                );
                drop(abandoned);
                if cancel {
                    context.cancel();
                }
                release.send(()).unwrap();
                let result = take(&runtime, &input, &done);
                assert_eq!(result.is_err(), cancel);
                assert!(matches!(
                    take(&runtime, &finish, &finished),
                    Ok(Output::Ready)
                ));
                assert_eq!(*lock(&events), if cancel { vec![2] } else { vec![1, 2] });
                drain(&runtime);
            }
        }
    }

    #[test]
    fn close_revokes_a_reserved_input_and_its_queued_finalizer() {
        let runtime = runtime(2);
        let target = admit(&runtime);
        let sequence = PayloadSequence::new(Arc::clone(&runtime), target.cap());
        let pending = sequence.reserve(false).unwrap();
        let terminal = sequence.reserve(true).unwrap();
        let (notify, done) = channel();
        let operation = runtime
            .finalize_payload_ordered(
                target.cap(),
                Some(terminal),
                WorkContext::new(),
                Box::new(move || {
                    notify.send(()).unwrap();
                }),
                |_| Ok(Box::new(|_, _, _| panic!("close prevents finalization"))),
            )
            .unwrap();
        runtime.request_resource_close(target.cap()).unwrap();
        drop(pending);
        assert!(take(&runtime, &operation, &done).is_err());
        drain(&runtime);
    }
    #[test]
    fn later_inputs_cannot_overtake_an_unready_reservation_and_shutdown_drains_them() {
        for shutdown in [false, true] {
            let runtime = runtime(3);
            let target = admit(&runtime);
            let a = admit(&runtime);
            let b = admit(&runtime);
            let sequence = PayloadSequence::new(Arc::clone(&runtime), target.cap());
            let first = sequence.reserve(false).unwrap();
            let second = sequence.reserve(false).unwrap();
            let events = Arc::new(Mutex::new(Vec::new()));
            let (entered, running) = channel();
            let (release, blocked) = channel();
            let (notify, done_a) = channel();
            let observed = Arc::clone(&events);
            let job_a = runtime
                .submit_payload(
                    a.cap(),
                    WorkContext::new(),
                    Box::new(move || {
                        let _ = notify.send(());
                    }),
                    |_| {
                        Ok(Box::new(move |_, _, _| {
                            entered.send(()).unwrap();
                            blocked.recv_timeout(Duration::from_secs(5)).unwrap();
                            Ok(Output::OrderedPayloadContinuation {
                                reservation: first,
                                work: Box::new(move |_, _, _| {
                                    lock(&observed).push(1);
                                    Ok(Output::Ready)
                                }),
                            })
                        }))
                    },
                )
                .unwrap();
            running.recv_timeout(Duration::from_secs(5)).unwrap();
            let (entered, ready_b) = channel();
            let (notify, done_b) = channel();
            let observed = Arc::clone(&events);
            let job_b = runtime
                .submit_payload(
                    b.cap(),
                    WorkContext::new(),
                    Box::new(move || {
                        let _ = notify.send(());
                    }),
                    |_| {
                        Ok(Box::new(move |_, _, _| {
                            entered.send(()).unwrap();
                            Ok(Output::OrderedPayloadContinuation {
                                reservation: second,
                                work: Box::new(move |_, _, _| {
                                    lock(&observed).push(2);
                                    Ok(Output::Ready)
                                }),
                            })
                        }))
                    },
                )
                .unwrap();
            ready_b.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(done_b.try_recv().is_err());
            let last = sequence.reserve(true).unwrap();
            let (notify, finished) = channel();
            let observed = Arc::clone(&events);
            let finalizer = runtime
                .finalize_payload_ordered(
                    target.cap(),
                    Some(last),
                    WorkContext::new(),
                    Box::new(move || {
                        let _ = notify.send(());
                    }),
                    |_| {
                        Ok(Box::new(move |_, _, _| {
                            lock(&observed).push(3);
                            Ok(Output::Ready)
                        }))
                    },
                )
                .unwrap();
            if shutdown {
                runtime.begin_close();
            }
            release.send(()).unwrap();
            if shutdown {
                drain(&runtime);
                assert!(lock(&events).is_empty());
                assert_eq!(runtime.inspect().retained, 0);
            } else {
                assert!(matches!(take(&runtime, &job_a, &done_a), Ok(Output::Ready)));
                assert!(matches!(take(&runtime, &job_b, &done_b), Ok(Output::Ready)));
                assert!(matches!(
                    take(&runtime, &finalizer, &finished),
                    Ok(Output::Ready)
                ));
                assert_eq!(*lock(&events), vec![1, 2, 3]);
                drain(&runtime);
            }
        }
    }
}
