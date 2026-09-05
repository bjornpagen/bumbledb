//! Owned pinned read object plus one ephemeral borrowed frame (C4/C7).
//!
//! [`OwnedRead`] owns the LMDB [`OwnedSnapshot`] together with the shared
//! schema, closed-row and image-cache owners. It is `Send` and `!Sync` —
//! the snapshot moves between workers whole; it does not invent an unsafe
//! cross-thread lifetime. Metadata (`generation`, `witness`) is read from
//! this snapshot; a second transaction is never opened for it.
//!
//! [`ReadFrame`] borrows that owner for one operation and carries that
//! operation's explicit [`WorkContext`]. Snapshot age does not donate a
//! deadline or work budget.
//!
//! ```compile_fail
//! fn require_sync<T: Sync>() {}
//! require_sync::<bumbledb::OwnedRead<()>>();
//! ```
//! ```compile_fail
//! fn require_send<T: Send>() {}
//! require_send::<bumbledb::ReadFrame<'static, ()>>();
//! ```
//! ```compile_fail
//! fn require_insert(frame: &mut bumbledb::ReadFrame<'_, ()>) {
//!     let _ = frame.insert;
//! }
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use crate::api::prepared::{Answers, BindArgs, CompleteResult, ParamArg, PreparedQuery};
use crate::error::{DynIdError, Error, Result};
use crate::ir::{Query, Value};
use crate::schema::Schema;
use crate::storage::GenerationId;
use crate::storage::store::OwnedSnapshot;
use crate::work::WorkContext;
use crate::work::cache::GenerationHandle;
use crate::image::cache::ImageCache;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::closed::ClosedRows;
use super::get as get_path;
use super::row_reader::RowReader;
use super::tx::row_error;
use super::{Fact, Key};

/// Owned pinned read object (C4/C7): LMDB snapshot plus shared
/// schema/closed/cache owners. Not a borrowed [`super::Db`] lease.
pub struct OwnedRead<S> {
    pub(super) schema: Arc<Schema>,
    pub(super) closed: Arc<ClosedRows>,
    pub(super) snapshot: OwnedSnapshot,
    pub(super) cache: Arc<ImageCache>,
    /// [`ImageCache::acquire`](ImageCache::acquire) is the pin. There is
    /// no `pin_generation`.
    pub(super) pin: GenerationHandle,
    pub(super) marker: PhantomData<fn() -> S>,
}

/// Short borrowed per-operation frame over an [`OwnedRead`] (C4).
/// Fresh work is passed here; it is not the snapshot's lifetime deadline.
pub struct ReadFrame<'read, S> {
    pub(super) owner: &'read OwnedRead<S>,
    pub(super) work: &'read WorkContext,
}

/// Historical name for [`ReadFrame`]. Prefer [`ReadFrame`].
pub type ReadInstance<'read, S> = ReadFrame<'read, S>;

impl<S> OwnedRead<S> {
    #[must_use]
    pub fn schema(&self) -> &Schema {
        self.schema.as_ref()
    }

    pub(crate) fn schema_arc(&self) -> &Arc<Schema> {
        &self.schema
    }

    pub(crate) fn cache(&self) -> &Arc<ImageCache> {
        &self.cache
    }

    #[must_use]
    pub fn snapshot(&self) -> &OwnedSnapshot {
        &self.snapshot
    }

    /// The generation this pin witnessed — from this snapshot, not a new one.
    #[must_use]
    pub fn generation(&self) -> GenerationId {
        self.snapshot.generation()
    }

    /// The cache generation pin acquired at snapshot time.
    /// [`crate::image::cache::ImageCache::acquire`] is the pin.
    #[must_use]
    pub fn generation_handle(&self) -> GenerationHandle {
        self.pin.clone()
    }

    /// Open one operation frame. Fresh work is the operation's budget.
    #[must_use]
    pub fn frame<'read>(&'read self, work: &'read WorkContext) -> ReadFrame<'read, S> {
        ReadFrame { owner: self, work }
    }

    /// # Errors
    /// Prepare-time validation (C05; produced by the query lane).
    pub fn prepare(&self, query: &Query, work: &WorkContext) -> Result<PreparedQuery<S>> {
        self.frame(work).prepare(query)
    }

