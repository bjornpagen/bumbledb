use super::Db;
use crate::api::prepared::{PreparedQuery, prepare, prepare_program};
use crate::error::Result;
use crate::ir::{Program, Query};

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

    /// Prepares a program — the recursion cut's surface
    /// (`docs/architecture/20-query-ir.md` § engine recursion;
    /// `docs/architecture/40-execution.md` § the fixpoint driver): the
    /// whole program validates under the program roster (the
    /// well-formedness screen, the strata judge, per-predicate typing —
    /// `ir/validate::validate_program`). A no-`Idb` program prepares as
    /// its output predicate's query — the degenerate embedding, zero
    /// new code paths (`lean/Bumbledb/Exec/Fixpoint.lean:
    /// degenerate_embedding`); a recursive program prepares its
    /// delta-variant plans and executes under the per-stratum fixpoint
    /// driver, computing `lean/Bumbledb/Exec/Fixpoint.lean:
    /// evalProgram`'s answers (`program_eval_sound`).
    ///
    /// # Errors
    ///
    /// The program roster's [`crate::error::ValidationError`]s; `Lmdb`
    /// from the statistics reads. At execution a recursive program may
    /// additionally raise the typed
    /// [`crate::error::Error::FixpointBudgetExceeded`]
    /// ([`PreparedQuery::set_fixpoint_budget`] is the host policy knob).
    pub fn prepare_program(&self, program: &Program) -> Result<PreparedQuery<'_, S>> {
        let txn = self.env.read_txn()?;
        prepare_program(&txn, &self.cache, &self.schema, program)
    }
}
