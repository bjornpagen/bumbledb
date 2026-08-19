//! Sealed query surface and the generic core behind an admitted instance.

use std::marker::PhantomData;
use std::sync::Arc;

use super::{CodecRead, Fact, Key, Probe, ScratchPool, codec_seal};
use crate::encoding::InternId;
use crate::error::{DynIdError, Result};
use crate::image::FrozenSource;
use crate::ir::{Query, Value};
use crate::schema::Schema;
use crate::storage::catalog::{CatalogRead, FactCursor};
use crate::storage::env::CatalogIdentity;
use crate::storage::read;
use bumbledb_theory::schema::{RelationId, StatementId};

mod instance_seal {
    pub trait Sealed {}
}

/// Query surface shared by admitted instances. Foreign implementations
/// are unrepresentable.
pub trait Instance<S>: instance_seal::Sealed {
    /// Prepares a query against this instance's catalog.
    ///
    /// # Errors
    ///
    /// Validation errors; `Corruption` from catalog reads.
    fn prepare(&self, query: &Query) -> Result<crate::PreparedQuery<S>>;

    /// Executes a prepared query bound to this instance.
    ///
    /// # Errors
    ///
    /// `ForeignPreparedQuery` when `prepared` came from another
    /// instance; bind and storage errors otherwise.
    fn execute(
        &self,
        prepared: &mut crate::PreparedQuery<S>,
        params: &[crate::ParamArg<'_>],
        out: &mut crate::Answers,
    ) -> Result<()>;

    /// Full-relation scan of decoded dynamic facts.
    ///
    /// # Errors
    ///
    /// `UnknownRelation`; per-item `Corruption`.
    fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_>;

    /// Typed full-relation scan.
    ///
    /// # Errors
    ///
    /// As [`Self::scan`].
    fn scan_facts<'a, F: Fact<'a, Schema = S>>(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'a>;

    /// Membership of a typed fact.
    ///
    /// # Errors
    ///
    /// Dictionary resolution errors.
    fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool>;

    /// Membership of a dynamic fact.
    ///
    /// # Errors
    ///
    /// `FactShape` on an unknown relation or arity/type mismatch.
    fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool>;

    /// Keyed lookup of a typed fact.
    ///
    /// # Errors
    ///
    /// `FactShape` when a manual `Key` impl lies about its statement.
    fn get<'a, K: Key<'a, Schema = S>>(&'a self, key: K) -> Result<Option<K::Fact>>;

    /// Keyed lookup through a data-supplied key statement.
    ///
    /// # Errors
    ///
    /// As [`crate::ReadInstance::get_dyn`].
    fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>>;

    /// Exact live row count of `relation`.
    ///
    /// # Errors
    ///
    /// `UnknownRelation`; `Corruption` on a malformed counter.
    fn row_count(&self, relation: RelationId) -> Result<u64>;
}

/// Generic owner of an admitted catalog plus scratch.
pub(crate) struct InstanceCore<Src, S> {
    pub(crate) schema: Arc<Schema>,
    pub(crate) identity: CatalogIdentity,
    pub(crate) source: Src,
    scratch: ScratchPool,
    marker: PhantomData<fn() -> S>,
}

impl<Src, S> InstanceCore<Src, S> {
    pub(crate) fn assemble(schema: Arc<Schema>, identity: CatalogIdentity, source: Src) -> Self {
        Self {
            schema,
            identity,
            source,
            scratch: ScratchPool::new(),
            marker: PhantomData,
        }
    }

    pub(crate) fn with_scratch<R>(
        &self,
        body: impl FnOnce(&mut super::ReadScratch) -> Result<R>,
    ) -> Result<R> {
        let mut scratch = self.scratch.take();
        let out = body(&mut scratch);
        self.scratch.restore(scratch);
        out
    }
}

impl<S> InstanceCore<FrozenSource, S> {
    pub(crate) fn new(schema: Arc<Schema>, source: FrozenSource) -> Self {
        Self::assemble(schema, CatalogIdentity::mint(), source)
    }
}

impl<S> instance_seal::Sealed for super::OwnedInstance<S> {}
impl<S> instance_seal::Sealed for super::ReadInstance<'_, S> {}

impl<S> codec_seal::Sealed for super::OwnedInstance<S> {}

impl<S> CodecRead<S> for super::OwnedInstance<S> {
    fn schema(&self) -> &Schema {
        self.core.schema.as_ref()
    }

    fn lookup_str(&self, value: &str) -> Result<Option<InternId>> {
        self.core.source.catalog.dict_lookup(value.as_bytes())
    }

    fn resolve_str(&self, id: InternId) -> Result<&str> {
        let stored = self.core.source.catalog.dict_resolve(id)?;
        std::str::from_utf8(stored).map_err(|_| {
            crate::error::Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(id.raw()))
        })
    }
}

impl<S> Instance<S> for super::OwnedInstance<S> {
    fn prepare(&self, query: &Query) -> Result<crate::PreparedQuery<S>> {
        crate::api::prepared::prepare_on(
            &self.core.identity,
            &self.core.source.catalog,
            &self.core.source,
            Arc::clone(&self.core.schema),
            query,
        )
    }

    fn execute(
        &self,
        prepared: &mut crate::PreparedQuery<S>,
        params: &[crate::ParamArg<'_>],
        out: &mut crate::Answers,
    ) -> Result<()> {
        prepared.execute_on(
            &self.core.identity,
            &self.core.source.catalog,
            &self.core.source,
            params,
            out,
        )
    }

    fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        self.scan_dyn(rel)
    }

    fn scan_facts<'a, F: Fact<'a, Schema = S>>(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'a> {
        self.scan_typed()
    }

    fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        self.contains_fact(fact)
    }

    fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.contains_values(rel, values)
    }

