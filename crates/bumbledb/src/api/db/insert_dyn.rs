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
    /// `ClosedRelationWrite` on a closed relation (`bulk_load_dyn` shares
    /// this entry per fact); `FactShape` on an arity, type-kind,
    /// fixed-interval width/ray, `bytes<N>` length, or UTF-8 mismatch
    /// between `values` and the relation's declaration
    /// (`schema::value_matches` is the rule set — ETL input is
    /// data, so shape problems are typed; a closed-relation *handle*
    /// value is a plain `u64` here, range-judged only by a declared
    /// containment at commit); otherwise as [`WriteTx::insert`].
    pub fn insert_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.refuse_closed(rel)?;
        let encoded = self.encode_dyn(rel, values, InternMode::Mint)?;
        debug_assert!(encoded, "the minting mode always encodes");
        self.delta.insert(&self.view, rel, &self.scratch)
    }
}
