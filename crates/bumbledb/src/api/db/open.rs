//! Create / open / publish over the successor store owner. Create and open
//! are distinct native operations (C04): create refuses an existing
//! destination and publishes through the store's staged-directory protocol;
//! open verifies family/layout/schema against one read view before adopting
//! anything. Durability is LMDB defaults on every path — the `*_nosync`
//! constructor family stays deleted (ENG-008).

use std::path::Path;
use std::sync::Arc;

use super::Db;
use crate::error::{Admission, Error, Result};
use crate::image::cache::ImageCache;
use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_final_state};
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::store::{MapPolicy, Store};
use crate::work::{CachePolicy, WorkContext};

impl<S: Theory> Db<S> {
    /// Create a new durable database under an explicit operation allowance.
    /// The declared theory is judged over the empty state (with its sealed
    /// closed extensions) before any directory is touched; an unsatisfiable
    /// declaration is a rejection, not a store.
    /// # Errors
    /// Schema validation, destination refusals, storage failure, stopped work.
    pub fn create(path: &Path, schema: S, work: WorkContext) -> Result<Admission<Self>> {
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
            Store::create(path, &schema, MapPolicy::default()).map_err(Error::from_store)?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::TraceArgs::Count(1),
        );
        Ok(Admission::Accepted(Self::assemble(store, schema, work)?))
    }

    /// Open an existing database under an explicit operation allowance.
    /// Family, layout and schema fingerprint are verified against one read
    /// view before adoption; refusal mutates nothing.
    /// # Errors
    /// Schema validation, recognition/lock refusals, storage failure, stopped work.
    pub fn open(path: &Path, schema: S, work: WorkContext) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        work.checkpoint().map_err(|error| {
            Error::from_store(crate::storage::store::StoreError::Work(error))
        })?;
        let store = Store::open(path, &schema, MapPolicy::default()).map_err(Error::from_store)?;
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
            Store::create(path, &schema, MapPolicy::default()).map_err(Error::from_store)?;
        Self::assemble(store, schema, work)
    }
}

impl<S> Db<S> {
    pub(super) fn assemble(store: Store, schema: Schema, work: WorkContext) -> Result<Self> {
        work.checkpoint().map_err(|error| {
            Error::from_store(crate::storage::store::StoreError::Work(error))
        })?;
        let schema = Arc::new(schema);
        let closed = Arc::new(super::closed::ClosedRows::build(schema.as_ref(), &work)?);
        let cache = Arc::new(ImageCache::with_policy(
            schema.as_ref(),
            CachePolicy::platform_default(),
        ));
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
    /// private staging directory, then publish atomically (CORE-016).
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
        let store = Store::install_populated(
            path,
            &schema,
            MapPolicy::default(),
            &work,
            |stage, work| {
                stage.apply(&changes, work)?;
                Ok(())
            },
        )
        .map_err(Error::from_store)?;
        Self::assemble(store, schema, work)
    }
}
