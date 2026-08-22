use super::*;
use crate::error::{Error, Mismatch};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
};

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
fn a_fresh_store_is_format_8() {
    assert_eq!(FORMAT_VERSION, 8, "format ledger v8 is the only window");
    let dir = TempDir::new("env-format-8-birth");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let rtxn = env.read_txn().expect("read");
    let found = env
        .meta
        .get(rtxn.raw(), META_FORMAT_VERSION)
        .expect("get")
        .expect("format version present");
    assert_eq!(found, 8u32.to_le_bytes());
    assert_eq!(
        u32::from_le_bytes(found.try_into().expect("u32")),
        FORMAT_VERSION
    );
}

#[test]
fn create_refuses_an_existing_environment() {

    let dir = TempDir::new("env-create-refuses");
    let schema = schema();
    drop(Environment::create(dir.path(), &schema).expect("create"));
    let err = Environment::create(dir.path(), &schema).unwrap_err();
    assert!(matches!(err, Error::DestinationExists { .. }));
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
    let Error::SchemaMismatch { mismatch } = err else {
        panic!("expected fingerprint mismatch, got {err:?}");
    };
    assert_eq!(mismatch.witnessed.0, [0xA5; 32]);
    assert_eq!(
        mismatch.required,
        crate::schema::fingerprint::fingerprint(&schema)
    );
}

#[test]
fn corrupted_format_version_fails_before_fingerprint() {
    let dir = TempDir::new("env-format-mismatch");
    let schema = schema();
    {
        let env = Environment::create(dir.path(), &schema).expect("create");
        let mut wtxn = env.env.write_txn().expect("txn");
        env.meta
            .put(&mut wtxn, META_FORMAT_VERSION, &99u32.to_le_bytes())
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let err = Environment::open(dir.path(), &other_schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: 99,
                    required: FORMAT_VERSION,
                },
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
                barrier.wait();
                drop(txn);
            });
        }
    });
}

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
    env.read_txn().expect("snapshot after release");
}

fn forge_meta(dir: &TempDir, forge: impl FnOnce(&Environment, &mut heed::RwTxn)) {
    let env = Environment::create(dir.path(), &schema()).expect("create fixture store");
    let mut wtxn = env.env.write_txn().expect("txn");
    forge(&env, &mut wtxn);
    wtxn.commit().expect("commit forgery");
}

#[test]
fn a_v4_store_is_a_format_mismatch() {
    let dir = TempDir::new("env-marker-v4");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_FORMAT_VERSION, &4u32.to_le_bytes())
            .expect("backdate version");
    });
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: 4,
                    required: FORMAT_VERSION,
                },
            }
        ),
        "{err:?}"
    );
}

/// The capacity-cutover refusal (format v7, ruled 2026-07-24): a pre-cutover
/// store refuses on every open surface with the typed `FormatMismatch { found:
/// 7 }`.
#[test]
fn a_format_7_store_is_a_format_mismatch_on_every_open_surface() {
    let dir = TempDir::new("env-marker-v7-pre-admission");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_FORMAT_VERSION, &7u32.to_le_bytes())
            .expect("backdate version to format 7");
    });
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: 7,
                    required: FORMAT_VERSION,
                },
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_pre_cutover_v6_store_is_a_format_mismatch() {
    let dir = TempDir::new("env-marker-v6-pre-cutover");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_FORMAT_VERSION, &6u32.to_le_bytes())
            .expect("backdate version to the pre-cutover format");
    });
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: 6,
                    required: FORMAT_VERSION,
                },
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_v4_store_without_the_database_roster_is_a_format_mismatch() {
    let dir = TempDir::new("env-marker-v4-no-roster");
    std::fs::create_dir_all(dir.path()).expect("mkdir");
    {
        let env =
            open_env::open_env(dir.path(), open_env::OpenLane::Write).expect("raw fixture env");
        let mut wtxn = env.write_txn().expect("txn");
        let meta = env
            .create_database::<heed::types::Bytes, heed::types::Bytes>(&mut wtxn, Some("_meta"))
            .expect("create _meta only");
        meta.put(&mut wtxn, META_FORMAT_VERSION, &4u32.to_le_bytes())
            .expect("backdate version");
        wtxn.commit().expect("commit forgery");
    }
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: 4,
                    required: FORMAT_VERSION,
                },
            }
        ),
        "{err:?}"
    );
}

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
    let view = env.read_txn().expect("txn");
    let mut delta = crate::storage::delta::WriteDelta::new(&schema);
    assert!(matches!(
        delta.intern_str(&view, "novel").unwrap_err(),
        Error::Corruption(crate::error::CorruptionError::MalformedValue(
            "dict next id"
        ))
    ));
}

#[test]
fn a_mis_sized_meta_value_is_malformed_never_missing() {
    use crate::error::CorruptionError;
    let dir = TempDir::new("env-malformed-version");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_FORMAT_VERSION, &[5u8, 0, 0])
            .expect("truncate version");
    });
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MalformedValue("format version"))
        ),
        "{err:?}"
    );
    let dir = TempDir::new("env-malformed-fingerprint");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_FINGERPRINT, &[0xABu8; 31])
            .expect("truncate fingerprint");
    });
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MalformedValue("schema fingerprint"))
        ),
        "{err:?}"
    );
    let dir = TempDir::new("env-malformed-txid");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_TX_ID, &[1u8; 7])
            .expect("truncate tx id");
    });
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MalformedValue("tx id"))
        ),
        "{err:?}"
    );
}

#[test]
fn a_half_created_store_is_not_initialized_on_open() {
    let dir = TempDir::new("env-half-created-taxonomy");
    std::fs::create_dir_all(dir.path()).expect("mkdir");
    {
        let env = super::open_env::open_env(dir.path(), super::open_env::OpenLane::Write)
            .expect("raw env");
        let wtxn = env.write_txn().expect("txn");
        wtxn.commit().expect("commit nothing");
    }
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(matches!(err, Error::AlreadyInitialized), "{err:?}");
}
