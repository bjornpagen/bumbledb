//! The environment-wide transaction gate.
//!
//! `heed::Env::resize` is documented safe only with **no active
//! transactions**, and the library does not check that condition; a writer
//! mutex alone is insufficient. Every transaction the store creates —
//! owned snapshots and the single writer — holds a transaction pass. The gate
//! may retain one unborrowed read transaction between operations. Writers,
//! resize and close discard it under the admission mutex before proceeding;
//! it never pins pages across writer admission. Resize (and
//! close) take the gate exclusively: stop admitting new passes, wait for
//! live passes to drain, and observe caller cancellation. Resize cancellation
//! restores admission; close remains terminal and reports incomplete drain.
//! A competing exclusive holder returns [`StoreError::ResizeBlockedByReaders`].
//! Live counts and ages remain available; a live Rust borrow is never revoked.

use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use heed::{RoTxn, WithoutTls};

use super::error::{StoreError, StoreResult};
use crate::storage::GenerationId;
use crate::work::{AdmissionStamp, WorkContext};

const WAIT_QUANTUM: Duration = Duration::from_millis(1);

#[derive(Debug, Default)]
struct GateState {
    /// The usual single transaction needs no heap allocation. Additional
    /// concurrent passes live in the map; their memory is released on drop,
    /// rather than retaining the peak concurrency for the store's lifetime.
    inline: Option<(u64, AdmissionStamp)>,
    /// Other live pass ids → admission instant.
    live: BTreeMap<u64, AdmissionStamp>,
    next_pass: u64,
    /// Read passes older than this fence cannot return a reusable snapshot.
    read_fence: u64,
    writer_live: bool,
    /// Owns only an environment clone, never a store owner or gate pass.
    cached: Option<CachedRead>,
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

#[derive(Debug, Default)]
pub(crate) struct TransactionGate {
    core: Arc<GateCore>,
}

/// RAII admission for one transaction. Dropping it releases the slot and
/// wakes any drain waiter.
#[derive(Debug)]
pub(crate) struct GatePass {
    core: Arc<GateCore>,
    id: u64,
    admitted: AdmissionStamp,
    writer: bool,
    released: bool,
}

impl Drop for GatePass {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        let mut state = self
            .core
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.writer {
            state.writer_live = false;
            state.read_fence = state.next_pass;
        }
        let wake = release(&mut state, self.id);
        drop(state);
        if wake {
            self.core.changed.notify_all();
        }
    }
}

/// A coherent read transaction, movable only as a whole. It carries no
/// gate/store reference, so parking it cannot create an ownership cycle.
pub(crate) struct CachedRead {
    txn: RoTxn<'static, WithoutTls>,
    pub(crate) generation: GenerationId,
}

impl std::fmt::Debug for CachedRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedRead")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl CachedRead {
    pub(crate) fn new(txn: RoTxn<'static, WithoutTls>, generation: GenerationId) -> Self {
        Self { txn, generation }
    }
}

/// Must be dropped before the snapshot's `StoreInner`: its pass can own the
/// gate's last reference, and that gate can itself own a parked environment.
pub(crate) struct ReadLease {
    reader: Option<CachedRead>,
    pass: GatePass,
}

impl ReadLease {
    pub(crate) fn new(reader: CachedRead, pass: GatePass) -> Self {
        Self {
            reader: Some(reader),
            pass,
        }
    }

    pub(crate) fn age(&self) -> Duration {
        self.pass.admitted.elapsed()
    }
}

impl Deref for ReadLease {
    type Target = RoTxn<'static, WithoutTls>;

    fn deref(&self) -> &Self::Target {
        &self.reader.as_ref().expect("live read lease").txn
    }
}

impl Drop for ReadLease {
    fn drop(&mut self) {
        let reader = self.reader.take().expect("read lease released once");
        let mut state = self
            .pass
            .core
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !state.writer_live
            && !state.draining
            && !state.closing
            && self.pass.id >= state.read_fence
            && state.cached.is_none()
        {
            state.cached = Some(reader);
        } else {
            // Abort before releasing admission: resize must never race even
            // the destruction of an uncached read transaction.
            drop(reader);
        }
        let wake = release(&mut state, self.pass.id);
        self.pass.released = true;
        drop(state);
        if wake {
            self.pass.core.changed.notify_all();
        }
        // The pass field (including its Arc) drops before this lease's
        // owning snapshot proceeds to drop its StoreInner field.
    }
}

