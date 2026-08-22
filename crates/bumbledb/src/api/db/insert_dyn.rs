use super::{AcceptedCollection, MutationReport, WriteTx};
use crate::error::Result;
use crate::ir::Value;
use crate::storage::delta::Disposition;
use bumbledb_theory::schema::RelationId;

impl<S> WriteTx<'_, S> {
    /// The whole collection is parsed before any row enters the delta.
    /// refusal, and the field roster run once.
    /// # Errors
    pub fn insert_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        self.mutation.load_dyn(rel, facts)
    }

    /// # Errors
    /// constructor already refused.
    #[doc(hidden)]
    pub fn insert_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.mutation
            .apply_accepted(collection, Disposition::Insert)
    }
}
