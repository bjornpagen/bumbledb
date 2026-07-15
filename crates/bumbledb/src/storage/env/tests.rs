use super::*;
use crate::error::Error;
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
};
use crate::testutil::TempDir;

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "x".into(),
                value_type: ValueType::U64,
                generation: Generation::Fresh,
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
            extension: None,
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
fn corrupted_stored_fingerprint_names_found_and_expected_images() {
    // Healthy sibling: the same schema and untouched metadata reopen cleanly.
    let control_dir = TempDir::new("env-fingerprint-corrupt-control");
    let schema = schema();
    drop(Environment::create(control_dir.path(), &schema).expect("create control"));
    drop(Environment::open(control_dir.path(), &schema).expect("open control"));

    let dir = TempDir::new("env-fingerprint-corrupt");
    {
        let env = Environment::create(dir.path(), &schema).expect("create");
        let mut wtxn = env.env.write_txn().expect("txn");
        env.meta
            .put(&mut wtxn, META_FINGERPRINT, &[0xA5; 32])
            .expect("perturb fingerprint");
        wtxn.commit().expect("commit");
    }
    let err = Environment::open(dir.path(), &schema).unwrap_err();
    let Error::SchemaMismatch { found, expected } = err else {
        panic!("expected fingerprint mismatch, got {err:?}");
    };
    assert_eq!(found.0, [0xA5; 32]);
    assert_eq!(expected, crate::schema::fingerprint::fingerprint(&schema));
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
    assert_eq!(rtxn.generation().expect("generation").value(), 0);
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

/// Forges the `_meta` of a freshly created durable store at `dir` and
/// closes it: the marker-matrix fixture (a stale or hostile store on
/// disk, no live handle).
fn forge_meta(dir: &TempDir, forge: impl FnOnce(&Environment, &mut heed::RwTxn)) {
    let env = Environment::create(dir.path(), &schema()).expect("create fixture store");
    let mut wtxn = env.env.write_txn().expect("txn");
    forge(&env, &mut wtxn);
    wtxn.commit().expect("commit forgery");
}

/// The stale/forged marker matrix (the fixit record: the reviewer's
/// probes, made permanent). Row 1 — a pre-v5 store (version 4, no kind
/// key, exactly what a v4 store looks like): BOTH constructors refuse
/// `FormatMismatch { found: 4 }` — the version check runs before the
/// kind check, so no silent kind adoption is reachable.
#[test]
fn a_v4_store_without_a_kind_key_is_a_format_mismatch_on_both_constructors() {
    let dir = TempDir::new("env-marker-v4-no-kind");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_FORMAT_VERSION, &4u32.to_le_bytes())
            .expect("backdate version");
        env.meta
            .delete(wtxn, META_STORE_KIND)
            .expect("delete kind key");
    });
    for err in [
        Environment::open(dir.path(), &schema()).unwrap_err(),
        Environment::ephemeral(dir.path(), &schema()).unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                Error::FormatMismatch {
                    found: 4,
                    expected: FORMAT_VERSION
                }
            ),
            "{err:?}"
        );
    }
}

/// Marker matrix row 2 — a v5 store with the kind key DELETED: the key
/// is absent, so BOTH constructors refuse `Corruption(MetaMissing)` —
/// never silent adoption of either kind.
#[test]
fn a_v5_store_with_the_kind_key_deleted_is_meta_missing_on_both_constructors() {
    let dir = TempDir::new("env-marker-kind-deleted");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .delete(wtxn, META_STORE_KIND)
            .expect("delete kind key");
    });
    for err in [
        Environment::open(dir.path(), &schema()).unwrap_err(),
        Environment::ephemeral(dir.path(), &schema()).unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                Error::Corruption(crate::error::CorruptionError::MetaMissing)
            ),
            "{err:?}"
        );
    }
}

/// Marker matrix row 3 — a garbage kind byte (7, which no kind encodes
/// to): the key is PRESENT but undecodable, so BOTH constructors refuse
/// `Corruption(StoreKindInvalid)` — corrupt data, not a missing key,
/// and never silent adoption.
#[test]
fn a_garbage_kind_byte_is_store_kind_invalid_on_both_constructors() {
    let dir = TempDir::new("env-marker-garbage-kind");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_STORE_KIND, &[7u8])
            .expect("plant garbage kind");
    });
    for err in [
        Environment::open(dir.path(), &schema()).unwrap_err(),
        Environment::ephemeral(dir.path(), &schema()).unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                Error::Corruption(crate::error::CorruptionError::StoreKindInvalid)
            ),
            "{err:?}"
        );
    }
}

/// Marker matrix row 4 — a wide kind value (two bytes where the
/// encoding is exactly one): present but undecodable, so BOTH
/// constructors refuse `Corruption(StoreKindInvalid)`.
#[test]
fn a_wide_kind_value_is_store_kind_invalid_on_both_constructors() {
    let dir = TempDir::new("env-marker-wide-kind");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_STORE_KIND, &[0u8, 0u8])
            .expect("plant wide kind");
    });
    for err in [
        Environment::open(dir.path(), &schema()).unwrap_err(),
        Environment::ephemeral(dir.path(), &schema()).unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                Error::Corruption(crate::error::CorruptionError::StoreKindInvalid)
            ),
            "{err:?}"
        );
    }
}

