use super::{MutationReport, WriteTx};
use crate::error::Result;
use crate::ir::Value;
use crate::storage::delta::Disposition;
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
        let rows: Vec<_> = facts.into_iter().collect();
        let parsed = self.parse_dyn_collection(rel, &rows)?;
        self.apply_collection(rel, Disposition::Delete, parsed, |tx, row, bytes| {
            let layout = tx.schema.relation(rel).layout();
            tx.encode_parsed_resolve(&row, layout, bytes)
        })
    }
}
