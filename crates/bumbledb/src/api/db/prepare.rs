use super::Db;
use crate::api::prepared::{PreparedQuery, prepare};
use crate::error::Result;
use crate::ir::Query;

impl<S> Db<S> {
    /// # Errors

    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>> {
        let txn = self.env.read_txn()?;
        prepare(
            &txn,
            &self.cache,
            std::sync::Arc::clone(&self.schema),
            query,
        )
    }
}