    fn get<'a, K: Key<'a, Schema = S>>(&'a self, key: K) -> Result<Option<K::Fact>> {
        self.get_typed(&key)
    }

    fn get_dyn(
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

    fn row_count(&self, relation: RelationId) -> Result<u64> {
        self.relation_row_count(relation)
    }
}

impl<S> Instance<S> for super::ReadInstance<'_, S> {
    fn prepare(&self, query: &Query) -> Result<crate::PreparedQuery<S>> {
        super::ReadInstance::prepare(self, query)
    }

    fn execute(
        &self,
        prepared: &mut crate::PreparedQuery<S>,
        params: &[crate::ParamArg<'_>],
        out: &mut crate::Answers,
    ) -> Result<()> {
        super::ReadInstance::execute(self, prepared, params, out)
    }

    fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        super::ReadInstance::scan(self, rel)
    }

    fn scan_facts<'a, F: Fact<'a, Schema = S>>(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'a> {
        super::ReadInstance::scan_facts(self)
    }

    fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        super::ReadInstance::contains(self, fact)
    }

    fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        super::ReadInstance::contains_dyn(self, rel, values)
    }

    fn get<'a, K: Key<'a, Schema = S>>(&'a self, key: K) -> Result<Option<K::Fact>> {
        super::ReadInstance::get(self, key)
    }

    fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>> {
        super::ReadInstance::get_dyn(self, relation, key, key_values)
    }

    fn row_count(&self, relation: RelationId) -> Result<u64> {
        super::ReadInstance::row_count(self, relation)
    }
}

impl<S> super::OwnedInstance<S> {
    fn scan_dyn(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        let Some(relation) = self.core.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        if let Some(extension) = relation.body().closed_rows() {
            let layout = relation.layout();
            let mut idx = 0usize;
            return Ok(OwnedFactScan::Closed(std::iter::from_fn(move || {
                let row = extension.get(idx)?;
                idx += 1;
                Some(crate::encoding::decode_values(
                    layout.encoded(&row.fact),
                    |id| Ok(Box::from(self.resolve_str(InternId::from_raw(id))?)),
                ))
            })));
        }
        let mut cursor = self.core.source.catalog.scan_facts(rel)?;
        Ok(OwnedFactScan::Store(std::iter::from_fn(
            move || match FactCursor::next(&mut cursor) {
                Ok(Some(entry)) => {
                    match read::check_width(&self.core.schema, rel, entry.row, entry.bytes) {
                        Ok(view) => Some(crate::encoding::decode_values(view, |id| {
                            Ok(Box::from(self.resolve_str(InternId::from_raw(id))?))
                        })),
                        Err(error) => Some(Err(error)),
                    }
                }
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            },
        )))
    }

    fn scan_typed<'a, F: Fact<'a, Schema = S>>(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'a> {
        let relation = self.core.schema.relation(F::RELATION);
        if let Some(extension) = relation.body().closed_rows() {
            let mut idx = 0usize;
            return Ok(OwnedFactScan::Closed(std::iter::from_fn(move || {
                let row = extension.get(idx)?;
                idx += 1;
                Some(F::decode(self, &row.fact))
            })));
        }
        let mut cursor = self.core.source.catalog.scan_facts(F::RELATION)?;
        Ok(OwnedFactScan::Store(std::iter::from_fn(
            move || match FactCursor::next(&mut cursor) {
                Ok(Some(entry)) => Some(F::decode(self, entry.bytes)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            },
        )))
    }

    fn contains_fact<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        self.core.with_scratch(|scratch| {
            if matches!(
                fact.encode_probe(self, &mut scratch.bytes)?,
                Probe::ProvablyAbsent
            ) {
                return Ok(false);
            }
            if let Some(extension) = self.core.schema.relation(F::RELATION).body().closed_rows() {
                return Ok(extension
                    .iter()
                    .any(|row| row.fact.as_ref() == scratch.bytes.as_slice()));
            }
            let hash = crate::encoding::fact_hash(&scratch.bytes);
            self.core
                .source
                .catalog
                .membership_row(F::RELATION, &hash)
                .map(|row| row.is_some())
        })
    }

