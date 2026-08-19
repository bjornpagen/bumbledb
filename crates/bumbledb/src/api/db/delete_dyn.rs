use super::{MutationReport, WriteTx};
use crate::error::Result;
use crate::ir::Value;
use bumbledb_theory::schema::RelationId;

impl<S> WriteTx<'_, S> {
    /// Records a collection of dynamic deletes, symmetric to
    /// [`WriteTx::insert_dyn`] but never minting (as [`WriteTx::delete`]):
    /// a string or bytes value known to neither the delta nor the
    /// committed dictionary proves that row absent, so it no-ops without
    /// growing the dictionary.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::insert_dyn`].
    pub fn delete_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        self.mutation.delete_dyn(rel, facts)
    }
}
