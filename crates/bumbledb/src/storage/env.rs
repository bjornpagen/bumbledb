//! LMDB environment lifecycle, `_meta` contents, and transaction wrappers
//! (docs/architecture/40-storage.md). Authority: `docs/architecture/40-storage.md`, `60-api.md`.

use std::path::Path;

use heed::types::Bytes;
use heed::{AnyTls, Database, EnvOpenOptions, RoTxn, RwTxn, WithoutTls};

use crate::error::{CorruptionError, Error, Result};
use crate::schema::fingerprint::{fingerprint, SchemaFingerprint};
use crate::schema::Schema;

/// Storage format version, checked before the schema fingerprint on open.
pub const FORMAT_VERSION: u32 = 0;

/// Fixed map size: comfortably above the 1 GB scale axiom, allocated
/// sparsely by the OS. Not configurable — path-only public surface.
const MAP_SIZE: usize = 4 << 30;

/// `_meta` keys, single-byte.
const META_FORMAT_VERSION: &[u8] = &[0];
const META_FINGERPRINT: &[u8] = &[1];
const META_TX_ID: &[u8] = &[2];
const META_DICT_NEXT_ID: &[u8] = &[3];

/// The LMDB substrate: environment plus the three named databases.
///
/// Durability is LMDB defaults — fsync per commit; `NOSYNC`/`WRITEMAP`/
/// `MAPASYNC` are not expressible through this type.
pub struct Environment {
    env: heed::Env<WithoutTls>,
    meta: Database<Bytes, Bytes>,
    data: Database<Bytes, Bytes>,
    dict: Database<Bytes, Bytes>,
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment").finish_non_exhaustive()
    }
}

/// Opens the raw LMDB environment at `path`.
///
/// This is the single sanctioned `unsafe` block outside `exec::kernel`
/// (the 40-storage doc amendment): `heed 0.22` marks environment opening unsafe because
/// opening one environment path twice in a process is LMDB UB.
#[allow(unsafe_code)]
fn open_env(path: &Path) -> Result<heed::Env<WithoutTls>> {
    // MDB_NOTLS: reader slots belong to transaction objects, not threads —
    // a thread may pin an old snapshot while opening new ones (long-lived
    // readers across commits are a designed-for pattern, 40-storage).
    let mut options = EnvOpenOptions::new().read_txn_without_tls();
    options.map_size(MAP_SIZE).max_dbs(3);
    // SAFETY: bumbledb opens each environment through exactly this function,
    // and heed itself refuses (Error::EnvAlreadyOpened) to open a path that
    // is already open in this process, upholding LMDB's single-open rule.
    let env = unsafe { options.open(path)? };
    Ok(env)
}

impl Environment {
    /// Initializes a fresh environment: creates the three databases and
    /// writes format version, schema fingerprint, and storage tx id 0.
    ///
    /// # Errors
    ///
    /// `Io` on directory creation, `Lmdb` on any LMDB failure.
    pub fn create(path: &Path, schema: &Schema) -> Result<Self> {
        std::fs::create_dir_all(path)?;
        let env = open_env(path)?;
        let mut wtxn = env.write_txn()?;
        // Refuse to re-initialize: rewriting `_meta` over live `_data`
        // would reset the tx id and the dictionary counter — silent
        // corruption from one wrong call. Production create never
        // destroys data any more than production open does.
        if env
            .open_database::<Bytes, Bytes>(&wtxn, Some("_meta"))?
            .is_some()
        {
            return Err(Error::AlreadyInitialized);
        }
        let meta = env.create_database(&mut wtxn, Some("_meta"))?;
        let data = env.create_database(&mut wtxn, Some("_data"))?;
        let dict = env.create_database(&mut wtxn, Some("_dict"))?;
        meta.put(
            &mut wtxn,
            META_FORMAT_VERSION,
            FORMAT_VERSION.to_le_bytes().as_slice(),
        )?;
        meta.put(
            &mut wtxn,
            META_FINGERPRINT,
            fingerprint(schema).0.as_slice(),
        )?;
        meta.put(&mut wtxn, META_TX_ID, 0u64.to_le_bytes().as_slice())?;
        meta.put(&mut wtxn, META_DICT_NEXT_ID, 0u64.to_le_bytes().as_slice())?;
        wtxn.commit()?;
        Ok(Self {
            env,
            meta,
            data,
            dict,
        })
    }

