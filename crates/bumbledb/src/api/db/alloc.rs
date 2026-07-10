use super::{Fresh, WriteTx};
use crate::error::Result;
use crate::schema::FreshField;

impl<S> WriteTx<'_, S> {
    /// Mints the next fresh value for the newtype's field — insert new
    /// rows without reading a max (`10-data-model.md`).
    ///
    /// # Errors
    ///
    /// `FreshExhausted` at `u64::MAX`; `Lmdb` on the sequence read.
    pub fn alloc<T: Fresh<Schema = S>>(&mut self) -> Result<T> {
        self.delta
            .alloc(&self.view, T::RELATION, T::FIELD)
            .map(T::from_fresh)
    }

    /// Untyped fresh minting for ETL tooling: the witness carries the
    /// proof [`crate::Schema::fresh_field`] established at resolution, so
    /// the mint itself re-checks nothing — resolve once per relation, mint
    /// per row (`70-api.md` § ETL).
    ///
    /// # Errors
    ///
    /// As [`WriteTx::alloc`].
    pub fn alloc_at(&mut self, field: FreshField) -> Result<u64> {
        self.delta
            .alloc(&self.view, field.relation(), field.field())
    }
}
