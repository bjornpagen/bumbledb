use std::path::Path;
use std::sync::{Arc, Mutex};

use super::{Db, OwnedInstance};
use crate::error::{Admission, Result};
use crate::image::cache::ImageCache;
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::catalog::{HeapStage, admit_catalog};
use crate::storage::env::{Environment, EphemeralClass, PublishCatalog, StoreKind};

/// Complete-roster admission of the empty candidate — the same
/// [`admit_catalog`] path [`super::InstanceBuilder::admit`] uses.
/// Format 8 create and a fresh ephemeral call this before touching
/// `path`.
pub(crate) fn complete_admit_empty(schema: &Schema) -> Result<Admission<()>> {
    Ok(admit_catalog(schema, HeapStage::new(schema))?.map(|_| ()))
}

impl<S: Theory> Db<S> {
    /// Validates the definition's declared schema, complete-admits the
    /// empty candidate, then publishes a fresh format-8 environment
    /// at `path`. The definition value is the one the `schema!` macro's
    /// `pub Name;` header emits — `Db::create(path, Ledger)` — or a
    /// runtime-built [`crate::schema::SchemaDescriptor`].
    ///
    /// Empty that does not satisfy the theory is
    /// [`Admission::Rejected`]: no directory, no lease. A theory that
    /// needs initial facts uses [`super::InstanceBuilder::admit`] then
    /// [`Self::from_instance`].
    ///
    /// # Errors
    ///
    /// The typed [`crate::error::SchemaError`] on an invalid declaration;
    /// `DestinationExists` if `path` already exists; `Io`/`Lmdb` on
    /// environment creation failure.
    pub fn create(path: &Path, schema: S) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        let catalog = match admit_catalog(&schema, HeapStage::new(&schema))? {
            Admission::Accepted(catalog) => catalog,
            Admission::Rejected(violations) => return Ok(Admission::Rejected(violations)),
        };
        let env = Environment::publish(
            path,
            StoreKind::Durable,
            &PublishCatalog::frozen(&catalog, &schema),
        )?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::TraceArgs::Count(2),
        );
        Ok(Admission::Accepted(Self::assemble(env, schema)?))
    }

    /// Opens an existing environment, verifying format 8, store kind,
    /// roster, fingerprint, and descriptor — each mismatch is a typed
    /// hard failure. Production open never destroys data. Every earlier
    /// format is [`crate::error::Error::FormatMismatch`].
    ///
    /// # Errors
    ///
    /// The typed [`crate::error::SchemaError`] on an invalid declaration;
    /// `FormatMismatch`/`SchemaMismatch` on verification failure;
    /// `Io`/`Lmdb` otherwise.
    pub fn open(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Self::assemble(Environment::open(path, &schema)?, schema)
    }

    /// Opens or initializes an EPHEMERAL store at `path`. Fresh
    /// initialize and wipe-and-reinit complete-admit empty before any
    /// mutation — rejected empty is [`Admission::Rejected`] and mutates
    /// nothing. A published admitted format-8 instance reopens as
    /// [`Admission::Accepted`] without re-admitting empty.
    ///
    /// # Errors
    ///
    /// The typed [`crate::error::SchemaError`] on an invalid
    /// declaration; `StoreKindMismatch` on a durable store;
    /// `DestinationExists` on a foreign empty directory;
    /// `FormatMismatch`/`SchemaMismatch` on verification failure;
    /// `EnvironmentLocked`/`Io`/`Lmdb` otherwise.
    pub fn ephemeral(path: &Path, schema: S) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        match Environment::ephemeral_gated(path, &schema, |class| match class {
            EphemeralClass::Fresh | EphemeralClass::CrashVictim => complete_admit_empty(&schema),
            EphemeralClass::ExistingVerified => Ok(Admission::Accepted(())),
        })? {
            Admission::Rejected(violations) => Ok(Admission::Rejected(violations)),
            Admission::Accepted(env) => Ok(Admission::Accepted(Self::assemble(env, schema)?)),
        }
    }

    /// Sweeper-fixture birth: writes format 8 without complete-admit.
    /// Public [`Self::create`] refuses a theory whose empty does not hold.
    /// The bench crate's incremental-fence pins use this through the
    /// `ground-off` test-support feature.
    ///
    /// # Errors
    ///
    /// The typed [`crate::error::SchemaError`] on an invalid declaration;
    /// `DestinationExists` if `path` already exists; `Io`/`Lmdb` on
    /// environment creation failure.
    #[cfg(any(test, feature = "ground-off"))]
    pub fn create_store_without_admission(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Self::assemble(Environment::create(path, &schema)?, schema)
    }
}

impl<S> Db<S> {
    /// The one handle-construction site (readers: the constructors
    /// above and the exhume entry, `super::exhume`).
    pub(super) fn assemble(env: Environment, schema: Schema) -> Result<Self> {
        let generation = env.read_txn()?.generation()?;
        let schema = Arc::new(schema);
        Ok(Self {
            env,
            cache: ImageCache::new(schema.as_ref()),
            writer: Mutex::new(()),
            writer_thread: std::sync::atomic::AtomicU64::new(0),
            read_cache: Mutex::new(None),
            generation: std::sync::atomic::AtomicU64::new(generation.storage_word()),
            schema,
            scratch: Mutex::new(None),
            marker: std::marker::PhantomData,
        })
    }

    /// Raw-copies an admitted [`OwnedInstance`] into a new durable
    /// format-8 store at `path`. One write transaction: every `_data`
    /// and `_dict` byte, plus a freshly synthesized six-key `_meta`.
    /// Does not re-judge, reinsert facts, mint row ids, or re-intern
    /// strings. Images are not copied.
    ///
    /// ```compile_fail
    /// fn require_builder(path: &std::path::Path, builder: &bumbledb::InstanceBuilder<()>) {
    ///     let _ = bumbledb::Db::from_instance(path, builder);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// `DestinationExists` if `path` already exists, including as an
    /// empty directory; `PublishedButUnsynced` if rename succeeded but
    /// the parent dirent sync failed; `Io`/`Lmdb` otherwise.
    pub fn from_instance(path: &Path, instance: &OwnedInstance<S>) -> Result<Self> {
        let env = Environment::publish(
            path,
            StoreKind::Durable,
            &PublishCatalog::frozen(instance.catalog(), instance.schema()),
        )?;
        Self::assemble(env, instance.schema().clone())
    }

    /// Raw-copies an admitted [`OwnedInstance`] into a new ephemeral
    /// format-8 store at `path`. Same catalog copy as [`Self::from_instance`];
    /// `_meta` records [`StoreKind::Ephemeral`] and the dirty-marker
    /// lifecycle arms only after the destination is complete.
    ///
    /// # Errors
    ///
    /// As [`Self::from_instance`].
    pub fn ephemeral_from_instance(path: &Path, instance: &OwnedInstance<S>) -> Result<Self> {
        let env = Environment::publish(
            path,
            StoreKind::Ephemeral,
            &PublishCatalog::frozen(instance.catalog(), instance.schema()),
        )?;
        Self::assemble(env, instance.schema().clone())
    }
}

#[cfg(test)]
mod tests;
