//! Unproved heap candidate: collection mutation and overlay point reads.
//! No query preparation or execution. [`InstanceBuilder::admit`] consumes
//! the builder into an [`super::OwnedInstance`].

use super::OwnedInstance;
use super::get as get_path;
use super::mutation_core::{HeapMutation, MutationCore};
use super::{
    AcceptedCollection, CodecRead, CodecWrite, Fact, Fresh, FreshRange, Key, MutationReport, Probe,
    codec_seal,
};
use crate::encoding::InternId;
use crate::error::{Admission, FactShapeError, Result};
use crate::ir::Value;
use crate::schema::FreshField;
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::delta::Disposition;
use crate::storage::read;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

/// Mutable construction of an unproved candidate from an empty base.
/// Collection `load` / `delete`, `reserve`, and overlay `contains` /
/// keyed `get`. No query methods — an unproved candidate cannot be
/// queried. Packed freeze consumes the builder at admission (step 8).
/// `Send + !Sync`: a host may move the builder onto another thread for
/// admission.
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<bumbledb::InstanceBuilder<()>>();
/// ```
/// ```compile_fail
/// fn require_prepare(builder: &bumbledb::InstanceBuilder<()>) {
///     let _ = builder.prepare;
/// }
/// ```
/// ```compile_fail
/// fn require_execute(builder: &bumbledb::InstanceBuilder<()>) {
///     let _ = builder.execute;
/// }
/// ```
pub struct InstanceBuilder<S> {
    mutation: MutationCore<HeapMutation, S>,
}

impl<S: Theory> InstanceBuilder<S> {
    /// # Errors

    pub fn new(theory: S) -> Result<Self> {
        let schema = std::sync::Arc::new(theory.descriptor().validate()?);
        Ok(Self {
            mutation: MutationCore::heap(schema),
        })
    }
}

impl<S> InstanceBuilder<S> {
    fn observed_load(
        &mut self,
        load: impl FnOnce(&mut MutationCore<HeapMutation, S>) -> Result<MutationReport>,
    ) -> Result<MutationReport> {
        load(&mut self.mutation)
    }

    /// The whole collection is encoded before any member is staged.

    /// # Errors

    /// `TransactionPoisoned` if a prior apply failed after a prefix

    pub fn load<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.observed_load(|mutation| mutation.load(facts))
    }

    /// # Errors

    pub fn delete<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.mutation.delete(facts)
    }

    /// parsed before any member is staged.

    /// # Errors

    pub fn load_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        self.observed_load(|mutation| mutation.load_dyn(rel, facts))
    }

    /// # Errors

    pub fn delete_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        self.mutation.delete_dyn(rel, facts)
    }

    /// # Errors

    /// constructor already refused.
    #[doc(hidden)]
    pub fn load_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.observed_load(|mutation| mutation.apply_accepted(collection, Disposition::Insert))
    }

    /// # Errors

    #[doc(hidden)]
    pub fn delete_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.mutation
            .apply_accepted(collection, Disposition::Delete)
    }

    /// # Errors

    pub fn reserve<T: Fresh<Schema = S>>(&mut self, count: u64) -> Result<FreshRange<T>> {
        self.mutation.reserve(count)
    }

    /// # Errors

    pub fn reserve_at(&mut self, field: FreshField<S>, count: u64) -> Result<FreshRange<u64>> {
        self.mutation.reserve_at(field, count)
    }

    /// # Errors

    pub fn fresh_field(
        &self,
        relation: RelationId,
        field: FieldId,
    ) -> std::result::Result<FreshField<S>, FactShapeError> {
        self.mutation.schema().check_fresh_field(relation, field)?;
        Ok(FreshField::new(relation, field))
    }

    /// # Errors

    pub fn contains<'f, F: Fact<'f, Schema = S>>(&mut self, fact: &F) -> Result<bool> {
        self.mutation.contains(fact)
    }

    /// # Errors

    pub fn contains_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.mutation.contains_dyn(rel, values)
    }

    /// # Errors

    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `builder.get(id)`"
    )]
    pub fn get<'a, K: Key<'a, Schema = S>>(&'a mut self, key: K) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'a>>::RELATION;
        let (key_id, _) =
            get_path::key_statement_of(self.mutation.schema(), relation, K::STATEMENT)?;
        let mut key_bytes = std::mem::take(&mut self.mutation.scratch);
        key_bytes.clear();
        read::begin_determinant_key(&mut key_bytes, relation, K::STATEMENT);
        let filled = key.encode_determinant(self, &mut key_bytes);
        self.mutation.scratch = key_bytes;
        if matches!(filled?, Probe::ProvablyAbsent) {
            return Ok(None);
        }
        let this: &'a Self = self;
        match this
            .mutation
            .fact_by_key(relation, key_id, &this.mutation.scratch)?
        {
            Some(bytes) => K::Fact::decode(this, bytes).map(Some),
            None => Ok(None),
        }
    }

    /// # Errors

    pub fn get_dyn(
        &mut self,
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

    pub fn get_dyn_into(
        &mut self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        self.mutation.get_dyn_into(relation, key, key_values, out)
    }

    #[cfg(test)]
    pub(super) fn intern_count(&self) -> usize {
        self.mutation.backend.stage.intern_count()
    }

    /// # Errors

    /// `TransactionPoisoned` if a prior apply failed after a prefix

    pub fn admit(self) -> Result<Admission<OwnedInstance<S>>> {
        self.mutation.refuse_poisoned()?;
        let (schema, stage) = self.mutation.into_heap();
        let admission = crate::storage::catalog::admit_catalog(schema.as_ref(), stage)?;
        Ok(admission.map(|catalog| OwnedInstance::new(schema, catalog)))
    }
}

impl<S> codec_seal::Sealed for InstanceBuilder<S> {}

impl<S> CodecRead<S> for InstanceBuilder<S> {
    fn schema(&self) -> &Schema {
        self.mutation.schema()
    }

    fn lookup_str(&self, value: &str) -> Result<Option<InternId>> {
        CodecRead::lookup_str(&self.mutation, value)
    }

    fn resolve_str(&self, id: InternId) -> Result<&str> {
        CodecRead::resolve_str(&self.mutation, id)
    }
}

impl<S> CodecWrite<S> for InstanceBuilder<S> {
    fn intern_str(&mut self, value: &str) -> Result<InternId> {
        CodecWrite::intern_str(&mut self.mutation, value)
    }
}

#[cfg(test)]
mod tests;
