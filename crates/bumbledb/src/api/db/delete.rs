use super::{Fact, WriteTx};
use crate::error::Result;

impl WriteTx<'_> {
    /// Records a typed delete. Returns whether the final state changes (an
    /// idempotent no-op if the fact is absent).
    ///
    /// Encodes through the *delete* context: pending intern ids first, so
    /// a fact inserted and deleted within one transaction cancels
    /// exactly — but never minting. A string or bytes value known to
    /// neither the delta nor the committed dictionary proves the fact
    /// absent everywhere, so the delete short-circuits to `Ok(false)`
    /// without growing the dictionary.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::insert`].
    pub fn delete<F: Fact>(&mut self, fact: &F) -> Result<bool> {
        let mut bytes = std::mem::take(&mut self.scratch);
        bytes.clear();
        let changed = match fact.encode_delete(self, &mut bytes) {
            Ok(true) => self.delta.delete(&self.view, F::RELATION, &bytes),
            Ok(false) => Ok(false),
            Err(err) => Err(err),
        };
        self.scratch = bytes;
        changed
    }
}