    /// Opens an existing environment, verifying the storage format version
    /// first and the schema fingerprint second — each mismatch is a distinct
    /// hard failure.
    ///
    /// # Errors
    ///
    /// `FormatMismatch`, then `SchemaMismatch`; `Corruption(MetaMissing)` if
    /// the environment lacks bumbledb's databases or meta keys; `Lmdb`
    /// otherwise.
    pub fn open(path: &Path, schema: &Schema) -> Result<Self> {
        let env = open_env(path)?;
        // Database handles opened inside a transaction are private to it
        // until that transaction commits (LMDB dbi semantics): a read txn
        // would invalidate them on drop, so registration goes through a
        // write transaction that commits without writing anything.
        let rtxn = env.write_txn()?;
        let meta: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_meta"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let data: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;

        let found_version = read_u32(&meta, &rtxn, META_FORMAT_VERSION)?;
        if found_version != FORMAT_VERSION {
            return Err(Error::FormatMismatch {
                found: found_version,
                expected: FORMAT_VERSION,
            });
        }
        let stored: [u8; 32] = meta
            .get(&rtxn, META_FINGERPRINT)?
            .and_then(|b| b.try_into().ok())
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let expected = fingerprint(schema);
        if stored != expected.0 {
            return Err(Error::SchemaMismatch {
                found: SchemaFingerprint(stored),
                expected,
            });
        }
        rtxn.commit()?;
        Ok(Self {
            env,
            meta,
            data,
            dict,
        })
    }

    /// Begins a read snapshot.
    ///
    /// # Errors
    ///
    /// `Lmdb` on transaction failure (e.g. reader-slot exhaustion).
    pub fn read_txn(&self) -> Result<ReadTxn<'_>> {
        Ok(ReadTxn {
            env: self,
            txn: self.env.read_txn()?,
            generation: std::cell::OnceCell::new(),
        })
    }

    /// Begins the write transaction (LMDB serializes writers).
    ///
    /// # Errors
    ///
    /// `Lmdb` on transaction failure.
    pub fn write_txn(&self) -> Result<WriteTxn<'_>> {
        Ok(WriteTxn {
            env: self,
            txn: self.env.write_txn()?,
        })
    }
}

impl Environment {
    /// The `_dict` database handle (reader: `storage::dict`).
    pub(crate) fn dict(&self) -> Database<Bytes, Bytes> {
        self.dict
    }

    /// The `_data` database handle (readers: `storage::delta` probes,
    /// `storage::commit`).
    pub(crate) fn data(&self) -> Database<Bytes, Bytes> {
        self.data
    }
}

fn read_u32(meta: &Database<Bytes, Bytes>, rtxn: &RoTxn<'_, AnyTls>, key: &[u8]) -> Result<u32> {
    let bytes: [u8; 4] = meta
        .get(rtxn, key)?
        .and_then(|b| b.try_into().ok())
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(meta: &Database<Bytes, Bytes>, rtxn: &RoTxn<'_, AnyTls>, key: &[u8]) -> Result<u64> {
    let bytes: [u8; 8] = meta
        .get(rtxn, key)?
        .and_then(|b| b.try_into().ok())
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
    Ok(u64::from_le_bytes(bytes))
}

/// A read snapshot over the environment.
pub struct ReadTxn<'env> {
    env: &'env Environment,
    txn: RoTxn<'env, WithoutTls>,
    /// Snapshot-constant by definition (the tx id is read *inside* this
    /// snapshot), so one `_meta` get serves every `generation()` caller —
    /// the cache asks once per occurrence per execution otherwise.
    generation: std::cell::OnceCell<u64>,
}

