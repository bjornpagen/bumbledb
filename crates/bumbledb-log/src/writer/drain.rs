//! Group commit: one drain packs the per-braid queue into one
//! discipline run; a composite rejection falls back one-by-one.

use std::sync::Arc;
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

use bumbledb::Violations;

use crate::braids::BraidId;
use crate::codec::{ByteSink, Op, append_value};

use super::{
    Core, DRAIN_MAX_BYTES, DRAIN_MAX_WRITES, Durability, Error, Inner, Live, ObjectStore, Result,
    Settled, StepHook, Theory, lock,
};

/// The detached publisher's result — a value the writer must consume.
/// Standing down is not a swallow: another drain already resolved
/// these bytes. `Ran` is `resolve_backlog`'s own outcome.
#[must_use = "the writer consumes the publisher result"]
enum Publisher {
    StoodDown,
    Ran(Result<()>),
}

impl Publisher {
    fn consume(self) {
        match self {
            Self::StoodDown | Self::Ran(Ok(())) => {}
            Self::Ran(Err(error)) => {
                eprintln!("bumbledb-log: detached publisher: {error}");
            }
        }
    }
}

#[derive(Clone)]
pub(crate) enum Resolved {
    Accepted {
        braid: BraidId,
        generation: u64,
        durability: Durability,
    },
    Rejected(Violations),
    Failed(Arc<Error>),
}

pub(crate) struct Waiter {
    pub(crate) slot: Mutex<Option<Resolved>>,
    pub(crate) cv: Condvar,
}

