//! Admitted heap instance: an immutable admitted final set plus query.
//! Mutation is unrepresentable; construction is [`super::InstanceBuilder::admit`]
//! (or the query lane's own materialization seams). `Send + Sync`: hosts
//! may share an admitted instance across threads.
//!
//! ```compile_fail
//! fn require_load(instance: &mut bumbledb::OwnedInstance<()>) {
//!     let _ = instance.load::<()>([]);
//! }
//! ```
//! ```compile_fail
//! fn require_generation(instance: &bumbledb::OwnedInstance<()>) {
//!     let _ = instance.generation();
//! }
//! ```

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::Answers;
use crate::ParamArg;
use crate::PreparedQuery;
use crate::error::{DynIdError, Result};
use crate::ir::{Query, Value};
use crate::schema::Schema;
use crate::work::WorkContext;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::closed::ClosedRows;
use super::get as get_path;
use super::row_reader::RowReader;
use super::tx::{encode_values, row_error};
use super::{Fact, Key, embedded_work};

pub struct OwnedInstance<S> {
    schema: Arc<Schema>,
    closed: Arc<ClosedRows>,
    /// The admitted final set: canonical rows per relation, sorted by full
    /// canonical bytes (set semantics; binary-search membership).
    relations: BTreeMap<RelationId, Vec<Box<[u8]>>>,
    marker: PhantomData<fn() -> S>,
}

impl<S> OwnedInstance<S> {
    pub(super) fn seal(
        schema: Arc<Schema>,
        closed: Arc<ClosedRows>,
        relations: BTreeMap<RelationId, Vec<Box<[u8]>>>,
    ) -> Self {
        debug_assert!(
            relations
                .values()
                .all(|rows| rows.is_sorted_by(|a, b| a < b)),
            "admitted rows are strictly sorted canonical bytes"
        );
        Self {
            schema,
            closed,
            relations,
            marker: PhantomData,
        }
    }

    #[must_use]
    pub fn schema(&self) -> &Schema {
        self.schema.as_ref()
    }

    /// The shared schema witness behind this instance (C05 seam: prepared
    /// queries pin the `Arc` rather than re-clone the sealed schema).
    pub(crate) fn schema_arc(&self) -> &Arc<Schema> {
        &self.schema
    }

    /// Canonical rows of one relation, sorted by full canonical bytes. The
    /// query lane (C05) and the native bridge read the admitted set through
    /// this seam.
    #[doc(hidden)]
    #[must_use]
    pub fn relation_rows(&self, relation: RelationId) -> &[Box<[u8]>] {
        self.relations
            .get(&relation)
            .map_or(&[], |rows| rows.as_slice())
    }

