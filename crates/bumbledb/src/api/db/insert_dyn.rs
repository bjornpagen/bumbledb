use super::{InternMode, WriteTx};
use crate::error::Result;
use crate::ir::Value;
use crate::schema::RelationId;

impl<S> WriteTx<'_, S> {
    /// Records a dynamic insert (the ETL form): one [`Value`] per field in
    /// declaration order.
    ///
    /// # Errors
    ///
    /// `ClosedRelationWrite` on a closed relation (`bulk_load` shares this
    /// entry per fact); `FactShape` on an arity/type/enum-range/UTF-8
    /// mismatch between `values` and the relation's declaration (ETL input
    /// is data, so shape problems are typed); otherwise as
    /// [`WriteTx::insert`].
    pub fn insert_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.refuse_closed(rel)?;
        let encoded = self.encode_dyn(rel, values, InternMode::Mint)?;
        debug_assert!(encoded, "the minting mode always encodes");
        self.delta.insert(&self.view, rel, &self.scratch)
    }
}
