//! Create, open and publish. Create judges the declared theory over the
//! empty state first, then publishes through a staged directory; open checks
//! format and schema before touching anything.

use std::path::Path;
use std::sync::Arc;

use super::Db;
use crate::error::{Admission, Error, Result};
use crate::image::cache::ImageCache;
use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_final_state};
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::store::{DatabaseId, Options, Store};
use crate::work::WorkContext;

/// Judge the empty state, then create the store.
pub(super) fn create_validated<S>(
    path: &Path,
    schema: Schema,
    database: DatabaseId,
    options: Options,
    work: WorkContext,
) -> Result<Admission<Db<S>>> {
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
    let store = Store::create(path, &schema, database, options).map_err(Error::from_store)?;
    Ok(Admission::Accepted(Db::assemble(store, schema, work)?))
}

impl<S: Theory> Db<S> {
    /// Create a new database with default [`Options`]. An unsatisfiable
    /// theory is a rejection, and no directory is touched.
    /// # Errors
    /// As [`Db::create_with`].
    pub fn create(path: &Path, schema: S, work: WorkContext) -> Result<Admission<Self>> {
        Self::create_with(path, schema, Options::default(), work)
    }

    /// Create a new database at `path`, which must not exist.
    /// # Errors
    /// Schema validation, `DestinationExists`, storage failure, cancellation.
    pub fn create_with(
        path: &Path,
        schema: S,
        options: Options,
        work: WorkContext,
    ) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        create_validated(path, schema, DatabaseId::mint(), options, work)
    }

    /// Open an existing database with default [`Options`].
    /// # Errors
    /// As [`Db::open_with`].
    pub fn open(path: &Path, schema: S, work: WorkContext) -> Result<Self> {
        Self::open_with(path, schema, Options::default(), work)
    }

    /// Open an existing database; a refusal mutates nothing.
    /// # Errors
    /// Schema validation, `NotABumbleDb`, a foreign schema, a held lock,
    /// storage failure, cancellation.
    pub fn open_with(path: &Path, schema: S, options: Options, work: WorkContext) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        work.checkpoint().map_err(Error::from)?;
        let store = Store::open(path, &schema, options).map_err(Error::from_store)?;
        Self::assemble(store, schema, work)
    }
}

impl<S> Db<S> {
    pub(super) fn assemble(store: Store, schema: Schema, work: WorkContext) -> Result<Self> {
        work.checkpoint().map_err(Error::from)?;
        let schema = Arc::new(schema);
        let cache = Arc::new(ImageCache::new(schema.as_ref()));
        Ok(Self {
            store,
            schema,
            cache,
            marker: std::marker::PhantomData,
        })
    }

    /// Publish an admitted heap instance as a new database at `path`, which
    /// must not exist. The instance was judged at admission; its rows are
    /// written in a staged directory and published atomically.
    ///
    /// ```compile_fail
    /// fn require_builder(path: &std::path::Path, builder: &bumbledb::InstanceBuilder<()>) {
    ///     let _ = bumbledb::Db::from_instance(path, builder);
    /// }
    /// ```
    ///
    /// # Errors
    /// `DestinationExists`, `Full`, storage failure, cancellation.
    pub fn from_instance(
        path: &Path,
        instance: &super::OwnedInstance<S>,
        work: WorkContext,
    ) -> Result<Self> {
        let schema = instance.schema().clone();
        let changes = instance.change_set_of_rows(&work)?;
        let store = Store::install_populated(
            path,
            &schema,
            DatabaseId::mint(),
            Options::default(),
            |store| {
                let mut owner = store.writer(&work)?;
                owner
                    .prepare_decided(std::slice::from_ref(&changes))?
                    .seal(crate::storage::store::HostChanges::NONE)?
                    .commit()
                    .map(|_| ())
            },
        )
        .map_err(Error::from_store)?;
        Self::assemble(store, schema, work)
    }
}
