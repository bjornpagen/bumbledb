use std::path::Path;
use std::sync::Mutex;

use super::Db;
use crate::error::Result;
use crate::image::cache::ImageCache;
use crate::schema::Schema;
use crate::storage::env::Environment;

impl<'s> Db<'s> {
    /// Initializes a fresh environment at `path` with the schema's
    /// fingerprint and opens it.
    ///
    /// # Errors
    ///
    /// `Io`/`Lmdb` on environment creation failure.
    pub fn create(path: &Path, schema: &'s Schema) -> Result<Self> {
        Ok(Self::assemble(Environment::create(path, schema)?, schema))
    }

    /// Opens an existing environment, verifying the format version and
    /// then the schema fingerprint — each mismatch is a typed hard
    /// failure. Production open never destroys data.
    ///
    /// # Errors
    ///
    /// `FormatMismatch`/`SchemaMismatch` on verification failure;
    /// `Io`/`Lmdb` otherwise.
    pub fn open(path: &Path, schema: &'s Schema) -> Result<Self> {
        Ok(Self::assemble(Environment::open(path, schema)?, schema))
    }

    fn assemble(env: Environment, schema: &'s Schema) -> Self {
        Self {
            env,
            cache: ImageCache::new(),
            writer: Mutex::new(()),
            writer_thread: std::sync::atomic::AtomicU64::new(0),
            read_cache: Mutex::new(None),
            commit_seq: std::sync::atomic::AtomicU64::new(0),
            schema,
        }
    }
}
