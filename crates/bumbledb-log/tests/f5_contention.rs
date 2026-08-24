//! Conformance F5 — contention under the one loss path. N in-process
//! writers over one `FsStore` prefix, each with its own replica
//! directory; the standing gates on every fixture: per-braid logs
//! gap-free with each slot created once, every `prev` hash verified,
//! every acked commit appearing exactly once, all replicas converging
//! on `catalog_digest`, and the wholeness identity
//! `generation == Σ vector + |pending|` asserted on every store — the
//! invariant the loss path must never bend. A loss whose effects the
//! winner already performed re-judges to the engine's net no-op and
//! lands `Accepted` at the current generation with nothing published;
//! a disjoint-shaped loss re-judges and publishes with a fresh
//! header at tip+1; a conflicting loss produces the serial verdict;
//! the ambiguous-outcome GET-verify law resolves injected response
//! drops; and both `Err::Contention` causes come from dedicated
//! livelock fixtures whose terminal re-judgment sources the payload.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, StatementId, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Value, Violation};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{Batch, BatchHeader, Codec, Op, OpKind};
use bumbledb_log::manifest::{Head, log_key};
use bumbledb_log::replica::{Opened, Replica};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{
    Create, Etag, Fetched, ObjectStore, Poll, Result as StoreResult, StoreError, StoreKey, Swap,
};
use bumbledb_log::writer::{
    Commit, ContentionCause, Durability, Error, Options, StepControl, StepHook, Writer,
    WriterOpened, WriterStep,
};

const RECIPE: RelationId = RelationId(0);
const STEP: RelationId = RelationId(1);
const NOTE: RelationId = RelationId(2);
const VENUE: RelationId = RelationId(3);
const BOOKING: RelationId = RelationId(4);

const BOOKING_CAPACITY: StatementId = StatementId(3);

/// The venue capacity ceiling: tight enough that fixtures can fill it
/// exactly and race the remaining slack.
const CEILING: u64 = 1_000;

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("f5_{tag}_{}_{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Three braids: recipe+step (key + containment), note alone, and
/// venue+booking under a tight capacity ceiling — the hot-key and
/// hot-capacity-parent adversaries live on the first and third; the
/// mostly-disjoint fleets live on the second.
fn theory() -> SchemaDescriptor {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    let side = |relation: RelationId, fields: &[u16]| Side {
        relation,
        projection: fields.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::from([]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "recipe".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("title", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "step".into(),
                fields: vec![
                    field("recipe", ValueType::U64),
                    field("name", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "note".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("body", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "venue".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            },
            RelationDescriptor {
                name: "booking".into(),
                fields: vec![
                    field("venue", ValueType::U64),
                    field("units", ValueType::U64),
                ],
                extension: None,
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RECIPE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(STEP, &[0]),
                target: side(RECIPE, &[0]),
            },
            StatementDescriptor::Functionality {
                relation: VENUE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(VENUE, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 0,
                hi: Some(Bound::Lit(CEILING)),
                source: side(BOOKING, &[0]),
            },
        ],
    }
}

fn codec() -> Codec {
    let descriptor = theory();
    let schema = descriptor.clone().validate().expect("fixture validates");
    let fingerprint = schema_fingerprint(&schema).0;
    Codec::new(&descriptor, fingerprint)
}

fn kitchen_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(RECIPE).expect("recipe braid")
}

fn note_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(NOTE).expect("note braid")
}

fn venue_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(VENUE).expect("venue braid")
}

fn recipe_row(id: u64, title: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(title.into())])
}

fn step_row(recipe: u64, name: &str) -> Box<[Value]> {
    Box::from([Value::U64(recipe), Value::String(name.into())])
}

fn note_row(id: u64, body: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(body.into())])
}

fn booking_row(venue: u64, units: u64) -> Box<[Value]> {
    Box::from([Value::U64(venue), Value::U64(units)])
}

fn insert(relation: RelationId, row: Box<[Value]>) -> Op {
    Op {
        kind: OpKind::Insert,
        relation,
        rows: vec![row],
    }
}

fn open_writer<S: ObjectStore + 'static>(
    store: S,
    dir: &Path,
    writer_id: u64,
) -> Writer<SchemaDescriptor, S> {
    match Writer::open(store, "", dir, theory(), Options::new(writer_id)).expect("open writer") {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn open_at(root: PathBuf, dir: &Path, writer_id: u64) -> Writer<SchemaDescriptor, FsStore> {
    open_writer(FsStore::new(root), dir, writer_id)
}

/// The wholeness identity `generation == Σ vector + |pending|` — the
/// invariant the loss path must never bend, asserted on every
/// writer after every fixture.
fn assert_whole<S, H>(writer: &Writer<SchemaDescriptor, S, H>, context: &str)
where
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    let generation = writer.with_db(|db| db.generation().expect("generation").value());
    let sum: u64 = writer.vector().values().sum();
    let pending = u64::from(writer.backlog().is_some());
    assert_eq!(
        generation,
        sum + pending,
        "the wholeness identity on {context}"
    );
}

/// The standing gates on the published prefix: per-braid logs gap-free
/// (the on-disk slot names are exactly `1..=tip`, so each slot was
/// created once), every header passing the chain battery (slot
/// identity, `prev` hash, monotone timestamp). Returns the decoded
/// batches per braid.
fn verify_log(root: &Path) -> BTreeMap<BraidId, Vec<Batch>> {
    let codec = codec();
    let store = FsStore::new(root.to_path_buf());
    let mut decoded: BTreeMap<BraidId, Vec<Batch>> = BTreeMap::new();
    for braid in codec.braids().components().keys() {
        let dir = root.join("log").join(braid.to_string());
        let mut slots: Vec<u64> = Vec::new();
        if dir.exists() {
            for entry in std::fs::read_dir(&dir).expect("list log dir") {
                let name = entry.expect("dir entry").file_name();
                let name = name.to_str().expect("slot names are ascii");
                if name.len() != 16 {
                    // Lock sidecars and unpublished temps; a slot name
                    // is exactly sixteen hex digits.
                    continue;
                }
                slots.push(u64::from_str_radix(name, 16).expect("slot name is hex"));
            }
        }
        slots.sort_unstable();
        let tip = u64::try_from(slots.len()).expect("tip fits u64");
        assert_eq!(
            slots,
            (1..=tip).collect::<Vec<u64>>(),
            "log gap-free with each slot created once on {braid}"
        );
        let mut prev = [0u8; 32];
        let mut prev_ts = 0u64;
        let mut batches = Vec::new();
        for slot in 1..=tip {
            let fetched = store
                .get(&log_key("", *braid, slot))
                .expect("get slot")
                .expect("walked slot exists");
            let batch = codec.decode(&fetched.bytes).expect("decode slot");
            assert_eq!(batch.header.braid, *braid, "header braid identity");
            assert_eq!(batch.header.braid_gen, slot, "header slot identity");
            assert_eq!(
                batch.header.prev, prev,
                "prev hash verified at {braid}/{slot}"
            );
            assert!(
                batch.header.timestamp >= prev_ts,
                "monotone timestamps at {braid}/{slot}"
            );
            prev = *blake3::hash(&fetched.bytes).as_bytes();
            prev_ts = batch.header.timestamp;
            batches.push(batch);
        }
        decoded.insert(*braid, batches);
    }
    decoded
}

/// Convergence: a fresh replica replays the whole prefix under apply's
/// own battery (chain discipline, publish-law instrument), lands
/// whole, and reports its catalog digest.
fn converged_digest(root: &Path) -> [u8; 32] {
    let dir = temp_dir("verify_replica");
    let opened = Replica::open(
        FsStore::new(root.to_path_buf()),
        "",
        &dir.join("r"),
        theory(),
    )
    .expect("open verifying replica");
    let Opened::Ready(replica) = opened else {
        panic!("verifying replica refused");
    };
    assert!(
        replica.wedged().is_empty(),
        "no corruption-class refusal anywhere in the log"
    );
    let generation = replica.db().generation().expect("generation").value();
    let sum: u64 = replica.vector().values().sum();
    assert_eq!(
        generation, sum,
        "the wholeness identity on the verifying replica"
    );
    replica.db().catalog_digest().expect("catalog digest")
}

fn writer_digest<S, H>(writer: &Writer<SchemaDescriptor, S, H>) -> [u8; 32]
where
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    writer.with_db(|db| db.catalog_digest().expect("catalog digest"))
}

