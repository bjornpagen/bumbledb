#![cfg(test)]

use crate::error::Result;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::RelationId;

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Records a delete, netted against committed state
    /// (docs/architecture/50-storage.md). See [`WriteDelta::apply`].
    pub fn delete(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<bool> {
        self.apply(view, rel, fact_bytes, Disposition::Delete)
    }
}
