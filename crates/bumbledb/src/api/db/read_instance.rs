//! One admitted read lease: an owned coherent snapshot plus the schema.
//! Executes prepared queries and point reads and exports relations. Handed
//! to [`super::Db::read`] closures. `!Send + !Sync` — a read lease does not
//! cross threads (the snapshot moves between workers whole only through
//! the store's own owned-snapshot capability, not through a lease).
//!
//! ```compile_fail
//! fn require_send<T: Send>() {}
//! require_send::<bumbledb::ReadInstance<'static, ()>>();
//! ```
//! ```compile_fail
//! fn require_sync<T: Sync>() {}
//! require_sync::<bumbledb::ReadInstance<'static, ()>>();
//! ```
//! ```compile_fail
//! fn require_insert(instance: &mut bumbledb::ReadInstance<'_, ()>) {
//!     let _ = instance.insert;
//! }
//! ```

use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::api::prepared::{Answers, ParamArg, PreparedQuery};
use crate::error::{DynIdError, Error, Result};
use crate::ir::{Query, Value};
use crate::schema::Schema;
use crate::storage::GenerationId;
use crate::storage::store::OwnedSnapshot;
use crate::work::WorkContext;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::closed::ClosedRows;
use super::get as get_path;
use super::row_reader::RowReader;
use super::tx::row_error;
use super::{Fact, Key};

pub struct ReadInstance<'db, S> {
    pub(super) schema: &'db Arc<Schema>,
    pub(super) closed: &'db ClosedRows,
    pub(super) snapshot: OwnedSnapshot,
    pub(super) work: WorkContext,
    pub(super) thread_bound: PhantomData<Rc<()>>,
    pub(super) marker: PhantomData<fn() -> S>,
}

impl<S> ReadInstance<'_, S> {
    #[must_use]
    pub fn schema(&self) -> &Schema {
        self.schema.as_ref()
    }

    /// The shared schema witness behind this lease (C05 seam: prepared
    /// queries pin the `Arc` rather than re-clone the sealed schema).
    pub(crate) fn schema_arc(&self) -> &Arc<Schema> {
        self.schema
    }

    /// The one coherent snapshot behind this lease (C05 seam: the query
    /// lane's cursors and probes read through this exact transaction).
    #[doc(hidden)]
    #[must_use]
    pub fn snapshot(&self) -> &OwnedSnapshot {
        &self.snapshot
    }

    /// The lease's work allowance (embedded default, or the integration
    /// caller's budget once the native lane threads one through).
    #[doc(hidden)]
    #[must_use]
    pub fn work(&self) -> &WorkContext {
        &self.work
    }

    /// The generation this lease witnessed — one coherent snapshot.
    /// # Errors
    /// None today; kept fallible for lease-shaped forward compatibility.
    pub fn generation(&self) -> Result<GenerationId> {
        Ok(self.snapshot.generation())
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
    pub fn execute_collect<'p, P: crate::api::prepared::BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
    ) -> Result<Answers> {
        prepared.execute_collect(self, params)
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
        let Some(rel) = self.schema.relation_checked(relation) else {
            return Err(DynIdError::UnknownRelation { relation }.into());
        };
        match rel.body().closed_rows() {
            Some(rows) => Ok(rows.len() as u64),
            None => self.snapshot.row_count(relation).map_err(Error::from_store),
        }
    }

    /// # Errors
    /// Unknown relation, storage failure, or a malformed stored row.
    pub fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        let Some(relation) = self.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let fields = relation.fields();
        if let Some(rows) = self.closed.get(rel) {
            return Ok(ScanRows::Closed(
                rows.iter().map(|row| Ok(row.values.to_vec())),
            ));
        }
        let iterator = self.snapshot.rows(rel).map_err(Error::from_store)?;
        Ok(ScanRows::Store(iterator.map(move |entry| {
            let (_, bytes) = entry.map_err(Error::from_store)?;
            let decoded = crate::canonical::decode(fields, bytes, &self.work).map_err(row_error)?;
            Ok(decoded.values)
        })))
    }

    /// # Errors
    /// Storage failure or a malformed stored row.
    pub fn scan_facts<'lease, F: Fact<'lease, Schema = S>>(
        &'lease self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'lease> {
        if let Some(rows) = self.closed.get(F::RELATION) {
            return Ok(ScanRows::Closed(
                rows.iter()
                    .map(|row| F::decode(RowReader::new(&row.canonical)?)),
            ));
        }
        let iterator = self.snapshot.rows(F::RELATION).map_err(Error::from_store)?;
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
        if let Some(rows) = self.closed.get(relation) {
            return Ok(rows.iter().any(|row| row.values.as_ref() == values));
        }
        let bytes = super::tx::encode_values(self.schema.as_ref(), relation, values, &self.work)?;
        self.snapshot
            .contains(relation, &bytes, &self.work)
            .map_err(Error::from_store)
    }

    /// # Errors
    /// Shape refusals or storage failure.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `instance.get(id)`: \
                  generated key structs are small — by-value keeps every \
                  call site free of `&` noise"
    )]
    pub fn get<'lease, K: Key<'lease, Schema = S>>(
        &'lease self,
        key: K,
    ) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'lease>>::RELATION;
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let (_, statement) =
            get_path::key_statement_of(self.schema.as_ref(), relation, K::STATEMENT)?;
        let mut key_values = Vec::new();
        key.append_key_values(&mut key_values)?;
        get_path::check_key_shape(
            self.schema.as_ref(),
            relation,
            &statement.projection,
            &key_values,
        )?;
        let result = if let Some(rows) = self.closed.get(relation) {
            match get_path::closed_row_by_key(rows, statement, &key_values) {
                Some(row) => K::Fact::decode(RowReader::new(&row.canonical)?).map(Some),
                None => Ok(None),
            }
        } else {
            match get_path::find_snapshot_row(
                &self.snapshot,
                self.schema.as_ref(),
                relation,
                &statement.projection,
                &key_values,
                &self.work,
            )? {
                Some(bytes) => K::Fact::decode(RowReader::new(bytes)?).map(Some),
                None => Ok(None),
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
    pub fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>> {
        let mut out = Vec::new();
        Ok(self
            .get_dyn_into(relation, key, key_values, &mut out)?
            .then_some(out))
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
        out.clear();
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let (_, statement) = get_path::key_statement_of(self.schema.as_ref(), relation, key)?;
        get_path::check_key_shape(
            self.schema.as_ref(),
            relation,
            &statement.projection,
            key_values,
        )?;
        let hit = if let Some(rows) = self.closed.get(relation) {
            match get_path::closed_row_by_key(rows, statement, key_values) {
                Some(row) => {
                    out.extend(row.values.iter().cloned());
                    true
                }
                None => false,
            }
        } else {
            match get_path::find_snapshot_row(
                &self.snapshot,
                self.schema.as_ref(),
                relation,
                &statement.projection,
                key_values,
                &self.work,
            )? {
                Some(bytes) => {
                    let fields = self.schema.relation(relation).fields();
                    let decoded =
                        crate::canonical::decode(fields, bytes, &self.work).map_err(row_error)?;
                    out.extend(decoded.values);
                    true
                }
                None => false,
            }
        };
        span.set_flag(hit);
        span.end();
        Ok(hit)
    }
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
