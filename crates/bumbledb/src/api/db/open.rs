//! Create / open / publish over the successor store owner. Create and open
//! are distinct native operations (C04): create refuses an existing
//! destination and publishes through the store's staged-directory protocol;
//! open verifies family/layout/schema against one read view before adopting
//! anything. Durability is LMDB defaults on every path — the `*_nosync`
//! constructor family stays deleted (ENG-008).

use std::path::Path;
use std::sync::Arc;

use super::{Db, OwnedInstance, embedded_work};
use crate::error::{Admission, Error, Result};
use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_final_state};
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::store::{MapPolicy, Store, UnindexedRows};

impl<S: Theory> Db<S> {
    /// Create a new durable database. The declared theory is judged over
    /// the empty state (with its sealed closed extensions) before any
    /// directory is touched; an unsatisfiable declaration is a rejection,
    /// not a store.
    /// # Errors
    /// Schema validation, destination refusals, storage failure.
    pub fn create(path: &Path, schema: S) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        let work = embedded_work()?;
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
        let store =
            Store::create(path, &schema, MapPolicy::default()).map_err(Error::from_store)?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::TraceArgs::Count(1),
        );
        Ok(Admission::Accepted(Self::assemble(store, schema)?))
    }

    /// Open an existing database. Family, layout and schema fingerprint are
    /// verified against one read view before adoption; refusal mutates
    /// nothing.
    /// # Errors
    /// Schema validation, recognition/lock refusals, storage failure.
    pub fn open(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        let store = Store::open(path, &schema, MapPolicy::default()).map_err(Error::from_store)?;
        Self::assemble(store, schema)
    }

    /// Create without the empty-state admission judgment — grounding-off
    /// test support only; never a production constructor.
    /// # Errors
    /// As [`Db::open`].
    #[cfg(any(test, feature = "ground-off"))]
    pub fn create_store_without_admission(path: &Path, schema: S) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        let store =
            Store::create(path, &schema, MapPolicy::default()).map_err(Error::from_store)?;
        Self::assemble(store, schema)
    }
}

impl<S> Db<S> {
    pub(super) fn assemble(store: Store, schema: Schema) -> Result<Self> {
        let work = embedded_work()?;
        let schema = Arc::new(schema);
        let closed = Arc::new(super::closed::ClosedRows::build(schema.as_ref(), &work)?);
        Ok(Self {
            store,
            schema,
            closed,
            marker: std::marker::PhantomData,
        })
    }

    /// Publish an admitted heap instance as a new durable database at
    /// `path`: create the store, then adopt every admitted row in one
    /// durable transaction (already-judged content; the store's own copy
    /// protocol re-derives membership physically).
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
    pub fn from_instance(path: &Path, instance: &OwnedInstance<S>) -> Result<Self> {
        let work = embedded_work()?;
        let store = Store::create(path, instance.schema(), MapPolicy::default())
            .map_err(Error::from_store)?;
        let db = Self::assemble(store, instance.schema().clone())?;
        // One judged commit adopts the whole admitted content: the store
        // prepare path re-judges the final state (an admitted instance
        // passes) and commits facts + generation together.
        let changes = instance.change_set_of_rows(&work)?;
        let judge = crate::storage::store::SchemaJudge::new(db.schema.as_ref());
        // The writer borrows `db.store`; the whole judged commit lives in
        // this block so the borrow ends before `db` moves out.
        {
            let mut owner = db.store.writer(&work).map_err(Error::from_store)?;
            match owner
                .prepare(&changes, &UnindexedRows, &judge)
                .map_err(Error::from_store)?
            {
                crate::storage::store::Prepared::Rejected(judged) => {
                    // `OwnedInstance` is unconstructible except through a
                    // completed admission by this same judge over this same
                    // final state, so a re-judgment rejection means the
                    // invariant was violated somewhere between admit and
                    // publish — an integrity failure, not a domain outcome.
                    drop(judged);
                    return Err(Error::Corruption(
                        crate::error::CorruptionError::MalformedValue(
                            "admitted instance failed re-judgment at publish",
                        ),
                    ));
                }
                crate::storage::store::Prepared::Admitted(prepared) => {
                    let sealed = prepared
                        .seal(crate::storage::store::HostChanges {
                            records: &[],
                            attachment: crate::storage::store::AttachmentChange::Keep,
                        })
                        .map_err(Error::from_store)?;
                    sealed.commit().map_err(Error::from_store)?;
                }
            }
        }
        Ok(db)
    }
}
