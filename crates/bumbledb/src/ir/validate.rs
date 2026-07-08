//! The single validation boundary (docs/architecture/20-query-ir.md): IR in, [`ValidatedQuery`]
//! witness out. Everything downstream trusts the witness and re-checks
//! nothing (post-mortem §38: v5 validated one plan four times).
//!
//! The roster, transcribed from `docs/architecture/20-query-ir.md` and
//! checked off in code order below — it is exhaustive by contract:
//!
//!  1. unknown relation ids
//!  2. unknown field ids
//!  3. duplicate `FieldId` in one atom's bindings
//!  4. variable type conflicts (structural)
//!  5. literal-vs-field type mismatches
//!  6. enum ordinal out of range for the field's variant list
//!  7. param anchor conflicts (an *unanchored* param is unwritable by
//!     construction: every param position is itself an anchor) and
//!     non-dense param ids
//!  8. comparisons violating the type rules (Eq/Ne all types; order ops
//!     U64/U64 and I64/I64 only; no cross-type, ever)
//!  9. constant comparisons (no variable side)
//! 10. unbound find variables (Datalog safety; includes aggregate inputs)
//! 11. comparison-only variables
//! 12. empty finds
//! 13. duplicate find terms
//! 14. no atoms
//! 15. aggregate input types (Sum/Min/Max integers only; Count nullary)
//! 16. aggregate over a group-key variable
//! 17. planner caps: more than `MAX_OCCURRENCES` atoms or more than 128
//!     distinct variables (rejected here so downstream id widths and
//!     bitset sizes are true invariants)

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{ParamId, Query, VarId};
use crate::schema::ValueType;

#[allow(clippy::module_inception)]
mod validate;
mod context;
mod param_types;
mod sink_vars;

pub use validate::validate;

/// The sealed witness: the query plus the derived tables downstream layers
/// trust. Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedQuery {
    query: Query,
    var_types: BTreeMap<VarId, ValueType>,
    param_types: BTreeMap<ParamId, ValueType>,
    /// Non-aggregated find variables — the group key under aggregation.
    group_key: BTreeSet<VarId>,
}

impl ValidatedQuery {
    /// The validated query, verbatim.
    #[must_use]
    pub fn query(&self) -> &Query {
        &self.query
    }

    /// The resolved structural type of a variable.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an unknown `VarId` (the witness
    /// resolved every variable).
    #[must_use]
    pub fn var_type(&self, var: VarId) -> &ValueType {
        &self.var_types[&var]
    }

    /// The group key: non-aggregated find variables (test observability;
    /// production reads it only through [`Self::sink_vars`]).
    #[cfg(test)]
    #[must_use]
    pub fn group_key(&self) -> &BTreeSet<VarId> {
        &self.group_key
    }
}

/// Accumulated typing state while walking the query.
#[derive(Default)]
struct Context {
    var_types: BTreeMap<VarId, ValueType>,
    param_types: BTreeMap<ParamId, ValueType>,
    /// Params seen anywhere (each must end up anchored).
    params_seen: BTreeSet<ParamId>,
    /// Variables bound by at least one atom.
    atom_vars: BTreeSet<VarId>,
}

#[cfg(test)]
mod tests;
