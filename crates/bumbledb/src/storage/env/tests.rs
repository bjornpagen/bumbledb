use super::*;
use crate::error::Error;
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

/// The ephemeral crash contract (R18), clean side: the dirty marker is
/// set while the session lives, a clean close (the handle's drop)
/// clears it, and contents survive the clean handoff.
#[test]
fn a_clean_ephemeral_close_clears_the_marker_and_contents_survive() {
    let dir = TempDir::new("env-ephemeral-clean-close");
    let schema = schema();
    let marker = dirty_marker_path(dir.path());
    let raw = |env: &Environment, k: &[u8], v: &[u8]| {
        let mut wtxn = env.write_txn().expect("txn");
        let data = env.data();
        data.put(wtxn.raw_mut(), k, v).expect("put");
        wtxn.commit().expect("commit");
    };
    {
        let env = Environment::ephemeral(dir.path(), &schema).expect("create ephemeral");
        assert!(
            marker.try_exists().expect("probe"),
            "the marker is set for the session's whole life"
        );
        raw(&env, b"Zprobe", b"alive");
    }
    assert!(
        !marker.try_exists().expect("probe"),
        "a clean close clears the marker"
    );
    let env = Environment::ephemeral(dir.path(), &schema).expect("clean reopen");
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(
        env.data().get(rtxn.raw(), b"Zprobe").expect("get"),
        Some(&b"alive"[..]),
        "contents survive a clean process handoff"
    );
}

/// The ephemeral crash contract (R18), crash side: a set marker at
/// reopen — power loss or a process death that never reached clean
/// close — wipes the store and re-initializes it. The possibly-torn
/// store is never opened; reopening after a crash yields a valid empty
/// store, always.
#[test]
fn a_marker_set_reopen_wipes_and_reinitializes_the_ephemeral_store() {
    let dir = TempDir::new("env-ephemeral-crash-reopen");
    let schema = schema();
    {
        let env = Environment::ephemeral(dir.path(), &schema).expect("create ephemeral");
        let mut wtxn = env.write_txn().expect("txn");
        let data = env.data();
        data.put(wtxn.raw_mut(), b"Zprobe", b"doomed").expect("put");
        wtxn.commit().expect("commit");
    }
    // Plant the crash state: the marker a dead session leaves behind
    // (drop never ran, or its sync was never proven).
    std::fs::File::create(dirty_marker_path(dir.path())).expect("plant marker");
    // The wipe covers `lock.mdb` too (directory mode — the lockfile a
    // NOSUBDIR env would call `data.mdb-lock`): a torn lockfile that
    // survived the wipe would fail every reopen with `MDB_INVALID`
    // under a live reader's shared lock, permanently. Removal is
    // observable as a fresh inode — LMDB recreates the file on open.
    use std::os::unix::fs::MetadataExt;
    let lockfile = dir.path().join("lock.mdb");
    let torn_ino = std::fs::metadata(&lockfile).expect("stat lock.mdb").ino();
    let env = Environment::ephemeral(dir.path(), &schema).expect("crash reopen");
    assert_ne!(
        std::fs::metadata(&lockfile).expect("stat lock.mdb").ino(),
        torn_ino,
        "the wipe removed the possibly-torn lock.mdb — the reopen minted a fresh one"
    );
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(
        env.data().get(rtxn.raw(), b"Zprobe").expect("get"),
        None,
        "the wipe left a valid EMPTY store — the crash contract's whole extent"
    );
    assert_eq!(
        rtxn.generation().expect("generation").value(),
        0,
        "re-initialized from birth"
    );
}

/// Durable stores never mint a dirty marker — the crash contract is the
/// ephemeral kind's alone.
#[test]
fn a_durable_store_never_mints_a_dirty_marker() {
    let dir = TempDir::new("env-durable-no-marker");
    let schema = schema();
    let marker = dirty_marker_path(dir.path());
    {
        let env = Environment::create(dir.path(), &schema).expect("create");
        assert!(!marker.try_exists().expect("probe"));
        drop(env);
    }
    assert!(!marker.try_exists().expect("probe"));
    drop(Environment::open(dir.path(), &schema).expect("open"));
    assert!(!marker.try_exists().expect("probe"));
}

/// The lock law is a writer law (R17): exhume opens `MDB_RDONLY`, takes
/// no advisory lock, and reads without any write permission — the
/// archival lane on read-only media. The fixture removes the writer's
/// lock file and drops the write bits a lock-taking open would need
/// (LMDB's own reader table stays writable — a chmod fixture cannot
/// spell EROFS; on a genuinely read-only FILESYSTEM mdb.c omits the
/// lockfile under `MDB_RDONLY`, `mdb_env_setup_locks`).
#[test]
fn exhume_takes_no_lock_and_reads_without_write_permission() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("env-exhume-readonly");
    let schema = schema();
    drop(Environment::create(dir.path(), &schema).expect("create"));
    std::fs::remove_file(dir.path().join("bumbledb.lock")).expect("remove writer lock");
    let set = |p: &std::path::Path, mode: u32| {
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).expect("chmod");
    };
    set(&dir.path().join("data.mdb"), 0o444);
    set(dir.path(), 0o555);
    let exhumed = Environment::exhume(dir.path());
    // Restore before asserting so the TempDir cleans up either way.
    set(dir.path(), 0o755);
    set(&dir.path().join("data.mdb"), 0o644);
    let exhumed = exhumed.expect("exhume works on read-only media, lockless");
    assert!(
        !dir.path()
            .join("bumbledb.lock")
            .try_exists()
            .expect("probe"),
        "no advisory lock was created — the lock law is a writer law"
    );
    assert_eq!(exhumed.kind, StoreKind::Durable);
    let rtxn = exhumed.env.read_txn().expect("read snapshot");
    assert_eq!(rtxn.generation().expect("generation").value(), 0);
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

