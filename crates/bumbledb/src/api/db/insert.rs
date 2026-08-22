use super::{Fact, MutationReport, WriteTx};
use crate::error::Result;

impl<S> WriteTx<'_, S> {
    /// # Errors

    /// after a prefix entered the delta.
    pub fn insert<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.mutation.load(facts)
    }
}
