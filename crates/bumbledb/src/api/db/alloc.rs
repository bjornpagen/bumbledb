use super::{Serial, WriteTx};
use crate::error::Result;
use crate::schema::SerialField;

impl<S> WriteTx<'_, S> {
    /// Mints the next serial value for the newtype's field — insert new
    /// rows without reading a max (`10-data-model.md`).
    ///
    /// # Errors
    ///
    /// `SerialExhausted` at `u64::MAX`; `Lmdb` on the sequence read.
    pub fn alloc<T: Serial<Schema = S>>(&mut self) -> Result<T> {
        self.delta
            .alloc(&self.view, T::RELATION, T::FIELD)
            .map(T::from_serial)
    }

    /// Untyped serial minting for ETL tooling: the witness carries the
    /// proof [`crate::Schema::serial_field`] established at resolution, so
    /// the mint itself re-checks nothing — resolve once per relation, mint
    /// per row (`70-api.md` § ETL).
    ///
    /// # Errors
    ///
    /// As [`WriteTx::alloc`].
    pub fn alloc_at(&mut self, field: SerialField) -> Result<u64> {
        self.delta
            .alloc(&self.view, field.relation(), field.field())
    }
}
