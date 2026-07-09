use super::*;
use crate::error::Error;
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
};
use crate::testutil::TempDir;

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "x".into(),
                value_type: ValueType::U64,
                generation: Generation::Serial,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn other_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Other".into(),
            fields: vec![],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

#[test]
fn create_then_open_round_trips() {
    let dir = TempDir::new("env-round-trip");
    let schema = schema();
    {
        let env = Environment::create(dir.path(), &schema).expect("create");
        drop(env);
    }
    Environment::open(dir.path(), &schema).expect("open after create");
}

#[test]
fn create_refuses_an_existing_environment() {
    // Re-initializing `_meta` over live data would reset the tx id and
    // dictionary counter — create must refuse, open must still work.
    let dir = TempDir::new("env-create-refuses");
    let schema = schema();
    drop(Environment::create(dir.path(), &schema).expect("create"));
    let err = Environment::create(dir.path(), &schema).unwrap_err();
    assert!(matches!(err, Error::AlreadyInitialized));
    Environment::open(dir.path(), &schema).expect("open still works");
}

#[test]
fn open_with_different_schema_fails_with_fingerprint_error() {
    let dir = TempDir::new("env-schema-mismatch");
    drop(Environment::create(dir.path(), &schema()).expect("create"));
    let err = Environment::open(dir.path(), &other_schema()).unwrap_err();
    assert!(matches!(err, Error::SchemaMismatch { .. }), "{err:?}");
}

#[test]
fn corrupted_format_version_fails_before_fingerprint() {
    let dir = TempDir::new("env-format-mismatch");
    let schema = schema();
    {
        let env = Environment::create(dir.path(), &schema).expect("create");
        // Corrupt the format version through the private handles.
        let mut wtxn = env.env.write_txn().expect("txn");
        env.meta
            .put(&mut wtxn, META_FORMAT_VERSION, &99u32.to_le_bytes())
            .expect("put");
        wtxn.commit().expect("commit");
    }
    // Open with a *different* schema too: the format error must win —
    // the version check runs before the fingerprint check.
    let err = Environment::open(dir.path(), &other_schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                found: 99,
                expected: FORMAT_VERSION
            }
        ),
        "{err:?}"
    );
}

#[test]
fn generation_is_zero_on_fresh_database() {
    let dir = TempDir::new("env-generation-zero");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let rtxn = env.read_txn().expect("read txn");
    assert_eq!(rtxn.generation().expect("generation"), 0);
}

/// The reader-slot cap is a mechanism, not a promise: >126 concurrent
/// read snapshots — past LMDB's default reader table — open and hold
/// simultaneously under the fixed [`MAX_READERS`]. Threads rendezvous on
/// a barrier so every snapshot is provably live at once, then release.
#[test]
fn holds_more_read_snapshots_than_lmdb_default() {
    const READERS: usize = 160;
    let dir = TempDir::new("env-many-readers");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let barrier = std::sync::Barrier::new(READERS);
    std::thread::scope(|s| {
        for _ in 0..READERS {
            s.spawn(|| {
                let txn = env.read_txn().expect("snapshot within MAX_READERS");
                barrier.wait(); // all 160 slots held at once
                drop(txn);
            });
        }
    });
}

/// The snapshot past the reader table is the typed error naming the
/// limit — cheap to provoke because `MDB_NOTLS` binds slots to
/// transaction objects, so one thread exhausts the table alone.
#[test]
fn the_snapshot_past_the_reader_table_is_a_typed_error() {
    let dir = TempDir::new("env-readers-full");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let mut held = Vec::with_capacity(MAX_READERS as usize);
    for _ in 0..MAX_READERS {
        held.push(env.read_txn().expect("slot within the table"));
    }
    let err = env.read_txn().map(|_| ()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::ReadersFull {
                max_readers: MAX_READERS
            }
        ),
        "{err:?}"
    );
    drop(held);
    // Released slots are reusable: the table was full, not poisoned.
    env.read_txn().expect("snapshot after release");
}

/// A stored `u64::MAX` dictionary counter —
/// the miss sentinel, never mintable — is typed Corruption at the
/// read, not an assert.
#[test]
fn a_corrupt_dict_counter_is_typed_corruption() {
    let dir = TempDir::new("env-corrupt-dict-counter");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let mut wtxn = env.env.write_txn().expect("txn");
        env.meta
            .put(&mut wtxn, META_DICT_NEXT_ID, &u64::MAX.to_le_bytes())
            .expect("plant");
        wtxn.commit().expect("commit");
    }
    let rtxn = env.read_txn().expect("txn");
    let err = rtxn.dict_next_id().unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(crate::error::CorruptionError::MalformedValue(
                "dict next id"
            ))
        ),
        "{err:?}"
    );
    // The write path surfaces the same typed error on the next
    // intern-bearing transaction.
    let view = env.read_txn().expect("txn");
    let mut delta = crate::storage::delta::WriteDelta::new(&schema);
    assert!(matches!(
        delta.intern_str(&view, "novel").unwrap_err(),
        Error::Corruption(crate::error::CorruptionError::MalformedValue(
            "dict next id"
        ))
    ));
}