impl ReadTxn<'_> {
    /// The reader's generation: the storage tx id read from `_meta` *inside
    /// this snapshot* — never an in-process counter. This is the
    /// race-closing rule of `docs/architecture/40-storage.md`; the 40-storage doc keys
    /// the image cache on it.
    ///
    /// # Errors
    ///
    /// `Corruption(MetaMissing)` if the tx-id key is absent or malformed.
    pub fn generation(&self) -> Result<u64> {
        if let Some(g) = self.generation.get() {
            return Ok(*g);
        }
        let g = read_u64(&self.env.meta, &self.txn, META_TX_ID)?;
        Ok(*self.generation.get_or_init(|| g))
    }

    /// The committed dictionary next-id as of this snapshot (reader: the
    /// delta's lazy pending-intern counter).
    pub(crate) fn dict_next_id(&self) -> Result<u64> {
        read_u64(&self.env.meta, &self.txn, META_DICT_NEXT_ID)
    }

    /// The underlying heed transaction (reader: `storage::dict` lookups).
    pub(crate) fn raw(&self) -> &RoTxn<'_, AnyTls> {
        &self.txn
    }

    /// The owning environment (reader: `storage::dict`).
    pub(crate) fn env(&self) -> &Environment {
        self.env
    }
}

/// The write transaction over the environment.
pub struct WriteTxn<'env> {
    env: &'env Environment,
    txn: RwTxn<'env>,
}

impl<'env> WriteTxn<'env> {
    /// Commits (fsync per LMDB defaults).
    ///
    /// # Errors
    ///
    /// `Lmdb` on commit failure; nothing persists.
    pub fn commit(self) -> Result<()> {
        self.txn.commit()?;
        Ok(())
    }

    /// Aborts: drops the transaction, nothing persists.
    pub fn abort(self) {
        drop(self.txn);
    }

    /// Advances the storage tx id (reader: the 40-storage doc's commit step 4; the id
    /// advances iff the delta changed logical state).
    pub(crate) fn put_generation(&mut self, generation: u64) -> Result<()> {
        self.env.meta.put(
            &mut self.txn,
            META_TX_ID,
            generation.to_le_bytes().as_slice(),
        )?;
        Ok(())
    }

    /// Reads the dictionary next-id counter (reader: `storage::dict`'s
    /// direct-write intern, test-only since the delta's pending-intern set
    /// re-homed the live path in the 40-storage doc).
    #[cfg(test)]
    pub(crate) fn dict_next_id(&self) -> Result<u64> {
        read_u64(&self.env.meta, &self.txn, META_DICT_NEXT_ID)
    }

    /// Writes the dictionary next-id counter.
    pub(crate) fn put_dict_next_id(&mut self, next: u64) -> Result<()> {
        self.env.meta.put(
            &mut self.txn,
            META_DICT_NEXT_ID,
            next.to_le_bytes().as_slice(),
        )?;
        Ok(())
    }

    /// The underlying heed transaction (reader: `storage::dict` — LMDB
    /// write transactions read their own writes).
    pub(crate) fn raw(&self) -> &RoTxn<'_, AnyTls> {
        &self.txn
    }

    /// The underlying heed transaction, mutably (reader: `storage::dict`).
    pub(crate) fn raw_mut(&mut self) -> &mut RwTxn<'env> {
        &mut self.txn
    }

    /// The owning environment (reader: `storage::dict`).
    pub(crate) fn env(&self) -> &Environment {
        self.env
    }

    /// The current committed generation as seen by this write transaction.
    ///
    /// # Errors
    ///
    /// `Corruption(MetaMissing)` if the tx-id key is absent or malformed.
    pub fn generation(&self) -> Result<u64> {
        read_u64(&self.env.meta, &self.txn, META_TX_ID)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SchemaError;
    use crate::schema::{
        ConstraintDescriptor, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId,
        SchemaDescriptor, ValueType,
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
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    fn other_schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Other".into(),
                fields: vec![],
                constraints: vec![],
            }],
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

    #[test]
    fn oversized_guard_key_schema_rejected_at_construction() {
        // 62 u64 fields in one unique constraint = 496 bytes > MAX_GUARD_WIDTH.
        let fields: Vec<FieldDescriptor> = (0..62)
            .map(|i| FieldDescriptor {
                name: format!("f{i}").into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            })
            .collect();
        let err = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Wide".into(),
                fields,
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "all".into(),
                    fields: (0..62).map(FieldId).collect(),
                }],
            }],
        }
        .validate()
        .unwrap_err();
        assert_eq!(
            err,
            SchemaError::GuardKeyTooWide {
                relation: RelationId(0),
                constraint: crate::schema::ConstraintId(0),
                width: 496
            }
        );
    }
}
