//! The environment-wide transaction gate.
//!
//! `heed::Env::resize` is documented safe only with **no active
//! transactions**, and the library does not check that condition; a writer
//! mutex alone is insufficient. Every transaction the store creates —
//! owned snapshots and the single writer — holds a [`GatePass`]. Resize (and
//! close) take the gate exclusively: stop admitting new passes, wait for
//! live passes to drain within the caller's work budget, and either proceed
//! or return a typed [`StoreError::ResizeBlockedByReaders`] naming the live
//! count and oldest age so the caller can release its snapshots. A live Rust
//! borrow is never invalidated to meet a deadline.

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use super::error::{StoreError, StoreResult};
use crate::work::WorkContext;

const WAIT_QUANTUM: Duration = Duration::from_millis(1);

#[derive(Debug, Default)]
struct GateState {
    /// Live pass id → admission instant.
    live: BTreeMap<u64, Instant>,
    next_pass: u64,
    /// A resize/close is draining; new passes wait (resize) or refuse (close).
    draining: bool,
    /// Terminal: the store is closing; new passes refuse forever.
    closing: bool,
}

#[derive(Debug, Default)]
struct GateCore {
    state: Mutex<GateState>,
    changed: Condvar,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TransactionGate {
    core: Arc<GateCore>,
}

/// RAII admission for one transaction. Dropping it releases the slot and
/// wakes any drain waiter.
#[derive(Debug)]
pub(crate) struct GatePass {
    core: Arc<GateCore>,
    id: u64,
}

impl Drop for GatePass {
    fn drop(&mut self) {
        let mut state = self
            .core
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.live.remove(&self.id);
        drop(state);
        self.core.changed.notify_all();
    }
}

/// Exclusive access: no live passes exist and none are admitted until this
/// guard drops.
#[derive(Debug)]
pub(crate) struct ExclusiveGuard {
    core: Arc<GateCore>,
}

impl Drop for ExclusiveGuard {
    fn drop(&mut self) {
        let mut state = self
            .core
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.draining = false;
        drop(state);
        self.core.changed.notify_all();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GateSnapshot {
    pub live: u64,
    pub oldest_age: Option<Duration>,
}

impl TransactionGate {
    /// Admit one transaction. Waits through a resize drain (bounded by the
    /// caller's work context) and refuses a closing store.
    pub(crate) fn enter(&self, work: &WorkContext) -> StoreResult<GatePass> {
        loop {
            {
                let mut state = self
                    .core
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if state.closing {
                    return Err(StoreError::Closed);
                }
                if !state.draining {
                    let id = state.next_pass;
                    state.next_pass = state.next_pass.checked_add(1).ok_or(StoreError::Closed)?;
                    state.live.insert(id, Instant::now());
                    return Ok(GatePass {
                        core: Arc::clone(&self.core),
                        id,
                    });
                }
                let (_state, _timeout) = self
                    .core
                    .changed
                    .wait_timeout(state, WAIT_QUANTUM)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            work.checkpoint()?;
        }
    }

    /// Take the gate exclusively for resize. Blocks new admission and waits
    /// for live passes within the work budget; on exhaustion restores
    /// admission and reports the live diagnostics.
    pub(crate) fn exclusive(&self, work: &WorkContext) -> StoreResult<ExclusiveGuard> {
        {
            let mut state = self
                .core
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closing {
                return Err(StoreError::Closed);
            }
            if state.draining {
                // One resize/close at a time; a concurrent exclusive holder
                // reports as blocking rather than deadlocking.
                let snapshot = snapshot_of(&state);
                return Err(StoreError::ResizeBlockedByReaders {
                    live_transactions: snapshot.live.max(1),
                    oldest_age: snapshot.oldest_age,
                });
            }
            state.draining = true;
        }
        let guard = ExclusiveGuard {
            core: Arc::clone(&self.core),
        };
        loop {
            {
                let state = self
                    .core
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if state.live.is_empty() {
                    return Ok(guard);
                }
                if let Err(stopped) = work.checkpoint() {
                    let snapshot = snapshot_of(&state);
                    drop(state);
                    drop(guard); // restores admission
                    return Err(match stopped {
                        crate::work::WorkError::Cancelled => StoreError::Work(stopped),
                        _ => StoreError::ResizeBlockedByReaders {
                            live_transactions: snapshot.live,
                            oldest_age: snapshot.oldest_age,
                        },
                    });
                }
                let (_state, _timeout) = self
                    .core
                    .changed
                    .wait_timeout(state, WAIT_QUANTUM)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
    }

    /// Terminal close: refuse all future admission, then wait for live
    /// passes within the work budget. Returns the live diagnostics either
    /// way; the caller reports incomplete close honestly and may join later.
    pub(crate) fn begin_close(&self, work: &WorkContext) -> (bool, GateSnapshot) {
        {
            let mut state = self
                .core
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closing = true;
        }
        loop {
            let state = self
                .core
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let snapshot = snapshot_of(&state);
            if snapshot.live == 0 {
                return (true, snapshot);
            }
            if work.checkpoint().is_err() {
                return (false, snapshot);
            }
            let (_state, _timeout) = self
                .core
                .changed
                .wait_timeout(state, WAIT_QUANTUM)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    pub(crate) fn live(&self) -> GateSnapshot {
        let state = self
            .core
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot_of(&state)
    }
}

fn snapshot_of(state: &GateState) -> GateSnapshot {
    GateSnapshot {
        live: state.live.len() as u64,
        oldest_age: state.live.values().min().map(Instant::elapsed),
    }
}
