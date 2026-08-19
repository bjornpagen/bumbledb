use super::{Db, Fresh, FreshRange, WriteTx};
use crate::error::{FactShapeError, Result};
use crate::schema::FreshField;
use bumbledb_theory::schema::{FieldId, RelationId};

impl<S> Db<S> {
    /// Resolves `(relation, field)` to the schema-bound [`FreshField`]
    /// witness — ids and generation validated here, once per relation;
    /// [`WriteTx::reserve_at`] mints a range thereafter (`70-api.md` § ETL).
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
    /// Mints `count` consecutive fresh values for the newtype's field —
    /// insert new rows without reading a max (`10-data-model.md`).
    /// `count == 0` is [`FreshRange::Empty`] and does not read or advance
    /// the sequence. `count == 1` is a singleton range.
    ///
    /// # Errors
    ///
    /// `ClosedRelationWrite` on a closed relation (its rows are ground
    /// axioms, never minted — `fresh` is already refused at declaration,
    /// so only a hand-written impl can reach this); `FreshExhausted` at
    /// `u64::MAX`; `FactShape` when the sequence init's generation check
    /// refuses the constants (same story: only a hand-written impl can
    /// mis-aim them); `Lmdb` on the sequence read;
    /// `TransactionPoisoned` if a prior apply in this transaction failed
    /// after a prefix entered the delta.
    pub fn reserve<T: Fresh<Schema = S>>(&mut self, count: u64) -> Result<FreshRange<T>> {
        self.mutation.reserve(count)
    }

    /// Untyped fresh minting for ETL tooling: the witness
    /// [`Db::fresh_field`] resolves is bound to this transaction's schema
    /// typestate `S`, so a foreign schema's witness is a compile error —
    /// resolve once per relation, mint a range (`70-api.md` § ETL). At
    /// the dyn boundary (`Db<SchemaDescriptor>` handles share one
    /// typestate) the binding proves nothing across descriptors, so the
    /// sequence's per-transaction lazy init re-checks the generation and
    /// refuses typed; the steady-state mint re-checks nothing. No
    /// closed-relation check runs here: `fresh` on a closed relation is
    /// refused at declaration, so a closed relation's witness is
    /// unconstructible.
    ///
    /// **The proof is schema-BOUND** (the 1.0.0 ruling that reversed
    /// witness-carries-the-proof): a foreign `schema!`'s witness is a
    /// type mismatch (the `foreign_fresh_witness` compile-fail fixture),
    /// and a foreign *descriptor*'s witness at the dyn boundary refuses
    /// typed, never mints — pinned live by
    /// `a_foreign_witness_is_refused_typed_not_minted`.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::reserve`]; `FactShape` here is the dyn boundary's
    /// foreign-witness refusal.
    pub fn reserve_at(&mut self, field: FreshField<S>, count: u64) -> Result<FreshRange<u64>> {
        self.mutation.reserve_at(field, count)
    }
}
