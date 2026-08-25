//! Shared fixtures for the replica lane's tests: a two-braid theory, a
//! test-side batch writer that keeps its own chain state, and a
//! checkpoint publisher over `FsStore`.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValidateDescriptor as _, ValueType,
};
use bumbledb::{Db, Theory, Value};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::manifest::{
    Checkpoint, Head, Manifest, ckpt_mdb_key, create_manifest, log_key, manifest_key,
    publish_checkpoint,
};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{Create, ObjectStore};

pub const RECIPE: RelationId = RelationId(0);
pub const STEP: RelationId = RelationId(1);
pub const NOTE: RelationId = RelationId(2);

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("lane_d_{tag}_{}_{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Two braids: `recipe` and `step` joined by a containment (every
/// step's recipe id must exist), and `note` alone.
pub fn theory() -> SchemaDescriptor {
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
        ],
    }
}

pub fn codec() -> Codec {
    let descriptor = theory();
    let schema = descriptor.clone().validate().expect("fixture validates");
    let fingerprint = schema_fingerprint(&schema).0;
    Codec::new(&descriptor, fingerprint)
}

pub fn fingerprint() -> [u8; 32] {
    let schema = theory().validate().expect("fixture validates");
    schema_fingerprint(&schema).0
}

pub fn kitchen_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(RECIPE).expect("recipe braid")
}

pub fn note_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(NOTE).expect("note braid")
}

pub fn insert_recipe(id: u64) -> Op {
    Op {
        kind: OpKind::Insert,
        relation: RECIPE,
        rows: vec![Box::from([Value::U64(id)])],
    }
}

pub fn insert_step(recipe: u64, name: &str) -> Op {
    Op {
        kind: OpKind::Insert,
        relation: STEP,
        rows: vec![Box::from([Value::U64(recipe), Value::String(name.into())])],
    }
}

pub fn insert_note(id: u64, body: &str) -> Op {
    Op {
        kind: OpKind::Insert,
        relation: NOTE,
        rows: vec![Box::from([Value::U64(id), Value::String(body.into())])],
    }
}

/// The test-side writer: encodes and publishes slots while keeping its
/// own per-braid chain state, so tests can drive the log without the
/// writer lane existing.
pub struct TestLog {
    pub store: FsStore,
    pub prefix: String,
    pub codec: Codec,
    pub heads: BTreeMap<BraidId, Head>,
    pub writer: u64,
}

impl TestLog {
    pub fn new(root: PathBuf, prefix: &str) -> Self {
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
            writer: 7,
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

    /// Encodes, publishes, and advances the test chain. Returns the
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

    /// Publishes raw bytes into a slot without the encode discipline —
    /// the tool for planting dishonest objects.
    pub fn publish_raw(&mut self, braid: BraidId, bytes: &[u8], ts: u64) -> u64 {
        let head = self.heads.get_mut(&braid).expect("known braid");
        let slot = head.g + 1;
        let key = log_key(&self.prefix, braid, slot);
        assert!(matches!(
            self.store
                .put_create(&key, bytes)
                .expect("publish raw slot"),
            Create::Created(_)
        ));
        head.g = slot;
        head.hash = *blake3::hash(bytes).as_bytes();
        head.ts = ts.max(head.ts);
        slot
    }

    /// Compacts `db` and publishes it as the current checkpoint under
    /// the checkpoint order, with heads taken from the test chain.
    pub fn checkpoint<T: Theory + Clone>(&self, db: &Db<T>, scratch: &std::path::Path) -> [u8; 32] {
        let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let compact_dir = scratch.join(format!("compact_{seq}"));
        db.compact(&compact_dir).expect("compact");
        let bytes = std::fs::read(compact_dir.join("data.mdb")).expect("compacted store file");
        let prev = self
            .store
            .get(&manifest_key(&self.prefix))
            .expect("manifest get")
            .and_then(|fetched| Manifest::parse(&fetched.bytes).ok()?.checkpoint);
        let doc = Checkpoint {
            braids: self.heads.clone(),
            catalog: db.catalog_digest().expect("catalog digest"),
            writer: self.writer,
            prev,
        };
        let digest = doc.digest();

        let _ = self
            .store
            .put_create(&ckpt_mdb_key(&self.prefix, &digest), &bytes)
            .expect("put checkpoint object");
        let published = publish_checkpoint(&self.store, &self.prefix, self.codec.braids(), &doc)
            .expect("publish checkpoint");
        assert!(matches!(
            published,
            bumbledb_log::manifest::Published::Replaced
        ));
        digest
    }
}
