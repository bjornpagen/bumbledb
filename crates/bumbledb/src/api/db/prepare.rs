use super::Db;
use crate::api::prepared::{PreparedQuery, prepare};
use crate::error::Result;
use crate::ir::Query;

impl<S> Db<S> {
    /// Prepares a query against current statistics (pin-at-prepare) —
    /// the ONE prepare entry (`docs/architecture/70-api.md`). The
    /// prepared query outlives the internal snapshot and is reusable
    /// across [`Db::read`] closures.
    ///
    /// Validation is [`crate::ir::validate::validate`] on `&Query`
    /// only. Interiors evaluate once, then — on Reach
    /// (`Query::rec` is `Some`) — a rec least fixpoint, then main. A CQ
    /// (`Query::rec` is `None`) never enters the reach driver. At execution a derived table may
    /// raise the typed [`crate::error::Error::DerivedBudgetExceeded`]
    /// ([`PreparedQuery::set_derived_budget`] is the host policy knob).
    ///
    /// # Errors
    ///
    /// The 20-query-ir doc's [`crate::error::ValidationError`] roster
    /// at prepare time; `Lmdb` from the statistics reads.
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
