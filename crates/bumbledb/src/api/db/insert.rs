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
        self.with_scratch(|tx, bytes| {
            fact.encode_write(tx, bytes)?;
            tx.delta.insert(&tx.view, F::RELATION, bytes)
        })
    }
}
