use super::{Fact, WriteTx};
use crate::error::Result;

impl<S> WriteTx<'_, S> {
    /// Records a typed insert. Returns whether the final state changes (an
    /// idempotent no-op if the fact is already present). Takes the fact at
    /// any lifetime — the encode path reads variable-width fields as
    /// borrows into the delta's arena copy; nothing here wants ownership.
    ///
    /// # Errors
    ///
    /// `ClosedRelationWrite` on a closed relation (ground axioms are not
    /// writable); `Lmdb` on the membership probe or dictionary reads.
    pub fn insert<'f, F: Fact<'f, Schema = S>>(&mut self, fact: &F) -> Result<bool> {
        self.refuse_closed(F::RELATION)?;
        self.with_scratch(|tx, bytes| {
            fact.encode_write(tx, bytes)?;
            tx.delta.insert(&tx.view, F::RELATION, bytes)
        })
    }
}