/// A test-side publisher with its own chain state, for planting
/// competitor and adversarial slots without a second writer.
struct TestPublisher {
    store: FsStore,
    codec: Codec,
    heads: BTreeMap<BraidId, Head>,
    writer: u64,
}

impl TestPublisher {
    fn attach(root: PathBuf) -> Self {
        let codec = codec();
        let heads = codec
            .braids()
            .components()
            .keys()
            .map(|braid| {
                (
                    *braid,
                    Head {
                        g: 0,
                        hash: [0u8; 32],
                        ts: 0,
                    },
                )
            })
            .collect();
        Self {
            store: FsStore::new(root),
            codec,
            heads,
            writer: 9_001,
        }
    }

    fn encode(&self, braid: BraidId, ops: &[Op], ts: u64) -> (u64, Vec<u8>) {
        let head = self.heads[&braid];
        let header = BatchHeader {
            fingerprint: *self.codec.fingerprint(),
            braid,
            braid_gen: head.g + 1,
            prev: head.hash,
            writer: self.writer,
            timestamp: ts.max(head.ts),
        };
        let bytes = self
            .codec
            .encode(&header, ops)
            .expect("encode fixture batch");
        (head.g + 1, bytes)
    }

    fn publish_bytes(&mut self, braid: BraidId, slot: u64, bytes: &[u8], ts: u64) {
        let key = log_key("", braid, slot);
        assert!(matches!(
            self.store.put_create(&key, bytes).expect("publish slot"),
            Create::Created(_)
        ));
        let head = self.heads.get_mut(&braid).expect("known braid");
        head.g = slot;
        head.hash = *blake3::hash(bytes).as_bytes();
        head.ts = ts.max(head.ts);
    }

    fn publish(&mut self, braid: BraidId, ops: &[Op], ts: u64) -> u64 {
        let (slot, bytes) = self.encode(braid, ops, ts);
        self.publish_bytes(braid, slot, &bytes, ts);
        slot
    }
}

/// Crashes exactly once at the (allow+1)-th occurrence of `step`, then
/// disables itself so recovery runs clean.
struct CrashOnce {
    step: WriterStep,
    remaining: AtomicU64,
}

impl CrashOnce {
    fn new(step: WriterStep, allow: u64) -> Self {
        Self {
            step,
            remaining: AtomicU64::new(allow),
        }
    }
}

impl StepHook for CrashOnce {
    fn observe(&self, step: WriterStep) -> StepControl {
        if step != self.step {
            return StepControl::Continue;
        }
        if self
            .remaining
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
            .is_ok()
        {
            StepControl::Continue
        } else {
            self.remaining.store(u64::MAX, Ordering::SeqCst);
            StepControl::Crash
        }
    }
}

/// The injected ambiguous outcome: performs every store verb for real
/// but, while armed, drops the response of a log-slot `put_create` —
/// after any effect — behind an infrastructure `Err`. The GET-verify
/// law is the only sound resolution.
struct DropResponses {
    inner: FsStore,
    remaining: AtomicU64,
}

impl DropResponses {
    fn new(root: PathBuf, drops: u64) -> Self {
        Self {
            inner: FsStore::new(root),
            remaining: AtomicU64::new(drops),
        }
    }
}

impl ObjectStore for DropResponses {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> StoreResult<Create> {
        let created = self.inner.put_create(key, bytes)?;
        if key.as_str().starts_with("log/")
            && self
                .remaining
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
                .is_ok()
        {
            return Err(StoreError {
                op: "put_create",
                key: key.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "response dropped after effect",
                ),
            });
        }
        Ok(created)
    }

    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> StoreResult<Swap> {
        self.inner.put_swap(key, bytes, etag)
    }

    fn delete(&self, key: &StoreKey) -> StoreResult<()> {
        self.inner.delete(key)
    }
}

/// What the racing store plants at every contested slot: distinct
/// notes (fully disjoint — the `SlotRace` shape), or distinct bookings
/// under one shared venue parent, sized `base_units + seq` so the
/// fixture prices exactly when the ceiling convicts the loser's
/// terminal re-judgment (the `HotKey` shape).
#[derive(Clone, Copy)]
enum Competitor {
    Notes,
    Bookings { venue: u64, base_units: u64 },
}

struct PlanterState {
    braid: BraidId,
    remaining: AtomicU64,
    seq: AtomicU64,
    head: Mutex<Head>,
    kind: Competitor,
    codec: Codec,
}

/// Wraps `FsStore` and, while armed, wins every `put_create` on the
/// target braid's log keys by planting a chain-valid competitor batch
/// first — the deterministic livelock tool.
struct RacingStore {
    inner: FsStore,
    state: std::sync::Arc<PlanterState>,
}

struct PlanterHandle(std::sync::Arc<PlanterState>);

impl PlanterHandle {
    fn plants(&self) -> u64 {
        self.0.seq.load(Ordering::SeqCst)
    }

    fn arm(&self, plants: u64) {
        self.0.remaining.store(plants, Ordering::SeqCst);
    }

    /// Walks the actual log to the braid's tip so competitor batches
    /// chain onto slots published while the planter was disarmed.
    fn seed_from(&self, root: PathBuf) {
        let store = FsStore::new(root);
        let state = &self.0;
        let mut head = Head {
            g: 0,
            hash: [0u8; 32],
            ts: 0,
        };
        loop {
            let key = log_key("", state.braid, head.g + 1);
            let Some(fetched) = store.get(&key).expect("seed walk") else {
                break;
            };
            let batch = state.codec.decode(&fetched.bytes).expect("seed decode");
            head.g += 1;
            head.hash = *blake3::hash(&fetched.bytes).as_bytes();
            head.ts = batch.header.timestamp;
        }
        *state.head.lock().expect("planter head") = head;
    }
}

impl RacingStore {
    fn new(root: PathBuf, braid: BraidId, plants: u64, kind: Competitor) -> (Self, PlanterHandle) {
        let state = std::sync::Arc::new(PlanterState {
            braid,
            remaining: AtomicU64::new(plants),
            seq: AtomicU64::new(0),
            head: Mutex::new(Head {
                g: 0,
                hash: [0u8; 32],
                ts: 0,
            }),
            kind,
            codec: codec(),
        });
        (
            Self {
                inner: FsStore::new(root),
                state: std::sync::Arc::clone(&state),
            },
            PlanterHandle(state),
        )
    }