/// Marker matrix row 5 — the DOUBLE fault: a pre-v5 store (version 4,
/// no kind key) that also lacks the v5 database roster (`_data`/
/// `_dict`). BOTH constructors refuse `FormatMismatch { found: 4 }`:
/// the probe and `verify_and_open` share ONE check precedence —
/// version, kind, roster, fingerprint — so the version check precedes
/// the roster check everywhere (a pre-v5 store's database layout is
/// not this version's to judge; convicting it of corruption would
/// misname a merely-old store, and the two constructors would name
/// different corruption for the same bytes).
#[test]
fn a_v4_store_without_the_database_roster_is_a_format_mismatch_on_both_constructors() {
    let dir = TempDir::new("env-marker-v4-no-roster");
    {
        // A raw LMDB environment holding ONLY a backdated `_meta`: the
        // doubly-faulted shape no bumbledb constructor can produce
        // (initialize creates the roster in one atomic commit) but a
        // damaged or forged store can carry.
        let env = open_env::open_env(dir.path(), open_env::OpenLane::Write(StoreKind::Durable))
            .expect("raw fixture env");
        let mut wtxn = env.write_txn().expect("txn");
        let meta = env
            .create_database::<heed::types::Bytes, heed::types::Bytes>(&mut wtxn, Some("_meta"))
            .expect("create _meta only");
        meta.put(&mut wtxn, META_FORMAT_VERSION, &4u32.to_le_bytes())
            .expect("backdate version");
        wtxn.commit().expect("commit forgery");
    }
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

/// The foreign-env non-mutation lock (the twin of the durable-store
/// byte-identity test in `tests/ephemeral.rs`): `Environment::ephemeral`
/// probed against someone else's LMDB environment refuses
/// `AlreadyInitialized` and leaves the foreign `data.mdb` byte-identical
/// — the probe fires the refusal before the ephemeral-flagged reopen
/// ever holds an environment the refusal protects.
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
/// must refuse `SchemaMismatch` BEFORE the ephemeral-flagged reopen
/// holds the file. The refusal leaves the file byte-identical.
#[test]
fn ephemeral_schema_mismatch_refusal_leaves_the_data_file_byte_identical() {
    let dir = TempDir::new("env-ephemeral-fingerprint-untouched");
    // A small data file carrying the ephemeral kind: forge the kind on a
    // durable-created store (the compacted-ephemeral shape, built
    // without ever running the ephemeral constructor under test).
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

    // Length via metadata first: any length change must fail loudly,
    // never by reading a grown file into the byte compare.
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
/// ephemeral-flagged reopen — byte-identical, same as every other
/// refusal.
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

/// Mis-sized meta values are the malformed-value corruption NAMING the
/// key — never `MetaMissing` (one decode discipline for every `_meta`
/// value, the split the store-kind matrix above pins; ruled 2026-07-23,
/// R18). The two states point at opposite remedies, so one error value
/// never encodes both.
#[test]
fn a_mis_sized_meta_value_is_malformed_never_missing() {
    use crate::error::CorruptionError;
    // Truncated format version — first in the check precedence, so both
    // constructors diagnose it.
    let dir = TempDir::new("env-malformed-version");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_FORMAT_VERSION, &[5u8, 0, 0])
            .expect("truncate version");
    });
    for err in [
        Environment::open(dir.path(), &schema()).unwrap_err(),
        Environment::ephemeral(dir.path(), &schema()).unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::MalformedValue("format version"))
            ),
            "{err:?}"
        );
    }
    // Truncated fingerprint (durable open — the ephemeral constructor
    // refuses the kind first).
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
    // Truncated tx id — open verifies other keys, so the first
    // generation read raises the diagnosis.
    let dir = TempDir::new("env-malformed-txid");
    forge_meta(&dir, |env, wtxn| {
        env.meta
            .put(wtxn, META_TX_ID, &[1u8; 7])
            .expect("truncate tx id");
    });
    let env = Environment::open(dir.path(), &schema()).expect("open verifies other keys");
    let err = env.read_txn().expect("txn").generation().unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MalformedValue("tx id"))
        ),
        "{err:?}"
    );
}

/// The half-created store (empty root, no `_meta` — the crash window
/// between environment creation and the meta commit) is classified ONCE
/// (`read_meta::classify_meta_block`; ruled 2026-07-23, R18): open
/// refuses it with the typed `NotInitialized` — never `Corruption` —
/// and the ephemeral constructor treats the same state as fresh.
#[test]
fn a_half_created_store_is_not_initialized_on_open_and_fresh_on_ephemeral() {
    let dir = TempDir::new("env-half-created-taxonomy");
    {
        let env = super::open_env::open_env(
            dir.path(),
            super::open_env::OpenLane::Write(StoreKind::Durable),
        )
        .expect("raw env");
        let wtxn = env.write_txn().expect("txn");
        wtxn.commit().expect("commit nothing");
    }
    let err = Environment::open(dir.path(), &schema()).unwrap_err();
    assert!(matches!(err, Error::NotInitialized), "{err:?}");
    // The same never-born state initializes fresh under the ephemeral
    // constructor (its create-or-open contract).
    drop(Environment::ephemeral(dir.path(), &schema()).expect("fresh init"));
}
