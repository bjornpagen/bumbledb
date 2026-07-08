use super::{InternMode, WriteTx};
use crate::error::Result;
use crate::ir::Value;
use crate::schema::RelationId;

impl WriteTx<'_> {
    /// Records a dynamic delete, symmetric to [`WriteTx::insert_dyn`] but
    /// never minting (as [`WriteTx::delete`]): a string or bytes value
    /// known to neither the delta nor the committed dictionary proves the
    /// fact absent, so the delete no-ops without growing the dictionary.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::insert_dyn`].
    pub fn delete_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        if !self.encode_dyn(rel, values, InternMode::Resolve)? {
            return Ok(false);
        }
        self.delta.delete(&self.view, rel, &self.scratch)
    }
}
