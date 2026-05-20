//! Internal LMDB storage boundary for Bumbledb.
//!
//! This crate intentionally keeps all LMDB details behind opaque environment and
//! transaction types. Higher layers should not depend on raw LMDB handles.

pub mod allocation;
pub mod benchmark;
mod error;
#[cfg(feature = "test-failpoints")]
pub mod failpoints;
#[cfg(not(feature = "test-failpoints"))]
mod failpoints;
mod free_join;
mod hash_trie;
mod planner_stats;
mod query;
mod query_image;
mod sorted_trie;
mod storage;
mod storage_schema;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use heed::types::Bytes;
use heed::{CompactionOption, Database, Env, EnvOpenOptions, RoTxn, RwTxn, WithoutTls};

use bumbledb_core::query_ir::TypedQuery;

pub use error::*;
pub use free_join::{
    AccessId, AggregatePlan, AggregateTerm, AtomId, FreeJoinPlan, NodeId, NodeImpl, OutputPlan,
    PayloadDemand, PlanEstimates, PlanNode, ProjectPlan, SubAtom, VarId,
};
pub use hash_trie::{
    HashNode, HashTrieIndex, HashTrieStats, LeafMode, PrefixProbe, PrefixRows, RowSet,
};
pub use planner_stats::PlannerStatsCacheDiagnostics;
pub use query::{
    AllocationPhaseStats, CostKey, InputBindings, InputId, MissingIndexRecommendation,
    NodeRowEstimate, NormAtom, NormAtomField, NormFindTerm, NormInput, NormOperand, NormPredicate,
    NormTerm, NormVar, NormalizedQuery, OptimizerTrace, PlanCandidate, PlanCounters, PlanFamily,
    PredicateId, PreparedQuery, QueryAllocationStats, QueryCountOutput, QueryNodeTiming,
    QueryOutput, QueryPlan, QueryRuntimeKind, QueryTimings, ResultColumn, VariableEstimate,
};
pub use query_image::{
    ColumnImage, EncodedRef, FieldId, FieldImage, FixedColumn, PreparedPlanCacheDiagnostics,
    QueryImage, QueryImageCache, QueryImageCacheDiagnostics, QueryImageKey, QueryImageStats,
    RelationId, RelationImage, RelationIndexImage, RelationStats, RowId, RowRange, RowSetRef,
};
pub use sorted_trie::{
    EncodedOwned, IndexSpec, LinearIter, SortedTrieIndex, SortedTrieIter, TrieFrame, TrieIter,
    TrieLevel, TrieStats,
};
pub use storage::{
    EncodedComponent, FieldValues, IdentityValue, IndexScan, KeyValues, Row, ScanItem, Value,
};
pub use storage_schema::{
    AccessPathDescriptor, BulkLoadReport, ColumnSegmentDescriptor, IndexSegmentDescriptor,
    IndexStatsSummary, SegmentDescriptor, StorageSchema,
};

/// Current on-disk storage format version.
pub const STORAGE_FORMAT_VERSION: u32 = 1;

const DEFAULT_MAP_SIZE: usize = 64 * 1024 * 1024 * 1024;
const DEFAULT_MAX_READERS: u32 = 1024;
const FIXED_DATABASE_COUNT: u32 = 3;

const META_DB: &str = "_meta";
const INDEX_DB: &str = "_index";
const DICT_DB: &str = "_dict";

const DATA_FILE: &str = "data.mdb";
const STORAGE_FORMAT_VERSION_KEY: &[u8] = b"storage_format_version";
const SCHEMA_FINGERPRINT_KEY: &[u8] = b"schema_fingerprint";

type RawDatabase = Database<Bytes, Bytes>;

/// Fixed LMDB databases opened by every environment.
#[derive(Clone, Copy)]
struct Databases {
    meta: RawDatabase,
    index: RawDatabase,
    dict: RawDatabase,
}

