//! Admitted heap instance: an immutable packed catalog plus query.
use std::sync::Arc;

use crate::Answers;
use crate::ParamArg;
use crate::PreparedQuery;
use crate::error::Result;
use crate::image::FrozenSource;
use crate::ir::{Query, Value};
use crate::schema::Schema;
use crate::storage::catalog::{CatalogRead, FrozenCatalog};
#[cfg(test)]
use crate::storage::env::CatalogIdentity;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::instance::InstanceCore;
use super::{Fact, Key};

/// An admitted heap instance. Mutation is unrepresentable. Query
/// methods are inherent on this type.
/// `Send + Sync`: hosts may share an admitted instance across threads.
/// ```compile_fail
/// fn require_load(instance: &mut bumbledb::OwnedInstance<()>) {
///     let _ = instance.load::<()>([]);
/// }
/// ```
/// ```compile_fail
/// fn require_generation(instance: &bumbledb::OwnedInstance<()>) {
///     let _ = instance.generation();
/// }
/// ```
pub struct OwnedInstance<S> {
    pub(super) core: InstanceCore<FrozenSource, S>,
}

impl<S> OwnedInstance<S> {
    pub(crate) fn new(schema: std::sync::Arc<Schema>, catalog: FrozenCatalog) -> Self {
        let source = FrozenSource::new(schema.as_ref(), catalog);
        Self {
            core: InstanceCore::new(schema, source),
        }
    }

    #[must_use]
    pub fn schema(&self) -> &Schema {
        self.core.schema.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn identity(&self) -> &CatalogIdentity {
        &self.core.identity
    }

    #[must_use]
    pub(crate) fn catalog(&self) -> &FrozenCatalog {
        &self.core.source.catalog
    }

    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        let catalog = self.core.source.catalog.byte_size();
        let images = self
            .core
            .schema
            .relations()
            .iter()
            .enumerate()
            .filter_map(|(idx, _)| {
                let id = bumbledb_theory::schema::RelationId(u32::try_from(idx).ok()?);
                self.core.source.peek_image(id)
            })
            .map(|image| image.byte_size())
            .sum::<usize>();
        catalog + images
    }

    #[cfg(test)]
    pub(crate) fn peek_image(
        &self,
        relation: bumbledb_theory::schema::RelationId,
    ) -> Option<std::sync::Arc<crate::image::RelationImage>> {
        self.core.source.peek_image(relation)
    }

    /// # Errors
    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>> {
        crate::api::prepared::prepare_on(
            &self.core.identity,
            &self.core.source.catalog,
            &self.core.source,
            Arc::clone(&self.core.schema),
            query,
        )
    }

    /// # Errors
    pub fn execute(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: &[ParamArg<'_>],
        out: &mut Answers,
    ) -> Result<()> {
        prepared.execute_on(
            &self.core.identity,
            &self.core.source.catalog,
            &self.core.source,
            params,
            out,
        )
    }

    /// # Errors
    pub fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        self.scan_dyn(rel)
    }

    /// # Errors
    pub fn scan_facts<'a, F: Fact<'a, Schema = S>>(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'a> {
        self.scan_typed()
    }

    /// # Errors
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        self.contains_fact(fact)
    }

    /// # Errors
    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.contains_values(rel, values)
    }

    /// # Errors
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the public get takes Key by value to match ReadInstance::get"
    )]
    pub fn get<'a, K: Key<'a, Schema = S>>(&'a self, key: K) -> Result<Option<K::Fact>> {
        self.get_typed(&key)
    }

    /// # Errors
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
    /// # Panics
    pub fn count(&self, relation: RelationId) -> Result<u64> {
        let Some(rel) = self.core.schema.relation_checked(relation) else {
            return Err(crate::error::DynIdError::UnknownRelation { relation }.into());
        };
        match rel.body().closed_rows() {
            Some(rows) => Ok(u64::try_from(rows.len()).expect("bounded extension")),
            None => self.core.source.catalog.row_count(relation),
        }
    }
}

#[cfg(test)]
mod tests;
