use super::{Serial, WriteTx};
use crate::error::Result;
use crate::schema::{FieldId, RelationId};

impl WriteTx<'_> {
    /// Mints the next serial value for the newtype's field — insert new
    /// rows without reading a max (`10-data-model.md`).
    ///
    /// # Errors
    ///
    /// `SerialExhausted` at `u64::MAX`; `Lmdb` on the sequence read.
    pub fn alloc<T: Serial>(&mut self) -> Result<T> {
        self.delta
            .alloc(&self.view, T::RELATION, T::FIELD)
            .map(T::from_serial)
    }

    /// Untyped serial minting for ETL tooling.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::alloc`].
    ///
    /// # Panics
    ///
    /// If `field` is not `Serial` generation — the untyped path is the
    /// caller's responsibility to point at a serial field.
    pub fn alloc_dyn(&mut self, rel: RelationId, field: FieldId) -> Result<u64> {
        self.delta.alloc(&self.view, rel, field)
    }
}
