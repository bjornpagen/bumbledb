//! Conformance lane 6 — PITR, gc, and the vector: a 500-commit
//! multi-braid history with checkpoints every 64 by vector-sum; restore
//! to every recorded vector reproduces its recorded digest; every
//! checkpoint's catalog claim verifies at open and against a
//! replay-reaching store; the backlink chain walks from the manifest
//! with GETs alone; by-time restore maps through publish-clamped
//! timestamps and refuses a non-monotone batch at apply; gc deletes
//! exactly the retention law's set per braid; the 404 duality holds in
//! both directions; a hibernated replica behind a gc'd horizon never
//! serves stale reads as fresh.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValidateDescriptor as _, ValueType,
};
use bumbledb::{Admission, Db, Theory, Value};
use bumbledb_log::apply::{Applied, ApplyRefusal, ChainCause, apply};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::gc::{Gc, Restore, RestoreRefusal, gc, restore_by_time, restore_to_vector};
use bumbledb_log::manifest::{
    Checkpoint, Head, Manifest, Published, ckpt_doc_key, ckpt_mdb_key, create_manifest, log_key,
    manifest_key, publish_checkpoint,
};
use bumbledb_log::replica::{OpenRefusal, Opened, Provenance, Refreshed, Replica, Vector};
use bumbledb_log::sidecar::Chain;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{Create, ObjectStore};
use bumbledb_log::writer::{Options, Writer, WriterOpened};

const RECIPE: RelationId = RelationId(0);
const STEP: RelationId = RelationId(1);
const NOTE: RelationId = RelationId(2);
const VENUE: RelationId = RelationId(3);

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-f6-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Three braids: recipe+step joined by a containment (with a key on
/// recipe), note alone, and venue alone under its own key.
fn theory() -> SchemaDescriptor {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "recipe".into(),
                fields: vec![field("id", ValueType::U64)],
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
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RECIPE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: STEP,
                    projection: Box::from([FieldId(0)]),
                    selection: Box::from([]),
                },
                target: Side {
                    relation: RECIPE,
                    projection: Box::from([FieldId(0)]),
                    selection: Box::from([]),
                },
            },
            StatementDescriptor::Functionality {
                relation: VENUE,
                projection: Box::from([FieldId(0)]),
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

fn insert_recipe(id: u64) -> Op {
    Op {
        kind: OpKind::Insert,
        relation: RECIPE,
        rows: vec![Box::from([Value::U64(id)])],
    }
}

fn insert_note(id: u64, body: &str) -> Op {
    Op {
        kind: OpKind::Insert,
        relation: NOTE,
        rows: vec![Box::from([Value::U64(id), Value::String(body.into())])],
    }
}

/// The test-side publisher: encodes and publishes slots while keeping
/// its own per-braid chain state, so fixtures can drive the log with
/// explicit timestamps.
struct TestLog {
    store: FsStore,
    prefix: String,
    codec: Codec,
    heads: BTreeMap<BraidId, Head>,
    writer: u64,
}

impl TestLog {
    fn new(root: PathBuf, prefix: &str) -> Self {
        let codec = codec();
        let store = FsStore::new(root);
        let manifest = Manifest {
            fingerprint: *codec.fingerprint(),
            checkpoint: None,
        };
        assert!(matches!(
            create_manifest(&store, prefix, &manifest).expect("create manifest"),
            Create::Created(_)
        ));
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
            store,
            prefix: prefix.to_string(),
            codec,
            heads,
            writer: 6006,
        }
    }

    /// Encodes at the head with the publish clamp `max(ts, head.ts)` —
    /// the monotone discipline the codec's apply refuses violations of.
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

    fn publish(&mut self, braid: BraidId, ops: &[Op], ts: u64) -> u64 {
        let (_, bytes) = self.encode(braid, ops, ts);
        self.publish_bytes(braid, &bytes, ts)
    }

    fn publish_bytes(&mut self, braid: BraidId, bytes: &[u8], ts: u64) -> u64 {
        let head = self.heads.get_mut(&braid).expect("known braid");
        let slot = head.g + 1;
        let key = log_key(&self.prefix, braid, slot);
        assert!(matches!(
            self.store.put_create(&key, bytes).expect("publish slot"),
            Create::Created(_)
        ));
        head.g = slot;
        head.hash = *blake3::hash(bytes).as_bytes();
        head.ts = ts.max(head.ts);
        slot
    }

    /// Compacts `db` and publishes it as the current checkpoint under
    /// the checkpoint order, heads taken from the test chain.
    fn checkpoint<T: Theory + Clone>(&self, db: &Db<T>, scratch: &Path) -> [u8; 32] {
        let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let compact_dir = scratch.join(format!("compact_{seq}"));
        db.compact(&compact_dir).expect("compact");
        let bytes = std::fs::read(compact_dir.join("data.mdb")).expect("compacted store file");
        let manifest_bytes = self
            .store
            .get(&manifest_key(&self.prefix))
            .expect("manifest get")
            .expect("manifest exists");
        let manifest = Manifest::parse(&manifest_bytes.bytes).expect("manifest parses");
        let doc = Checkpoint {
            braids: self.heads.clone(),
            catalog: db.catalog_digest().expect("catalog digest"),
            writer: self.writer,
            prev: manifest.checkpoint,
        };
        self.publish_checkpoint_doc(&bytes, &doc)
    }

    fn publish_checkpoint_doc(&self, mdb: &[u8], doc: &Checkpoint) -> [u8; 32] {
        let digest = doc.digest();
        let _ = self
            .store
            .put_create(&ckpt_mdb_key(&self.prefix, &digest), mdb)
            .expect("put checkpoint object");
        let published = publish_checkpoint(&self.store, &self.prefix, self.codec.braids(), doc)
            .expect("publish checkpoint");
        assert!(matches!(published, Published::Replaced));
        digest
    }
}

