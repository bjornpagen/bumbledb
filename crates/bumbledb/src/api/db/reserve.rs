use super::{Db, Fresh, FreshRange, WriteTx};
use crate::error::{FactShapeError, Result};
use crate::schema::FreshField;
use bumbledb_theory::schema::{FieldId, RelationId};

impl<S> Db<S> {
    /// # Errors

    pub fn fresh_field(
        &self,
        relation: RelationId,
        field: FieldId,
    ) -> std::result::Result<FreshField<S>, FactShapeError> {
        self.schema().check_fresh_field(relation, field)?;
        Ok(FreshField::new(relation, field))
    }
}

impl<S> WriteTx<'_, S> {
    /// # Errors

    /// axioms, never minted — `fresh` is already refused at declaration,

    /// after a prefix entered the delta.
    pub fn reserve<T: Fresh<Schema = S>>(&mut self, count: u64) -> Result<FreshRange<T>> {
        self.mutation.reserve(count)
    }

    /// refused at declaration, so a closed relation's witness is

    /// # Errors

    /// foreign-witness refusal.
    pub fn reserve_at(&mut self, field: FreshField<S>, count: u64) -> Result<FreshRange<u64>> {
        self.mutation.reserve_at(field, count)
    }
}
