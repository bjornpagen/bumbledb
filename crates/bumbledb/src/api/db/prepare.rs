use super::Db;
use crate::api::prepared::{PreparedQuery, prepare, prepare_program};
use crate::error::Result;
use crate::ir::ProgramRef;

impl<S> Db<S> {
    /// Prepares a query or program against current statistics
    /// (pin-at-prepare) — the ONE prepare entry (the unified-prepare
    /// ruling, `docs/architecture/70-api.md`): `db.prepare(&query)` and
    /// `db.prepare(&program)` both land here through
    /// [`ProgramRef`]'s borrowing conversions (borrowed by decision — an
    /// owned `impl Into<Program>` would clone unvalidated IR ahead of the
    /// nesting screen; the refusal is recorded on [`ProgramRef`]). The
    /// prepared query outlives the internal snapshot and is reusable
    /// across [`Db::read`] closures.
    ///
    /// A query prepares through the query pipeline — the degenerate
    /// one-predicate program, byte for byte
    /// (`lean/Bumbledb/Exec/Fixpoint.lean: degenerate_embedding`; the
    /// owned embedding is `From<Query> for Program`). A program
    /// validates under the program roster (the well-formedness screen,
    /// the strata judge, per-predicate typing —
    /// `ir/validate::validate_program`): a no-`Idb` program prepares as
    /// its output predicate's query — zero new code paths — and a
    /// recursive program prepares its delta-variant plans and executes
    /// under the per-stratum fixpoint driver, computing
    /// `lean/Bumbledb/Exec/Fixpoint.lean: evalProgram`'s answers
    /// (`program_eval_sound`).
    ///
    /// # Errors
    ///
    /// The 20-query-ir doc's [`crate::error::ValidationError`] roster
    /// (the query roster for a query, the program roster for a program),
    /// at prepare time; `Lmdb` from the statistics reads. At execution a
    /// recursive program may additionally raise the typed
    /// [`crate::error::Error::FixpointBudgetExceeded`]
    /// ([`PreparedQuery::set_fixpoint_budget`] is the host policy knob).
    pub fn prepare<'p>(&self, program: impl Into<ProgramRef<'p>>) -> Result<PreparedQuery<'_, S>> {
        let txn = self.env.read_txn()?;
        match program.into() {
            ProgramRef::Query(query) => prepare(&txn, &self.cache, &self.schema, query),
            ProgramRef::Program(program) => {
                prepare_program(&txn, &self.cache, &self.schema, program)
            }
        }
    }
}