/// Embedded LMDB environment wrapper.
pub struct Environment {
    env: Env<WithoutTls>,
    dbs: Databases,
    query_images: QueryImageCache,
    #[expect(
        dead_code,
        reason = "environment path is retained for diagnostics/debugging"
    )]
    path: PathBuf,
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
    /// Number of visible durable relation segments.
    pub visible_segments: usize,
    /// Total bytes stored by visible durable segment columns and indexes.
    pub visible_segment_bytes: usize,
    /// Relation diagnostics.
    pub relations: Vec<RelationDiagnostics>,
}

/// Relation-level diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationDiagnostics {
    /// Relation name.
    pub relation: String,
    /// Current row count.
    pub row_count: u64,
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
            path: path.to_path_buf(),
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

    /// Creates a new database and bulk-loads rows as the ETL migration path.
    ///
    /// This refuses to target a path that already contains `data.mdb`; migrations
    /// are explicit ETL into a new database, never in-place upgrades.
    #[tracing::instrument(name = "bumbledb.bulk_load_new", skip_all, fields(path = %path.as_ref().display(), schema = %schema.descriptor().fingerprint()))]
    pub fn bulk_load_new(
        path: impl AsRef<Path>,
        schema: &StorageSchema,
        rows: impl IntoIterator<Item = Row>,
    ) -> Result<(Self, BulkLoadReport)> {
        let path = path.as_ref();
        let data_path = path.join(DATA_FILE);
        if data_path.exists() {
            return Err(Error::bulk_load_target_exists(data_path));
        }

        let env = Self::open_with_schema(path, schema)?;
        let report = env.bulk_load(schema, rows)?;
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

    /// Bulk-loads rows in one write transaction and returns ETL diagnostics.
    #[tracing::instrument(name = "bumbledb.bulk_load", skip_all, fields(schema = %schema.descriptor().fingerprint()))]
    pub fn bulk_load(
        &self,
        schema: &StorageSchema,
        rows: impl IntoIterator<Item = Row>,
    ) -> Result<BulkLoadReport> {
        let rows_inserted = self.write(|txn| txn.bulk_load(schema, rows))?;
        self.read(|txn| {
            let report = BulkLoadReport {
                rows_inserted,
                storage_tx_id: txn.last_committed_tx_id()?,
                dictionary_entries: txn.dictionary_entry_count()?,
            };
            tracing::info!(
                rows_inserted = report.rows_inserted,
                storage_tx_id = report.storage_tx_id,
                dictionary_entries = report.dictionary_entries,
                "bulk load committed"
            );
            Ok(report)
        })
    }

    /// Copies this database into `target_dir` using LMDB's copy API.
    #[tracing::instrument(name = "bumbledb.backup", skip_all, fields(target = %target_dir.as_ref().display(), compact = false))]
    pub fn backup_to_path(&self, target_dir: impl AsRef<Path>) -> Result<()> {
        self.copy_to_database_dir(target_dir.as_ref(), CompactionOption::Disabled)
    }

    /// Copies this database into `target_dir` with LMDB compaction enabled.
    #[tracing::instrument(name = "bumbledb.compact_copy", skip_all, fields(target = %target_dir.as_ref().display(), compact = true))]
    pub fn compact_copy_to_path(&self, target_dir: impl AsRef<Path>) -> Result<()> {
        self.copy_to_database_dir(target_dir.as_ref(), CompactionOption::Enabled)
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
                            entry_count: txn.index_entry_count(
                                schema,
                                &relation.name,
                                &path.index_name,
                            )?,
                            index: path.index_name,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                relations.push(RelationDiagnostics {
                    row_count: txn.relation_row_count(schema, &relation.name)?,
                    relation: relation.name.clone(),
                    indexes,
                });
            }

            let segments = txn.visible_segments(schema)?;
            let visible_segment_bytes = segments
                .iter()
                .map(|segment| {
                    segment
                        .columns
                        .iter()
                        .map(|column| column.byte_len)
                        .sum::<usize>()
                        + segment
                            .indexes
                            .iter()
                            .map(|index| index.byte_len)
                            .sum::<usize>()
                })
                .sum();

            Ok(StorageDiagnostics {
                schema_fingerprint: schema.descriptor().fingerprint().to_string(),
                storage_tx_id: txn.last_committed_tx_id()?,
                lmdb_last_tx_id: info.last_txn_id,
                lmdb_map_size: info.map_size,
                lmdb_max_readers: info.maximum_number_of_readers,
                lmdb_num_readers: info.number_of_readers,
                dictionary_entries: txn.dictionary_entry_count()?,
                visible_segments: segments.len(),
                visible_segment_bytes,
                relations,
            })
        })
    }

    /// Returns the immutable query image for the latest committed snapshot.
    pub fn query_image(&self, schema: &StorageSchema) -> Result<Arc<QueryImage>> {
        self.read(|txn| self.query_images.get_or_build(txn, schema))
    }

    /// Prepares a typed query for repeated execution against read snapshots.
    pub fn prepare_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
    ) -> Result<PreparedQuery> {
        Ok(PreparedQuery::new(schema, query.clone()))
    }

    /// Returns current query image cache diagnostics.
    pub fn query_image_cache_diagnostics(&self) -> QueryImageCacheDiagnostics {
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
            history_seq: 0,
            defer_relation_segments: false,
            touched_relation_segments: BTreeSet::new(),
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
            Some(STORAGE_FORMAT_VERSION) => {}
            Some(found) => {
                return Err(Error::storage_format_mismatch(
                    STORAGE_FORMAT_VERSION,
                    found,
                ));
            }
            None if had_data_file => return Err(Error::missing_storage_format_version()),
            None => write_u32(
                &dbs.meta,
                &mut txn,
                STORAGE_FORMAT_VERSION_KEY,
                STORAGE_FORMAT_VERSION,
            )?,
        }

        txn.commit()?;
        Ok(dbs)
    }

    fn copy_to_database_dir(&self, target_dir: &Path, compaction: CompactionOption) -> Result<()> {
        fs::create_dir_all(target_dir)?;
        self.env
            .copy_to_path(target_dir.join(DATA_FILE), compaction)?;
        Ok(())
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
    txn: RoTxn<'env, WithoutTls>,
    dbs: Databases,
    query_images: &'env QueryImageCache,
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
    txn: RwTxn<'env>,
    dbs: Databases,
    active_tx_id: Option<u64>,
    history_seq: u32,
    defer_relation_segments: bool,
    touched_relation_segments: BTreeSet<u16>,
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
mod tests {
    use super::*;
    use crate::benchmark::{benchmark_queries, benchmark_rows, benchmark_schema};
    use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, ValueType};

    const MARKER_KEY: &[u8] = b"test_marker";
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn opens_initializes_and_reopens_metadata() -> TestResult {
        let dir = tempfile::tempdir()?;

        let env = Environment::open(dir.path())?;
        assert_eq!(env.storage_format_version()?, STORAGE_FORMAT_VERSION);
        assert_eq!(env.max_readers(), DEFAULT_MAX_READERS);
        assert!(env.max_key_size() > 0);
        drop(env);

        let env = Environment::open(dir.path())?;
        assert_eq!(env.storage_format_version()?, STORAGE_FORMAT_VERSION);
        Ok(())
    }

    #[test]
    fn write_commits_on_success() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;

        env.write(|txn| {
            txn.put_meta_bytes(MARKER_KEY, b"committed")?;
            Ok::<(), Error>(())
        })?;

        let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY))?;
        assert_eq!(marker.as_deref(), Some(&b"committed"[..]));
        Ok(())
    }

    #[test]
    fn write_aborts_on_error() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;

        let result: Result<()> = env.write(|txn| {
            txn.put_meta_bytes(MARKER_KEY, b"aborted")?;
            Err(Error::internal("intentional abort"))
        });

        assert!(result.is_err());
        let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY))?;
        assert_eq!(marker, None);
        Ok(())
    }

    #[test]
    fn read_snapshot_is_stable_across_later_commit() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;

        env.write(|txn| {
            txn.put_meta_bytes(MARKER_KEY, b"before")?;
            Ok::<(), Error>(())
        })?;

        env.read(|read| {
            assert_eq!(
                read.get_meta_bytes(MARKER_KEY)?.as_deref(),
                Some(&b"before"[..])
            );

            env.write(|write| {
                write.put_meta_bytes(MARKER_KEY, b"after")?;
                Ok::<(), Error>(())
            })?;

            assert_eq!(
                read.get_meta_bytes(MARKER_KEY)?.as_deref(),
                Some(&b"before"[..])
            );
            Ok::<(), Error>(())
        })?;

        let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY))?;
        assert_eq!(marker.as_deref(), Some(&b"after"[..]));
        Ok(())
    }

    #[test]
    fn bulk_load_new_matches_row_by_row_results() -> TestResult {
        let rows = benchmark_rows(5);
        let schema = StorageSchema::new(benchmark_schema(), 511)?;

        let row_dir = tempfile::tempdir()?;
        let row_env = Environment::open_with_schema(row_dir.path(), &schema)?;
        row_env.write(|txn| {
            for row in &rows {
                txn.insert(&schema, row.clone())?;
            }
            Ok::<(), Error>(())
        })?;

        let bulk_dir = tempfile::tempdir()?;
        let (bulk_env, report) = Environment::bulk_load_new(bulk_dir.path(), &schema, rows)?;
        assert_eq!(report.rows_inserted, benchmark_rows(5).len());
        assert!(report.dictionary_entries > 0);

        let typed = (benchmark_queries()[0].build)(schema.descriptor())?;
        let query = row_env.prepare_query(&schema, &typed)?;
        let inputs = InputBindings::from_values([
            ("holder", Value::Identity(IdentityValue::Serial(1))),
            (
                "start",
                Value::Timestamp(bumbledb_core::encoding::TimestampMicros(0)),
            ),
            (
                "end",
                Value::Timestamp(bumbledb_core::encoding::TimestampMicros(1000)),
            ),
        ]);
        let row_result = row_env
            .read(|txn| txn.execute_prepared_query(&schema, &query, &inputs))?
            .rows;
        let bulk_result = bulk_env
            .read(|txn| txn.execute_prepared_query(&schema, &query, &inputs))?
            .rows;
        assert_eq!(sorted_rows(row_result), sorted_rows(bulk_result));
        Ok(())
    }

    #[test]
    fn bulk_load_failure_is_atomic() -> TestResult {
        let schema = StorageSchema::new(benchmark_schema(), 511)?;
        let dir = tempfile::tempdir()?;
        let env = Environment::open_with_schema(dir.path(), &schema)?;
        let mut rows = benchmark_rows(2);
        rows.push(rows[0].clone());

        let result = env.bulk_load(&schema, rows);
        assert!(matches!(
            result,
            Err(Error::Constraint(ConstraintError::DuplicateTuple { .. }))
        ));

        let diagnostics = env.storage_diagnostics(&schema)?;
        assert_eq!(diagnostics.storage_tx_id, 0);
        assert_eq!(diagnostics.dictionary_entries, 0);
        let segments = env.read(|txn| txn.visible_segments(&schema))?;
        assert!(segments.is_empty());
        assert!(
            diagnostics
                .relations
                .iter()
                .all(|relation| relation.row_count == 0)
        );
        Ok(())
    }

    #[test]
    fn schema_mismatch_fails_without_destroying_data() -> TestResult {
        let schema = StorageSchema::new(benchmark_schema(), 511)?;
        let dir = tempfile::tempdir()?;
        let env = Environment::open_with_schema(dir.path(), &schema)?;
        env.bulk_load(&schema, benchmark_rows(2))?;
        drop(env);

        let changed = StorageSchema::new(changed_schema(), 511)?;
        assert!(matches!(
            Environment::open_with_schema(dir.path(), &changed),
            Err(Error::Schema(SchemaError::SchemaMismatch { .. }))
        ));

        let env = Environment::open_with_schema(dir.path(), &schema)?;
        let diagnostics = env.storage_diagnostics(&schema)?;
        assert!(
            diagnostics
                .relations
                .iter()
                .any(|relation| relation.relation == "Posting" && relation.row_count > 0)
        );
        Ok(())
    }

    #[test]
    fn backup_and_compact_copy_create_usable_databases() -> TestResult {
        let schema = StorageSchema::new(benchmark_schema(), 511)?;
        let dir = tempfile::tempdir()?;
        let env = Environment::open_with_schema(dir.path(), &schema)?;
        env.bulk_load(&schema, benchmark_rows(4))?;

        let failed = env.bulk_load(&schema, vec![benchmark_rows(1)[0].clone()]);
        assert!(failed.is_err());

        let backup_dir = tempfile::tempdir()?;
        env.backup_to_path(backup_dir.path())?;
        let backup = Environment::open_with_schema(backup_dir.path(), &schema)?;

        let compact_dir = tempfile::tempdir()?;
        env.compact_copy_to_path(compact_dir.path())?;
        let compact = Environment::open_with_schema(compact_dir.path(), &schema)?;

        let typed = (benchmark_queries()[0].build)(schema.descriptor())?;
        let query = env.prepare_query(&schema, &typed)?;
        let inputs = InputBindings::from_values([
            ("holder", Value::Identity(IdentityValue::Serial(1))),
            (
                "start",
                Value::Timestamp(bumbledb_core::encoding::TimestampMicros(0)),
            ),
            (
                "end",
                Value::Timestamp(bumbledb_core::encoding::TimestampMicros(1000)),
            ),
        ]);

        let original = env
            .read(|txn| txn.execute_prepared_query(&schema, &query, &inputs))?
            .rows;
        let backup_rows = backup
            .read(|txn| txn.execute_prepared_query(&schema, &query, &inputs))?
            .rows;
        let compact_rows = compact
            .read(|txn| txn.execute_prepared_query(&schema, &query, &inputs))?
            .rows;

        assert_eq!(sorted_rows(original.clone()), sorted_rows(backup_rows));
        assert_eq!(sorted_rows(original), sorted_rows(compact_rows));
        Ok(())
    }

    #[test]
    fn bulk_load_target_must_be_new_and_large_fixture_reopens() -> TestResult {
        let schema = StorageSchema::new(benchmark_schema(), 511)?;
        let dir = tempfile::tempdir()?;
        let (env, report) = Environment::bulk_load_new(dir.path(), &schema, benchmark_rows(12))?;
        assert!(report.rows_inserted > 50);
        assert!(report.dictionary_entries >= 12);
        drop(env);

        assert!(matches!(
            Environment::bulk_load_new(dir.path(), &schema, benchmark_rows(1)),
            Err(Error::Storage(StorageError::BulkLoadTargetExists { .. }))
        ));

        let env = Environment::open_with_schema(dir.path(), &schema)?;
        let diagnostics = env.storage_diagnostics(&schema)?;
        assert!(diagnostics.lmdb_map_size > 0);
        assert!(diagnostics.storage_tx_id > 0);
        assert!(diagnostics.visible_segments > 0);
        assert!(diagnostics.visible_segment_bytes > 0);
        Ok(())
    }

    fn sorted_rows(mut rows: Vec<Vec<Value>>) -> Vec<Vec<Value>> {
        rows.sort();
        rows
    }

    fn changed_schema() -> bumbledb_core::schema::SchemaDescriptor {
        let mut schema = benchmark_schema();
        schema.relations.push(
            RelationDescriptor::new(
                "Extra",
                vec![FieldDescriptor::new(
                    "id",
                    ValueType::Identity {
                        type_name: "ExtraId".to_owned(),
                        owning_relation: "Extra".to_owned(),
                        allocation: bumbledb_core::schema::IdentityAllocation::Serial,
                    },
                )],
            )
            .with_covering_unique("id", ["id"]),
        );
        schema
    }
}
