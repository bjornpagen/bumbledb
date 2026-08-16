use super::apply::ApplyRow;
use super::{Fact, MutationReport, WriteTx};
use crate::error::Result;
use crate::storage::delta::Disposition;

impl<S> WriteTx<'_, S> {
    /// Records a collection of typed inserts. Returns how many facts were
    /// consumed and how many changed the in-memory final-state view.
    /// Empty is lawful: `{ submitted: 0, changed: 0 }` and no engine
    /// request. Singleton is `[&fact]`.
    ///
    /// An empty typed collection cannot infer `F`: `tx.insert::<Holder>([])`.
    ///
    /// # Errors
    ///
    /// `ClosedRelationWrite` on a closed relation (ground axioms are not
    /// writable); `Lmdb` on the membership probe or dictionary reads;
    /// `TransactionPoisoned` if a prior apply in this transaction failed
    /// after a prefix entered the delta.
    pub fn insert<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.apply_collection(
            F::RELATION,
            Disposition::Insert,
            facts,
            |tx, fact, bytes| {
                fact.encode_write(tx, bytes)?;
                Ok(ApplyRow::Ready)
            },
        )
    }
}
