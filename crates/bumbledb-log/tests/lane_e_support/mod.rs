//! Shared fixtures for the writer lane's tests: a three-braid theory
//! with a key, a containment, and two capacities (one of them shaped as
//! a reservation relation), a test-side slot publisher, a crash-once
//! step hook, and two store wrappers — one that races every slot
//! attempt with a competitor batch, one that turns a create into the
//! ambiguous-PUT `Exists`.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::Value;
use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, StatementId, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::manifest::{Head, log_key};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{
    Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result as StoreResult, StoreKey, Swap,
};
use bumbledb_log::writer::{StepControl, StepHook, WriterStep};

pub const RECIPE: RelationId = RelationId(0);
pub const STEP: RelationId = RelationId(1);
pub const NOTE: RelationId = RelationId(2);
pub const VENUE: RelationId = RelationId(3);
pub const BOOKING: RelationId = RelationId(4);
pub const HOLD: RelationId = RelationId(5);

pub const RECIPE_KEY: StatementId = StatementId(0);
pub const STEP_IN_RECIPE: StatementId = StatementId(1);
pub const VENUE_KEY: StatementId = StatementId(2);
pub const BOOKING_CAPACITY: StatementId = StatementId(3);
pub const HOLD_CAPACITY: StatementId = StatementId(4);

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-e-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Three braids: recipe+step (key + containment), note alone, and
/// venue+booking+hold under two capacity statements sharing the venue
/// parent — `hold` carries the reservation shape (parent, units,
/// expiry).
pub fn theory() -> SchemaDescriptor {
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
            RelationDescriptor {
                name: "hold".into(),
                fields: vec![
                    field("venue", ValueType::U64),
                    field("units", ValueType::U64),
                    field("expiry", ValueType::U64),
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
                hi: Some(Bound::Lit(100_000)),
                source: side(BOOKING, &[0]),
            },
            StatementDescriptor::Capacity {
                target: side(VENUE, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 0,
                hi: Some(Bound::Lit(100_000)),
                source: side(HOLD, &[0]),
            },
        ],
    }
}

pub fn codec() -> Codec {
    let descriptor = theory();
    let schema = descriptor.clone().validate().expect("fixture validates");
    let fingerprint = schema_fingerprint(&schema).0;
    Codec::new(&descriptor, fingerprint)
}

pub fn kitchen_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(RECIPE).expect("recipe braid")
}

pub fn note_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(NOTE).expect("note braid")
}

pub fn venue_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(VENUE).expect("venue braid")
}

pub fn recipe_row(id: u64, title: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(title.into())])
}

pub fn step_row(recipe: u64, name: &str) -> Box<[Value]> {
    Box::from([Value::U64(recipe), Value::String(name.into())])
}

pub fn note_row(id: u64, body: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(body.into())])
}

pub fn insert(relation: RelationId, row: Box<[Value]>) -> Op {
    Op {
        kind: OpKind::Insert,
        relation,
        rows: vec![row],
    }
}

/// A test-side publisher that keeps its own chain state, so tests can
/// plant competing slots without a second writer.
pub struct TestLog {
    pub store: FsStore,
    pub prefix: String,
    pub codec: Codec,
    pub heads: BTreeMap<BraidId, Head>,
    pub writer: u64,
}

impl TestLog {
    /// Attaches to an existing store root (the writer under test
    /// creates the manifest).
    pub fn attach(root: PathBuf, prefix: &str) -> Self {
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
            prefix: prefix.to_string(),
            codec,
            heads,
            writer: 7001,
        }
    }

    pub fn encode(&self, braid: BraidId, ops: &[Op], ts: u64) -> (u64, Vec<u8>) {
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

    /// Encodes, publishes, and advances the test chain; returns the
    /// slot number.
    pub fn publish(&mut self, braid: BraidId, ops: &[Op], ts: u64) -> u64 {
        let (slot, bytes) = self.encode(braid, ops, ts);
        let key = log_key(&self.prefix, braid, slot);
        assert!(matches!(
            self.store.put_create(&key, &bytes).expect("publish slot"),
            Create::Created(_)
        ));
        let head = self.heads.get_mut(&braid).expect("known braid");
        head.g = slot;
        head.hash = *blake3::hash(&bytes).as_bytes();
        head.ts = ts.max(head.ts);
        slot
    }
}

/// Crashes exactly once, at the (allow+1)-th occurrence of `step`, then
/// disables itself so recovery and later commits run clean.
pub struct CrashOnce {
    step: WriterStep,
    remaining: AtomicU64,
}

