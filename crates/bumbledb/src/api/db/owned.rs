//! Admitted heap instance: an immutable packed catalog plus query.

use crate::Answers;
use crate::ParamArg;
use crate::PreparedQuery;
use crate::error::Result;
use crate::image::FrozenSource;
use crate::ir::{Query, Value};
use crate::schema::Schema;
use crate::storage::catalog::FrozenCatalog;
#[cfg(test)]
use crate::storage::env::CatalogIdentity;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::instance::{Instance, InstanceCore};
use super::{Fact, Key};

/// An admitted heap instance. Mutation is unrepresentable. Query
/// methods live on the sealed [`crate::Instance`] trait.
///
/// `Send + Sync`: hosts may share an admitted instance across threads.
///
/// ```compile_fail
/// fn require_load(instance: &mut bumbledb::OwnedInstance<()>) {
///     let _ = instance.load::<()>([]);
/// }
/// ```
///
/// ```compile_fail
/// fn require_generation(instance: &bumbledb::OwnedInstance<()>) {
///     let _ = instance.generation();
/// }
/// ```
///
/// ```compile_fail
/// fn require_staleness(instance: &bumbledb::OwnedInstance<()>) {
///     let _ = instance.staleness;
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

    #[cfg(test)]
    pub(crate) fn peek_image(
        &self,
        relation: bumbledb_theory::schema::RelationId,
    ) -> Option<std::sync::Arc<crate::image::RelationImage>> {
        self.core.source.peek_image(relation)
    }

    /// Prepares a query against this admitted catalog.
    ///
    /// # Errors
    ///
    /// As [`Instance::prepare`].
    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>> {
        Instance::prepare(self, query)
    }

    /// Executes a prepared query bound to this instance.
    ///
    /// # Errors
    ///
    /// As [`Instance::execute`].
    pub fn execute(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: &[ParamArg<'_>],
        out: &mut Answers,
    ) -> Result<()> {
        Instance::execute(self, prepared, params, out)
    }

    /// Full-relation scan of decoded dynamic facts.
    ///
    /// # Errors
    ///
    /// As [`Instance::scan`].
    pub fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        Instance::scan(self, rel)
    }

    /// Typed full-relation scan.
    ///
    /// # Errors
    ///
    /// As [`Instance::scan_facts`].
    pub fn scan_facts<'a, F: Fact<'a, Schema = S>>(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'a> {
        Instance::scan_facts(self)
    }

    /// Membership of a typed fact.
    ///
    /// # Errors
    ///
    /// As [`Instance::contains`].
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        Instance::contains(self, fact)
    }

    /// Membership of a dynamic fact.
    ///
    /// # Errors
    ///
    /// As [`Instance::contains_dyn`].
    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        Instance::contains_dyn(self, rel, values)
    }

    /// Keyed lookup of a typed fact.
    ///
    /// # Errors
    ///
    /// As [`Instance::get`].
    pub fn get<'a, K: Key<'a, Schema = S>>(&'a self, key: K) -> Result<Option<K::Fact>> {
        Instance::get(self, key)
    }

    /// Keyed lookup through a data-supplied key statement.
    ///
    /// # Errors
    ///
    /// As [`Instance::get_dyn`].
    pub fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>> {
        Instance::get_dyn(self, relation, key, key_values)
    }

    /// Exact live row count of `relation`.
    ///
    /// # Errors
    ///
    /// As [`Instance::row_count`].
    pub fn row_count(&self, relation: RelationId) -> Result<u64> {
        Instance::row_count(self, relation)
    }
}

#[cfg(test)]
mod tests;