fn release(state: &mut GateState, id: u64) -> bool {
    if state.inline.is_some_and(|(live, _)| live == id) {
        state.inline = None;
    } else {
        let removed = state.live.remove(&id);
        debug_assert!(removed.is_some(), "a pass is released exactly once");
    }
    (state.draining || state.closing) && state.inline.is_none() && state.live.is_empty()
}

#[derive(Clone, Copy)]
enum Admission {
    Other,
    Read,
    Write,
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
        self.admit(work, Admission::Other).map(|(pass, _)| pass)
    }

    /// Taking the reusable transaction and admitting its borrower are one
    /// transition: no writer or resize can enter between them.
    pub(crate) fn enter_read(
        &self,
        work: &WorkContext,
    ) -> StoreResult<(GatePass, Option<CachedRead>)> {
        self.admit(work, Admission::Read)
    }

    pub(crate) fn enter_write(&self, work: &WorkContext) -> StoreResult<GatePass> {
        self.admit(work, Admission::Write).map(|(pass, _)| pass)
    }

    fn admit(
        &self,
        work: &WorkContext,
        admission: Admission,
    ) -> StoreResult<(GatePass, Option<CachedRead>)> {
        work.checkpoint()?;
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
                    let writer = matches!(admission, Admission::Write);
                    // Production callers already own the writer mutex. Do
                    // not let an accidental second admission invalidate the
                    // first writer's return fence or block inside LMDB.
                    if writer && state.writer_live {
                        return Err(StoreError::ReentrantWriter);
                    }
                    let id = state.next_pass;
                    state.next_pass = state.next_pass.checked_add(1).ok_or(StoreError::Closed)?;
                    if writer {
                        state.writer_live = true;
                        state.read_fence = state.next_pass;
                        drop(state.cached.take());
                    }
                    let admitted = AdmissionStamp::now();
                    if state.inline.is_none() {
                        state.inline = Some((id, admitted));
                    } else {
                        state.live.insert(id, admitted);
                    }
                    let cached = if matches!(admission, Admission::Read) {
                        state.cached.take()
                    } else {
                        None
                    };
                    return Ok((
                        GatePass {
                            core: Arc::clone(&self.core),
                            id,
                            admitted,
                            writer,
                            released: false,
                        },
                        cached,
                    ));
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
    /// for live passes. Cancellation restores admission and returns the
    /// cancellation error; a competing exclusive holder reports diagnostics.
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
            drop(state.cached.take());
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
                if state.inline.is_none() && state.live.is_empty() {
                    return Ok(guard);
                }
                if let Err(stopped) = work.checkpoint() {
                    drop(state);
                    drop(guard); // restores admission
                    return Err(StoreError::Work(stopped));
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
    /// passes until completion or cancellation. Returns the live diagnostics either
    /// way; the caller reports incomplete close honestly and may join later.
    pub(crate) fn begin_close(&self, work: &WorkContext) -> (bool, GateSnapshot) {
        {
            let mut state = self
                .core
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closing = true;
            drop(state.cached.take());
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
        live: state.live.len() as u64 + u64::from(state.inline.is_some()),
        // IDs and admission instants both increase under the gate mutex.
        oldest_age: state
            .inline
            .as_ref()
            .map(|(_, instant)| instant)
            .into_iter()
            .chain(state.live.first_key_value().map(|(_, instant)| instant))
            .min()
            .map(|stamp| stamp.elapsed()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::store::candidate::Prepared;
    use crate::storage::store::host::{AttachmentChange, HostChanges};
    use crate::storage::store::store_env::{CloseReport, Store};
    use crate::storage::store::tests::{
        AdmitAll, FirstFieldKey, NOTE, cancel_after, change_set, commit_changes, create_default,
        host_put, note, open_default, schema, store_dir, tiny_map, work,
    };

    fn cached_id(store: &Store) -> Option<usize> {
        store
            .inner
            .gate
            .core
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .cached
            .as_ref()
            .map(|reader| reader.txn.id())
    }

    #[test]
    fn a_reused_reader_has_a_fresh_pass_age_shared_with_gate_diagnostics() {
        let (_dir, path) = store_dir("gate-pass-age");
        let store = create_default(&path);
        let gate = &store.inner.gate;
        let (mut pass, cached) = gate.enter_read(&work()).unwrap();
        assert!(cached.is_none());
        let txn = store.inner.env.clone().static_read_txn().unwrap();
        let txn_id = txn.id();
        // Age the first admission deterministically, without waiting. The
        // native transaction may survive this pass, but its age must not.
        let earlier = AdmissionStamp::now()
            .earlier_by(Duration::from_secs(10))
            .unwrap();
        pass.admitted = earlier;
        gate.core.state.lock().unwrap().inline = Some((pass.id, earlier));
        let lease = ReadLease::new(CachedRead::new(txn, GenerationId::initial()), pass);
        assert!(lease.age() >= Duration::from_secs(10));
        drop(lease);
        let before = AdmissionStamp::now();
        let (pass, cached) = gate.enter_read(&work()).unwrap();
        let reader = cached.expect("same parked reader");
        assert_eq!(reader.txn.id(), txn_id);
        assert!(pass.admitted >= before);
        assert_eq!(
            gate.core.state.lock().unwrap().inline,
            Some((pass.id, pass.admitted)),
        );
        drop(ReadLease::new(reader, pass));
        assert_eq!(gate.live().live, 0);
    }

    #[test]
    fn quiescent_reads_reuse_one_slot_without_counting_it_as_a_live_borrow() {
        let (_dir, path) = store_dir("gate-read-reuse");
        let store = create_default(&path);
        let first = store.snapshot(&work()).unwrap();
        let id = first.read_txn().id();
        let second = store.snapshot(&work()).unwrap();
        assert_eq!(store.inner.gate.live().live, 2);
        assert_eq!(cached_id(&store), None);
        drop(first);
        assert_eq!(cached_id(&store), Some(id));
        let third = store.snapshot(&work()).unwrap();
        assert_eq!(
            cached_id(&store),
            None,
            "admission consumes the cached slot"
        );
        assert_eq!(third.read_txn().id(), id);
        drop(second);
        assert_eq!(cached_id(&store), Some(id));
        std::thread::scope(|scope| {
            scope.spawn(move || drop(third)).join().unwrap();
        });
        assert_eq!(cached_id(&store), Some(id));
        assert_eq!(store.inner.gate.live().live, 0);
        assert!(store.inner.gate.live().oldest_age.is_none());
    }

    #[test]
    fn readers_before_and_during_a_write_cannot_repopulate_a_stale_cache() {
        let (_dir, path) = store_dir("gate-read-fence");
        let store = create_default(&path);
        commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, note(1, "old"))], &[]),
        );
        let before = store.snapshot(&work()).unwrap();
        let old_id = before.read_txn().id();
        drop(store.snapshot(&work()).unwrap());
        assert_eq!(cached_id(&store), Some(old_id));
        let context = work();
        let mut owner = store.writer(&context).unwrap();
        let changes = change_set(&schema(), &[(NOTE, note(2, "new"))], &[]);
        let prepared = match owner.prepare(&changes, &FirstFieldKey, &AdmitAll).unwrap() {
            Prepared::Admitted(prepared) => prepared,
            Prepared::Rejected(never) => match never {},
        };
        assert_eq!(
            cached_id(&store),
            None,
            "writer admission aborts the idle txn"
        );
        let during = store.snapshot(&context).unwrap();
        assert_eq!(during.read_txn().id(), old_id);
        let records = host_put(b"receipt", b"committed");
        let committed = prepared
            .seal(HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(b"new attachment"),
            })
            .unwrap()
            .commit()
            .unwrap();
        drop(owner);
        assert_eq!(before.row_count(NOTE).unwrap(), 1);
        assert_eq!(during.row_count(NOTE).unwrap(), 1);
        drop(before);
        assert_eq!(
            cached_id(&store),
            None,
            "pre-write read cannot park after commit"
        );
        let fresh = store.snapshot(&context).unwrap();
        let new_id = fresh.read_txn().id();
        assert!(new_id > old_id);
        assert_eq!(fresh.generation(), committed.generation);
        assert_eq!(fresh.row_count(NOTE).unwrap(), 2);
        assert_eq!(
            fresh.attachment().unwrap(),
            Some(b"new attachment".as_slice())
        );
        assert_eq!(
            fresh.host_record(b"receipt").unwrap(),
            Some(b"committed".as_slice())
        );
        drop(during);
        assert_eq!(
            cached_id(&store),
            None,
            "read admitted during writer cannot park"
        );
        drop(fresh);
        assert_eq!(cached_id(&store), Some(new_id));
    }

    #[test]
    fn metadata_only_noop_and_aborted_writes_all_end_the_reuse_epoch() {
        let (_dir, path) = store_dir("gate-host-fence");
        let store = create_default(&path);
        let old = store.snapshot(&work()).unwrap();
        let generation = old.generation();
        drop(store.snapshot(&work()).unwrap());
        let context = work();
        let mut owner = store.writer(&context).unwrap();
        let committed = owner
            .prepare_unchanged()
            .unwrap()
            .seal(HostChanges {
                records: &host_put(b"receipt", b"host-only"),
                attachment: AttachmentChange::Put(b"host-only"),
            })
            .unwrap()
            .commit()
            .unwrap();
        assert!(committed.generation > generation);
        let fresh = store.snapshot(&context).unwrap();
        let id = fresh.read_txn().id();
        assert_eq!(fresh.generation(), committed.generation);
        assert_eq!(fresh.row_count(NOTE).unwrap(), 0);
        assert_eq!(fresh.attachment().unwrap(), Some(b"host-only".as_slice()));
        assert_eq!(
            fresh.host_record(b"receipt").unwrap(),
            Some(b"host-only".as_slice())
        );
        drop(fresh);
        drop(old);
        assert_eq!(
            cached_id(&store),
            Some(id),
            "late old return cannot replace fresh cache"
        );
        let noop = owner.prepare_unchanged().unwrap();
        assert_eq!(cached_id(&store), None);
        let noop = noop
            .seal(HostChanges {
                records: &[],
                attachment: AttachmentChange::Keep,
            })
            .unwrap()
            .commit()
            .unwrap();
        assert!(!noop.changed);
        assert_eq!(noop.generation, committed.generation);
        let after_noop = store.snapshot(&context).unwrap();
        let after_noop_id = after_noop.read_txn().id();
        drop(after_noop);
        let aborted = owner.prepare_unchanged().unwrap();
        assert_eq!(cached_id(&store), None);
        drop(store.snapshot(&context).unwrap());
        assert_eq!(
            cached_id(&store),
            None,
            "a return during a writer cannot park"
        );
        let during = store.snapshot(&context).unwrap();
        aborted.abort();
        drop(during);
        assert_eq!(cached_id(&store), None);
        drop(store.snapshot(&context).unwrap());
        assert_eq!(cached_id(&store), Some(after_noop_id));
    }

    #[test]
    fn resize_and_close_evict_idle_reads_but_never_invalidate_caller_reads() {
        let (_dir, path) = store_dir("gate-read-resize-close");
        let store = Store::create(&path, &schema(), tiny_map()).unwrap().0;
        drop(store.snapshot(&work()).unwrap());
        let grown = store.grow(&work(), None).unwrap();
        assert!(grown.new_map_bytes > grown.old_map_bytes);
        assert_eq!(cached_id(&store), None);
        let held = store.snapshot(&work()).unwrap();
        drop(store.snapshot(&work()).unwrap());
        let stopped = work();
        stopped.cancel();
        assert!(matches!(
            store.grow(&stopped, None),
            Err(StoreError::Work(_))
        ));
        assert_eq!(cached_id(&store), None);
        assert_eq!(held.row_count(NOTE).unwrap(), 0);
        drop(store.snapshot(&work()).unwrap());
        assert!(matches!(
            store.close(&stopped),
            CloseReport::Incomplete {
                live_transactions: 1,
                ..
            }
        ));
        assert_eq!(cached_id(&store), None);
        drop(held);
        assert_eq!(
            cached_id(&store),
            None,
            "late return cannot repopulate a closing store"
        );
        assert_eq!(store.close(&work()), CloseReport::Closed);
        assert!(matches!(store.snapshot(&work()), Err(StoreError::Closed)));
        drop(store);
        drop(open_default(&path));
    }

    #[test]
    fn unwinding_and_the_final_snapshot_release_environment_before_directory_lock() {
        let (_dir, path) = store_dir("gate-read-unwind");
        let store = create_default(&path);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _snapshot = store.snapshot(&work()).unwrap();
                panic!("abort a read operation");
            }))
            .is_err()
        );
        assert_eq!(store.inner.gate.live().live, 0);
        assert!(cached_id(&store).is_some());
        let snapshot = store.snapshot(&work()).unwrap();
        drop(store);
        assert!(matches!(
            Store::open(&path, &schema(), tiny_map()),
            Err(StoreError::StoreLocked { .. })
        ));
        drop(snapshot);
        // A cached txn must be destroyed before the kernel lock releases,
        // including when the snapshot was the final StoreInner owner.
        let reopened = open_default(&path);
        drop(reopened.snapshot(&work()).unwrap());
        drop(reopened);
        drop(open_default(&path));
    }

    #[test]
    fn failed_capture_releases_admission_and_does_not_cache_an_invalid_generation() {
        use crate::storage::store::format::K_GENERATION;
        let (_dir, path) = store_dir("gate-read-invalid-generation");
        let store = create_default(&path);
        let generation = store.snapshot(&work()).unwrap().generation();
        let mut txn = store.gated_write_txn(&work()).unwrap();
        store
            .inner
            .meta
            .put(&mut txn.txn, K_GENERATION, b"bad")
            .unwrap();
        txn.commit().unwrap();
        assert!(matches!(
            store.snapshot(&work()),
            Err(StoreError::Corruption(_))
        ));
        assert_eq!(store.inner.gate.live().live, 0);
        assert_eq!(cached_id(&store), None);
        let mut txn = store.gated_write_txn(&work()).unwrap();
        store
            .inner
            .meta
            .put(
                &mut txn.txn,
                K_GENERATION,
                &generation.storage_word().to_be_bytes(),
            )
            .unwrap();
        txn.commit().unwrap();
        assert_eq!(store.snapshot(&work()).unwrap().generation(), generation);
    }

    #[test]
    fn stopped_work_does_not_consume_a_cached_read_or_admit_any_pass() {
        let (_dir, path) = store_dir("gate-read-stopped-work");
        let store = create_default(&path);
        drop(store.snapshot(&work()).unwrap());
        let id = cached_id(&store).unwrap();
        let cancelled = work();
        cancelled.cancel();
        let expired = cancel_after(Duration::ZERO);
        for stopped in [&cancelled, &expired] {
            assert!(matches!(store.snapshot(stopped), Err(StoreError::Work(_))));
            assert!(matches!(
                store.gated_write_txn(stopped),
                Err(StoreError::Work(_))
            ));
            assert_eq!(cached_id(&store), Some(id));
            assert_eq!(store.inner.gate.live().live, 0);
        }
        assert_eq!(store.snapshot(&work()).unwrap().read_txn().id(), id);
    }

    #[test]
    fn failed_writer_begin_and_poisoned_mutex_preserve_gate_fences() {
        let gate = TransactionGate::default();
        let context = work();
        let writer = gate.enter_write(&context).unwrap();
        assert!(matches!(
            gate.enter_write(&context),
            Err(StoreError::ReentrantWriter)
        ));
        let during = gate.enter(&context).unwrap();
        // The same drop path runs if heed refuses write_txn after admission.
        drop(writer);
        {
            let state = gate.core.state.lock().unwrap();
            assert!(!state.writer_live);
            assert!(during.id < state.read_fence);
        }
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _state = gate.core.state.lock().unwrap();
                panic!("poison the admission mutex");
            }))
            .is_err()
        );
        drop(during);
        drop(gate.enter_write(&context).unwrap());
        assert_eq!(gate.live().live, 0);
        drop(gate.exclusive(&context).unwrap());
        gate.core
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .next_pass = u64::MAX;
        assert!(matches!(
            gate.enter_write(&context),
            Err(StoreError::Closed)
        ));
        assert_eq!(gate.live().live, 0);
    }

    #[test]
    fn returning_a_read_during_drain_aborts_instead_of_parking() {
        let (_dir, path) = store_dir("gate-read-return-drain");
        let store = create_default(&path);
        let held = store.snapshot(&work()).unwrap();
        // Freeze the gate at the exclusive drain transition, before the
        // last borrower returns. No resize happens until it has drained.
        store.inner.gate.core.state.lock().unwrap().draining = true;
        drop(held);
        assert_eq!(store.inner.gate.live().live, 0);
        assert_eq!(cached_id(&store), None);
        store.inner.gate.core.state.lock().unwrap().draining = false;
        store.grow(&work(), None).unwrap();
        drop(store.snapshot(&work()).unwrap());
        assert!(cached_id(&store).is_some());
    }

    #[test]
    fn serial_passes_never_populate_the_overflow_map() {
        let gate = TransactionGate::default();
        let work = work();
        for _ in 0..100 {
            let pass = gate.enter(&work).expect("enter");
            let state = gate.core.state.lock().expect("state");
            assert_eq!(state.inline.map(|(id, _)| id), Some(pass.id));
            assert!(state.live.is_empty());
            assert_eq!(snapshot_of(&state).live, 1);
            drop(state);
            drop(pass);
            assert_eq!(gate.live().live, 0);
        }
        assert!(gate.live().oldest_age.is_none());
    }

    #[test]
    fn reused_inline_slot_does_not_hide_older_overflow_passes() {
        let gate = TransactionGate::default();
        let work = work();
        let first = gate.enter(&work).expect("first");
        let second = gate.enter(&work).expect("second");
        let third = gate.enter(&work).expect("third");
        drop(first);
        let replacement = gate.enter(&work).expect("reuse inline");
        {
            let mut state = gate.core.state.lock().expect("state");
            // Distinct synthetic instants exercise oldest-age selection
            // without sleeping or relying on host timer resolution.
            let older = AdmissionStamp::now()
                .earlier_by(Duration::from_secs(10))
                .expect("test instant has ten seconds of history");
            *state.live.get_mut(&second.id).expect("second in map") = older;
            let snapshot = snapshot_of(&state);
            assert_eq!(snapshot.live, 3);
            assert!(snapshot.oldest_age.expect("oldest") >= Duration::from_secs(10));
            assert_eq!(state.inline.map(|(id, _)| id), Some(replacement.id));
        }
        drop(third);
        drop(replacement);
        assert_eq!(gate.live().live, 1);
        drop(second);
        assert_eq!(gate.live().live, 0);
        let exclusive = gate.exclusive(&work).expect("fully drained");
        drop(exclusive);
        drop(gate.enter(&work).expect("admission restored"));
    }

    #[test]
    fn close_tracks_both_inline_and_overflow_passes_and_stays_terminal() {
        let gate = TransactionGate::default();
        let work = work();
        let inline = gate.enter(&work).expect("inline");
        let overflow = gate.enter(&work).expect("overflow");
        let stopped = crate::storage::store::tests::work();
        stopped.cancel();
        let (closed, snapshot) = gate.begin_close(&stopped);
        assert!(!closed);
        assert_eq!(snapshot.live, 2);
        assert!(matches!(gate.enter(&work), Err(StoreError::Closed)));
        drop(inline);
        assert_eq!(gate.begin_close(&stopped).1.live, 1);
        drop(overflow);
        assert_eq!(
            gate.begin_close(&work),
            (
                true,
                GateSnapshot {
                    live: 0,
                    oldest_age: None,
                }
            )
        );
        assert!(matches!(gate.enter(&work), Err(StoreError::Closed)));
        assert!(matches!(gate.exclusive(&work), Err(StoreError::Closed)));
    }
}