fn open_replica(root: &Path, dir: &Path) -> Replica<SchemaDescriptor, FsStore> {
    match Replica::open(FsStore::new(root.to_path_buf()), "", dir, theory()).expect("open") {
        Opened::Ready(replica) => *replica,
        Opened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

/// Replays from genesis toward `target` with GETs and `apply` alone —
/// the replay-reaching store of the checkpoint content-claim gate.
fn replay_forward(
    store: &FsStore,
    prefix: &str,
    codec: &Codec,
    db: &Db<SchemaDescriptor>,
    chain: &mut Chain,
    target: &Vector,
) {
    for (braid, goal) in target.iter() {
        while chain.position(braid).g < goal {
            let slot = chain.position(braid).g + 1;
            let fetched = store
                .get(&log_key(prefix, braid, slot))
                .expect("get slot")
                .expect("slot exists");
            match apply(db, chain, codec, braid, slot, &fetched.bytes).expect("apply") {
                Applied::Advanced { .. } | Applied::Absorbed { .. } => {}
                other => panic!("replay stopped at {braid}/{slot}: {other:?}"),
            }
        }
    }
}

fn create_db(dir: &Path) -> Db<SchemaDescriptor> {
    match Db::create(dir, theory()).expect("create") {
        Admission::Accepted(db) => db,
        Admission::Rejected(violations) => panic!("theory rejected: {violations:?}"),
    }
}

/// Walks the checkpoint backlink chain from the manifest with GETs
/// alone; returns `(digest, doc)` newest first, ending at the first
/// document the walk cannot fetch.
fn walk_backlinks(store: &FsStore, prefix: &str, codec: &Codec) -> Vec<([u8; 32], Checkpoint)> {
    let manifest = Manifest::parse(
        &store
            .get(&manifest_key(prefix))
            .expect("get manifest")
            .expect("manifest exists")
            .bytes,
    )
    .expect("manifest parses");
    let mut out = Vec::new();
    let mut cursor = manifest.checkpoint;
    while let Some(digest) = cursor {
        let Some(fetched) = store
            .get(&ckpt_doc_key(prefix, &digest))
            .expect("get checkpoint doc")
        else {
            break;
        };
        let doc = Checkpoint::parse(&fetched.bytes, codec.braids()).expect("doc parses");
        cursor = doc.prev;
        out.push((digest, doc));
    }
    out
}

#[test]
#[allow(clippy::too_many_lines)]
fn five_hundred_commits_restore_to_every_recorded_vector() {
    let root = temp_dir("history");
    let writer_dir = root.join("w");
    let restores = temp_dir("history_restores");
    let store = FsStore::new(root.clone());
    let codec = codec();
    let braids = [
        kitchen_braid(&codec),
        note_braid(&codec),
        venue_braid(&codec),
    ];

    let writer = match Writer::open(
        FsStore::new(root.clone()),
        "",
        &writer_dir,
        theory(),
        Options::new(42),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    writer.set_checkpoint_cadence(64, u64::MAX);

    // 500 commits round-robin over the three braids; after each, the
    // vector and catalog digest are the recorded restore points.
    let mut records: Vec<(Vector, [u8; 32])> = Vec::with_capacity(500);
    for i in 0..500u64 {
        let outcome = writer
            .commit(|batch| {
                match i % 3 {
                    0 => batch.insert(RECIPE, [Box::from([Value::U64(i)]) as Box<[Value]>]),
                    1 => batch.insert(
                        NOTE,
                        [
                            Box::from([Value::U64(i), Value::String(format!("n{i}").into())])
                                as Box<[Value]>,
                        ],
                    ),
                    _ => batch.insert(VENUE, [Box::from([Value::U64(i)]) as Box<[Value]>]),
                }
                Ok(())
            })
            .expect("commit");
        assert!(matches!(outcome, Admission::Accepted(_)));
        writer.quiesce();
        let digest = writer
            .with_db(|db| db.catalog_digest().expect("catalog digest"))
            .expect("db");
        records.push((writer.vector(), digest));
    }
    drop(writer);

    // Checkpoints every 64 by vector-sum: the backlink chain walks from
    // the manifest to every retained checkpoint, GETs alone, and runs
    // out at the first checkpoint's null backlink.
    let walked = walk_backlinks(&store, "", &codec);
    assert_eq!(walked.len(), 7, "500 commits at cadence 64 publish 7");
    assert_eq!(walked.last().expect("first checkpoint").1.prev, None);
    let mut sums: Vec<u64> = walked.iter().map(|(_, doc)| doc.sum()).collect();
    sums.reverse();
    assert_eq!(sums, vec![64, 128, 192, 256, 320, 384, 448]);

    // Every checkpoint's catalog claim verifies at open: a restore to
    // exactly its vector seeds from it, and the seed refuses any claim
    // the opened bytes disagree with.
    for (index, (_, doc)) in walked.iter().enumerate() {
        let target = doc.vector();
        let record = records
            .iter()
            .find(|(vector, _)| *vector == target)
            .expect("every checkpoint vector was recorded");
        let dir = restores.join(format!("ckpt_{index}"));
        let (db, vector) =
            match restore_to_vector(&store, "", &dir, &theory(), &target).expect("restore") {
                Restore::Restored { db, vector } => (db, vector),
                Restore::Refused(refusal) => panic!("checkpoint restore refused: {refusal:?}"),
            };
        assert_eq!(vector, target);
        assert_eq!(
            db.catalog_digest().expect("catalog digest"),
            record.1,
            "the checkpoint claim agrees with the recorded digest"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ... and against a replay-reaching store: one store replays from
    // genesis through every checkpoint vector in order and compares the
    // carried claim at each.
    let replay_dir = restores.join("replay_reach");
    let db = create_db(&replay_dir);
    let mut chain = Chain::genesis(codec.braids());
    for (_, doc) in walked.iter().rev() {
        replay_forward(&store, "", &codec, &db, &mut chain, &doc.vector());
        assert_eq!(
            db.catalog_digest().expect("catalog digest"),
            doc.catalog,
            "a replay-reaching store agrees with the checkpoint claim"
        );
    }
    drop(db);

    // Restore to every recorded vector reproduces its recorded digest.
    for (index, (target, digest)) in records.iter().enumerate() {
        let dir = restores.join("r");
        let (db, vector) =
            match restore_to_vector(&store, "", &dir, &theory(), target).expect("restore") {
                Restore::Restored { db, vector } => (db, vector),
                Restore::Refused(refusal) => panic!("restore {index} refused: {refusal:?}"),
            };
        assert_eq!(vector, *target, "the restored vector is the reported truth");
        assert_eq!(
            db.catalog_digest().expect("catalog digest"),
            *digest,
            "restore {index} reproduces the recorded digest"
        );
        drop(db);
        std::fs::remove_dir_all(&dir).expect("clear restore dir");
        for braid in braids {
            assert!(
                target.braids().any(|id| id == braid),
                "records carry every braid"
            );
        }
    }
}

#[test]
fn no_list_operation_exists_in_the_driver_source() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let needles = [
        "read_dir",
        "ReadDir",
        "walkdir",
        "list_objects",
        "ListObjects",
        "list_keys",
        "fn list",
    ];
    let mut stack = vec![src];
    let mut scanned = 0u64;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read source tree") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read source file");
            for needle in needles {
                assert!(
                    !text.contains(needle),
                    "{} contains `{needle}` — checkpoint discovery is the backlink walk, \
                     GETs alone",
                    path.display()
                );
            }
            scanned += 1;
        }
    }
    assert!(scanned >= 10, "the walk visited the whole driver source");
}

#[test]
fn by_time_restore_maps_through_publish_clamped_timestamps() {
    let root = temp_dir("bytime");
    let local = temp_dir("bytime_local");
    let mut log = TestLog::new(root, "");
    let kitchen = kitchen_braid(&log.codec);

    // Slot 2 arrives with a host clock behind the head; the publish
    // clamp stores it at the head's own timestamp.
    log.publish(kitchen, &[insert_recipe(1)], 3_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_500);
    log.publish(kitchen, &[insert_recipe(3)], 3_500);

    let mapped = |t_ms: u64, tag: &str| -> Vector {
        match restore_by_time(&log.store, "", &local.join(tag), &theory(), t_ms).expect("restore") {
            Restore::Restored { vector, .. } => vector,
            Restore::Refused(refusal) => panic!("by-time restore refused: {refusal:?}"),
        }
    };

    // At 2999 nothing qualifies: the clamped timestamp, not the host's
    // 2500, is what the mapping consults.
    assert_eq!(mapped(2_999, "t2999").at(kitchen), 0);
    // At 3000 both clamped slots qualify at once — the mapped set is a
    // prefix by construction.
    assert_eq!(mapped(3_000, "t3000").at(kitchen), 2);
    assert_eq!(mapped(3_499, "t3499").at(kitchen), 2);
    assert_eq!(mapped(3_500, "t3500").at(kitchen), 3);
}

#[test]
fn a_non_monotone_timestamp_batch_is_refused_at_apply() {
    let root = temp_dir("nonmono");
    let local = temp_dir("nonmono_local");
    let mut log = TestLog::new(root, "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 5_000);

    // Hand-built header undercutting the head timestamp: chain-valid in
    // every other respect, published raw around the encode clamp.
    let head = log.heads[&kitchen];
    let header = BatchHeader {
        fingerprint: *log.codec.fingerprint(),
        braid: kitchen,
        braid_gen: head.g + 1,
        prev: head.hash,
        writer: log.writer,
        timestamp: 4_000,
    };
    let bytes = log
        .codec
        .encode(&header, &[insert_recipe(2)])
        .expect("encode non-monotone batch");
    log.publish_bytes(kitchen, &bytes, 4_000);

    // Refused at apply, with the timestamp cause proved by both values.
    let db = create_db(&local.join("direct"));
    let mut chain = Chain::genesis(log.codec.braids());
    let slot1 = log
        .store
        .get(&log_key("", kitchen, 1))
        .expect("get")
        .expect("slot 1")
        .bytes;
    assert!(matches!(
        apply(&db, &mut chain, &log.codec, kitchen, 1, &slot1).expect("apply"),
        Applied::Advanced { .. }
    ));
    match apply(&db, &mut chain, &log.codec, kitchen, 2, &bytes).expect("apply") {
        Applied::Refused(ApplyRefusal::ChainMismatch {
            cause:
                ChainCause::Timestamp {
                    header_ts,
                    chain_ts,
                },
            writer,
            ..
        }) => {
            assert_eq!(header_ts, 4_000);
            assert_eq!(chain_ts, 5_000);
            assert_eq!(writer, log.writer, "the header convicts its publisher");
        }
        other => panic!("expected the timestamp refusal, got {other:?}"),
    }
    drop(db);

    // By-time restore refuses the same batch at apply instead of
    // mapping around it.
    match restore_by_time(&log.store, "", &local.join("t"), &theory(), 10_000).expect("restore") {
        Restore::Refused(RestoreRefusal::Corrupt(ApplyRefusal::ChainMismatch {
            cause: ChainCause::Timestamp { .. },
            ..
        })) => {}
        Restore::Refused(other) => panic!("wrong refusal: {other:?}"),
        Restore::Restored { .. } => panic!("a non-monotone batch must refuse, not map around"),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn gc_deletes_exactly_the_retention_laws_set_per_braid() {
    let root = temp_dir("gc_exact");
    let local = temp_dir("gc_exact_local");
    let scratch = temp_dir("gc_exact_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    log.publish(kitchen, &[insert_recipe(3)], 4_900);
    log.publish(kitchen, &[insert_recipe(4)], 4_950);
    log.publish(notes, &[insert_note(1, "floor")], 1_000);
    let builder = open_replica(&root, &local.join("b1"));
    let first_digest = log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);
    log.publish(notes, &[insert_note(2, "old but above the floor")], 1_100);

    // Window 1500 at now 5000: eligible = strictly below the floor
    // vector and older than the window. Exactly kitchen slots 1 and 2
    // qualify; kitchen 3 is young, kitchen 4 is the floor slot, the
    // first note slot is its braid's floor, and the second note slot is
    // ancient but above the floor vector — exempt regardless of age.
    let swept = match gc(&log.store, "", &log.codec, 1_500, 5_000).expect("gc") {
        Gc::Swept(sweep) => sweep,
        other => panic!("expected a sweep, got {other:?}"),
    };
    assert_eq!(
        swept.log_deleted,
        vec![
            log_key("", kitchen, 1).to_string(),
            log_key("", kitchen, 2).to_string(),
        ],
        "exactly the law's set, walked upward per braid"
    );
    assert_eq!(swept.checkpoints_deleted, Vec::<[u8; 32]>::new());
    for (braid, slot, expected) in [
        (kitchen, 1, false),
        (kitchen, 2, false),
        (kitchen, 3, true),
        (kitchen, 4, true),
        (notes, 1, true),
        (notes, 2, true),
    ] {
        assert_eq!(
            log.store
                .get(&log_key("", braid, slot))
                .expect("get")
                .is_some(),
            expected,
            "survivor set at {braid}/{slot}"
        );
    }

    // A second sweep under the same clock deletes nothing more: the law
    // names a set, not a process.
    let swept = match gc(&log.store, "", &log.codec, 1_500, 5_000).expect("gc") {
        Gc::Swept(sweep) => sweep,
        other => panic!("expected a sweep, got {other:?}"),
    };
    assert_eq!(swept.log_deleted, Vec::<String>::new());
    assert_eq!(swept.checkpoints_deleted, Vec::<[u8; 32]>::new());

    // A later checkpoint moves the floor; the next sweep takes the
    // newly eligible tails and the superseded checkpoint, and the
    // backlink walk from the manifest truncates at the deleted doc.
    log.publish(kitchen, &[insert_recipe(5)], 9_000);
    log.publish(notes, &[insert_note(3, "third")], 9_000);
    let builder = open_replica(&root, &local.join("b2"));
    let second_digest = log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    let swept = match gc(&log.store, "", &log.codec, 1_500, 100_000).expect("gc") {
        Gc::Swept(sweep) => sweep,
        other => panic!("expected a sweep, got {other:?}"),
    };
    assert_eq!(
        swept.log_deleted,
        vec![
            log_key("", kitchen, 3).to_string(),
            log_key("", kitchen, 4).to_string(),
            log_key("", notes, 1).to_string(),
            log_key("", notes, 2).to_string(),
        ]
    );
    assert_eq!(swept.checkpoints_deleted, vec![first_digest]);
    assert!(
        log.store
            .get(&ckpt_doc_key("", &second_digest))
            .expect("get")
            .is_some(),
        "the current checkpoint is always exempt"
    );

    let walked = walk_backlinks(&log.store, "", &log.codec);
    assert_eq!(walked.len(), 1, "the walk truncates at the swept doc");
    assert_eq!(walked[0].0, second_digest);
    assert_eq!(walked[0].1.prev, Some(first_digest));

    // Behind the swept checkpoint is beyond retention now.
    match restore_to_vector(
        &log.store,
        "",
        &local.join("beyond"),
        &theory(),
        &Vector::from(BTreeMap::from([(kitchen, 4), (notes, 1)])),
    )
    .expect("restore")
    {
        Restore::Refused(RestoreRefusal::BeyondRetention { digest }) => {
            assert_eq!(digest, first_digest);
        }
        Restore::Refused(other) => panic!("wrong refusal: {other:?}"),
        Restore::Restored { .. } => panic!("a gc'd base must refuse"),
    }
}

#[test]
fn the_404_duality_is_pinned_both_directions() {
    let root = temp_dir("duality");
    let local = temp_dir("duality_local");
    let scratch = temp_dir("duality_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    log.publish(notes, &[insert_note(1, "one")], 1_000);

    // Two directories parked below the eventual floor: one two slots
    // behind, one exactly one slot behind.
    let below = local.join("below");
    drop(open_replica(&root, &below));
    log.publish(kitchen, &[insert_recipe(3)], 3_000);
    let at_floor = local.join("at_floor");
    drop(open_replica(&root, &at_floor));
    log.publish(kitchen, &[insert_recipe(4)], 4_000);

    let builder = open_replica(&root, &local.join("b"));
    let floor_digest = log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    // Manufacture 404s below and exactly at the floor vector.
    log.store.delete(&log_key("", kitchen, 3)).expect("delete");
    log.store.delete(&log_key("", kitchen, 4)).expect("delete");

    // Below the floor: the 404 is a hole, never the tip — the stale
    // directory is discarded and the store re-seeds from the current
    // checkpoint instead of serving its old vector as fresh.
    let replica = open_replica(&root, &below);
    assert_eq!(replica.provenance(), Provenance::Checkpoint);
    assert_eq!(replica.vector().at(kitchen), 4);
    drop(replica);

    // Exactly at the floor vector: still a hole (at-or-below).
    let replica = open_replica(&root, &at_floor);
    assert_eq!(replica.provenance(), Provenance::Checkpoint);
    assert_eq!(replica.vector().at(kitchen), 4);

    // Above the floor: the same 404 is the tip. The seeded store probes
    // one past the floor, finds nothing, and serves honestly.
    let mut replica = replica;
    match replica.refresh().expect("refresh") {
        Refreshed::Vector(vector) => assert_eq!(vector.at(kitchen), 4),
        Refreshed::Refused(refusal) => panic!("refresh refused: {refusal:?}"),
    }
    drop(replica);

    // A pre-existing directory already at the floor vector reads the
    // 404 above it as the tip too: no discard, no re-seed.
    let fresh = local.join("fresh");
    let replica = open_replica(&root, &fresh);
    drop(replica);
    let replica = open_replica(&root, &fresh);
    assert_eq!(replica.provenance(), Provenance::LocalDir);
    assert_eq!(replica.vector().at(kitchen), 4);

    // The checkpoint document itself is what decides the split.
    let doc = walk_backlinks(&log.store, "", &log.codec);
    assert_eq!(doc[0].0, floor_digest);
    assert_eq!(doc[0].1.braids[&kitchen].g, 4);
}

#[test]
fn a_hibernated_replica_behind_a_gcd_horizon_never_serves_stale_as_fresh() {
    let root = temp_dir("hibernate");
    let local = temp_dir("hibernate_local");
    let scratch = temp_dir("hibernate_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    let sleeper = local.join("sleeper");
    drop(open_replica(&root, &sleeper));

    for slot in 3..=8u64 {
        log.publish(kitchen, &[insert_recipe(slot)], slot * 1_000);
    }
    log.publish(notes, &[insert_note(1, "awake")], 8_000);
    let builder = open_replica(&root, &local.join("b"));
    let fresh_digest = builder
        .db()
        .expect("db")
        .catalog_digest()
        .expect("catalog digest");
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    // Real retention passes the sleeper's vector: everything below the
    // floor is old enough to die.
    let swept = match gc(&log.store, "", &log.codec, 100, 1_000_000).expect("gc") {
        Gc::Swept(sweep) => sweep,
        other => panic!("expected a sweep, got {other:?}"),
    };
    assert_eq!(swept.log_deleted.len(), 7, "kitchen tail below the floor");

    // Waking up two slots behind a gc'd horizon: the stale directory
    // must not come up serving its old vector as the tip. The open
    // discards it and re-seeds from the current checkpoint.
    let replica = open_replica(&root, &sleeper);
    assert_eq!(replica.provenance(), Provenance::Checkpoint);
    assert_eq!(replica.vector().at(kitchen), 8);
    assert_eq!(replica.vector().at(notes), 1);
    assert_eq!(
        replica
            .db()
            .expect("db")
            .catalog_digest()
            .expect("catalog digest"),
        fresh_digest,
        "the woken store serves the fresh state, whole"
    );
    replica
        .db()
        .expect("db")
        .read(|instance| {
            assert!(instance.contains_dyn(RECIPE, &[Value::U64(8)])?);
            Ok(())
        })
        .expect("read");
}

/// Builds the poisoned compaction: the full honest history replayed,
/// except the last slot of the recipe braid lands through a manual
/// engine write carrying the slot's own ops plus one extra row — the
/// generation matches the vector sum exactly, and only the content
/// differs from the honest state.
fn poisoned_checkpoint(log: &TestLog, dir: &Path, scratch: &Path) -> (Vec<u8>, [u8; 32]) {
    let codec = &log.codec;
    let kitchen = kitchen_braid(codec);
    let db = create_db(dir);
    let mut chain = Chain::genesis(codec.braids());
    let target: Vector = log
        .heads
        .iter()
        .map(|(braid, head)| (*braid, head.g))
        .collect();
    for (braid, goal) in target.iter() {
        let honest_goal = if braid == kitchen { goal - 1 } else { goal };
        while chain.position(braid).g < honest_goal {
            let slot = chain.position(braid).g + 1;
            let fetched = log
                .store
                .get(&log_key(&log.prefix, braid, slot))
                .expect("get slot")
                .expect("slot exists");
            match apply(&db, &mut chain, codec, braid, slot, &fetched.bytes).expect("apply") {
                Applied::Advanced { .. } => {}
                other => panic!("poison replay stopped at {braid}/{slot}: {other:?}"),
            }
        }
    }
    let last = log.heads[&kitchen].g;
    let fetched = log
        .store
        .get(&log_key(&log.prefix, kitchen, last))
        .expect("get slot")
        .expect("slot exists");
    let batch = codec.decode(&fetched.bytes).expect("decode last slot");
    let admission = db
        .write(|tx| {
            for op in &batch.ops {
                match op.kind {
                    OpKind::Insert => {
                        tx.insert_dyn(op.relation, op.rows.iter())?;
                    }
                    OpKind::Delete => {
                        tx.delete_dyn(op.relation, op.rows.iter())?;
                    }
                }
            }
            tx.insert_dyn(
                RECIPE,
                [Box::from([Value::U64(999_999)]) as Box<[Value]>].iter(),
            )?;
            Ok(())
        })
        .expect("engine write");
    assert!(matches!(admission, Admission::Accepted(_)));
    let sum = target.sum().expect("sum");
    assert_eq!(
        db.generation().expect("generation").value(),
        sum,
        "one extra row at the right generation"
    );
    let compact_dir = scratch.join("poison_compact");
    db.compact(&compact_dir).expect("compact");
    let bytes = std::fs::read(compact_dir.join("data.mdb")).expect("compacted store file");
    let catalog = db.catalog_digest().expect("catalog digest");
    (bytes, catalog)
}

#[test]
fn a_lying_checkpoint_with_an_honest_claim_is_refused_at_fresh_open() {
    let root = temp_dir("liar_open");
    let local = temp_dir("liar_open_local");
    let scratch = temp_dir("liar_open_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    log.publish(kitchen, &[insert_recipe(3)], 3_000);
    log.publish(notes, &[insert_note(1, "one")], 1_000);
    log.publish(notes, &[insert_note(2, "two")], 2_000);

    let honest = open_replica(&root, &local.join("honest"));
    let honest_catalog = honest
        .db()
        .expect("db")
        .catalog_digest()
        .expect("catalog digest");
    drop(honest);

    let (poison_bytes, poison_catalog) = poisoned_checkpoint(&log, &local.join("poison"), &scratch);
    assert_ne!(poison_catalog, honest_catalog);
    let liar = 666;
    let doc = Checkpoint {
        braids: log.heads.clone(),
        catalog: honest_catalog,
        writer: liar,
        prev: None,
    };
    let poison_digest = log.publish_checkpoint_doc(&poison_bytes, &doc);

    // Fresh open seeds from the lie: digest and generation both check
    // out, honestly copied heads check out — the catalog claim is the
    // one instrument left, and it refuses, naming the publisher.
    match Replica::open(
        FsStore::new(root.clone()),
        "",
        &local.join("victim"),
        theory(),
    )
    .expect("open")
    {
        Opened::Refused(OpenRefusal::CatalogMismatch {
            digest,
            writer,
            carried,
            computed,
        }) => {
            assert_eq!(digest, poison_digest);
            assert_eq!(writer, liar);
            assert_eq!(carried, honest_catalog);
            assert_eq!(computed, poison_catalog);
        }
        Opened::Refused(other) => panic!("wrong refusal: {other:?}"),
        Opened::Ready(_) => panic!("a lying checkpoint must refuse at fresh open"),
    }

    // The restore path runs the same gauntlet.
    match restore_to_vector(
        &log.store,
        "",
        &local.join("restore"),
        &theory(),
        &Vector::from(BTreeMap::from([(kitchen, 3), (notes, 2)])),
    )
    .expect("restore")
    {
        Restore::Refused(RestoreRefusal::Open(OpenRefusal::CatalogMismatch { writer, .. })) => {
            assert_eq!(writer, liar);
        }
        Restore::Refused(other) => panic!("wrong refusal: {other:?}"),
        Restore::Restored { .. } => panic!("a lying checkpoint must refuse at restore"),
    }
}

#[test]
fn a_self_consistent_lying_checkpoint_is_refused_by_a_replay_reaching_store() {
    let root = temp_dir("liar_replay");
    let local = temp_dir("liar_replay_local");
    let scratch = temp_dir("liar_replay_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    log.publish(kitchen, &[insert_recipe(3)], 3_000);
    log.publish(notes, &[insert_note(1, "one")], 1_000);

    // An honest replica replays the whole log by itself before any
    // checkpoint exists.
    let witness_dir = local.join("witness");
    let mut witness = open_replica(&root, &witness_dir);
    assert_eq!(witness.provenance(), Provenance::Bootstrap);
    assert_eq!(witness.vector().at(kitchen), 3);

    // The lie is self-consistent: the poisoned bytes carry their own
    // catalog digest, so a fresh seed verifies clean. Only a store that
    // reached the same vector by its own replay can convict it.
    let (poison_bytes, poison_catalog) = poisoned_checkpoint(&log, &local.join("poison"), &scratch);
    let liar = 667;
    let doc = Checkpoint {
        braids: log.heads.clone(),
        catalog: poison_catalog,
        writer: liar,
        prev: None,
    };
    let poison_digest = log.publish_checkpoint_doc(&poison_bytes, &doc);

    // The heartbeat adopts the lying floor; the witness stands at
    // exactly its vector and the comparison convicts the publisher.
    witness.set_heartbeat_every(1);
    match witness.refresh().expect("refresh") {
        Refreshed::Refused(OpenRefusal::CatalogMismatch {
            digest,
            writer,
            carried,
            ..
        }) => {
            assert_eq!(digest, poison_digest);
            assert_eq!(writer, liar);
            assert_eq!(carried, poison_catalog);
        }
        Refreshed::Refused(other) => panic!("wrong refusal: {other:?}"),
        Refreshed::Vector(_) => {
            panic!("a replay-reaching store must refuse the lying claim")
        }
    }
    drop(witness);

    // The same conviction lands at open on the witness's surviving
    // directory: catch-up reaches the lying floor's vector and refuses
    // before anything serves.
    match Replica::open(FsStore::new(root), "", &witness_dir, theory()).expect("open") {
        Opened::Refused(OpenRefusal::CatalogMismatch { writer, .. }) => {
            assert_eq!(writer, liar);
        }
        Opened::Refused(other) => panic!("wrong refusal: {other:?}"),
        Opened::Ready(_) => panic!("the open path must run the same comparison"),
    }
}
