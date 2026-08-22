#![cfg(test)]

use crate::error::Result;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::RelationId;

use super::{DeltaEffect, Disposition, WriteDelta};

impl WriteDelta<'_> {
    pub fn delete(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<DeltaEffect> {
        self.apply(view, rel, fact_bytes, Disposition::Delete)
    }
}
