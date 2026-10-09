//! Close drain: every transaction the store opens holds a [`GatePass`].
//! Close refuses new passes and waits for live ones, observing the caller's
//! cancellation; a live Rust borrow is never revoked.

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::work::WorkContext;

/// How often a draining close re-checks the caller's cancellation flag.
const WAIT_QUANTUM: Duration = Duration::from_millis(1);

#[derive(Debug, Default)]
struct GateState {
    /// Live pass id → admission instant. Ids and instants both increase.
    live: BTreeMap<u64, Instant>,
    next_pass: u64,
    closing: bool,
}

#[derive(Debug, Default)]
pub(crate) struct TransactionGate {
    state: Arc<Mutex<GateState>>,
    released: Arc<Condvar>,
}

/// One admitted transaction; dropping it releases the slot and wakes a
/// draining close.
#[derive(Debug)]
pub(crate) struct GatePass {
    state: Arc<Mutex<GateState>>,
    released: Arc<Condvar>,
    id: u64,
}

impl Drop for GatePass {
    fn drop(&mut self) {
        let mut state = lock(&self.state);
        state.live.remove(&self.id);
        let wake = state.closing && state.live.is_empty();
        drop(state);
        if wake {
            self.released.notify_all();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GateSnapshot {
    pub(crate) live: u64,
    pub(crate) oldest_age: Option<Duration>,
}

fn lock(state: &Mutex<GateState>) -> std::sync::MutexGuard<'_, GateState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn snapshot_of(state: &GateState) -> GateSnapshot {
    GateSnapshot {
        live: state.live.len() as u64,
        oldest_age: state
            .live
            .first_key_value()
            .map(|(_, admitted)| admitted.elapsed()),
    }
}

impl TransactionGate {
    /// Admit one transaction; a closing store refuses.
    pub(crate) fn enter(&self, work: &WorkContext) -> Result<GatePass> {
        work.checkpoint()?;
        let mut state = lock(&self.state);
        if state.closing {
            return Err(Error::Closed);
        }
        let id = state.next_pass;
        state.next_pass += 1;
        state.live.insert(id, Instant::now());
        Ok(GatePass {
            state: Arc::clone(&self.state),
            released: Arc::clone(&self.released),
            id,
        })
    }

    /// Terminal close: refuse all future admission, then wait for live
    /// passes until they drain or the caller cancels. Returns whether the
    /// gate drained and the live diagnostics either way.
    pub(crate) fn begin_close(&self, work: &WorkContext) -> (bool, GateSnapshot) {
        let mut state = lock(&self.state);
        state.closing = true;
        loop {
            let snapshot = snapshot_of(&state);
            if snapshot.live == 0 {
                return (true, snapshot);
            }
            if work.checkpoint().is_err() {
                return (false, snapshot);
            }
            state = self
                .released
                .wait_timeout(state, WAIT_QUANTUM)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
    }

    #[cfg(test)]
    fn live(&self) -> GateSnapshot {
        snapshot_of(&lock(&self.state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work() -> WorkContext {
        WorkContext::new()
    }

    #[test]
    fn passes_count_while_live_and_release_on_drop() {
        let gate = TransactionGate::default();
        let first = gate.enter(&work()).unwrap();
        let second = gate.enter(&work()).unwrap();
        assert_eq!(gate.live().live, 2);
        assert!(gate.live().oldest_age.is_some());
        drop(first);
        assert_eq!(gate.live().live, 1);
        drop(second);
        assert_eq!(
            gate.live(),
            GateSnapshot {
                live: 0,
                oldest_age: None
            }
        );
    }

    #[test]
    fn close_reports_live_passes_until_they_drain_and_stays_terminal() {
        let gate = TransactionGate::default();
        let held = gate.enter(&work()).unwrap();
        let stopped = work();
        stopped.cancel();
        let (drained, snapshot) = gate.begin_close(&stopped);
        assert!(!drained);
        assert_eq!(snapshot.live, 1);
        assert!(matches!(gate.enter(&work()), Err(Error::Closed)));
        drop(held);
        assert_eq!(
            gate.begin_close(&work()),
            (
                true,
                GateSnapshot {
                    live: 0,
                    oldest_age: None
                }
            )
        );
        assert!(matches!(gate.enter(&work()), Err(Error::Closed)));
    }

    #[test]
    fn a_draining_close_wakes_when_another_thread_releases_the_last_pass() {
        let gate = TransactionGate::default();
        let held = gate.enter(&work()).unwrap();
        std::thread::scope(|scope| {
            let closer = scope.spawn(|| gate.begin_close(&work()));
            while !lock(&gate.state).closing {
                std::thread::yield_now();
            }
            drop(held);
            assert!(closer.join().unwrap().0);
        });
    }

    #[test]
    fn stopped_work_admits_no_pass() {
        let gate = TransactionGate::default();
        let stopped = work();
        stopped.cancel();
        assert!(matches!(gate.enter(&stopped), Err(Error::Cancelled)));
        assert_eq!(gate.live().live, 0);
    }
}