    fn maybe_plant(&self, key: &StoreKey) {
        let state = &self.state;
        let log_prefix = format!("log/{}/", state.braid);
        if !key.as_str().starts_with(&log_prefix) {
            return;
        }
        if state
            .remaining
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
            .is_err()
        {
            return;
        }
        let seq = state.seq.fetch_add(1, Ordering::SeqCst);
        let ops = match state.kind {
            Competitor::Notes => vec![insert(NOTE, note_row(20_000 + seq, "racer"))],
            // Distinct units per plant: no accidental row identity,
            // which would turn the loser's re-apply into a legitimate
            // net no-op.
            Competitor::Bookings { venue, base_units } => {
                vec![insert(BOOKING, booking_row(venue, base_units + seq))]
            }
        };
        let mut head = state.head.lock().expect("planter head");
        let header = BatchHeader {
            fingerprint: *state.codec.fingerprint(),
            braid: state.braid,
            braid_gen: head.g + 1,
            prev: head.hash,
            writer: 999,
            timestamp: head.ts.max(1),
        };
        let bytes = state
            .codec
            .encode(&header, &ops)
            .expect("encode competitor");
        let slot_key = log_key("", state.braid, head.g + 1);
        assert_eq!(
            &slot_key, key,
            "the planter plants exactly the contested slot"
        );
        assert!(matches!(
            self.inner
                .put_create(&slot_key, &bytes)
                .expect("plant competitor"),
            Create::Created(_)
        ));
        head.g += 1;
        head.hash = *blake3::hash(&bytes).as_bytes();
        head.ts = header.timestamp;
    }
}

impl ObjectStore for RacingStore {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> StoreResult<Create> {
        self.maybe_plant(key);
        self.inner.put_create(key, bytes)
    }

    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> StoreResult<Swap> {
        self.inner.put_swap(key, bytes, etag)
    }

    fn delete(&self, key: &StoreKey) -> StoreResult<()> {
        self.inner.delete(key)
    }
}

#[test]
fn disjoint_loss_rejudges_once_and_publishes_at_tip_plus_one() {
    let root = temp_dir("disjoint");
    let writer_a = open_at(root.clone(), &root.join("wa"), 1);
    let writer_b = open_at(root.clone(), &root.join("wb"), 2);
    let codec = codec();
    let braid = note_braid(&codec);

    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(NOTE, [note_row(1, "theirs")]);
                Ok(())
            })
            .expect("winner commit"),
        Commit::Accepted { generation: 1, .. }
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2, "ours")]);
            Ok(())
        })
        .expect("loser commit");
    let Commit::Accepted {
        generation,
        durability,
        ..
    } = outcome
    else {
        panic!("a disjoint loss lands");
    };
    assert_eq!(generation, 2, "the re-judged publish lands at tip+1");
    assert_eq!(durability, Durability::Published);
    assert_eq!(writer_b.losses(), 1, "one loss, one re-judgment");

    let batches = verify_log(&root);
    let slots = &batches[&braid];
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].header.writer, 1);
    assert_eq!(slots[1].header.writer, 2, "the fresh header is ours");
    assert!(
        slots[1].header.timestamp >= slots[0].header.timestamp,
        "timestamp clamped against the winner"
    );

    assert_whole(&writer_a, "the slot winner");
    assert_whole(&writer_b, "the publishing loser");
    assert_eq!(
        writer_digest(&writer_b),
        converged_digest(&root),
        "the re-opened loser applied in log order and converges byte-for-byte"
    );
    writer_b.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(1, "theirs"))?);
            assert!(instance.contains_dyn(NOTE, &note_row(2, "ours"))?);
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn identical_effects_race_lands_accepted_at_the_winners_generation() {
    let root = temp_dir("identical");
    let dir_b = root.join("wb");
    let writer_a = open_at(root.clone(), &root.join("wa"), 1);
    let writer_b = open_at(root.clone(), &dir_b, 2);
    let codec = codec();
    let braid = note_braid(&codec);

    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(NOTE, [note_row(7, "same")]);
                Ok(())
            })
            .expect("winner commit"),
        Commit::Accepted { generation: 1, .. }
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(7, "same")]);
            Ok(())
        })
        .expect("loser commit");
    let Commit::Accepted {
        generation,
        durability,
        ..
    } = outcome
    else {
        panic!("a loss whose effects the winner performed reports Accepted");
    };
    assert_eq!(generation, 1, "the winner's generation, not a new slot");
    assert_eq!(durability, Durability::Published);
    assert_eq!(writer_b.losses(), 1, "one loss, one re-judged net no-op");

    let batches = verify_log(&root);
    assert_eq!(batches[&braid].len(), 1, "the log never gains a no-op slot");
    assert_whole(&writer_a, "the winner");
    assert_whole(&writer_b, "the absorbed loser");
    assert_eq!(writer_digest(&writer_a), writer_digest(&writer_b));
    assert_eq!(writer_digest(&writer_b), converged_digest(&root));
}

#[test]
fn strict_superset_race_lands_accepted_with_the_residue_present() {
    let root = temp_dir("superset");
    let dir_b = root.join("wb");
    let writer_a = open_at(root.clone(), &root.join("wa"), 1);
    let writer_b = open_at(root.clone(), &dir_b, 2);
    let codec = codec();
    let braid = note_braid(&codec);

    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(NOTE, [note_row(11, "shared"), note_row(12, "residue")]);
                Ok(())
            })
            .expect("winner commit"),
        Commit::Accepted { generation: 1, .. }
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(11, "shared")]);
            Ok(())
        })
        .expect("loser commit");
    let Commit::Accepted { generation, .. } = outcome else {
        panic!("a strictly contained loss reports the winner's outcome");
    };
    assert_eq!(generation, 1, "accepted at the current generation");
    assert_eq!(writer_b.losses(), 1);

    writer_b.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(11, "shared"))?);
            assert!(
                instance.contains_dyn(NOTE, &note_row(12, "residue"))?,
                "the winner's residue is present after the re-open"
            );
            Ok(())
        })
        .expect("read");
    });
    let batches = verify_log(&root);
    assert_eq!(batches[&braid].len(), 1, "the log never gains a no-op slot");
    assert_whole(&writer_b, "the rebuilt loser");
    assert_eq!(writer_digest(&writer_b), converged_digest(&root));
}

#[test]
fn conflicting_loss_produces_the_serial_verdict() {
    let root = temp_dir("conflict");
    let writer_a = open_at(root.clone(), &root.join("wa"), 1);
    let writer_b = open_at(root.clone(), &root.join("wb"), 2);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(RECIPE, [recipe_row(1, "winner")]);
                Ok(())
            })
            .expect("winner commit"),
        Commit::Accepted { generation: 1, .. }
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(1, "loser")]);
            Ok(())
        })
        .expect("loser commit");
    let Commit::Rejected(violations) = outcome else {
        panic!("exactly the verdict serial execution would have produced");
    };
    assert!(
        violations
            .iter()
            .any(|violation| matches!(violation, Violation::Functionality { .. })),
        "the double-booking is refused with the typed FD violation"
    );
    assert_eq!(writer_b.losses(), 1, "one loss, one re-judgment");

    let batches = verify_log(&root);
    assert_eq!(batches[&braid].len(), 1, "a rejection publishes nothing");
    assert_whole(&writer_b, "the rejected loser");
    writer_b.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(RECIPE, &recipe_row(1, "winner"))?);
            assert!(!instance.contains_dyn(RECIPE, &recipe_row(1, "loser"))?);
            Ok(())
        })
        .expect("read");
    });
    assert_eq!(writer_digest(&writer_b), converged_digest(&root));
}

