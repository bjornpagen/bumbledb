//! Minimal LMDB boundary retained after purging the v4 engine.
//!
//! The old v4 storage and query implementation was deleted so the
//! Free Join paper alignment PRDs can rebuild from a clean substrate. This crate
//! intentionally exposes only stable shell types until the ordered PRDs replace
//! the implementation with v5 storage, GHT/COLT, and formal Free Join execution.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use bumbledb_core::query_ir::TypedQuery;
use bumbledb_core::schema::{SchemaDescriptor, ValueType};

mod error;
pub(crate) mod query;
pub(crate) mod storage_format;

pub use error::{Error, Result};

/// Current on-disk storage format version for the rebuild line.
pub const STORAGE_FORMAT_VERSION: u32 = storage_format::STORAGE_FORMAT_VERSION;

/// Compiled storage schema shell.
#[derive(Clone, Debug)]
pub struct StorageSchema {
    descriptor: SchemaDescriptor,
    max_key_size: usize,
}

impl StorageSchema {
    /// Validates and stores a schema descriptor for future v5 storage work.
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
#[derive(Clone, Debug)]
pub struct Environment {
    path: PathBuf,
}

impl Environment {
    /// Opens or creates the environment directory.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        std::fs::create_dir_all(path.as_ref())?;
        storage_format::open_or_init_format(path.as_ref())?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Opens an environment and validates the supplied schema shell.
    pub fn open_with_schema(path: impl AsRef<Path>, schema: &StorageSchema) -> Result<Self> {
        let _ = schema.descriptor();
        Self::open(path)
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
        let _ = &self.path;
        511
    }

    /// Returns the rebuild storage format version.
    pub fn storage_format_version(&self) -> Result<u32> {
        storage_format::read_format_version(&self.path)
    }

    /// Bulk load is unavailable until PRD 08 rebuilds v5 writes.
    pub fn bulk_load(
        &self,
        _schema: &StorageSchema,
        _facts: impl IntoIterator<Item = Fact>,
    ) -> Result<BulkLoadReport> {
        Err(Error::unavailable("bulk_load", "PRD 08"))
    }

    /// Runs a read closure against a shell read transaction.
    pub fn read<T, E>(
        &self,
        f: impl for<'txn> FnOnce(&ReadTxn<'txn>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        let txn = ReadTxn { _env: PhantomData };
        f(&txn)
    }

    /// Runs a write closure against a shell write transaction.
    pub fn write<T, E>(
        &self,
        f: impl for<'txn> FnOnce(&mut WriteTxn<'txn>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        let mut txn = WriteTxn { _env: PhantomData };
        f(&mut txn)
    }

    /// Returns the environment path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Opaque read transaction shell.
#[derive(Clone, Debug)]
pub struct ReadTxn<'env> {
    _env: PhantomData<&'env ()>,
}

impl ReadTxn<'_> {
    /// Query execution is unavailable until formal Free Join is rebuilt.
    pub fn execute_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        _inputs: &InputBindings,
    ) -> Result<QueryResultSet> {
        let _normalized = query::normalize::normalize_query(schema.descriptor(), query)?;
        Err(Error::unavailable("execute_query", "PRD 12"))
    }

    /// Relation counts are unavailable until PRD 08 rebuilds v5 reads.
    pub fn relation_fact_count(&self, _schema: &StorageSchema, _relation: &str) -> Result<u64> {
        Err(Error::unavailable("relation_fact_count", "PRD 08"))
    }
}

/// Opaque write transaction shell.
#[derive(Clone, Debug)]
pub struct WriteTxn<'env> {
    _env: PhantomData<&'env mut ()>,
}

impl WriteTxn<'_> {
    /// Insert is unavailable until PRD 08 rebuilds v5 writes.
    pub fn insert(&mut self, _schema: &StorageSchema, _fact: Fact) -> Result<InsertOutcome> {
        Err(Error::unavailable("insert", "PRD 08"))
    }

    /// Delete is unavailable until PRD 08 rebuilds v5 writes.
    pub fn delete(&mut self, _schema: &StorageSchema, _fact: Fact) -> Result<DeleteOutcome> {
        Err(Error::unavailable("delete", "PRD 08"))
    }

    /// Bulk load is unavailable until PRD 08 rebuilds v5 writes.
    pub fn bulk_load(
        &mut self,
        _schema: &StorageSchema,
        _facts: impl IntoIterator<Item = Fact>,
    ) -> Result<usize> {
        Err(Error::unavailable("write_bulk_load", "PRD 08"))
    }
}

/// Logical relation fact.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact {
    relation: String,
    values: BTreeMap<String, Value>,
}

impl Fact {
    /// Creates a fact for `relation`.
    pub fn new(
        relation: impl Into<String>,
        values: impl IntoIterator<Item = (impl Into<String>, Value)>,
    ) -> Self {
        Self {
            relation: relation.into(),
            values: values
                .into_iter()
                .map(|(field, value)| (field.into(), value))
                .collect(),
        }
    }

    /// Returns the relation name.
    pub fn relation(&self) -> &str {
        &self.relation
    }

    /// Returns a field value.
    pub fn value(&self, field: &str) -> Option<&Value> {
        self.values.get(field)
    }

    /// Returns all values keyed by field name.
    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }
}

/// Logical storage value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// Boolean.
    Bool(bool),
    /// Unsigned 64-bit integer.
    U64(u64),
    /// Signed 64-bit integer.
    I64(i64),
    /// Database-generated monotonic nominal `u64` sequence value.
    Serial(u64),
    /// Closed enum represented as a stable one-byte code.
    Enum(u8),
    /// UTF-8 string.
    String(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
}

impl Value {
    /// Returns whether this value matches a schema value type.
    pub fn matches_type(&self, value_type: &ValueType) -> bool {
        matches!(
            (self, value_type),
            (Value::Bool(_), ValueType::Bool)
                | (Value::U64(_), ValueType::U64)
                | (Value::I64(_), ValueType::I64)
                | (Value::Serial(_), ValueType::Serial { .. })
                | (Value::Enum(_), ValueType::Enum { .. })
                | (Value::String(_), ValueType::String)
                | (Value::Bytes(_), ValueType::Bytes)
        )
    }
}

/// Result of inserting a set fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    /// The fact was newly inserted.
    Inserted,
    /// The exact fact was already present.
    AlreadyPresent,
}

/// Result of deleting an exact set fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    /// The fact was present and deleted.
    Deleted,
    /// The exact fact was absent.
    Absent,
}

/// Query input bindings keyed by input name without `$`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputBindings {
    values: BTreeMap<String, Value>,
}

impl InputBindings {
    /// Creates empty input bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates bindings from key/value pairs.
    pub fn from_values(values: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        }
    }

    /// Returns a bound input value by name.
    pub fn value(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }
}

/// Result column metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultColumn {
    /// Projected variable.
    Variable(String),
}

/// One fact in a query result set.
pub type ResultFact = Vec<Value>;

/// Duplicate-free query result set in canonical fact order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryResultSet {
    /// Result columns in projection order.
    pub columns: Vec<ResultColumn>,
    /// Result facts in canonical order.
    pub facts: Vec<ResultFact>,
}

impl QueryResultSet {
    /// Builds a canonical result set from possibly unordered facts.
    pub fn new(columns: Vec<ResultColumn>, mut facts: Vec<ResultFact>) -> Self {
        facts.sort();
        facts.dedup();
        Self { columns, facts }
    }

    /// Number of facts in the set.
    pub fn cardinality(&self) -> usize {
        self.facts.len()
    }
}
