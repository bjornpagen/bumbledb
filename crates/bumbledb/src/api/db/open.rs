use std::path::Path;
use std::sync::{Arc, Mutex};

use super::{Db, OwnedInstance};
use crate::error::{Admission, Result};
use crate::image::cache::ImageCache;
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::catalog::{HeapStage, admit_catalog};
use crate::storage::env::{Environment, PublishCatalog};

/// Format 8 create calls this before touching `path`.
#[cfg(test)]
pub(crate) fn complete_admit_empty(schema: &Schema) -> Result<Admission<()>> {
    Ok(admit_catalog(schema, HeapStage::new(schema))?.map(|_| ()))
}

impl<S: Theory> Db<S> {

    /// # Errors

    pub fn create(path: &Path, schema: S) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        let catalog = match admit_catalog(&schema, HeapStage::new(&schema))? {
            Admission::Accepted(catalog) => catalog,
            Admission::Rejected(violations) => return Ok(Admission::Rejected(violations)),
        };
        let env = Environment::publish(path, &PublishCatalog::frozen(&catalog, &schema))?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::TraceArgs::Count(2),
        );
        Ok(Admission::Accepted(Self::assemble(env, schema)?))
    }

    /// # Errors

    pub fn open(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Self::assemble(Environment::open(path, &schema)?, schema)
    }

    /// # Errors

    #[doc(hidden)]
    pub fn create_nosync(path: &Path, schema: S) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        let catalog = match admit_catalog(&schema, HeapStage::new(&schema))? {
            Admission::Accepted(catalog) => catalog,
            Admission::Rejected(violations) => return Ok(Admission::Rejected(violations)),
        };
        let env = Environment::publish_nosync(path, &PublishCatalog::frozen(&catalog, &schema))?;
        Ok(Admission::Accepted(Self::assemble(env, schema)?))
    }

    /// # Errors

    #[doc(hidden)]
    pub fn open_nosync(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Self::assemble(Environment::open_nosync(path, &schema)?, schema)
    }

    /// # Errors

    #[cfg(any(test, feature = "ground-off"))]
    pub fn create_store_without_admission(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        Self::assemble(Environment::create(path, &schema)?, schema)
    }
}

impl<S> Db<S> {

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

    /// ```compile_fail

    /// ```

    /// # Errors

    pub fn from_instance(path: &Path, instance: &OwnedInstance<S>) -> Result<Self> {
        let env = Environment::publish(
            path,
            &PublishCatalog::frozen(instance.catalog(), instance.schema()),
        )?;
        Self::assemble(env, instance.schema().clone())
    }

    /// # Errors

    #[doc(hidden)]
    pub fn from_instance_nosync(path: &Path, instance: &OwnedInstance<S>) -> Result<Self> {
        let env = Environment::publish_nosync(
            path,
            &PublishCatalog::frozen(instance.catalog(), instance.schema()),
        )?;
        Self::assemble(env, instance.schema().clone())
    }
}

#[cfg(test)]
mod tests;
