use super::{Fact, MutationReport, WriteTx};
use crate::error::Result;

impl<S> WriteTx<'_, S> {

    /// # Errors

    pub fn delete<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.mutation.delete(facts)
    }
}
