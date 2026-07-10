use std::path::Path;
use std::sync::Mutex;

use super::Db;
use crate::error::Result;
use crate::image::cache::ImageCache;
use crate::schema::{Schema, SchemaDef};
use crate::storage::env::Environment;

impl<S: SchemaDef> Db<S> {
    /// Validates the definition's declared schema, initializes a fresh
    /// environment at `path` with its fingerprint, and opens it. The
    /// definition value is the one the `schema!` macro's `pub Name;`
    /// header emits — `Db::create(path, Ledger)` — or a runtime-built
    /// [`crate::schema::SchemaDescriptor`].
    ///
    /// # Errors
    ///
    /// The typed [`crate::error::SchemaError`] on an invalid declaration;
    /// `Io`/`Lmdb` on environment creation failure.
    pub fn create(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Ok(Self::assemble(Environment::create(path, &schema)?, schema))
    }

    /// Opens an existing environment, verifying the format version and
    /// then the schema fingerprint — each mismatch is a typed hard
    /// failure. Production open never destroys data.
    ///
    /// # Errors
    ///
    /// The typed [`crate::error::SchemaError`] on an invalid declaration;
    /// `FormatMismatch`/`SchemaMismatch` on verification failure;
    /// `Io`/`Lmdb` otherwise.
    pub fn open(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Ok(Self::assemble(Environment::open(path, &schema)?, schema))
    }

    fn assemble(env: Environment, schema: Schema) -> Self {
        Self {
            env,
            cache: ImageCache::new(),
            writer: Mutex::new(()),
            writer_thread: std::sync::atomic::AtomicU64::new(0),
            read_cache: Mutex::new(None),
            commit_seq: std::sync::atomic::AtomicU64::new(0),
            schema,
            marker: std::marker::PhantomData,
        }
    }
}
