//! Group commit: one loop per braid, concurrent commits partition and
//! queue, a drain packs into ONE batch and one transaction by law —
//! the composite may accept what solo runs would reject — and a
//! composite rejection falls back one-by-one in queue order. Packing
//! is forced deterministically by parking an unrelated commit inside
//! its slot PUT: the core stays busy, the callers queue behind it, and
//! the next drain picks them together.

mod lane_e_support;

use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bumbledb::{Admission, SchemaDescriptor};
use bumbledb_log::manifest::log_key;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{
    Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result as StoreResult, StoreKey, Swap,
};
use bumbledb_log::writer::{Options, Slotted, Writer, WriterOpened};
use lane_e_support::{
    NOTE, RECIPE, STEP, codec, kitchen_braid, note_row, recipe_row, step_row, temp_dir, theory,
};

/// Parks the first log-slot create until the gate opens, holding the
/// commit core busy while other callers queue.
struct HoldFirstPut {
    inner: FsStore,
    gate: std::sync::Arc<AtomicBool>,
    tripped: AtomicBool,
}

impl HoldFirstPut {
    fn new(root: std::path::PathBuf) -> (Self, std::sync::Arc<AtomicBool>) {
        let gate = std::sync::Arc::new(AtomicBool::new(false));
        (
            Self {
                inner: FsStore::new(root),
                gate: std::sync::Arc::clone(&gate),
                tripped: AtomicBool::new(false),
            },
            gate,
        )
    }
}

impl ObjectStore for HoldFirstPut {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        if key.as_str().starts_with("log/") && !self.tripped.swap(true, Ordering::SeqCst) {
            while !self.gate.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        self.inner.put_create(key, body)
    }

    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> StoreResult<Swap> {
        self.inner.put_swap(key, body, etag)
    }

    fn delete(&self, key: &StoreKey) -> StoreResult<()> {
        self.inner.delete(key)
    }
}

type GatedWriter = Writer<SchemaDescriptor, HoldFirstPut>;

fn open_gated(
    root: std::path::PathBuf,
    dir: &std::path::Path,
) -> (GatedWriter, std::sync::Arc<AtomicBool>) {
    let (store, gate) = HoldFirstPut::new(root);
    let writer = match Writer::open(store, "", dir, theory(), Options::new(51)).expect("open") {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    (writer, gate)
}

/// Runs `holder` on one thread (it parks in its slot PUT), queues the
/// two kitchen callers behind it, opens the gate, and returns both
/// kitchen outcomes.
fn packed_pair(
    writer: &GatedWriter,
    gate: &AtomicBool,
    first: impl FnOnce() -> bumbledb_log::writer::Result<Admission<Slotted<()>>> + Send,
    second: impl FnOnce() -> bumbledb_log::writer::Result<Admission<Slotted<()>>> + Send,
) -> (Admission<Slotted<()>>, Admission<Slotted<()>>) {
    let start = Barrier::new(2);
    std::thread::scope(|scope| {
        let holder = scope.spawn(|| {
            start.wait();
            writer.commit(|batch| {
                batch.insert(NOTE, [note_row(9_000, "hold the core")]);
                Ok(())
            })
        });
        start.wait();
        // The holder reaches its slot PUT and parks with the core held.
        std::thread::sleep(Duration::from_millis(50));
        let first_task = scope.spawn(first);
        let second_task = scope.spawn(second);
        // Both callers queue behind the busy core before the gate opens.
        std::thread::sleep(Duration::from_millis(150));
        gate.store(true, Ordering::SeqCst);
        let hold = holder.join().expect("join holder").expect("holder commit");
        assert!(matches!(hold, Admission::Accepted(_)));
        (
            first_task.join().expect("join").expect("commit"),
            second_task.join().expect("join").expect("commit"),
        )
    })
}

#[test]
fn a_drain_packs_concurrent_commits_into_one_transaction() {
    let root = temp_dir("pack");
    let dir = root.join("w");
    let (writer, gate) = open_gated(root.clone(), &dir);

    let (step_outcome, recipe_outcome) = packed_pair(
        &writer,
        &gate,
        || {
            writer.commit(|batch| {
                batch.insert(STEP, [step_row(7, "mix")]);
                Ok(())
            })
        },
        || {
            writer.commit(|batch| {
                batch.insert(RECIPE, [recipe_row(7, "cake")]);
                Ok(())
            })
        },
    );

    // The step alone would reject (no recipe 7); the drain is one
    // transaction, so the engine judges the composite's final state.
    let Admission::Accepted(Slotted { slot: s1, .. }) = step_outcome else {
        panic!("the composite accepted what a solo run would reject");
    };
    let Admission::Accepted(Slotted { slot: s2, .. }) = recipe_outcome else {
        panic!("accepted expected");
    };
    assert_eq!(s1, 1);
    assert_eq!(s2, 1, "one batch, one slot, one object");

    let codec = codec();
    let braid = kitchen_braid(&codec);
    let store = FsStore::new(root);
    let slot = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("one slot");
    let batch = codec.decode(&slot.bytes).expect("decode");
    assert_eq!(batch.ops.len(), 2, "both callers packed into one batch");
    assert!(store.get(&log_key("", braid, 2)).expect("get").is_none());
}

#[test]
fn a_rejected_composite_falls_back_one_by_one() {
    let root = temp_dir("fallback");
    let dir = root.join("w");
    let (writer, gate) = open_gated(root.clone(), &dir);

    let (guilty, innocent) = packed_pair(
        &writer,
        &gate,
        || {
            writer.commit(|batch| {
                batch.insert(STEP, [step_row(99, "orphan")]);
                Ok(())
            })
        },
        || {
            writer.commit(|batch| {
                batch.insert(RECIPE, [recipe_row(50, "pie")]);
                Ok(())
            })
        },
    );

    assert!(
        matches!(guilty, Admission::Rejected(_)),
        "the guilty write gets its own serial rejection"
    );
    assert!(
        matches!(innocent, Admission::Accepted(Slotted { slot: 1, .. })),
        "an innocent write never fails for a neighbor's violation"
    );
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let store = FsStore::new(root);
    let slot = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("the innocent write published alone");
    let batch = codec.decode(&slot.bytes).expect("decode");
    assert_eq!(batch.ops.len(), 1);
    assert_eq!(batch.ops[0].relation, RECIPE);
    writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(RECIPE, &recipe_row(50, "pie"))?);
                assert!(!instance.contains_dyn(STEP, &step_row(99, "orphan"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
}
