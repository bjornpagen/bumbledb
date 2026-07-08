use super::{Fact, WriteTx};
use crate::error::Result;

impl WriteTx<'_> {
    /// Records a typed insert. Returns whether the final state changes (an
    /// idempotent no-op if the fact is already present).
    ///
    /// # Errors
    ///
    /// `Lmdb` on the membership probe or dictionary reads.
    pub fn insert<F: Fact>(&mut self, fact: &F) -> Result<bool> {
        let mut bytes = std::mem::take(&mut self.scratch);
        bytes.clear();
        let changed = fact
            .encode_write(self, &mut bytes)
            .and_then(|()| self.delta.insert(&self.view, F::RELATION, &bytes));
        self.scratch = bytes;
        changed
    }
}
