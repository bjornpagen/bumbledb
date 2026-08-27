//! Conformance F5 — contention under the one loss path. Two writers
//! over one `FsStore` prefix, each with its own replica directory;
//! the standing gates on every fixture: per-braid logs gap-free with
//! each slot created once, every `prev` hash verified, every acked
//! commit appearing exactly once, all replicas converging on
//! `catalog_digest`, and the wholeness identity
//! `generation ≡ generation(chain)` asserted on every store — the
//! invariant the loss path must never bend. A loss whose effects the
//! winner already performed re-judges to the engine's net no-op and
//! lands `Accepted` at the current tip with nothing published;
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
use bumbledb::{Admission, Value, Violation};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{Batch, BatchHeader, Codec, Op, OpKind};
use bumbledb_log::manifest::{Head, log_key};
use bumbledb_log::replica::{Opened, Replica};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{
    Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result as StoreResult, StoreError, StoreKey,
    Swap,
};
use bumbledb_log::writer::{
    ContentionCause, Durability, Error, Options, Slotted, StepControl, StepHook, Writer,
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
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-f5-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Three braids: recipe+step (key + containment), note alone, and
/// venue+booking under a tight capacity ceiling — the hot-key
/// adversary lives on the first and third; the disjoint-loss
/// fixtures live on the second.
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

/// The wholeness identity `generation ≡ generation(chain)` — the
/// invariant the loss path must never bend, asserted on every
/// writer after every fixture.
fn assert_whole<S, H>(writer: &Writer<SchemaDescriptor, S, H>, context: &str)
where
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    let generation = writer
        .with_db(|db| db.generation().expect("generation").value())
        .expect("a fixture writer is Mounted");
    assert_eq!(
        generation,
        writer.chain().generation(),
        "generation ≡ generation(chain) on {context}"
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
    let generation = replica
        .db()
        .expect("db")
        .generation()
        .expect("generation")
        .value();
    let sum: u64 = replica.vector().sum().expect("sum");
    assert_eq!(
        generation, sum,
        "the wholeness identity on the verifying replica"
    );
    replica
        .db()
        .expect("db")
        .catalog_digest()
        .expect("catalog digest")
}

fn writer_digest<S, H>(writer: &Writer<SchemaDescriptor, S, H>) -> [u8; 32]
where
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    writer
        .with_db(|db| db.catalog_digest().expect("catalog digest"))
        .expect("db")
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

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        let created = self.inner.put_create(key, body)?;
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

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        self.maybe_plant(key);
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
        Admission::Accepted(Slotted { slot: 1, .. })
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2, "ours")]);
            Ok(())
        })
        .expect("loser commit");
    let Admission::Accepted(Slotted {
        slot, durability, ..
    }) = outcome
    else {
        panic!("a disjoint loss lands");
    };
    assert_eq!(slot, 2, "the re-judged publish lands at tip+1");
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
    writer_b
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(NOTE, &note_row(1, "theirs"))?);
                assert!(instance.contains_dyn(NOTE, &note_row(2, "ours"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
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
        Admission::Accepted(Slotted { slot: 1, .. })
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(7, "same")]);
            Ok(())
        })
        .expect("loser commit");
    let Admission::Accepted(Slotted {
        slot, durability, ..
    }) = outcome
    else {
        panic!("a loss whose effects the winner performed reports Accepted");
    };
    assert_eq!(slot, 1, "the winner's slot, not a new one");
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
        Admission::Accepted(Slotted { slot: 1, .. })
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(11, "shared")]);
            Ok(())
        })
        .expect("loser commit");
    let Admission::Accepted(Slotted { slot, .. }) = outcome else {
        panic!("a strictly contained loss reports the winner's outcome");
    };
    assert_eq!(slot, 1, "accepted at the current tip");
    assert_eq!(writer_b.losses(), 1);

    writer_b
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(NOTE, &note_row(11, "shared"))?);
                assert!(
                    instance.contains_dyn(NOTE, &note_row(12, "residue"))?,
                    "the winner's residue is present after the re-open"
                );
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
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
        Admission::Accepted(Slotted { slot: 1, .. })
    ));
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(1, "loser")]);
            Ok(())
        })
        .expect("loser commit");
    let Admission::Rejected(violations) = outcome else {
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
    writer_b
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(RECIPE, &recipe_row(1, "winner"))?);
                assert!(!instance.contains_dyn(RECIPE, &recipe_row(1, "loser"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
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
        Admission::Accepted(Slotted { slot: 1, .. })
    ));
    let writer_b = open_at(root.clone(), &root.join("wb"), 2);
    assert_eq!(writer_b.vector().at(braid), 1);

    // The winner takes slot 2 with the shared insert plus residue.
    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(NOTE, [note_row(1, "shared"), note_row(2, "residue")]);
                Ok(())
            })
            .expect("winner commit"),
        Admission::Accepted(Slotted { slot: 2, .. })
    ));

    // The loser shares one row with the winner and its other op is
    // base-redundant: the re-judgment lands the engine's no-op —
    // nothing published, `Accepted` at the current tip.
    let outcome = writer_b
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "shared"), note_row(3, "base")]);
            Ok(())
        })
        .expect("loser commit");
    let Admission::Accepted(Slotted {
        slot, durability, ..
    }) = outcome
    else {
        panic!("the evaporated loser reports Accepted");
    };
    assert_eq!(slot, 2, "the current tip, not a new slot");
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
    assert_eq!(recovered.vector().at(braid), 41, "catch-up plus our slot");
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
    recovered
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(NOTE, &note_row(1, "mine"))?);
                for slot in 0..40u64 {
                    assert!(instance.contains_dyn(NOTE, &note_row(100 + slot, "foreign"))?);
                }
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
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
    writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(
                    instance.contains_dyn(NOTE, &note_row(1_000, "starved"))?,
                    "reads serve the applied pending"
                );
                Ok(())
            })
            .expect("read");
        })
        .expect("db");

    // The planter is spent: the next commit publishes the retained
    // batch at the tip before its own.
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2_000, "later")]);
            Ok(())
        })
        .expect("commit after the race");
    assert!(matches!(
        outcome,
        Admission::Accepted(Slotted { slot: 18, .. })
    ));
    assert_eq!(writer.backlog(), None);
    assert_whole(&writer, "the drained writer");
    let batches = verify_log(&root);
    assert_eq!(batches[&braid].len(), 18);
    assert_eq!(batches[&braid][16].header.writer, 6);
    converged_digest(&root);
    writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(NOTE, &note_row(2_000, "later"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
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
        Admission::Accepted(Slotted { slot: 1, .. })
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
        Admission::Accepted(Slotted {
            slot: 1,
            durability: Durability::Published,
            ..
        })
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
    assert!(matches!(
        outcome,
        Admission::Accepted(Slotted { slot: 2, .. })
    ));
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
    writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(NOTE, &note_row(50, "competitor"))?);
                assert!(instance.contains_dyn(NOTE, &note_row(51, "ours"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
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

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        if key.as_str().contains("log/c00000002/") && !self.tripped.swap(true, Ordering::SeqCst) {
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
        Admission::Accepted(Slotted { slot: 1, .. })
    ));
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(BOOKING, [booking_row(1, CEILING - 1)]);
                Ok(())
            })
            .expect("fill the ceiling"),
        Admission::Accepted(Slotted { slot: 2, .. })
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
        assert!(matches!(hold, Admission::Accepted(_)));
        (
            cure_task.join().expect("join").expect("commit"),
            insert_task.join().expect("join").expect("commit"),
        )
    });
    let Admission::Accepted(Slotted { slot: s1, .. }) = cure else {
        panic!("the composite accepts");
    };
    let Admission::Accepted(Slotted { slot: s2, .. }) = insert_outcome else {
        panic!("the composite accepts what a solo run would reject");
    };
    assert_eq!(s1, 3);
    assert_eq!(s2, 3, "one batch, one slot, one object");

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
        Admission::Accepted(_)
    ));
    assert!(matches!(
        solo.commit(|batch| {
            batch.insert(BOOKING, [booking_row(1, CEILING - 1)]);
            Ok(())
        })
        .expect("fill the ceiling"),
        Admission::Accepted(_)
    ));
    let solo_outcome = solo
        .commit(|batch| {
            batch.insert(BOOKING, [booking_row(1, 50)]);
            Ok(())
        })
        .expect("solo verdict");
    let Admission::Rejected(violations) = solo_outcome else {
        panic!("solo rejects where the composite accepted");
    };
    assert!(
        violations
            .iter()
            .any(|violation| matches!(violation, Violation::Capacity { .. })),
        "the solo rejection is the typed capacity violation"
    );
}
