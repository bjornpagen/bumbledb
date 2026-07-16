use super::{Db, Fresh, WriteTx};
use crate::error::{FactShapeError, Result};
use crate::schema::{FieldId, FreshField, RelationId};

impl<S> Db<S> {
    /// Resolves `(relation, field)` to the schema-bound [`FreshField`]
    /// witness — ids and generation validated here, once per relation;
    /// [`WriteTx::alloc_at`] mints per row thereafter (`70-api.md` § ETL).
    /// The witness carries this handle's schema typestate `S`, so handing
    /// it to another schema's transaction is a compile error (the witness
    /// binding — see [`FreshField`] for the dyn-boundary half of the law).
    ///
    /// # Errors
    ///
    /// `UnknownRelation`/`UnknownField` on an out-of-range id;
    /// `NotAFreshField` when the field's generation is not `Fresh` — ids
    /// at this surface are data, so every mis-aimed resolution is a typed
    /// error, never a panic.
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
    /// Mints the next fresh value for the newtype's field — insert new
    /// rows without reading a max (`10-data-model.md`).
    ///
    /// # Errors
    ///
    /// `ClosedRelationWrite` on a closed relation (its rows are ground
    /// axioms, never minted — `fresh` is already refused at declaration,
    /// so only a hand-written impl can reach this); `FreshExhausted` at
    /// `u64::MAX`; `FactShape` when the sequence init's generation check
    /// refuses the constants (same story: only a hand-written impl can
    /// mis-aim them); `Lmdb` on the sequence read.
    pub fn alloc<T: Fresh<Schema = S>>(&mut self) -> Result<T> {
        self.refuse_closed(T::RELATION)?;
        self.delta
            .alloc(&self.view, T::RELATION, T::FIELD)
            .map(T::from_fresh)
    }

    /// Untyped fresh minting for ETL tooling: the witness
    /// [`Db::fresh_field`] resolves is bound to this transaction's schema
    /// typestate `S`, so a foreign schema's witness is a compile error —
    /// resolve once per relation, mint per row (`70-api.md` § ETL). At
    /// the dyn boundary (`Db<SchemaDescriptor>` handles share one
    /// typestate) the binding proves nothing across descriptors, so the
    /// sequence's per-transaction lazy init re-checks the generation and
    /// refuses typed; the steady-state mint re-checks nothing. No
    /// closed-relation check runs here: `fresh` on a closed relation is
    /// refused at declaration, so a closed relation's witness is
    /// unconstructible.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::alloc`]; `FactShape` here is the dyn boundary's
    /// foreign-witness refusal.
    pub fn alloc_at(&mut self, field: FreshField<S>) -> Result<u64> {
        self.delta
            .alloc(&self.view, field.relation(), field.field())
    }
}
