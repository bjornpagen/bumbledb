#![allow(clippy::result_large_err)]

use std::path::{Path, PathBuf};
use std::rc::Rc;

use bumbledb_core::query_ir::TypedQuery;
use bumbledb_core::schema::SchemaDescriptor;
use heed::types::Bytes;
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, WithoutTls};

pub(crate) mod base_image;
pub(crate) mod colt;
pub(crate) mod colt_filter;
pub(crate) mod colt_schema;
pub mod diagnostics;
mod error;
pub(crate) mod query;
pub(crate) mod storage_format;
pub(crate) mod storage_v5;
pub(crate) mod tuple;
mod types;

pub use error::{Error, Result};
pub use query::trace::{
    ExecutionModePublic, ProfiledQueryResult, QUERY_TRACING_ENABLED, QueryExecutionOptions,
    QueryTrace, QueryTraceMetadata, TraceCounters, TracePhase, TraceSpan,
};
pub(crate) use types::FactView;
pub use types::{
    DeleteOutcome, Fact, FactRef, InputBindings, InsertOutcome, QueryResultSet, ResultColumn,
    ResultFact, Value, ValueRef,
};

#[cfg(test)]
#[global_allocator]
static TEST_ALLOCATOR: diagnostics::TrackingAllocator<std::alloc::System> =
    diagnostics::TrackingAllocator::system();

pub(crate) type RawDatabase = Database<Bytes, Bytes>;

const DEFAULT_MAP_SIZE: usize = 64 * 1024 * 1024 * 1024;
const DEFAULT_MAX_READERS: u32 = 1024;
const FIXED_DATABASE_COUNT: u32 = 3;
const META_DB: &str = "_meta";
const DATA_DB: &str = "_data";
const DICT_DB: &str = "_dict";
const DATA_FILE: &str = "data.mdb";

#[derive(Clone, Copy)]
pub(crate) struct Databases {
    pub(crate) meta: RawDatabase,
    pub(crate) data: RawDatabase,
    pub(crate) dict: RawDatabase,
}

pub const STORAGE_FORMAT_VERSION: u32 = storage_format::STORAGE_FORMAT_VERSION;

#[derive(Clone, Debug)]
pub struct StorageSchema {
    descriptor: SchemaDescriptor,
    max_key_size: usize,
}

impl StorageSchema {
    pub fn new(descriptor: SchemaDescriptor, max_key_size: usize) -> Result<Self> {
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            max_key_size,
        })
    }

    /// Returns the logical schema descriptor.
    pub fn descriptor(&self) -> &SchemaDescriptor {
        &self.descriptor
    }

    /// Returns the LMDB maximum key size used during schema construction.
    pub fn max_key_size(&self) -> usize {
        self.max_key_size
    }
}

/// Bulk ETL load report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BulkLoadReport {
    /// Number of newly inserted facts.
    pub facts_inserted: usize,
    /// Logical storage transaction ID after load.
    pub storage_tx_id: u64,
    /// Number of dictionary entries after load.
    pub dictionary_entries: usize,
}

/// Embedded LMDB environment shell.
pub struct Environment {
    env: Env<WithoutTls>,
    dbs: Databases,
    path: PathBuf,
}

impl Environment {
    /// Opens or creates the environment directory.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let had_data_file = path.join(DATA_FILE).exists();
        std::fs::create_dir_all(path)?;
        let mut options = EnvOpenOptions::new().read_txn_without_tls();
        options
            .map_size(DEFAULT_MAP_SIZE)
            .max_dbs(FIXED_DATABASE_COUNT)
            .max_readers(DEFAULT_MAX_READERS);
        // SAFETY: The target directory exists and LMDB options are fully initialized.
        let env = unsafe { options.open(path)? };
        let dbs = Self::open_databases(&env, had_data_file)?;
        Ok(Self {
            env,
            dbs,
            path: path.to_path_buf(),
        })
    }

    /// Opens an environment and validates the supplied schema shell.
    pub fn open_with_schema(path: impl AsRef<Path>, schema: &StorageSchema) -> Result<Self> {
        let env = Self::open(path)?;
        env.verify_schema(schema)?;
        Ok(env)
    }

    /// Creates a new database and bulk-loads facts.
    pub fn bulk_load_new(
        path: impl AsRef<Path>,
        schema: &StorageSchema,
        facts: impl IntoIterator<Item = Fact>,
    ) -> Result<(Self, BulkLoadReport)> {
        let env = Self::open_with_schema(path, schema)?;
        let report = env.bulk_load(schema, facts)?;
        Ok((env, report))
    }

    /// Returns a conservative LMDB key-size placeholder.
    pub fn max_key_size(&self) -> usize {
        self.env.max_key_size()
    }

    /// Returns the rebuild storage format version.
    pub fn storage_format_version(&self) -> Result<u32> {
        self.read(|txn| txn.storage_format_version())
    }

    /// Verifies or initializes the stored schema fingerprint.
    pub fn verify_schema(&self, schema: &StorageSchema) -> Result<()> {
        self.write(|txn| storage_v5::verify_schema(txn.dbs, &mut txn.txn, schema.descriptor()))
    }

    pub fn bulk_load(
        &self,
        schema: &StorageSchema,
        facts: impl IntoIterator<Item = Fact>,
    ) -> Result<BulkLoadReport> {
        let facts_inserted = self.write(|txn| txn.bulk_load(schema, facts))?;
        self.read(|txn| {
            Ok(BulkLoadReport {
                facts_inserted,
                storage_tx_id: txn.storage_tx_id()?,
                dictionary_entries: txn.dictionary_entry_count()?,
            })
        })
    }

    /// Runs a read closure against a shell read transaction.
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
            base_images: base_image::BaseImageCache::default(),
        };
        f(&read)
    }

    /// Runs a write closure against a shell write transaction.
    pub fn write<T, E>(
        &self,
        f: impl for<'txn> FnOnce(&mut WriteTxn<'txn>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        let txn = self.env.write_txn().map_err(Error::from).map_err(E::from)?;
        let mut write = WriteTxn { txn, dbs: self.dbs };
        match f(&mut write) {
            Ok(value) => {
                let WriteTxn { txn, .. } = write;
                txn.commit().map_err(Error::from).map_err(E::from)?;
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    /// Returns the environment path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn open_databases(env: &Env<WithoutTls>, had_data_file: bool) -> Result<Databases> {
        let mut txn = env.write_txn()?;
        let dbs = Databases {
            meta: env.create_database(&mut txn, Some(META_DB))?,
            data: env.create_database(&mut txn, Some(DATA_DB))?,
            dict: env.create_database(&mut txn, Some(DICT_DB))?,
        };
        storage_v5::init_metadata(dbs, &mut txn, had_data_file)?;
        txn.commit()?;
        Ok(dbs)
    }
}

/// Opaque read transaction shell.
pub struct ReadTxn<'env> {
    pub(crate) txn: RoTxn<'env, WithoutTls>,
    pub(crate) dbs: Databases,
    pub(crate) base_images: base_image::BaseImageCache,
}

