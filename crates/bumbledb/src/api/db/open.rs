//! Create, open and publish. Create refuses an existing destination and
//! publishes through the store's staged-directory protocol; open verifies
//! family, layout and schema before touching anything.

use std::path::Path;
use std::sync::Arc;

use super::Db;
use crate::error::{Admission, Error, Result};
use crate::image::cache::ImageCache;
use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_final_state};
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::store::{DEFAULT_MAP_CEILING, Store};
use crate::work::WorkContext;

/// Environment options fixed at open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// The virtual map reservation in bytes: address space, not RAM or disk.
    /// A write that needs more pages fails with [`Error::Full`].
    pub map_ceiling: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            map_ceiling: DEFAULT_MAP_CEILING,
        }
    }
}

impl<S: Theory> Db<S> {
    /// Create a new durable database with default [`Options`].
    /// # Errors
    /// As [`Db::create_with`].
    pub fn create(path: &Path, schema: S, work: WorkContext) -> Result<Admission<Self>> {
        Self::create_with(path, schema, Options::default(), work)
    }

    /// Open an existing database with default [`Options`].
    /// # Errors
    /// As [`Db::open_with`].
    pub fn open(path: &Path, schema: S, work: WorkContext) -> Result<Self> {
        Self::open_with(path, schema, Options::default(), work)
    }

    /// Create a new durable database with cooperative cancellation.
    /// The declared theory is judged over the empty state (with its sealed
    /// closed extensions) before any directory is touched; an unsatisfiable
    /// declaration is a rejection, not a store.
    /// # Errors
    /// Schema validation, destination refusals, storage failure, stopped work.
    pub fn create_with(
        path: &Path,
        schema: S,
        options: Options,
        work: WorkContext,
    ) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        match judge_final_state(&schema, &MapState::new(), &work, JudgeBudget::default())
            .map_err(super::violations::judge_refusal)?
        {
            Judgment::Admitted => {}
            Judgment::Rejected(violations) => {
                return Ok(Admission::Rejected(
                    super::violations::violations_from_judged(&schema, violations, &work)?,
                ));
            }
        }
        let (store, _fresh) =
            Store::create(path, &schema, options.map_ceiling).map_err(Error::from_store)?;
        Ok(Admission::Accepted(Self::assemble(store, schema, work)?))
    }

    /// Open an existing database with cooperative cancellation.
    /// Family, layout and schema fingerprint are verified against one read
    /// view before adoption; refusal mutates nothing.
    /// # Errors
    /// Schema validation, recognition/lock refusals, storage failure, stopped work.
    pub fn open_with(path: &Path, schema: S, options: Options, work: WorkContext) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        work.checkpoint()
            .map_err(|error| Error::from_store(crate::storage::store::StoreError::Work(error)))?;
        let store = Store::open(path, &schema, options.map_ceiling).map_err(Error::from_store)?;
        Self::assemble(store, schema, work)
    }

    /// Create without the empty-state admission judgment — grounding-off
    /// test support only; never a production constructor.
    /// # Errors
    /// As [`Db::open`].
    #[cfg(any(test, feature = "ground-off"))]
    pub fn create_store_without_admission(
        path: &Path,
        schema: S,
        work: WorkContext,
    ) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        let (store, _fresh) =
            Store::create(path, &schema, DEFAULT_MAP_CEILING).map_err(Error::from_store)?;
        Self::assemble(store, schema, work)
    }
}

impl<S> Db<S> {
    pub(super) fn assemble(store: Store, schema: Schema, work: WorkContext) -> Result<Self> {
        work.checkpoint()
            .map_err(|error| Error::from_store(crate::storage::store::StoreError::Work(error)))?;
        let schema = Arc::new(schema);
        let closed = Arc::new(super::closed::ClosedRows::build(schema.as_ref(), &work)?);
        let cache = Arc::new(ImageCache::new(schema.as_ref()));
        Ok(Self {
            store,
            schema,
            closed,
            cache,
            marker: std::marker::PhantomData,
        })
    }

    /// Publish an admitted heap instance as a new durable database at
    /// `path` through the staged install protocol: populate and judge in a
    /// private staging directory, then publish atomically.
    ///
    /// ```compile_fail
    /// fn require_builder(path: &std::path::Path, builder: &bumbledb::InstanceBuilder<()>) {
    ///     let _ = bumbledb::Db::from_instance(path, builder);
    /// }
    /// ```
    ///
    /// # Errors
    /// `DestinationExists` if `path` already exists; storage failure
    /// otherwise.
    pub fn from_instance(
        path: &Path,
        instance: &super::OwnedInstance<S>,
        work: WorkContext,
    ) -> Result<Self> {
        let schema = instance.schema().clone();
        let changes = instance.change_set_of_rows(&work)?;
        let store =
            Store::install_populated(path, &schema, DEFAULT_MAP_CEILING, &work, |stage, work| {
                stage.apply(&changes, work)?;
                Ok(())
            })
            .map_err(Error::from_store)?;
        Self::assemble(store, schema, work)
    }
}