#[test]
fn evaporating_loss_rejudges_to_the_net_noop_and_publishes_nothing() {
    let root = temp_dir("evaporate");
    let writer_a = open_at(root.clone(), &root.join("wa"), 1);
    let codec = codec();
    let braid = note_braid(&codec);

    // The shared base carries the row the loser's second op will
    // evaporate against.
    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(NOTE, [note_row(3, "base")]);
                Ok(())
            })
            .expect("base commit"),
        Commit::Accepted { generation: 1, .. }
    ));
    let writer_b = open_at(root.clone(), &root.join("wb"), 2);
    assert_eq!(writer_b.vector()[&braid], 1);

    // The winner takes slot 2 with the shared insert plus residue.
    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(NOTE, [note_row(1, "shared"), note_row(2, "residue")]);
                Ok(())
            })
            .expect("winner commit"),
        Commit::Accepted { generation: 2, .. }
    ));

    // The loser shares one row with the winner and its other op is
    // base-redundant: the re-judgment lands the engine's no-op —
    // nothing published, `Accepted` at the current generation.
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "shared"), note_row(3, "base")]);
            Ok(())
        })
        .expect("loser commit");
    let Commit::Accepted {
        generation,
        durability,
        ..
    } = outcome
    else {
        panic!("the evaporated loser reports Accepted");
    };
    assert_eq!(generation, 2, "the current generation, not a new slot");
    assert_eq!(durability, Durability::Published);
    assert_eq!(writer_b.losses(), 1, "one loss, one re-judgment");

    let batches = verify_log(&root);
    assert_eq!(batches[&braid].len(), 2, "the log never gains a no-op slot");
    assert_whole(&writer_b, "the evaporated loser");
    assert_eq!(writer_digest(&writer_a), writer_digest(&writer_b));
    assert_eq!(writer_digest(&writer_b), converged_digest(&root));
}

#[test]
fn stale_pending_resolves_through_re_open_with_one_race_at_tip() {
    let root = temp_dir("stale");
    let dir = root.join("w");
    let crashed = match Writer::open_hooked(
        FsStore::new(root.clone()),
        "",
        &dir,
        theory(),
        Options::new(5),
        CrashOnce::new(WriterStep::ApplyLocal, 0),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    let err = crashed
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "mine")]);
            Ok(())
        })
        .expect_err("crash injected after the local apply");
    assert!(matches!(err, Error::InjectedCrash { .. }));
    drop(crashed);

    // The braid grows forty slots behind the writer's back.
    let codec = codec();
    let braid = note_braid(&codec);
    let mut log = TestPublisher::attach(root.clone());
    for slot in 0..40u64 {
        log.publish(braid, &[insert(NOTE, note_row(100 + slot, "foreign"))], 5);
    }

    let recovered = open_at(root.clone(), &dir, 5);
    assert_eq!(recovered.backlog(), None, "resolved at open");
    assert_eq!(recovered.vector()[&braid], 41, "catch-up plus our slot");
    assert_eq!(
        recovered.losses(),
        1,
        "the re-open IS the catch-up: one loss at the stale slot, one race at tip"
    );

    let batches = verify_log(&root);
    let slots = &batches[&braid];
    assert_eq!(slots.len(), 41);
    assert_eq!(slots[40].header.writer, 5, "our commit landed at tip+1");
    assert_whole(&recovered, "the recovered stale writer");
    converged_digest(&root);
    recovered.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(1, "mine"))?);
            for slot in 0..40u64 {
                assert!(instance.contains_dyn(NOTE, &note_row(100 + slot, "foreign"))?);
            }
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn slot_race_livelock_surfaces_contention_with_pending_kept() {
    let root = temp_dir("slotrace");
    let codec = codec();
    let braid = note_braid(&codec);
    let (store, planter) = RacingStore::new(root.clone(), braid, 16, Competitor::Notes);
    let writer = open_writer(store, &root.join("w"), 6);

    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1_000, "starved")]);
            Ok(())
        })
        .expect_err("the bound converts starvation into a typed signal");
    let Error::Contention { braid: got, cause } = err else {
        panic!("Contention expected, got {err:?}");
    };
    assert_eq!(got, braid);
    assert_eq!(cause, ContentionCause::SlotRace { tip: 16 });
    assert_eq!(planter.plants(), 16);
    assert_eq!(writer.backlog(), Some(braid), "the applied commit is kept");
    assert_whole(&writer, "the contended writer with its pending term");
    writer.with_db(|db| {
        db.read(|instance| {
            assert!(
                instance.contains_dyn(NOTE, &note_row(1_000, "starved"))?,
                "reads serve the applied pending"
            );
            Ok(())
        })
        .expect("read");
    });

    // The planter is spent: the next commit publishes the retained
    // batch at the tip before its own.
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2_000, "later")]);
            Ok(())
        })
        .expect("commit after the race");
    assert!(matches!(outcome, Commit::Accepted { generation: 18, .. }));
    assert_eq!(writer.backlog(), None);
    assert_whole(&writer, "the drained writer");
    let batches = verify_log(&root);
    assert_eq!(batches[&braid].len(), 18);
    assert_eq!(batches[&braid][16].header.writer, 6);
    converged_digest(&root);
    writer.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(2_000, "later"))?);
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn hot_key_livelock_surfaces_contention_with_the_violation_payload() {
    // Racer units sized so the sixteenth loss is the first whose
    // re-judgment the ceiling convicts: the terminal re-judgment is
    // the engine's capacity rejection, and its violation is the
    // HotKey payload.
    const RACER_UNITS: u64 = 55;
    const LOSER_UNITS: u64 = 50;
    let racer_fill: u64 = (0..16).map(|seq| RACER_UNITS + seq).sum();
    assert!(racer_fill <= CEILING, "the racers' own replay stays legal");
    assert!(racer_fill - (RACER_UNITS + 15) + LOSER_UNITS <= CEILING);
    assert!(racer_fill + LOSER_UNITS > CEILING);

    let root = temp_dir("hotkey");
    let codec = codec();
    let braid = venue_braid(&codec);
    let (store, planter) = RacingStore::new(
        root.clone(),
        braid,
        0,
        Competitor::Bookings {
            venue: 1,
            base_units: RACER_UNITS,
        },
    );
    let writer = open_writer(store, &root.join("w"), 7);

    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(VENUE, [Box::from([Value::U64(1)]) as Box<[Value]>]);
                Ok(())
            })
            .expect("venue setup"),
        Commit::Accepted { generation: 1, .. }
    ));
    planter.seed_from(root.clone());
    planter.arm(16);

    let err = writer
        .commit(|batch| {
            batch.insert(BOOKING, [booking_row(1, LOSER_UNITS)]);
            Ok(())
        })
        .expect_err("the racers turned the terminal re-judgment into a rejection");
    let Error::Contention { braid: got, cause } = err else {
        panic!("Contention expected, got {err:?}");
    };
    assert_eq!(got, braid);
    let ContentionCause::HotKey { statement, values } = cause else {
        panic!("HotKey expected, got {cause:?}");
    };
    assert_eq!(statement, BOOKING_CAPACITY, "the violation names itself");
    assert!(
        values.contains(&Value::U64(1)),
        "the offending fact's raw values carry the parent determinant: {values:?}"
    );
    assert_eq!(
        writer.backlog(),
        None,
        "a rejected terminal re-judgment clears the pending"
    );
    assert_whole(&writer, "the hot-key loser after the rejection");
    assert_eq!(planter.plants(), 16);
}

#[test]
fn dropped_response_after_landed_create_resolves_by_get_verify() {
    let root = temp_dir("drop_landed");
    let store = DropResponses::new(root.clone(), 1);
    let writer = open_writer(store, &root.join("w"), 8);
    let codec = codec();
    let braid = note_braid(&codec);

    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "landed")]);
            Ok(())
        })
        .expect("the GET-verify law absorbs the drop");
    assert!(matches!(
        outcome,
        Commit::Accepted {
            generation: 1,
            durability: Durability::Published,
            ..
        }
    ));
    assert_eq!(writer.backlog(), None);
    assert_eq!(
        writer.losses(),
        0,
        "the GET-verify absorption is not a loss"
    );

    let batches = verify_log(&root);
    let slots = &batches[&braid];
    assert_eq!(slots.len(), 1, "the acked commit appears exactly once");
    assert_eq!(slots[0].header.writer, 8);
    assert_whole(&writer, "the drop-absorbing writer");
    assert_eq!(writer_digest(&writer), converged_digest(&root));
}

