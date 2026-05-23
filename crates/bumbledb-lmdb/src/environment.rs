use std::fs;
use std::path::Path;
#[cfg(test)]
use std::sync::Arc;

use heed::types::Bytes;
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, WithoutTls};

use crate::{BulkLoadReport, Error, Fact, QueryImageCache, Result, StorageSchema, failpoints};
#[cfg(test)]
use crate::{QueryImage, QueryImageCacheDiagnostics};

pub(crate) type RawDatabase = Database<Bytes, Bytes>;

const DEFAULT_MAP_SIZE: usize = 64 * 1024 * 1024 * 1024;
const DEFAULT_MAX_READERS: u32 = 1024;
const FIXED_DATABASE_COUNT: u32 = 3;

const META_DB: &str = "_meta";
const INDEX_DB: &str = "_index";
const DICT_DB: &str = "_dict";

const DATA_FILE: &str = "data.mdb";
const STORAGE_FORMAT_VERSION_KEY: &[u8] = b"storage_format_version";
const SCHEMA_FINGERPRINT_KEY: &[u8] = b"schema_fingerprint";

/// Fixed LMDB databases opened by every environment.
#[derive(Clone, Copy)]
pub(crate) struct Databases {
    pub(crate) meta: RawDatabase,
    pub(crate) index: RawDatabase,
    pub(crate) dict: RawDatabase,
}

/// Embedded LMDB environment wrapper.
pub struct Environment {
    env: Env<WithoutTls>,
    dbs: Databases,
    pub(crate) query_images: QueryImageCache,
}

/// Storage diagnostics for the current database snapshot and LMDB environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageDiagnostics {
    /// Schema fingerprint rendered as lowercase hex.
    pub schema_fingerprint: String,
    /// Current storage transaction ID from Bumbledb metadata.
    pub storage_tx_id: u64,
    /// LMDB environment last transaction ID.
    pub lmdb_last_tx_id: usize,
    /// LMDB configured map size in bytes.
    pub lmdb_map_size: usize,
    /// LMDB maximum reader slots.
    pub lmdb_max_readers: u32,
    /// LMDB current reader count.
    pub lmdb_num_readers: u32,
    /// Number of reverse dictionary entries.
    pub dictionary_entries: usize,
    /// Relation diagnostics.
    pub relations: Vec<RelationDiagnostics>,
}

/// Relation-level diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationDiagnostics {
    /// Relation name.
    pub relation: String,
    /// Current fact count.
    pub fact_count: u64,
    /// Index diagnostics for this relation.
    pub indexes: Vec<IndexDiagnostics>,
}

/// Index-level diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexDiagnostics {
    /// Index name.
    pub index: String,
    /// Current entry count.
    pub entry_count: u64,
}

impl Environment {
    /// Opens or creates an LMDB environment at `path`.
    #[tracing::instrument(name = "bumbledb.open", skip_all, fields(path = %path.as_ref().display()))]
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let had_data_file = path.join(DATA_FILE).exists();
        fs::create_dir_all(path)?;

        let mut options = EnvOpenOptions::new().read_txn_without_tls();
        options
            .map_size(DEFAULT_MAP_SIZE)
            .max_dbs(FIXED_DATABASE_COUNT)
            .max_readers(DEFAULT_MAX_READERS);

        // SAFETY: LMDB environment options are fully initialized above and the
        // target directory exists. The returned environment owns the LMDB handle.
        let env = unsafe { options.open(path)? };
        let dbs = Self::open_fixed_databases(&env, had_data_file)?;
        tracing::info!(
            max_readers = DEFAULT_MAX_READERS,
            max_dbs = FIXED_DATABASE_COUNT,
            "opened LMDB environment"
        );

