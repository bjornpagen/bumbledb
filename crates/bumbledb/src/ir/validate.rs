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
//!  4. variable type conflicts (structural — interval-field bindings
//!     anchor *bivalently*; see [`Context::resolve_bivalents`])
//!  5. literal-vs-field and param-anchor type mismatches (non-UTF-8
//!     String literals and `start >= end` interval literals included)
//!  6. enum ordinal out of range for the field's variant list (bindings
//!     and comparisons, each precisely diagnosed)
//!  7. param anchor conflicts (an *unanchored* param is unwritable by
//!     construction: every param position is itself an anchor) and
//!     non-dense param ids — dense across scalars and sets jointly
//!  8. a `ParamId` used both scalar and set; a `ParamSet` under any
//!     operator but `Eq`; an interval-typed `ParamSet` anchor
//!  9. comparisons violating the type rules (Eq/Ne all types; order ops
//!     U64/U64 and I64/I64 only — an interval operand under an order op
//!     gets its own diagnostic; Overlaps two intervals of one element;
//!     Contains interval × same-element interval or element)
//! 10. constant comparisons (no variable side) and self-comparisons
//! 11. point variables bound only by membership (no enumerable domain)
//! 12. negated-atom variables not bound by any positive atom (negated
//!     atoms bind nothing; a query with no positive atoms is invalid)
//! 13. unbound find variables (Datalog safety; includes aggregate inputs)
//! 14. comparison-only variables
//! 15. empty finds
//! 16. duplicate find terms
//! 17. no positive atoms
//! 18. aggregate input types (Sum/Min/Max integers only; CountDistinct
//!     every type; Count nullary)
//! 19. aggregate over a group-key variable
//! 20. mixed Arg and fold aggregates; Arg terms with differing keys or
//!     directions; a non-orderable Arg key
//! 21. planner caps: more than `MAX_OCCURRENCES` atom occurrences —
//!     negated occurrences counted — or more than 128 distinct variables
//!     (rejected here so downstream id widths and bitset sizes are true
//!     invariants)

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{ParamId, Query, VarId};
use crate::schema::{IntervalElement, ValueType};

mod context;
mod finds;
mod param_types;
mod sink_vars;
#[allow(clippy::module_inception)]
mod validate;

pub use validate::validate;

/// The sealed witness: the query plus the derived tables downstream layers
/// trust. Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedQuery {
    query: Query,
    var_types: BTreeMap<VarId, ValueType>,
    param_types: BTreeMap<ParamId, ValueType>,
    /// Param ids bound as sets (`Term::ParamSet`); their entry in
    /// `param_types` is the *element* type.
    set_params: BTreeSet<ParamId>,
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

    /// Every variable with its resolved type, in id order (the slot-layout
    /// roster — normalization builds the binding-slot widths from it).
    pub fn var_types(&self) -> impl Iterator<Item = (VarId, &ValueType)> {
        self.var_types.iter().map(|(v, t)| (*v, t))
    }

    /// The resolved type of a scalar param (for a set param this is the
    /// *element* type).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an unknown `ParamId` (the
    /// witness anchored every param).
    #[must_use]
    pub fn param_type(&self, param: ParamId) -> &ValueType {
        &self.param_types[&param]
    }

    /// The group key: non-aggregated find variables (test observability;
    /// production reads it only through [`Self::sink_vars`]).
    #[cfg(test)]
    #[must_use]
    pub fn group_key(&self) -> &BTreeSet<VarId> {
        &self.group_key
    }
}

/// One inference slot: collapsed to a single structural type, or still
/// bivalent (see [`Context::resolve_bivalents`], the resolution rule).
#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeSlot {
    /// Named by at least one monovalent anchor.
    Mono(ValueType),
    /// Anchored only by interval-field positions so far: the term is
    /// either `Interval(element)` (value equality) or `element`-typed
    /// (membership).
    Bivalent(IntervalElement),
}

/// How a param id is used: a scalar (`Term::Param`) or a set
/// (`Term::ParamSet`) — one or the other, never both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParamKind {
    Scalar,
    Set,
}

/// Accumulated typing state while walking the query.
#[derive(Default)]
struct Context {
    var_slots: BTreeMap<VarId, TypeSlot>,
    param_slots: BTreeMap<ParamId, TypeSlot>,
    /// Every param seen, with its scalar-vs-set role (doubles as the
    /// density-check roster).
    param_kinds: BTreeMap<ParamId, ParamKind>,
    /// Variables bound by at least one positive atom (any field kind).
    atom_vars: BTreeSet<VarId>,
    /// Variables with at least one positive *scalar*-field binding — the
    /// enumerable-domain witnesses for the membership-only rule.
    scalar_bound_vars: BTreeSet<VarId>,
    /// Variables occurring in negated atoms (the negation safety rule).
    negated_vars: BTreeSet<VarId>,
}

#[cfg(test)]
mod tests;
