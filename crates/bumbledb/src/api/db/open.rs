use std::path::Path;
use std::sync::Mutex;

use super::{CommitSeq, Db};
use crate::error::Result;
use crate::image::cache::ImageCache;
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::env::Environment;

impl<S: Theory> Db<S> {
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

    /// Opens or initializes an EPHEMERAL store at `path` — a distinct
    /// constructor, never a flag on `create`/`open`
    /// (`docs/architecture/70-api.md` § environment lifecycle). The
    /// store kind is marked on disk: `Db::open` on an ephemeral store
    /// and `Db::ephemeral` on a durable store are each the typed
    /// [`crate::error::Error::StoreKindMismatch`]. The environment
    /// carries `MDB_WRITEMAP|MDB_NOSYNC`, so commits skip the fullfsync
    /// boundary — a machine crash loses the store BY THE KIND'S OWN
    /// CLAIM (device-independent: ephemeral-on-SSD is legitimate);
    /// process-kill atomicity, the dependency judgment, [`crate::WriteTx`] point
    /// reads, and every other semantic are identical to a durable store.
    /// The sighting: staging stores judged before ETL into a durable
    /// store, analysis working sets, scratch stores.
    ///
    /// A missing or empty directory initializes fresh; an existing
    /// ephemeral store opens with the same version/fingerprint
    /// verification as `open` (create-or-open — scratch stores earn the
    /// convenience because a mistaken fresh store at a typo'd path
    /// destroys nothing durable).
    ///
    /// # Errors
    ///
    /// The typed [`crate::error::SchemaError`] on an invalid
    /// declaration; `StoreKindMismatch` on a durable store;
    /// `AlreadyInitialized` on a foreign LMDB environment;
    /// `FormatMismatch`/`SchemaMismatch` on verification failure;
    /// `EnvironmentLocked`/`Io`/`Lmdb` otherwise.
    pub fn ephemeral(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Ok(Self::assemble(
            Environment::ephemeral(path, &schema)?,
            schema,
        ))
    }

    /// The one handle-construction site (readers: the three constructors
    /// above and the exhume entry, `super::exhume`).
    pub(super) fn assemble(env: Environment, schema: Schema) -> Self {
        Self {
            env,
            cache: ImageCache::new(&schema),
            writer: Mutex::new(()),
            writer_thread: std::sync::atomic::AtomicU64::new(0),
            read_cache: Mutex::new(None),
            commit_seq: std::sync::atomic::AtomicU64::new(CommitSeq::INITIAL.atomic_word()),
            schema,
            marker: std::marker::PhantomData,
        }
    }
}