        Ok(Self {
            env,
            dbs,
            query_images: QueryImageCache::default(),
        })
    }

    /// Opens or creates a database and verifies the schema fingerprint.
    ///
    /// If the database has no stored schema fingerprint yet, this writes one. If a
    /// different fingerprint is already stored, open fails without modifying data.
    #[tracing::instrument(name = "bumbledb.open_with_schema", skip_all, fields(path = %path.as_ref().display(), schema = %schema.descriptor().fingerprint()))]
    pub fn open_with_schema(path: impl AsRef<Path>, schema: &StorageSchema) -> Result<Self> {
        let env = Self::open(path)?;
        env.verify_schema(schema)?;
        Ok(env)
    }

    /// Creates a new database and bulk-loads facts as the ETL migration path.
    ///
    /// This refuses to target a path that already contains `data.mdb`; migrations
    /// are explicit ETL into a new database, never in-place upgrades.
    #[tracing::instrument(name = "bumbledb.bulk_load_new", skip_all, fields(path = %path.as_ref().display(), schema = %schema.descriptor().fingerprint()))]
    pub fn bulk_load_new(
        path: impl AsRef<Path>,
        schema: &StorageSchema,
        facts: impl IntoIterator<Item = Fact>,
    ) -> Result<(Self, BulkLoadReport)> {
        let path = path.as_ref();
        let data_path = path.join(DATA_FILE);
        if data_path.exists() {
            return Err(Error::bulk_load_target_exists(data_path));
        }

        let env = Self::open_with_schema(path, schema)?;
        let report = env.bulk_load(schema, facts)?;
        Ok((env, report))
    }

    /// Returns the configured maximum LMDB key size.
    pub fn max_key_size(&self) -> usize {
        self.env.max_key_size()
    }

    /// Returns the maximum reader slots configured for the environment.
    pub fn max_readers(&self) -> u32 {
        self.env.max_readers()
    }

    /// Clears stale LMDB reader slots.
    #[tracing::instrument(name = "bumbledb.reader_cleanup", skip_all)]
    pub fn clear_stale_readers(&self) -> Result<usize> {
        let cleared = self.env.clear_stale_readers()?;
        if cleared > 0 {
            tracing::warn!(cleared, "cleared stale LMDB readers");
        }
        Ok(cleared)
    }

    /// Reads the storage format version from metadata.
    pub fn storage_format_version(&self) -> Result<u32> {
        self.read(|txn| txn.storage_format_version())
    }

    /// Verifies or initializes this database's schema fingerprint.
    #[tracing::instrument(name = "bumbledb.verify_schema", skip_all, fields(schema = %schema.descriptor().fingerprint()))]
    pub fn verify_schema(&self, schema: &StorageSchema) -> Result<()> {
        let expected = schema.descriptor().fingerprint().0;
        self.write(
            |txn| match txn.dbs.meta.get(&txn.txn, SCHEMA_FINGERPRINT_KEY)? {
                Some(found) if found == expected.as_slice() => Ok(()),
                Some(found) => Err(Error::schema_mismatch(hex(&expected), hex(found))),
                None => Ok(txn.dbs.meta.put(
                    &mut txn.txn,
                    SCHEMA_FINGERPRINT_KEY,
                    expected.as_slice(),
                )?),
            },
        )
    }

    /// Bulk-loads facts in one write transaction and returns ETL diagnostics.
    #[tracing::instrument(name = "bumbledb.bulk_load", skip_all, fields(schema = %schema.descriptor().fingerprint()))]
    pub fn bulk_load(
        &self,
        schema: &StorageSchema,
        facts: impl IntoIterator<Item = Fact>,
    ) -> Result<BulkLoadReport> {
        let facts_inserted = self.write(|txn| txn.bulk_load(schema, facts))?;
        self.read(|txn| {
            let report = BulkLoadReport {
                facts_inserted,
                storage_tx_id: txn.last_committed_tx_id()?,
                dictionary_entries: txn.dictionary_entry_count()?,
            };
            tracing::info!(
                facts_inserted = report.facts_inserted,
                storage_tx_id = report.storage_tx_id,
                dictionary_entries = report.dictionary_entries,
                "bulk load committed"
            );
            Ok(report)
        })
    }

    /// Returns storage and LMDB diagnostics without exposing raw LMDB handles.
    pub fn storage_diagnostics(&self, schema: &StorageSchema) -> Result<StorageDiagnostics> {
        let info = self.env.info();
        self.read(|txn| {
            let mut relations = Vec::new();
            for relation in &schema.descriptor().relations {
                let indexes = schema
                    .access_paths(&relation.name)?
                    .into_iter()
                    .map(|path| {
                        Ok(IndexDiagnostics {
                            entry_count: txn.access_entry_count(
                                schema,
                                &relation.name,
                                &path.index_name,
                            )?,
                            index: path.index_name,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                relations.push(RelationDiagnostics {
                    fact_count: txn.relation_fact_count(schema, &relation.name)?,
                    relation: relation.name.clone(),
                    indexes,
                });
            }

            Ok(StorageDiagnostics {
                schema_fingerprint: schema.descriptor().fingerprint().to_string(),
                storage_tx_id: txn.last_committed_tx_id()?,
                lmdb_last_tx_id: info.last_txn_id,
                lmdb_map_size: info.map_size,
                lmdb_max_readers: info.maximum_number_of_readers,
                lmdb_num_readers: info.number_of_readers,
                dictionary_entries: txn.dictionary_entry_count()?,
                relations,
            })
        })
    }

    /// Returns the immutable query image for the latest committed snapshot.
    #[cfg(test)]
    pub(crate) fn query_image(&self, schema: &StorageSchema) -> Result<Arc<QueryImage>> {
        self.read(|txn| self.query_images.get_or_build(txn, schema))
    }

    /// Returns current query image cache diagnostics.
    #[cfg(test)]
    pub(crate) fn query_image_cache_diagnostics(&self) -> QueryImageCacheDiagnostics {
        self.query_images.diagnostics()
    }

    /// Runs a closure inside a read-only transaction.
    #[tracing::instrument(name = "bumbledb.read_txn", skip_all)]
    pub fn read<T, E>(
        &self,
        f: impl for<'txn> FnOnce(&ReadTxn<'txn>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        let txn = self.env.read_txn().map_err(Error::from).map_err(E::from)?;
        let read = ReadTxn {
            txn,
            dbs: self.dbs,
            query_images: &self.query_images,
        };
        f(&read)
    }

    /// Runs a closure inside a read-write transaction.
    #[tracing::instrument(name = "bumbledb.write_txn", skip_all)]
    pub fn write<T, E>(
        &self,
        f: impl for<'txn> FnOnce(&mut WriteTxn<'txn>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        let txn = self.env.write_txn().map_err(Error::from).map_err(E::from)?;
        let mut write = WriteTxn {
            txn,
            dbs: self.dbs,
            active_tx_id: None,
        };

        match f(&mut write) {
            Ok(value) => {
                let WriteTxn { txn, .. } = write;
                failpoints::check(failpoints::Failpoint::BeforeCommit)?;
                let _span = tracing::debug_span!("bumbledb.commit").entered();
                txn.commit().map_err(Error::from).map_err(E::from)?;
                tracing::debug!("write transaction committed");
                Ok(value)
            }
            Err(error) => {
                tracing::debug!("write transaction aborted");
                Err(error)
            }
        }
    }

    fn open_fixed_databases(env: &Env<WithoutTls>, had_data_file: bool) -> Result<Databases> {
        let _span = tracing::debug_span!("bumbledb.open_fixed_databases").entered();
        let mut txn = env.write_txn()?;
        let dbs = Databases {
            meta: env.create_database(&mut txn, Some(META_DB))?,
            index: env.create_database(&mut txn, Some(INDEX_DB))?,
            dict: env.create_database(&mut txn, Some(DICT_DB))?,
        };

        match read_u32(&dbs.meta, &txn, STORAGE_FORMAT_VERSION_KEY)? {
            Some(crate::STORAGE_FORMAT_VERSION) => {}
            Some(found) => {
                return Err(Error::storage_format_mismatch(
                    crate::STORAGE_FORMAT_VERSION,
                    found,
                ));
            }
            None if had_data_file => return Err(Error::missing_storage_format_version()),
            None => write_u32(
                &dbs.meta,
                &mut txn,
                STORAGE_FORMAT_VERSION_KEY,
                crate::STORAGE_FORMAT_VERSION,
            )?,
        }

        txn.commit()?;
        Ok(dbs)
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Opaque read transaction wrapper.
pub struct ReadTxn<'env> {
    pub(crate) txn: RoTxn<'env, WithoutTls>,
    pub(crate) dbs: Databases,
    pub(crate) query_images: &'env QueryImageCache,
}

impl ReadTxn<'_> {
    /// Reads the storage format version visible to this snapshot.
    pub fn storage_format_version(&self) -> Result<u32> {
        read_u32(&self.dbs.meta, &self.txn, STORAGE_FORMAT_VERSION_KEY)?
            .ok_or_else(Error::missing_storage_format_version)
    }

    #[cfg(test)]
    fn get_meta_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.dbs.meta.get(&self.txn, key)?.map(ToOwned::to_owned))
    }
}

/// Opaque write transaction wrapper.
pub struct WriteTxn<'env> {
    pub(crate) txn: RwTxn<'env>,
    pub(crate) dbs: Databases,
    pub(crate) active_tx_id: Option<u64>,
}

impl WriteTxn<'_> {
    #[cfg(test)]
    fn put_meta_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        Ok(self.dbs.meta.put(&mut self.txn, key, value)?)
    }
}

fn read_u32(db: &RawDatabase, txn: &RoTxn, key: &[u8]) -> Result<Option<u32>> {
    let Some(bytes) = db.get(txn, key)? else {
        return Ok(None);
    };

    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| Error::corrupt("u32 metadata must be four bytes"))?;
    Ok(Some(u32::from_be_bytes(bytes)))
}

fn write_u32(db: &RawDatabase, txn: &mut RwTxn, key: &[u8], value: u32) -> Result<()> {
    let bytes = value.to_be_bytes();
    Ok(db.put(txn, key, &bytes[..])?)
}

#[cfg(test)]
#[path = "environment/tests.rs"]
mod tests;