/// The foreign-env non-mutation lock (the twin of the durable-store
/// byte-identity test in `tests/ephemeral.rs`): `Environment::ephemeral`
/// probed against someone else's LMDB environment refuses
/// `AlreadyInitialized` and leaves the foreign `data.mdb` byte-identical
/// — the probe runs without `MDB_WRITEMAP`, so the 4 GiB ftruncate never
/// touches an environment the refusal protects.
#[test]
#[expect(
    unsafe_code,
    reason = "building the FOREIGN LMDB environment fixture requires heed's unsafe raw open; the engine under test never runs this code"
)]
fn ephemeral_refusal_on_a_foreign_env_leaves_the_data_file_byte_identical() {
    let dir = TempDir::new("env-ephemeral-foreign-untouched");
    {
        let mut options = heed::EnvOpenOptions::new();
        options.map_size(10 << 20).max_dbs(2);
        // SAFETY: this path is opened by no other handle in the process,
        // and the env is dropped (closed) before the probe below runs.
        let env = unsafe { options.open(dir.path()).expect("foreign env opens") };
        let mut wtxn = env.write_txn().expect("txn");
        let theirs = env
            .create_database::<Bytes, Bytes>(&mut wtxn, Some("theirs"))
            .expect("foreign named db");
        theirs
            .put(&mut wtxn, b"their-key", b"their-value")
            .expect("foreign row");
        wtxn.commit().expect("commit foreign contents");
    }

    let data = dir.path().join("data.mdb");
    let before = std::fs::read(&data).expect("read foreign data.mdb before");
    assert!(
        before.len() < 1 << 30,
        "fixture data file unexpectedly large: {} bytes",
        before.len()
    );

    let err = Environment::ephemeral(dir.path(), &schema()).unwrap_err();
    assert!(matches!(err, Error::AlreadyInitialized), "{err:?}");

    let after = std::fs::read(&data).expect("read foreign data.mdb after");
    assert_eq!(
        before.len(),
        after.len(),
        "the refusal changed the foreign data.mdb's length"
    );
    assert_eq!(
        before, after,
        "the refusal changed the foreign data.mdb's bytes"
    );
}

/// The fingerprint-mismatch non-mutation lock: a store that passes the
/// probe's version and kind checks but carries a DIFFERENT schema
/// fingerprint (the engine's own surface reaches this shape —
/// `Db::compact` of an ephemeral store writes a small `data.mdb` with
/// the ephemeral kind; a host with a skewed schema then reopens it)
/// must refuse `SchemaMismatch` BEFORE the `MDB_WRITEMAP` reopen whose
/// ftruncate would inflate `data.mdb` to the full 4 GiB map. The
/// refusal leaves the file byte-identical.
#[test]
fn ephemeral_schema_mismatch_refusal_leaves_the_data_file_byte_identical() {
    let dir = TempDir::new("env-ephemeral-fingerprint-untouched");
    // A small data file carrying the ephemeral kind: forge the kind on a
    // durable-created store (the compacted-ephemeral shape, without the
    // 4 GiB fixture an ephemeral create would leave behind).
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_STORE_KIND, &[StoreKind::Ephemeral.meta_byte()])
            .expect("mark ephemeral kind");
    });

    let data = dir.path().join("data.mdb");
    let before = std::fs::read(&data).expect("read data.mdb before");
    assert!(
        before.len() < 1 << 30,
        "fixture data file unexpectedly large: {} bytes",
        before.len()
    );

    let err = Environment::ephemeral(dir.path(), &other_schema()).unwrap_err();
    assert!(matches!(err, Error::SchemaMismatch { .. }), "{err:?}");

    // Length via metadata first: a 4 GiB ftruncate must fail loudly, not
    // by allocating 4 GiB into the byte compare.
    let after_len = std::fs::metadata(&data).expect("stat data.mdb after").len();
    assert_eq!(
        before.len() as u64,
        after_len,
        "the refusal changed data.mdb's length"
    );
    let after = std::fs::read(&data).expect("read data.mdb after");
    assert_eq!(before, after, "the refusal changed data.mdb's bytes");
}

/// The fingerprint-missing twin: a v5 store with the ephemeral kind but
/// NO fingerprint key refuses `Corruption(MetaMissing)` before the
/// `MDB_WRITEMAP` reopen — byte-identical, same as every other refusal.
#[test]
fn ephemeral_fingerprint_missing_refusal_leaves_the_data_file_byte_identical() {
    let dir = TempDir::new("env-ephemeral-no-fingerprint-untouched");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_STORE_KIND, &[StoreKind::Ephemeral.meta_byte()])
            .expect("mark ephemeral kind");
        env.meta
            .delete(wtxn, META_FINGERPRINT)
            .expect("delete fingerprint key");
    });

    let data = dir.path().join("data.mdb");
    let before = std::fs::read(&data).expect("read data.mdb before");

    let err = Environment::ephemeral(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(crate::error::CorruptionError::MetaMissing)
        ),
        "{err:?}"
    );

    let after_len = std::fs::metadata(&data).expect("stat data.mdb after").len();
    assert_eq!(
        before.len() as u64,
        after_len,
        "the refusal changed data.mdb's length"
    );
    let after = std::fs::read(&data).expect("read data.mdb after");
    assert_eq!(before, after, "the refusal changed data.mdb's bytes");
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
