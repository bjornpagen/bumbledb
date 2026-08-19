use super::apply::ApplyRow;
use super::{Fact, MutationReport, Probe, WriteTx};
use crate::error::Result;
use crate::storage::delta::Disposition;

impl<S> WriteTx<'_, S> {
    /// Records a collection of typed deletes. Returns how many facts were
    /// consumed and how many changed the in-memory final-state view.
    /// Empty is lawful: `{ submitted: 0, changed: 0 }` and no engine
    /// request. Singleton is `[&fact]`.
    ///
    /// Encodes through the *delete* context: pending intern ids first, so
    /// a fact inserted and deleted within one transaction cancels
    /// exactly — but never minting. A string or bytes value known to
    /// neither the delta nor the committed dictionary proves the fact
    /// absent, so that row short-circuits without growing the dictionary.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::insert`].
    pub fn delete<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.apply_collection(
            F::RELATION,
            Disposition::Delete,
            facts,
            |tx, fact, bytes| {
                if matches!(fact.encode_probe(tx, bytes)?, Probe::Encoded) {
                    Ok(ApplyRow::Ready)
                } else {
                    Ok(ApplyRow::Skip)
                }
            },
        )
    }
}