#[test]
fn dropped_response_on_a_lost_slot_takes_the_one_path() {
    let root = temp_dir("drop_lost");
    let store = DropResponses::new(root.clone(), u64::MAX);
    let writer = open_writer(store, &root.join("w"), 9);
    let codec = codec();
    let braid = note_braid(&codec);

    // The competitor lands through its own store handle, untouched by
    // the wrapper's drops.
    let mut log = TestPublisher::attach(root.clone());
    log.publish(braid, &[insert(NOTE, note_row(50, "competitor"))], 5);

    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(51, "ours")]);
            Ok(())
        })
        .expect("the probe proves the loss and the one path runs");
    assert!(matches!(outcome, Commit::Accepted { generation: 2, .. }));
    assert_eq!(
        writer.losses(),
        1,
        "the probe proved the loss and the one path re-judged once"
    );

    let batches = verify_log(&root);
    let slots = &batches[&braid];
    assert_eq!(slots.len(), 2, "each acked commit appears exactly once");
    assert_eq!(slots[1].header.writer, 9);
    assert_whole(&writer, "the loser behind the dropped response");
    converged_digest(&root);
    writer.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(50, "competitor"))?);
            assert!(instance.contains_dyn(NOTE, &note_row(51, "ours"))?);
            Ok(())
        })
        .expect("read");
    });
}

/// One writer's ledger from a fleet run: what it acked, and what it
/// still holds pending after `Err::Contention`.
struct Ledger {
    acked: Vec<Box<[Value]>>,
    submitted: Vec<Box<[Value]>>,
    contentions: u64,
}

/// The mostly-disjoint fleet: `n` in-process writers over one prefix,
/// each booking its own rows on one braid. Every loss re-judges at the
/// re-opened tip and publishes; the gates are structural — chains,
/// digests, and exactly-once acks — because structure is truth.
#[allow(clippy::too_many_lines)]
fn mostly_disjoint_fleet(n: u64) {
    const ROUNDS: u64 = 6;
    let root = temp_dir("fleet");
    let ledgers: Vec<Ledger> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..n)
            .map(|i| {
                let root = root.clone();
                scope.spawn(move || {
                    let dir = root.join(format!("w{i}"));
                    let writer = open_at(root.clone(), &dir, 100 + i);
                    let mut ledger = Ledger {
                        acked: Vec::new(),
                        submitted: Vec::new(),
                        contentions: 0,
                    };
                    for j in 0..ROUNDS {
                        let row = note_row(i * 10_000 + j, "fleet");
                        ledger.submitted.push(row.clone());
                        let outcome = writer.commit(|batch| {
                            batch.insert(NOTE, [row.clone()]);
                            Ok(())
                        });
                        match outcome {
                            Ok(Commit::Accepted { .. }) => ledger.acked.push(row),
                            Ok(Commit::Rejected(violations)) => {
                                panic!("a disjoint booking never rejects: {violations:?}")
                            }
                            Err(Error::Contention { .. }) => ledger.contentions += 1,
                            Err(error) => panic!("fleet commit failed: {error}"),
                        }
                    }
                    // Publication retries on the next commit: drain any
                    // retained pending so the ledger closes.
                    let mut flushes = 0u64;
                    while writer.backlog().is_some() {
                        flushes += 1;
                        let row = note_row(i * 10_000 + 900 + flushes, "flush");
                        ledger.submitted.push(row.clone());
                        match writer.commit(|batch| {
                            batch.insert(NOTE, [row.clone()]);
                            Ok(())
                        }) {
                            Ok(Commit::Accepted { .. }) => ledger.acked.push(row),
                            other => panic!("flush did not land: {other:?}"),
                        }
                    }
                    assert_whole(&writer, "a fleet writer at rest");
                    writer.quiesce();
                    ledger
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("fleet thread"))
            .collect()
    });

    let codec = codec();
    let braid = note_braid(&codec);
    let batches = verify_log(&root);
    converged_digest(&root);

    // Every acked commit appears exactly once — and, because every
    // batch here was eventually published, every submitted row does.
    // Rows key by their debug rendering: raw values render uniquely
    // and the engine's `Value` carries no ordering.
    let mut published: BTreeMap<String, u64> = BTreeMap::new();
    for batch in &batches[&braid] {
        for op in &batch.ops {
            assert_eq!(op.kind, OpKind::Insert);
            for row in &op.rows {
                *published.entry(format!("{row:?}")).or_insert(0) += 1;
            }
        }
    }
    assert!(
        published.values().all(|count| *count == 1),
        "no row is ever published twice"
    );
    let mut acked_total = 0u64;
    let mut contentions = 0u64;
    let mut submitted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for ledger in &ledgers {
        contentions += ledger.contentions;
        for row in &ledger.submitted {
            submitted.insert(format!("{row:?}"));
        }
        for row in &ledger.acked {
            acked_total += 1;
            assert_eq!(
                published.get(&format!("{row:?}")),
                Some(&1),
                "every acked commit appears exactly once"
            );
        }
    }
    assert!(
        published.keys().all(|row| submitted.contains(row)),
        "nothing reaches the log that no writer submitted"
    );
    // A commit that surfaced `Err::Contention` is honestly unacked:
    // either it applied and was retained — published by a later
    // drain, so its row appears once — or the backlog ahead of it
    // exhausted the bound first and its ops never entered a store at
    // all. Both are within the contract; a duplicate or an unsubmitted
    // row never is.
    let published_total = u64::try_from(published.len()).expect("count fits");
    assert!(
        published_total >= acked_total && published_total <= acked_total + contentions,
        "the log holds the acked commits plus at most the retained ones \
         ({acked_total} acked, {contentions} contended, {published_total} published)"
    );
}

#[test]
fn fleet_of_two_mostly_disjoint_converges() {
    mostly_disjoint_fleet(2);
}

#[test]
fn fleet_of_four_mostly_disjoint_converges() {
    mostly_disjoint_fleet(4);
}

#[test]
fn fleet_of_eight_mostly_disjoint_converges() {
    mostly_disjoint_fleet(8);
}