impl ReadTxn<'_> {
    pub fn storage_format_version(&self) -> Result<u32> {
        storage_v5::storage_format_version(self)
    }

    pub fn storage_tx_id(&self) -> Result<u64> {
        storage_v5::storage_tx_id(self)
    }

    pub fn dictionary_entry_count(&self) -> Result<usize> {
        storage_v5::dictionary_entry_count(self)
    }

    pub fn execute_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryResultSet> {
        query::executor::execute_query(self, schema, query, inputs)
    }

    /// Executes a query and returns the result with profiling diagnostics.
    pub fn execute_query_profiled(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        options: QueryExecutionOptions,
    ) -> Result<ProfiledQueryResult> {
        query::executor::execute_query_profiled(self, schema, query, inputs, options)
    }

    pub fn relation_fact_count(&self, schema: &StorageSchema, relation: &str) -> Result<u64> {
        storage_v5::relation_fact_count(self, schema, relation)
    }

    #[allow(dead_code)]
    pub(crate) fn relation_base_image(
        &self,
        schema: &StorageSchema,
        relation: &str,
        field_ids: impl IntoIterator<Item = usize>,
    ) -> Result<Rc<base_image::RelationBaseImage>> {
        base_image::relation_base_image(self, schema, relation, field_ids)
    }

    #[allow(dead_code)]
    pub(crate) fn relation_base_image_with_trace(
        &self,
        schema: &StorageSchema,
        relation: &str,
        field_ids: impl IntoIterator<Item = usize>,
        trace: &mut QueryTrace,
    ) -> Result<Rc<base_image::RelationBaseImage>> {
        base_image::relation_base_image_with_trace(self, schema, relation, field_ids, trace)
    }

    pub(crate) fn relation_base_image_filtered_with_trace(
        &self,
        schema: &StorageSchema,
        relation: &str,
        field_ids: impl IntoIterator<Item = usize>,
        filters: &[colt::SourceFilter],
        trace: &mut QueryTrace,
    ) -> Result<Rc<base_image::RelationBaseImage>> {
        base_image::relation_base_image_filtered_with_trace(
            self, schema, relation, field_ids, filters, trace,
        )
    }

    #[cfg(test)]
    pub(crate) fn debug_relation_facts(
        &self,
        schema: &StorageSchema,
        relation: &str,
    ) -> Result<Vec<Fact>> {
        storage_v5::debug_relation_facts(self, schema, relation)
    }
}

/// Opaque write transaction shell.
pub struct WriteTxn<'env> {
    pub(crate) txn: RwTxn<'env>,
    pub(crate) dbs: Databases,
}

impl WriteTxn<'_> {
    pub fn insert(&mut self, schema: &StorageSchema, fact: &Fact) -> Result<InsertOutcome> {
        storage_v5::insert(self, schema, fact)
    }

    pub fn insert_ref(
        &mut self,
        schema: &StorageSchema,
        fact: &FactRef<'_>,
    ) -> Result<InsertOutcome> {
        storage_v5::insert(self, schema, fact)
    }

    pub fn delete(&mut self, schema: &StorageSchema, fact: &Fact) -> Result<DeleteOutcome> {
        storage_v5::delete(self, schema, fact)
    }

    pub fn delete_ref(
        &mut self,
        schema: &StorageSchema,
        fact: &FactRef<'_>,
    ) -> Result<DeleteOutcome> {
        storage_v5::delete(self, schema, fact)
    }

    pub fn bulk_load(
        &mut self,
        schema: &StorageSchema,
        facts: impl IntoIterator<Item = Fact>,
    ) -> Result<usize> {
        storage_v5::bulk_load(self, schema, facts)
    }
}
