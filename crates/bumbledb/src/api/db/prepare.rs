use super::Db;
use crate::api::prepared::{PreparedQuery, prepare};
use crate::error::Result;
use crate::ir::Query;

impl<S> Db<S> {
    /// Prepares a query against current statistics (pin-at-prepare). The
    /// prepared query outlives the internal snapshot and is reusable
    /// across [`Db::read`] closures.
    ///
    /// # Errors
    ///
    /// The the 20-query-ir doc [`crate::error::ValidationError`] roster, at prepare
    /// time; `Lmdb` from the statistics reads.
    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<'_, S>> {
        let txn = self.env.read_txn()?;
        prepare(&txn, &self.cache, &self.schema, query)
    }
}