#[test]
fn hot_key_fleet_yields_one_winner_and_serial_rejections_per_round() {
    const WRITERS: u64 = 4;
    const ROUNDS: u64 = 6;
    let root = temp_dir("hotfleet");
    let barrier = std::sync::Barrier::new(usize::try_from(WRITERS).expect("fits"));
    let per_writer: Vec<Vec<Commit<()>>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WRITERS)
            .map(|i| {
                let root = root.clone();
                let barrier = &barrier;
                scope.spawn(move || {
                    let dir = root.join(format!("w{i}"));
                    let writer = open_at(root.clone(), &dir, 200 + i);
                    let mut outcomes = Vec::new();
                    for r in 0..ROUNDS {
                        barrier.wait();
                        let outcome = writer
                            .commit(|batch| {
                                batch.insert(
                                    RECIPE,
                                    [recipe_row(40_000 + r, &format!("writer {i}"))],
                                );
                                Ok(())
                            })
                            .expect("a conflict resolves to a verdict, never an Err");
                        outcomes.push(outcome);
                    }
                    assert_whole(&writer, "a hot-key fleet writer");
                    assert_eq!(
                        writer_digest(&writer),
                        converged_digest(&root),
                        "every store here applied in log order"
                    );
                    writer.quiesce();
                    outcomes
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("fleet thread"))
            .collect()
    });

    for r in 0..usize::try_from(ROUNDS).expect("fits") {
        let mut accepted = 0u64;
        let mut rejected = 0u64;
        for outcomes in &per_writer {
            match &outcomes[r] {
                Commit::Accepted { .. } => accepted += 1,
                Commit::Rejected(violations) => {
                    assert!(
                        violations
                            .iter()
                            .any(|violation| matches!(violation, Violation::Functionality { .. })),
                        "losers get the typed FD violation"
                    );
                    rejected += 1;
                }
            }
        }
        assert_eq!(accepted, 1, "exactly one winner per round");
        assert_eq!(rejected, WRITERS - 1, "serial rejections for the rest");
    }

    let codec = codec();
    let braid = kitchen_braid(&codec);
    let batches = verify_log(&root);
    assert_eq!(
        u64::try_from(batches[&braid].len()).expect("fits"),
        ROUNDS,
        "one slot per round: losers publish nothing"
    );
    let mut determinants: BTreeMap<u64, u64> = BTreeMap::new();
    for batch in &batches[&braid] {
        for op in &batch.ops {
            for row in &op.rows {
                let Value::U64(id) = row[0] else {
                    panic!("recipe determinant is u64")
                };
                *determinants.entry(id).or_insert(0) += 1;
            }
        }
    }
    assert!(
        determinants.values().all(|count| *count == 1),
        "zero duplicates: one row per hot determinant"
    );
}

#[test]
fn hot_capacity_parent_fleet_prices_the_slack_serially() {
    const WRITERS: u64 = 4;
    const ROUNDS: u64 = 5;
    let root = temp_dir("capfleet");
    let setup = open_at(root.clone(), &root.join("setup"), 300);
    assert!(matches!(
        setup
            .commit(|batch| {
                batch.insert(VENUE, [Box::from([Value::U64(1)]) as Box<[Value]>]);
                Ok(())
            })
            .expect("venue setup"),
        Commit::Accepted { generation: 1, .. }
    ));

    let barrier = std::sync::Barrier::new(usize::try_from(WRITERS).expect("fits"));
    let per_writer: Vec<Vec<(u64, Commit<()>)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WRITERS)
            .map(|i| {
                let root = root.clone();
                let barrier = &barrier;
                scope.spawn(move || {
                    let dir = root.join(format!("w{i}"));
                    let writer = open_at(root.clone(), &dir, 310 + i);
                    let mut outcomes = Vec::new();
                    for r in 0..ROUNDS {
                        barrier.wait();
                        // Distinct units per (writer, round) so set
                        // semantics never collapses two bookings into
                        // one row; all near 100 so the ceiling admits
                        // roughly half the demand.
                        let units = 90 + r * WRITERS + i;
                        let outcome = writer
                            .commit(|batch| {
                                batch.insert(BOOKING, [booking_row(1, units)]);
                                Ok(())
                            })
                            .expect("a capacity race resolves to a verdict");
                        outcomes.push((units, outcome));
                    }
                    assert_whole(&writer, "a hot-parent fleet writer");
                    writer.quiesce();
                    outcomes
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("fleet thread"))
            .collect()
    });

    let mut accepted_units = 0u64;
    let mut accepted = 0u64;
    let mut rejected = 0u64;
    for outcomes in &per_writer {
        for (units, outcome) in outcomes {
            match outcome {
                Commit::Accepted { .. } => {
                    accepted += 1;
                    accepted_units += units;
                }
                Commit::Rejected(violations) => {
                    assert!(
                        violations
                            .iter()
                            .any(|violation| matches!(violation, Violation::Capacity { .. })),
                        "over-slack bookings get the typed capacity violation"
                    );
                    rejected += 1;
                }
            }
        }
    }
    assert!(
        accepted_units <= CEILING,
        "the accepted total respects the ceiling: {accepted_units}"
    );
    assert!(accepted >= 1, "the first bookings fit");
    assert!(
        rejected >= 1,
        "demand doubled the ceiling, so rejections exist"
    );

    let codec = codec();
    let braid = venue_braid(&codec);
    let batches = verify_log(&root);
    let mut logged_units = 0u64;
    for batch in &batches[&braid] {
        for op in &batch.ops {
            if op.relation != BOOKING {
                continue;
            }
            for row in &op.rows {
                let Value::U64(units) = row[1] else {
                    panic!("booking units are u64")
                };
                logged_units += units;
            }
        }
    }
    assert_eq!(
        logged_units, accepted_units,
        "every acked booking appears exactly once and nothing else reached the log"
    );
    // The verifying replica replays every slot accepted — the serial
    // verdicts and the log agree.
    converged_digest(&root);
}

/// Parks the first log-slot create on the note braid until the gate
/// opens, holding the commit core busy while the venue callers queue —
/// the deterministic packing lever.
struct HoldNotePut {
    inner: FsStore,
    gate: std::sync::Arc<AtomicBool>,
    tripped: AtomicBool,
}

impl ObjectStore for HoldNotePut {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> StoreResult<Create> {
        if key.as_str().contains("log/c00000002/") && !self.tripped.swap(true, Ordering::SeqCst) {
            while !self.gate.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        self.inner.put_create(key, bytes)
    }

    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> StoreResult<Swap> {
        self.inner.put_swap(key, bytes, etag)
    }

    fn delete(&self, key: &StoreKey) -> StoreResult<()> {
        self.inner.delete(key)
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn packed_drain_delete_cures_the_solo_violation() {
    // Packed: one caller's delete cures another's violation — the
    // drain is one transaction by law, and the engine judges the
    // composite's final state.
    let root = temp_dir("drain");
    let gate = std::sync::Arc::new(AtomicBool::new(false));
    let store = HoldNotePut {
        inner: FsStore::new(root.clone()),
        gate: std::sync::Arc::clone(&gate),
        tripped: AtomicBool::new(false),
    };
    let writer = match Writer::open(store, "", &root.join("w"), theory(), Options::new(13))
        .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(VENUE, [Box::from([Value::U64(1)]) as Box<[Value]>]);
                Ok(())
            })
            .expect("venue setup"),
        Commit::Accepted { generation: 1, .. }
    ));
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(BOOKING, [booking_row(1, CEILING - 1)]);
                Ok(())
            })
            .expect("fill the ceiling"),
        Commit::Accepted { generation: 2, .. }
    ));

    let start = std::sync::Barrier::new(2);
    let (cure, insert_outcome) = std::thread::scope(|scope| {
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
        let cure_task = scope.spawn(|| {
            writer.commit(|batch| {
                batch.delete(BOOKING, [booking_row(1, CEILING - 1)]);
                Ok(())
            })
        });
        let insert_task = scope.spawn(|| {
            writer.commit(|batch| {
                batch.insert(BOOKING, [booking_row(1, 50)]);
                Ok(())
            })
        });
        // Both venue callers queue behind the busy core, then the gate
        // opens and the next drain picks them together.
        std::thread::sleep(Duration::from_millis(150));
        gate.store(true, Ordering::SeqCst);
        let hold = holder.join().expect("join holder").expect("holder commit");
        assert!(matches!(hold, Commit::Accepted { .. }));
        (
            cure_task.join().expect("join").expect("commit"),
            insert_task.join().expect("join").expect("commit"),
        )
    });
    let Commit::Accepted { generation: g1, .. } = cure else {
        panic!("the composite accepts");
    };
    let Commit::Accepted { generation: g2, .. } = insert_outcome else {
        panic!("the composite accepts what a solo run would reject");
    };
    assert_eq!(g1, 3);
    assert_eq!(g2, 3, "one batch, one generation, one object");

    let codec = codec();
    let braid = venue_braid(&codec);
    let batches = verify_log(&root);
    assert_eq!(batches[&braid].len(), 3);
    assert_eq!(
        batches[&braid][2].ops.len(),
        2,
        "both callers packed into one transaction"
    );
    // The verifying replica replays the composite as one transaction —
    // the documented outcome, not a surprise.
    converged_digest(&root);
    assert_whole(&writer, "the packing writer");

    // Solo control on a fresh prefix: the same insert without the
    // neighboring delete is the serial capacity rejection.
    let solo_root = temp_dir("drain_solo");
    let solo = open_at(solo_root.clone(), &solo_root.join("w"), 14);
    assert!(matches!(
        solo.commit(|batch| {
            batch.insert(VENUE, [Box::from([Value::U64(1)]) as Box<[Value]>]);
            Ok(())
        })
        .expect("venue setup"),
        Commit::Accepted { .. }
    ));
    assert!(matches!(
        solo.commit(|batch| {
            batch.insert(BOOKING, [booking_row(1, CEILING - 1)]);
            Ok(())
        })
        .expect("fill the ceiling"),
        Commit::Accepted { .. }
    ));
    let solo_outcome = solo
        .commit(|batch| {
            batch.insert(BOOKING, [booking_row(1, 50)]);
            Ok(())
        })
        .expect("solo verdict");
    let Commit::Rejected(violations) = solo_outcome else {
        panic!("solo rejects where the composite accepted");
    };
    assert!(
        violations
            .iter()
            .any(|violation| matches!(violation, Violation::Capacity { .. })),
        "the solo rejection is the typed capacity violation"
    );
}