impl CrashOnce {
    pub fn new(step: WriterStep, allow: u64) -> Self {
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

/// What the racing store plants: distinct notes (fully disjoint — the
/// `SlotRace` shape), or distinct bookings under one shared venue
/// parent, sized `base_units + seq` so the fixture prices exactly when
/// the ceiling convicts the loser's re-judgment (the `HotKey` shape).
#[derive(Clone, Copy)]
pub enum Competitor {
    Notes,
    Bookings { venue: u64, base_units: u64 },
}

struct RacerState {
    braid: BraidId,
    remaining: AtomicU64,
    seq: AtomicU64,
    head: Mutex<Head>,
    kind: Competitor,
    codec: Codec,
    prefix: String,
}

/// Wraps `FsStore` and, while armed, wins every `put_create` on the
/// target braid's log keys by planting a chain-valid competitor batch
/// first — the deterministic tool for driving the loss path to the
/// contention bound.
pub struct RacingStore {
    inner: FsStore,
    state: std::sync::Arc<RacerState>,
}

/// The test-side handle for observing and re-arming the racer.
pub struct RacerHandle(std::sync::Arc<RacerState>);

impl RacerHandle {
    pub fn plants(&self) -> u64 {
        self.0.seq.load(Ordering::SeqCst)
    }

    pub fn disarm(&self) {
        self.0.remaining.store(0, Ordering::SeqCst);
    }

    pub fn arm(&self, plants: u64) {
        self.0.remaining.store(plants, Ordering::SeqCst);
    }

    /// Walks the actual log to the braid's tip so competitor batches
    /// chain onto slots published while the racer was disarmed.
    pub fn seed_from(&self, root: PathBuf) {
        let store = FsStore::new(root);
        let state = &self.0;
        let mut head = Head {
            g: 0,
            hash: [0u8; 32],
            ts: 0,
        };
        loop {
            let key = log_key(&state.prefix, state.braid, head.g + 1);
            let Some(fetched) = store.get(&key).expect("seed walk") else {
                break;
            };
            let batch = state.codec.decode(&fetched.bytes).expect("seed decode");
            head.g += 1;
            head.hash = *blake3::hash(&fetched.bytes).as_bytes();
            head.ts = batch.header.timestamp;
        }
        *state.head.lock().expect("racer head") = head;
    }
}

impl RacingStore {
    pub fn new(
        root: PathBuf,
        prefix: &str,
        braid: BraidId,
        remaining: u64,
        kind: Competitor,
    ) -> (Self, RacerHandle) {
        let state = std::sync::Arc::new(RacerState {
            braid,
            remaining: AtomicU64::new(remaining),
            seq: AtomicU64::new(0),
            head: Mutex::new(Head {
                g: 0,
                hash: [0u8; 32],
                ts: 0,
            }),
            kind,
            codec: codec(),
            prefix: prefix.to_string(),
        });
        (
            Self {
                inner: FsStore::new(root),
                state: std::sync::Arc::clone(&state),
            },
            RacerHandle(state),
        )
    }

    fn maybe_plant(&self, key: &StoreKey) {
        let state = &self.state;
        let log_prefix = if state.prefix.is_empty() {
            format!("log/{}/", state.braid)
        } else {
            format!("{}/log/{}/", state.prefix, state.braid)
        };
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
            Competitor::Bookings { venue, base_units } => vec![insert(
                BOOKING,
                Box::from([Value::U64(venue), Value::U64(base_units + seq)]),
            )],
        };
        let mut head = self.state.head.lock().expect("racer head");
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
        let slot_key = log_key(&state.prefix, state.braid, head.g + 1);
        assert_eq!(&slot_key, key, "racer plants exactly the contested slot");
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

/// Performs the first log-slot create for real but reports `Exists` —
/// the ambiguous-PUT shape whose resolution is fetch-and-compare.
pub struct AmbiguousOnce {
    inner: FsStore,
    tripped: AtomicU64,
}

impl AmbiguousOnce {
    pub fn new(root: PathBuf) -> Self {
        Self {
            inner: FsStore::new(root),
            tripped: AtomicU64::new(0),
        }
    }
}

impl ObjectStore for AmbiguousOnce {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        let created = self.inner.put_create(key, body)?;
        if (key.as_str().contains("/log/") || key.as_str().starts_with("log/"))
            && let Create::Created(_) = created
            && self.tripped.swap(1, Ordering::SeqCst) == 0
        {
            return Ok(Create::Exists);
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
