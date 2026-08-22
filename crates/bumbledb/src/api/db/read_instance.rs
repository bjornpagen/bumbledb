use super::{Fact, Key, Probe, ReadInstance};
use crate::api::prepared::{Answers, ParamArg, PreparedQuery};
use crate::error::{DynIdError, Result};
use crate::ir::{Query, Value};
use crate::storage::catalog::CatalogRead;
use crate::storage::read;
use bumbledb_theory::schema::RelationId;

impl<S> ReadInstance<'_, S> {

    /// # Errors

    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>> {
        let catalog = self.core.source.catalog();
        crate::api::prepared::prepare_on(
            &self.core.identity,
            &catalog,
            &self.core.source,
            std::sync::Arc::clone(&self.core.schema),
            query,
        )
    }

    /// # Errors

    /// # Panics

    pub fn count(&self, relation: RelationId) -> Result<u64> {
        let Some(rel) = self.core.schema.relation_checked(relation) else {
            return Err(DynIdError::UnknownRelation { relation }.into());
        };
        match rel.body().closed_rows() {
            Some(rows) => Ok(u64::try_from(rows.len()).expect("bounded extension")),
            None => self.core.source.catalog().row_count(relation),
        }
    }

    /// # Errors

    pub fn execute<'p, P: crate::api::prepared::BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        prepared.execute(self.txn(), self.cache(), params, out)
    }

    /// # Errors

    pub fn execute_collect<'p, P: crate::api::prepared::BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
    ) -> Result<Answers> {
        prepared.execute_collect(self.txn(), self.cache(), params)
    }

    /// # Errors

    #[doc(hidden)]
    pub fn introspect(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, String)> {
        prepared.introspect(self.txn(), self.cache(), params)
    }

    /// # Errors

    pub fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        let Some(_relation) = self.core.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let iter = read::scan(self.txn(), self.core.schema.as_ref(), rel)?;
        Ok(iter.map(move |entry| {
            let (_, bytes) = entry?;
            crate::encoding::decode_values(bytes, |id| {
                Ok(Box::from(super::plumbing::resolve_string(
                    self,
                    crate::encoding::InternId::from_raw(id),
                )?))
            })
        }))
    }
}

impl<S> ReadInstance<'_, S> {

    /// # Errors

    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        self.with_scratch(|scratch| {
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
                .catalog()
                .membership_row(F::RELATION, &hash)
                .map(|row| row.is_some())
        })
    }

    /// # Errors

    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        let Some(relation) = self.core.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        self.with_scratch(|scratch| {
            if !super::collection::intern_value_row(
                rel,
                relation.fields(),
                values,
                &mut scratch.refs,
                |text| self.core.source.catalog().dict_lookup(text.as_bytes()),
            )? {
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
                .catalog()
                .membership_row(rel, &hash)
                .map(|row| row.is_some())
        })
    }

    /// # Errors

    pub fn get_dyn(
        &self,
        relation: RelationId,
        key: bumbledb_theory::schema::StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>> {
        let mut out = Vec::new();
        Ok(self
            .get_dyn_into(relation, key, key_values, &mut out)?
            .then_some(out))
    }

    /// # Errors

    pub fn get_dyn_into(
        &self,
        relation: RelationId,
        key: bumbledb_theory::schema::StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        out.clear();
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let (_, statement) =
            super::get::key_statement_of(self.core.schema.as_ref(), relation, key)?;
        let hit = self.with_scratch(|scratch| {
            let catalog = self.core.source.catalog();
            let key_bytes = &mut scratch.bytes;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if !super::get::encode_determinant_with(
                self.core.schema.as_ref(),
                relation,
                &statement.projection,
                key_values,
                key_bytes,
                |text| catalog.dict_lookup(text.as_bytes()),
            )? {
                return Ok(false);
            }
            let rel = self.core.schema.relation(relation);
            let determinant = &key_bytes[read::DETERMINANT_KEY_HEADER..];
            let bytes = match super::get::point_read(rel, statement, determinant) {
                super::get::PointRead::Closed => {
                    super::get::closed_fact_by_determinant(rel, statement, determinant)
                }
                super::get::PointRead::FreshRow { row_id } => {
                    match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            read::check_width(&self.core.schema, relation, row_id, fact)?;
                            Some(fact)
                        }
                        None => None,
                    }
                }
                super::get::PointRead::Determinant => match catalog.determinant_row(key_bytes)? {
                    Some(row_id) => match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            read::check_width(&self.core.schema, relation, row_id, fact)?;
                            Some(fact)
                        }
                        None => None,
                    },
                    None => None,
                },
            };
            let Some(fact) = bytes else {
                return Ok(false);
            };
            crate::encoding::decode_values_keyed_into(
                rel.layout().encoded(fact),
                &statement.projection,
                key_values,
                |id| {
                    Ok(Box::from(super::plumbing::resolve_string(
                        self,
                        crate::encoding::InternId::from_raw(id),
                    )?))
                },
                out,
            )?;
            Ok(true)
        })?;
        span.set_flag(hit);
        span.end();
        Ok(hit)
    }

    /// # Errors

    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `instance.get(id)`: fresh \
                  newtypes are Copy and generated key structs are small — \
                  by-value keeps every call site free of `&` noise"
    )]
    pub fn get<'lease, K: Key<'lease, Schema = S>>(
        &'lease self,
        key: K,
    ) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'lease>>::RELATION;
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let (_, statement) =
            super::get::key_statement_of(self.core.schema.as_ref(), relation, K::STATEMENT)?;
        let result = self.with_scratch(|scratch| {
            let catalog = self.core.source.catalog();
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
                    match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            let view =
                                read::check_width(&self.core.schema, relation, row_id, fact)?;
                            K::Fact::decode(self, view.bytes()).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                super::get::PointRead::Determinant => match catalog.determinant_row(key_bytes)? {
                    Some(row_id) => match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            let view =
                                read::check_width(&self.core.schema, relation, row_id, fact)?;
                            K::Fact::decode(self, view.bytes()).map(Some)
                        }
                        None => Ok(None),
                    },
                    None => Ok(None),
                },
            }
        });
        if let Ok(found) = &result {
            span.set_flag(found.is_some());
        }
        span.end();
        result
    }

    /// # Errors

    pub fn scan_facts<'lease, F: Fact<'lease, Schema = S>>(
        &'lease self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'lease> {
        let iter = read::scan(self.txn(), self.core.schema.as_ref(), F::RELATION)?;
        Ok(iter.map(move |entry| {
            let (_, bytes) = entry?;
            F::decode(self, bytes.bytes())
        }))
    }
}