/// The Feral uniqueness storm: 64 writers inserting one hot
/// determinant per round — their experiment leaked 70-6,300
/// duplicates; this gate is zero duplicates, one Accepted, and 63
/// typed FD rejections, every round. The per-round shape runs at the
/// exact Feral width of 64; the round count is scaled from their 100
/// to 16 under the wall-clock license (measured: 8 s per round — 63
/// discard-and-rebuild re-judgments each — puts 100 rounds past 13
/// minutes for one test).
#[test]
fn feral_uniqueness_storm_zero_duplicates() {
    const WRITERS: u64 = 64;
    const ROUNDS: u64 = 16;
    let root = temp_dir("feral_unique");
    let barrier = std::sync::Barrier::new(usize::try_from(WRITERS).expect("fits"));
    let per_writer: Vec<Vec<Commit<()>>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WRITERS)
            .map(|i| {
                let root = root.clone();
                let barrier = &barrier;
                scope.spawn(move || {
                    let dir = root.join(format!("w{i}"));
                    let writer = open_at(root.clone(), &dir, 400 + i);
                    // A tight checkpoint cadence keeps every loser's
                    // rebuild a seed-plus-short-tail instead of a full
                    // replay — the protocol's own pressure valve.
                    writer.set_checkpoint_cadence(32, u64::MAX);
                    let mut outcomes = Vec::new();
                    for r in 0..ROUNDS {
                        barrier.wait();
                        let outcome = writer
                            .commit(|batch| {
                                batch.insert(
                                    RECIPE,
                                    [recipe_row(70_000 + r, &format!("writer {i}"))],
                                );
                                Ok(())
                            })
                            .expect("one loss per loser per round never nears the bound");
                        outcomes.push(outcome);
                    }
                    assert_whole(&writer, "a storm writer");
                    writer.quiesce();
                    outcomes
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("storm thread"))
            .collect()
    });

    for r in 0..usize::try_from(ROUNDS).expect("fits") {
        let mut accepted = 0u64;
        let mut rejected = 0u64;
        for outcomes in &per_writer {
            match &outcomes[r] {
                Commit::Accepted { .. } => accepted += 1,
                Commit::Rejected(violations) => {
                    assert!(
                        violations
                            .iter()
                            .any(|violation| matches!(violation, Violation::Functionality { .. })),
                        "every loser gets the typed FD rejection"
                    );
                    rejected += 1;
                }
            }
        }
        assert_eq!(accepted, 1, "one Accepted per round");
        assert_eq!(rejected, WRITERS - 1, "63 typed FD rejections per round");
    }

    let codec = codec();
    let braid = kitchen_braid(&codec);
    let batches = verify_log(&root);
    assert_eq!(
        u64::try_from(batches[&braid].len()).expect("fits"),
        ROUNDS,
        "losers publish nothing: one slot per round"
    );
    let mut determinants: BTreeMap<u64, u64> = BTreeMap::new();
    for batch in &batches[&braid] {
        for op in &batch.ops {
            for row in &op.rows {
                let Value::U64(id) = row[0] else {
                    panic!("recipe determinant is u64")
                };
                *determinants.entry(id).or_insert(0) += 1;
            }
        }
    }
    assert_eq!(
        u64::try_from(determinants.len()).expect("fits"),
        ROUNDS,
        "every round's determinant landed"
    );
    assert!(
        determinants.values().all(|count| *count == 1),
        "zero duplicates leaked"
    );
    converged_digest(&root);
}