impl Waiter {
    pub(crate) fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            cv: Condvar::new(),
        }
    }

    pub(crate) fn resolve(&self, resolved: Resolved) {
        let mut slot = lock(&self.slot);
        if slot.is_none() {
            *slot = Some(resolved);
            self.cv.notify_all();
        }
    }

    pub(crate) fn get(&self) -> Option<Resolved> {
        lock(&self.slot).clone()
    }

    pub(crate) fn wait_briefly(&self) {
        let slot = lock(&self.slot);
        if slot.is_none() {
            let _ = self
                .cv
                .wait_timeout(slot, Duration::from_millis(1))
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

pub(crate) struct Request {
    pub(crate) ops: Vec<Op>,
    pub(crate) rows: u64,
    pub(crate) bytes: u64,
    pub(crate) waiter: Arc<Waiter>,
}
pub(crate) struct CountSink(pub(crate) u64);

impl ByteSink for CountSink {
    fn put(&mut self, bytes: &[u8]) {
        self.0 += bytes.len() as u64;
    }
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    pub(crate) fn measure(&self, ops: &[Op]) -> (u64, u64) {
        let mut rows = 0u64;
        let mut sink = CountSink(0);
        for op in ops {
            sink.0 += 9;
            let layout = self.maps.layouts.get(&op.relation);
            for row in &op.rows {
                rows += 1;
                if let Some(layout) = layout {
                    for (value, ty) in row.iter().zip(layout.iter()) {
                        let _ = append_value(&mut sink, value, *ty);
                    }
                }
            }
        }
        (rows, sink.0)
    }

    /// The per-braid drain: snapshot the queue up to the packing caps,
    /// resolve any retained backlog first, then run one composite
    /// discipline — one batch, one generation, one transaction by law.
    /// A composite rejection falls back one-by-one in queue order so an
    /// innocent write never fails for a neighbor's violation.
    pub(crate) fn drain(self: &Arc<Self>, core: &mut Core<T>, braid: BraidId) {
        self.reap();
        let mut picked: Vec<Request> = Vec::new();
        {
            let mut queue = lock(&self.queues[&braid]);
            let mut rows = 0u64;
            let mut bytes = 0u64;
            while let Some(front) = queue.front() {
                if !picked.is_empty()
                    && (rows + front.rows > DRAIN_MAX_WRITES
                        || bytes + front.bytes > DRAIN_MAX_BYTES)
                {
                    break;
                }
                rows += front.rows;
                bytes += front.bytes;
                picked.push(queue.pop_front().expect("front just peeked"));
            }
        }
        if picked.is_empty() {
            return;
        }
        let fail_all = |requests: &[Request], error: Error| {
            let shared = Arc::new(error);
            for request in requests {
                request
                    .waiter
                    .resolve(Resolved::Failed(Arc::clone(&shared)));
            }
        };
        if core.wedged.contains_key(&braid) {
            fail_all(&picked, Error::Wedged { braid });
            return;
        }
        if core.chain.pending.is_some()
            && let Err(error) = self.resolve_backlog(core, None, &mut Live::default())
        {
            fail_all(&picked, error);
            return;
        }

        let composite: Vec<Op> = picked
            .iter()
            .flat_map(|request| request.ops.iter().cloned())
            .collect();
        let waiters: Vec<Arc<Waiter>> = picked
            .iter()
            .map(|request| Arc::clone(&request.waiter))
            .collect();
        match self.discipline(
            core,
            braid,
            &composite,
            &mut Live::default(),
            Some(&waiters),
        ) {
            Ok(Settled::Accepted { generation }) => {
                for waiter in &waiters {
                    waiter.resolve(Resolved::Accepted {
                        braid,
                        generation,
                        durability: Durability::Published,
                    });
                }
            }
            Ok(Settled::Rejected(violations)) => {
                if picked.len() == 1 {
                    picked[0].waiter.resolve(Resolved::Rejected(violations));
                } else {
                    self.fallback(core, braid, &picked);
                }
            }
            Ok(Settled::Detached { bytes }) => {
                let segments: Vec<Vec<Op>> =
                    picked.iter().map(|request| request.ops.clone()).collect();
                self.spawn_publisher(braid, bytes, segments);
            }
            Err(error) => fail_all(&picked, error),
        }
    }

    /// One-by-one fallback for a rejected composite, each caller as its
    /// own transaction in queue order. Waiters a local ack already
    /// resolved keep their `LocalPending` answer — the honest arm.
    pub(crate) fn fallback(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        requests: &[Request],
    ) {
        for request in requests {
            match self.discipline(core, braid, &request.ops, &mut Live::default(), None) {
                Ok(Settled::Accepted { generation }) => {
                    request.waiter.resolve(Resolved::Accepted {
                        braid,
                        generation,
                        durability: Durability::Published,
                    });
                }
                Ok(Settled::Rejected(violations)) => {
                    request.waiter.resolve(Resolved::Rejected(violations));
                }
                Ok(Settled::Detached { .. }) => {
                    unreachable!("fallback passes no waiters, so no ack can detach")
                }
                Err(error) => {
                    request.waiter.resolve(Resolved::Failed(Arc::new(error)));
                }
            }
        }
    }

    /// The detached publisher: acks moved to the end of the local
    /// apply, so publication continues off the caller. Keyed by the
    /// pending bytes — if another drain resolved the backlog first, the
    /// publisher finds different bytes and stands down. The result is a
    /// `#[must_use]` value; the writer consumes it.
    pub(crate) fn spawn_publisher(
        self: &Arc<Self>,
        _braid: BraidId,
        bytes: Vec<u8>,
        segments: Vec<Vec<Op>>,
    ) {
        let inner = Arc::clone(self);
        let handle = std::thread::spawn(move || inner.publisher(&bytes, &segments).consume());
        self.reap();
        lock(&self.threads).push(handle);
    }

    /// Finished publisher and duty handles leave the vector here, on
    /// the drain, not only at `quiesce` — a writer's lifetime does not
    /// accumulate `JoinHandle`s.
    fn reap(&self) {
        let mut threads = lock(&self.threads);
        for handle in threads.extract_if(.., std::thread::JoinHandle::is_finished) {
            let _ = handle.join();
        }
    }

    fn publisher(self: &Arc<Self>, bytes: &[u8], segments: &[Vec<Op>]) -> Publisher {
        let mut core = lock(&self.core);
        let matches = core
            .chain
            .pending
            .as_ref()
            .is_some_and(|pending| pending.bytes == bytes);
        if matches {
            Publisher::Ran(self.resolve_backlog(&mut core, Some(segments), &mut Live::default()))
        } else {
            Publisher::StoodDown
        }
    }
}