    /// # Errors
    /// Shape refusals or storage failure.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `snapshot.get(id, work)`"
    )]
    pub fn get<'pin, K: Key<'pin, Schema = S>>(
        &'pin self,
        key: K,
        work: &'pin WorkContext,
    ) -> Result<Option<K::Fact>> {
        self.frame(work).get(key)
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        work: &WorkContext,
    ) -> Result<Option<Vec<Value>>> {
        self.frame(work).get_dyn(relation, key, key_values)
    }

    /// # Errors
    /// Unknown relation, or storage failure.
    pub fn count(&self, relation: RelationId) -> Result<u64> {
        let Some(rel) = self.schema.relation_checked(relation) else {
            return Err(DynIdError::UnknownRelation { relation }.into());
        };
        match rel.body().closed_rows() {
            Some(rows) => Ok(rows.len() as u64),
            None => self.snapshot.row_count(relation).map_err(Error::from_store),
        }
    }
}

impl<S> ReadFrame<'_, S> {
    #[must_use]
    pub fn schema(&self) -> &Schema {
        self.owner.schema()
    }

    pub(crate) fn schema_arc(&self) -> &Arc<Schema> {
        self.owner.schema_arc()
    }

    pub(crate) fn cache(&self) -> &Arc<ImageCache> {
        self.owner.cache()
    }

    #[must_use]
    pub fn work(&self) -> &WorkContext {
        self.work
    }

    #[must_use]
    pub fn snapshot(&self) -> &OwnedSnapshot {
        self.owner.snapshot()
    }

    /// The generation this pin witnessed — from the owned snapshot.
    #[must_use]
    pub fn generation(&self) -> Result<GenerationId> {
        Ok(self.owner.generation())
    }

    /// # Errors
    /// Prepare-time validation (C05; produced by the query lane).
    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>> {
        crate::api::prepared::prepare_on(self, query)
    }

    /// # Errors
    /// Execution failure (C05; produced by the query lane).
    pub fn execute<'p, P: crate::api::prepared::BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        prepared.execute(self, params, out)
    }

    /// # Errors
    /// Execution failure (C05; produced by the query lane).
    pub fn execute_collect<'p, P: BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
    ) -> Result<Answers> {
        prepared.execute_collect(self, params)
    }

    /// Collect against this frame. L12 also calls
    /// [`PreparedQuery::execute_collect_owned`] on the owning pin.
    ///
    /// # Errors
    /// As [`Self::execute_collect`].
    pub fn execute_collect_owned<'p, P: BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
    ) -> Result<Answers> {
        self.execute_collect(prepared, params)
    }

    /// # Errors
    /// As [`PreparedQuery::execute_complete`].
    pub fn execute_complete<'p, P: BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
    ) -> Result<CompleteResult> {
        prepared.execute_complete(self, params)
    }

    /// # Errors
    /// Execution failure (C05; produced by the query lane).
    #[doc(hidden)]
    pub fn introspect(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, String)> {
        prepared.introspect(self, params)
    }

    /// # Errors
    /// Unknown relation, or storage failure.
    pub fn count(&self, relation: RelationId) -> Result<u64> {
        self.owner.count(relation)
    }

    /// # Errors
    /// Unknown relation, storage failure, or a malformed stored row.
    pub fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        let Some(relation) = self.owner.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let fields = relation.fields();
        if let Some(rows) = self.owner.closed.get(rel) {
            return Ok(ScanRows::Closed(
                rows.iter().map(|row| Ok(row.values.to_vec())),
            ));
        }
        let iterator = self.owner.snapshot.rows(rel).map_err(Error::from_store)?;
        Ok(ScanRows::Store(iterator.map(move |entry| {
            let (_, bytes) = entry.map_err(Error::from_store)?;
            let decoded = crate::canonical::decode(fields, bytes, self.work).map_err(row_error)?;
            Ok(decoded.values().to_vec())
        })))
    }

    /// # Errors
    /// Storage failure or a malformed stored row.
    pub fn scan_facts<'lease, F: Fact<'lease, Schema = S>>(
        &'lease self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'lease> {
        if let Some(rows) = self.owner.closed.get(F::RELATION) {
            return Ok(ScanRows::Closed(
                rows.iter()
                    .map(|row| F::decode(RowReader::new(&row.canonical)?)),
            ));
        }
        let iterator = self
            .owner
            .snapshot
            .rows(F::RELATION)
            .map_err(Error::from_store)?;
        Ok(ScanRows::Store(iterator.map(move |entry| {
            let (_, bytes) = entry.map_err(Error::from_store)?;
            F::decode(RowReader::new(bytes)?)
        })))
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        let mut values = Vec::new();
        fact.append_values(&mut values)?;
        self.contains_values(F::RELATION, &values)
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.contains_values(rel, values)
    }

    fn contains_values(&self, relation: RelationId, values: &[Value]) -> Result<bool> {
        if let Some(rows) = self.owner.closed.get(relation) {
            return Ok(rows.iter().any(|row| row.values.as_ref() == values));
        }
        let bytes =
            super::tx::encode_values(self.owner.schema.as_ref(), relation, values, self.work)?;
        self.owner
            .snapshot
            .contains(relation, &bytes, self.work)
            .map_err(Error::from_store)
    }

    /// # Errors
    /// Shape refusals or storage failure.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `frame.get(id)`: \
                  generated key structs are small — by-value keeps every \
                  call site free of `&` noise"
    )]
    pub fn get<'lease, K: Key<'lease, Schema = S>>(
        &'lease self,
        key: K,
    ) -> Result<Option<K::Fact>> {
        self.get_with_work(self.work, key)
    }

    /// As [`Self::get`], under an explicit work context (native wire).
    /// # Errors
    /// As [`Self::get`].
    #[doc(hidden)]
    pub fn get_with_work<'lease, K: Key<'lease, Schema = S>>(
        &'lease self,
        work: &WorkContext,
        key: K,
    ) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'lease>>::RELATION;
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let mut key_values = Vec::new();
        key.append_key_values(&mut key_values)?;
        let result = match get_path::get_with_work(
            &self.owner.snapshot,
            self.owner.schema.as_ref(),
            self.owner.closed.as_ref(),
            relation,
            K::STATEMENT,
            &key_values,
            work,
        )? {
            None => Ok(None),
            Some(get_path::KeyedRowHit::Closed(row)) => {
                K::Fact::decode(RowReader::new(&row.canonical)?).map(Some)
            }
            Some(get_path::KeyedRowHit::Store(bytes)) => {
                K::Fact::decode(RowReader::new(bytes)?).map(Some)
            }
        };
        if let Ok(found) = &result {
            span.set_flag(found.is_some());
        }
        span.end();
        result
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn get_dyn_with_work(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        work: &WorkContext,
    ) -> Result<Option<Vec<Value>>> {
        let mut out = Vec::new();
        Ok(self
            .get_dyn_into_with_work(relation, key, key_values, &mut out, work)?
            .then_some(out))
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>> {
        self.get_dyn_with_work(relation, key, key_values, self.work)
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn get_dyn_into(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        self.get_dyn_into_with_work(relation, key, key_values, out, self.work)
    }

    /// As [`Self::get_dyn_into`], under an explicit work context.
    /// # Errors
    /// As [`Self::get_dyn_into`].
    #[doc(hidden)]
    pub fn get_dyn_into_with_work(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
        work: &WorkContext,
    ) -> Result<bool> {
        out.clear();
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let hit = match get_path::get_with_work(
            &self.owner.snapshot,
            self.owner.schema.as_ref(),
            self.owner.closed.as_ref(),
            relation,
            key,
            key_values,
            work,
        )? {
            None => false,
            Some(get_path::KeyedRowHit::Closed(row)) => {
                out.extend(row.values.iter().cloned());
                true
            }
            Some(get_path::KeyedRowHit::Store(bytes)) => {
                let fields = self.owner.schema.relation(relation).fields();
                let decoded = crate::canonical::decode(fields, bytes, work).map_err(row_error)?;
                out.extend(decoded.values().iter().cloned());
                true
            }
        };
        span.set_flag(hit);
        span.end();
        Ok(hit)
    }
}

impl<S> PreparedQuery<S> {
    /// Execute against an owned pin. Work is the operation budget; the
    /// pin is not a `ReadInstance` Send wrapper.
    ///
    /// # Errors
    /// As [`PreparedQuery::execute_collect`](PreparedQuery::execute_collect).
    pub fn execute_collect_owned<'p, P: BindArgs<'p>>(
        &mut self,
        owned: &OwnedRead<S>,
        work: &WorkContext,
        params: P,
    ) -> Result<Answers> {
        self.execute_collect(&owned.frame(work), params)
    }
}

#[cfg(test)]
fn _assert_owned_read_send<S: 'static>(read: OwnedRead<S>) -> impl Send {
    read
}

enum ScanRows<C, T> {
    Closed(C),
    Store(T),
}

impl<Item, C: Iterator<Item = Item>, T: Iterator<Item = Item>> Iterator for ScanRows<C, T> {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        match self {
            Self::Closed(iter) => iter.next(),
            Self::Store(iter) => iter.next(),
        }
    }
}