    fn contains_values(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        let Some(relation) = self.core.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        self.core.with_scratch(|scratch| {
            let parsed = super::encode_dyn::parse_dyn_row(rel, values, relation.fields())?;
            if !super::encode_dyn::intern_parsed_row(&parsed, &mut scratch.refs, |text| {
                self.core.source.catalog.dict_lookup(text.as_bytes())
            })? {
                return Ok(false);
            }
            crate::encoding::encode_fact(&scratch.refs, relation.layout(), &mut scratch.bytes);
            if let Some(extension) = relation.body().closed_rows() {
                return Ok(extension
                    .iter()
                    .any(|row| row.fact.as_ref() == scratch.bytes.as_slice()));
            }
            let hash = crate::encoding::fact_hash(&scratch.bytes);
            self.core
                .source
                .catalog
                .membership_row(rel, &hash)
                .map(|row| row.is_some())
        })
    }

    fn get_typed<'a, K: Key<'a, Schema = S>>(&'a self, key: &K) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'a>>::RELATION;
        let (_, statement) =
            super::get::key_statement_of(&self.core.schema, relation, K::STATEMENT)?;
        self.core.with_scratch(|scratch| {
            let key_bytes = &mut scratch.bytes;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if matches!(
                key.encode_determinant(self, key_bytes)?,
                Probe::ProvablyAbsent
            ) {
                return Ok(None);
            }
            let rel = self.core.schema.relation(relation);
            let determinant = &key_bytes[read::DETERMINANT_KEY_HEADER..];
            match super::get::point_read(rel, statement, determinant) {
                super::get::PointRead::Closed => {
                    match super::get::closed_fact_by_determinant(rel, statement, determinant) {
                        Some(fact) => K::Fact::decode(self, fact).map(Some),
                        None => Ok(None),
                    }
                }
                super::get::PointRead::FreshRow { row_id } => {
                    match self.core.source.catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            let view =
                                read::check_width(&self.core.schema, relation, row_id, fact)?;
                            K::Fact::decode(self, view.bytes()).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                super::get::PointRead::Determinant => {
                    match self.core.source.catalog.determinant_row(key_bytes)? {
                        Some(row_id) => {
                            match self.core.source.catalog.fetch_fact(relation, row_id)? {
                                Some(fact) => {
                                    let view = read::check_width(
                                        &self.core.schema,
                                        relation,
                                        row_id,
                                        fact,
                                    )?;
                                    K::Fact::decode(self, view.bytes()).map(Some)
                                }
                                None => Ok(None),
                            }
                        }
                        None => Ok(None),
                    }
                }
            }
        })
    }

    fn get_dyn_into(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        out.clear();
        let (_, statement) = super::get::key_statement_of(&self.core.schema, relation, key)?;
        self.core.with_scratch(|scratch| {
            let key_bytes = &mut scratch.bytes;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if !super::get::encode_determinant_with(
                &self.core.schema,
                relation,
                &statement.projection,
                key_values,
                key_bytes,
                |text| self.core.source.catalog.dict_lookup(text.as_bytes()),
            )? {
                return Ok(false);
            }
            let rel = self.core.schema.relation(relation);
            let determinant = &key_bytes[read::DETERMINANT_KEY_HEADER..];
            let fact = match super::get::point_read(rel, statement, determinant) {
                super::get::PointRead::Closed => {
                    super::get::closed_fact_by_determinant(rel, statement, determinant)
                        .map(Vec::from)
                }
                super::get::PointRead::FreshRow { row_id } => {
                    match self.core.source.catalog.fetch_fact(relation, row_id)? {
                        Some(bytes) => {
                            read::check_width(&self.core.schema, relation, row_id, bytes)?;
                            Some(bytes.to_vec())
                        }
                        None => None,
                    }
                }
                super::get::PointRead::Determinant => {
                    match self.core.source.catalog.determinant_row(key_bytes)? {
                        Some(row_id) => {
                            match self.core.source.catalog.fetch_fact(relation, row_id)? {
                                Some(bytes) => {
                                    read::check_width(&self.core.schema, relation, row_id, bytes)?;
                                    Some(bytes.to_vec())
                                }
                                None => None,
                            }
                        }
                        None => None,
                    }
                }
            };
            let Some(fact) = fact else {
                return Ok(false);
            };
            crate::encoding::decode_values_keyed_into(
                rel.layout().encoded(&fact),
                &statement.projection,
                key_values,
                |id| Ok(Box::from(self.resolve_str(InternId::from_raw(id))?)),
                out,
            )?;
            Ok(true)
        })
    }

    fn relation_row_count(&self, relation: RelationId) -> Result<u64> {
        let Some(rel) = self.core.schema.relation_checked(relation) else {
            return Err(DynIdError::UnknownRelation { relation }.into());
        };
        match rel.body().closed_rows() {
            Some(rows) => Ok(u64::try_from(rows.len()).expect("bounded extension")),
            None => self.core.source.catalog.row_count(relation),
        }
    }
}

/// Closed theory rows and stored catalog facts share one iterator type.
enum OwnedFactScan<C, S> {
    Closed(C),
    Store(S),
}

impl<T, C: Iterator<Item = T>, S: Iterator<Item = T>> Iterator for OwnedFactScan<C, S> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self {
            Self::Closed(iter) => iter.next(),
            Self::Store(iter) => iter.next(),
        }
    }
}