    /// Retained bytes of the admitted set (rows plus per-row ownership
    /// overhead) — a host budgeting figure, not an allocator measurement.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.relations
            .values()
            .flat_map(|rows| rows.iter())
            .map(|row| row.len() + std::mem::size_of::<Box<[u8]>>())
            .sum()
    }

    /// One sealed `ChangeSet` inserting the whole admitted set — the
    /// publish substrate for [`super::Db::from_instance`].
    pub(super) fn change_set_of_rows(&self, work: &WorkContext) -> Result<crate::ChangeSet> {
        let mut pending = std::collections::BTreeMap::new();
        for (relation, rows) in &self.relations {
            for row in rows {
                pending.insert((*relation, row.clone()), crate::changes::ChangeKind::Add);
            }
        }
        super::tx::change_set_of_pending(self.schema.as_ref(), &pending, work)
    }

    /// # Errors
    /// Prepare-time validation (C05; produced by the query lane).
    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>> {
        crate::api::prepared::prepare_owned(self, query)
    }

    /// # Errors
    /// Execution failure (C05; produced by the query lane).
    pub fn execute(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: &[ParamArg<'_>],
        out: &mut Answers,
    ) -> Result<()> {
        prepared.execute_owned(self, params, out)
    }

    /// # Errors
    /// Unknown relation, or a malformed admitted row (unreachable through
    /// admission).
    pub fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        let Some(relation) = self.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let fields = relation.fields();
        let work = embedded_work()?;
        if let Some(rows) = self.closed.get(rel) {
            return Ok(ScanRows::Closed(
                rows.iter().map(|row| Ok(row.values.to_vec())),
            ));
        }
        Ok(ScanRows::Heap(self.relation_rows(rel).iter().map(
            move |row| {
                let decoded = crate::canonical::decode(fields, row, &work).map_err(row_error)?;
                Ok(decoded.values)
            },
        )))
    }

    /// # Errors
    /// A malformed admitted row (unreachable through admission).
    pub fn scan_facts<'a, F: Fact<'a, Schema = S>>(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'a> {
        if let Some(rows) = self.closed.get(F::RELATION) {
            return Ok(ScanRows::Closed(
                rows.iter()
                    .map(|row| F::decode(RowReader::new(&row.canonical)?)),
            ));
        }
        Ok(ScanRows::Heap(
            self.relation_rows(F::RELATION)
                .iter()
                .map(|row| F::decode(RowReader::new(row)?)),
        ))
    }

    /// # Errors
    /// Shape refusals.
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        let mut values = Vec::new();
        fact.append_values(&mut values)?;
        self.contains_values(F::RELATION, &values)
    }

    /// # Errors
    /// Shape refusals.
    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.contains_values(rel, values)
    }

    fn contains_values(&self, relation: RelationId, values: &[Value]) -> Result<bool> {
        if let Some(rows) = self.closed.get(relation) {
            return Ok(rows.iter().any(|row| row.values.as_ref() == values));
        }
        let work = embedded_work()?;
        let bytes = encode_values(self.schema.as_ref(), relation, values, &work)?;
        Ok(self
            .relation_rows(relation)
            .binary_search_by(|row| row.as_ref().cmp(bytes.as_slice()))
            .is_ok())
    }

    /// # Errors
    /// Shape refusals.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the public get takes Key by value to match ReadInstance::get"
    )]
    pub fn get<'a, K: Key<'a, Schema = S>>(&'a self, key: K) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'a>>::RELATION;
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
        if let Some(rows) = self.closed.get(relation) {
            return match get_path::closed_row_by_key(rows, statement, &key_values) {
                Some(row) => K::Fact::decode(RowReader::new(&row.canonical)?).map(Some),
                None => Ok(None),
            };
        }
        match self.find_by_key(relation, &statement.projection, &key_values)? {
            Some(bytes) => K::Fact::decode(RowReader::new(bytes)?).map(Some),
            None => Ok(None),
        }
    }

    /// # Errors
    /// Shape refusals.
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
    /// Shape refusals.
    pub fn get_dyn_into(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        out.clear();
        let (_, statement) = get_path::key_statement_of(self.schema.as_ref(), relation, key)?;
        get_path::check_key_shape(
            self.schema.as_ref(),
            relation,
            &statement.projection,
            key_values,
        )?;
        if let Some(rows) = self.closed.get(relation) {
            return Ok(
                match get_path::closed_row_by_key(rows, statement, key_values) {
                    Some(row) => {
                        out.extend(row.values.iter().cloned());
                        true
                    }
                    None => false,
                },
            );
        }
        match self.find_by_key(relation, &statement.projection, key_values)? {
            Some(bytes) => {
                let fields = self.schema.relation(relation).fields();
                let work = embedded_work()?;
                let decoded = crate::canonical::decode(fields, bytes, &work).map_err(row_error)?;
                out.extend(decoded.values);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// # Errors
    /// Unknown relation.
    pub fn count(&self, relation: RelationId) -> Result<u64> {
        let Some(rel) = self.schema.relation_checked(relation) else {
            return Err(DynIdError::UnknownRelation { relation }.into());
        };
        match rel.body().closed_rows() {
            Some(rows) => Ok(rows.len() as u64),
            None => Ok(self.relation_rows(relation).len() as u64),
        }
    }

    fn find_by_key(
        &self,
        relation: RelationId,
        projection: &[bumbledb_theory::schema::FieldId],
        key_values: &[Value],
    ) -> Result<Option<&[u8]>> {
        let fields = self.schema.relation(relation).fields();
        let work = embedded_work()?;
        for row in self.relation_rows(relation) {
            let decoded = crate::canonical::decode(fields, row, &work).map_err(row_error)?;
            if get_path::projection_matches(&decoded.values, projection, key_values) {
                return Ok(Some(row));
            }
        }
        Ok(None)
    }
}

// The whole point of the admitted instance: shared across threads.
#[cfg(test)]
fn _assert_owned_instance_send_sync<S: 'static>(instance: OwnedInstance<S>) -> impl Send + Sync {
    instance
}

enum ScanRows<C, T> {
    Closed(C),
    Heap(T),
}

impl<Item, C: Iterator<Item = Item>, T: Iterator<Item = Item>> Iterator for ScanRows<C, T> {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        match self {
            Self::Closed(iter) => iter.next(),
            Self::Heap(iter) => iter.next(),
        }
    }
}
