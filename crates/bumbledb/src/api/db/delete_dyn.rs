use super::{AcceptedCollection, MutationReport, WriteTx};
use crate::error::Result;
use crate::ir::Value;
use crate::storage::delta::Disposition;
use bumbledb_theory::schema::RelationId;

impl<S> WriteTx<'_, S> {
    /// # Errors
    pub fn delete_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        self.mutation.delete_dyn(rel, facts)
    }

    /// # Errors
    #[doc(hidden)]
    pub fn delete_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.mutation
            .apply_accepted(collection, Disposition::Delete)
    }
}
