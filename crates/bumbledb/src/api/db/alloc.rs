use super::{Fresh, WriteTx};
use crate::error::Result;
use crate::schema::FreshField;

impl<S> WriteTx<'_, S> {
    /// Mints the next fresh value for the newtype's field — insert new
    /// rows without reading a max (`10-data-model.md`).
    ///
    /// # Errors
    ///
    /// `ClosedRelationWrite` on a closed relation (its rows are ground
    /// axioms, never minted — `fresh` is already refused at declaration,
    /// so only a hand-written impl can reach this); `FreshExhausted` at
    /// `u64::MAX`; `Lmdb` on the sequence read.
    pub fn alloc<T: Fresh<Schema = S>>(&mut self) -> Result<T> {
        self.refuse_closed(T::RELATION)?;
        self.delta
            .alloc(&self.view, T::RELATION, T::FIELD)
            .map(T::from_fresh)
    }

    /// Untyped fresh minting for ETL tooling: the witness carries the
    /// proof [`crate::Schema::fresh_field`] established at resolution, so
    /// the mint itself re-checks nothing — resolve once per relation, mint
    /// per row (`70-api.md` § ETL). No closed-relation check runs here:
    /// `fresh` on a closed relation is refused at declaration, so a
    /// closed relation's witness is unconstructible.
    ///
    /// **The proof is schema-scoped**: the witness must have been
    /// resolved by THIS database's schema. A witness minted by a
    /// *different* schema's `fresh_field` is outside the contract and
    /// nothing here re-binds it — cross-schema misuse is a programmer
    /// error a debug build asserts on (the desired typed refusal is
    /// pinned, `#[ignore]`d, by
    /// `a_foreign_witness_is_refused_typed_not_minted`, awaiting the
    /// owner ruling on which side of the witness-carries-the-proof
    /// decision gives).
    ///
    /// # Errors
    ///
    /// As [`WriteTx::alloc`].
    pub fn alloc_at(&mut self, field: FreshField) -> Result<u64> {
        self.delta
            .alloc(&self.view, field.relation(), field.field())
    }
}