/// The Feral association storm: one target-delete racing 64 concurrent
/// source-inserts per round — their experiment orphaned up to 6,400
/// rows; this gate is zero orphans and serial verdicts throughout, and
/// the non-vacuity counter proves the delete actually won rounds
/// instead of losing every race by accident. The race runs at the
/// exact Feral width of 64 inserters plus the deleter, for the full
/// 100 rounds.
#[test]
#[allow(clippy::too_many_lines)]
fn feral_association_storm_zero_orphans() {
    const INSERTERS: u64 = 64;
    const ROUNDS: u64 = 100;
    let root = temp_dir("feral_orphan");
    let start = std::sync::Barrier::new(usize::try_from(INSERTERS + 1).expect("fits"));
    let finish = std::sync::Barrier::new(usize::try_from(INSERTERS + 1).expect("fits"));
    std::thread::scope(|scope| {
        let deleter = {
            let root = root.clone();
            let start = &start;
            let finish = &finish;
            scope.spawn(move || {
                let dir = root.join("deleter");
                let writer = open_at(root.clone(), &dir, 500);
                writer.set_checkpoint_cadence(32, u64::MAX);
                let mut delete_wins = 0u64;
                for r in 0..ROUNDS {
                    // Seed the round's target before the race opens.
                    let mut seeded = false;
                    for _ in 0..10 {
                        match writer.commit(|batch| {
                            batch.insert(RECIPE, [recipe_row(80_000 + r, "base")]);
                            Ok(())
                        }) {
                            Ok(Commit::Accepted { .. }) => {
                                seeded = true;
                                break;
                            }
                            Ok(Commit::Rejected(violations)) => {
                                panic!("a fresh determinant never rejects: {violations:?}")
                            }
                            Err(Error::Contention { .. }) => {}
                            Err(error) => panic!("seed failed: {error}"),
                        }
                    }
                    assert!(seeded, "the seed lands within the retry budget");
                    start.wait();
                    match writer.commit(|batch| {
                        batch.delete(RECIPE, [recipe_row(80_000 + r, "base")]);
                        Ok(())
                    }) {
                        Ok(Commit::Accepted { .. }) => delete_wins += 1,
                        Err(Error::Contention { .. }) => {}
                        Ok(Commit::Rejected(violations)) => {
                            assert!(
                                violations.iter().any(|violation| matches!(
                                    violation,
                                    Violation::Containment { .. }
                                )),
                                "a refused target delete cites the containment"
                            );
                        }
                        Err(error) => panic!("delete failed: {error}"),
                    }
                    finish.wait();
                }
                while writer.backlog().is_some() {
                    writer
                        .commit(|batch| {
                            batch.insert(NOTE, [note_row(600_000, "deleter flush")]);
                            Ok(())
                        })
                        .expect("flush drains the backlog");
                }
                assert_whole(&writer, "the storm deleter");
                writer.quiesce();
                delete_wins
            })
        };
        let handles: Vec<_> = (1..=INSERTERS)
            .map(|i| {
                let root = root.clone();
                let start = &start;
                let finish = &finish;
                scope.spawn(move || {
                    let dir = root.join(format!("i{i}"));
                    let writer = open_at(root.clone(), &dir, 500 + i);
                    writer.set_checkpoint_cadence(32, u64::MAX);
                    for r in 0..ROUNDS {
                        start.wait();
                        match writer.commit(|batch| {
                            batch.insert(STEP, [step_row(80_000 + r, &format!("s{i}"))]);
                            Ok(())
                        }) {
                            Ok(Commit::Accepted { .. }) | Err(Error::Contention { .. }) => {}
                            Ok(Commit::Rejected(violations)) => {
                                assert!(
                                    violations.iter().any(|violation| matches!(
                                        violation,
                                        Violation::Containment { .. }
                                    )),
                                    "a refused source insert cites the containment"
                                );
                            }
                            Err(error) => panic!("insert failed: {error}"),
                        }
                        finish.wait();
                    }
                    while writer.backlog().is_some() {
                        writer
                            .commit(|batch| {
                                batch.insert(NOTE, [note_row(600_000 + i, "flush")]);
                                Ok(())
                            })
                            .expect("flush drains the backlog");
                    }
                    assert_whole(&writer, "a storm inserter");
                    writer.quiesce();
                })
            })
            .collect();
        let delete_wins = deleter.join().expect("deleter thread");
        assert!(
            delete_wins > 0,
            "non-vacuity: at least one target delete won its round"
        );
        for handle in handles {
            handle.join().expect("inserter thread");
        }
    });

    verify_log(&root);
    // Zero orphans, judged by a fresh replica of the published truth:
    // wherever any step survived, its recipe survived with it — the
    // serial verdicts never let the delete and an insert both win.
    let dir = temp_dir("orphan_check");
    let opened = Replica::open(FsStore::new(root.clone()), "", &dir.join("r"), theory())
        .expect("open verifying replica");
    let Opened::Ready(replica) = opened else {
        panic!("verifying replica refused");
    };
    assert!(replica.wedged().is_empty(), "serial verdicts throughout");
    replica
        .db()
        .read(|instance| {
            for r in 0..ROUNDS {
                let recipe_present =
                    instance.contains_dyn(RECIPE, &recipe_row(80_000 + r, "base"))?;
                for i in 1..=INSERTERS {
                    if instance.contains_dyn(STEP, &step_row(80_000 + r, &format!("s{i}")))? {
                        assert!(
                            recipe_present,
                            "zero orphans: round {r} step s{i} outlived its recipe"
                        );
                    }
                }
            }
            Ok(())
        })
        .expect("read");
    converged_digest(&root);
}

/// The deterministic sampler for the skew curve: cumulative Zipfian
/// weights over ranked keys.
#[allow(clippy::cast_precision_loss)]
fn zipf_cdf(keys: u64, skew: f64) -> Vec<f64> {
    let mut weights: Vec<f64> = (1..=keys)
        .map(|rank| 1.0 / (rank as f64).powf(skew))
        .collect();
    let total: f64 = weights.iter().sum();
    let mut cumulative = 0.0;
    for weight in &mut weights {
        cumulative += *weight / total;
        *weight = cumulative;
    }
    weights
}

struct XorShift(u64);

impl XorShift {
    #[allow(clippy::cast_precision_loss)]
    fn next_unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn zipf_key(cdf: &[f64], unit: f64) -> u64 {
    let index = cdf.partition_point(|cumulative| *cumulative < unit);
    u64::try_from(index.min(cdf.len() - 1)).expect("key fits") + 1
}

/// The hot-key skew curve's correctness half (F11 records its
/// throughput): 8 writers drawing recipe determinants Zipfian at skew
/// 0.99 over 64 keys, 25 commits each — the recorded parameters. The
/// first writer of a key wins it forever; every later dependent is the
/// serial FD rejection; the log never holds a duplicate determinant.
#[test]
fn zipfian_skew_keeps_verdicts_serial_and_keys_unique() {
    const WRITERS: u64 = 8;
    const COMMITS: u64 = 25;
    const KEYS: u64 = 64;
    const SKEW: f64 = 0.99;
    let root = temp_dir("zipf");
    let cdf = zipf_cdf(KEYS, SKEW);
    let ledgers: Vec<(Vec<u64>, Vec<u64>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WRITERS)
            .map(|i| {
                let root = root.clone();
                let cdf = &cdf;
                scope.spawn(move || {
                    let dir = root.join(format!("w{i}"));
                    let writer = open_at(root.clone(), &dir, 700 + i);
                    let mut rng = XorShift(0x5eed_0000 + i);
                    let mut sampled = Vec::new();
                    let mut accepted = Vec::new();
                    for j in 0..COMMITS {
                        let key = zipf_key(cdf, rng.next_unit());
                        sampled.push(key);
                        match writer.commit(|batch| {
                            batch.insert(RECIPE, [recipe_row(90_000 + key, &format!("w{i} c{j}"))]);
                            Ok(())
                        }) {
                            Ok(Commit::Accepted { .. }) => accepted.push(key),
                            Ok(Commit::Rejected(violations)) => {
                                assert!(
                                    violations.iter().any(|violation| matches!(
                                        violation,
                                        Violation::Functionality { .. }
                                    )),
                                    "a lost key is the serial FD rejection"
                                );
                            }
                            Err(Error::Contention { .. }) => {}
                            Err(error) => panic!("skewed commit failed: {error}"),
                        }
                    }
                    while writer.backlog().is_some() {
                        writer
                            .commit(|batch| {
                                batch.insert(NOTE, [note_row(700_000 + i, "flush")]);
                                Ok(())
                            })
                            .expect("flush drains the backlog");
                    }
                    assert_whole(&writer, "a skewed writer");
                    writer.quiesce();
                    (sampled, accepted)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("skew thread"))
            .collect()
    });

    let codec = codec();
    let braid = kitchen_braid(&codec);
    let batches = verify_log(&root);
    converged_digest(&root);
    let mut sampled: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut accepted: Vec<u64> = Vec::new();
    for (keys, wins) in &ledgers {
        sampled.extend(keys.iter().copied());
        accepted.extend(wins.iter().copied());
    }
    let mut determinants: BTreeMap<u64, u64> = BTreeMap::new();
    for batch in &batches[&braid] {
        for op in &batch.ops {
            for row in &op.rows {
                let Value::U64(id) = row[0] else {
                    panic!("recipe determinant is u64")
                };
                *determinants.entry(id - 90_000).or_insert(0) += 1;
            }
        }
    }
    assert!(
        determinants.values().all(|count| *count == 1),
        "zero duplicate determinants under skew"
    );
    assert!(
        determinants.keys().all(|key| sampled.contains(key)),
        "the log holds only sampled keys"
    );
    let mut accepted_sorted = accepted.clone();
    accepted_sorted.sort_unstable();
    accepted_sorted.dedup();
    assert_eq!(
        accepted_sorted.len(),
        accepted.len(),
        "no key is ever won twice"
    );
    assert!(
        accepted.iter().all(|key| determinants.contains_key(key)),
        "every acked win is in the log exactly once"
    );
}
