//! The IR→SQL translator (`docs/architecture/60-validation.md` § the
//! translation rules, normative; the doc names it infrastructure): total,
//! mechanical `Query` → `SQLite` SQL, faithful to set semantics. Where the
//! translator and the engine disagree, the hand-written goldens
//! ([`goldens`]) decide who is wrong — the 3-way arbitration anchor.
//!
//! Semantics mapping:
//! - Projection = `SELECT DISTINCT` over the find variables.
//! - An `Interval(E)` field is two INTEGER columns (`crate::sqlmap`):
//!   a membership binding becomes `f_start <= t AND t < f_end`, interval
//!   value equality compares the halves pairwise, `Contains`' point
//!   form is the membership formula, and an `Allen` mask is its basics'
//!   endpoint formulas OR'd (PRD 15 systematizes).
//! - Negation = one `NOT EXISTS (SELECT 1 FROM ...)` correlated subquery
//!   per negated atom, appended to the core's WHERE. Correlation reuses
//!   the positive joins' column aliases; the subqueries' own alias space
//!   (`n0`, `n1`, ...) is disjoint from `t0..`, so a relation joined
//!   positively *and* negated is aliased fresh by construction.
//! - Param sets = literal `IN (v1, ..., vk)` lists, re-rendered per
//!   execution — prepared-statement parity is NOT claimed for set-bound
//!   families (`60-validation.md` says so). The empty set renders `1 = 0`:
//!   `IN (NULL)` is the three-valued-logic trap.
//! - Aggregation = the normative template: fold over a `SELECT DISTINCT`
//!   of **all bound variables** (the distinct full binding set), grouped
//!   by the non-aggregated finds; a *global* aggregate appends
//!   `HAVING COUNT(*) > 0` so SQL's one-NULL-row-over-empty collapses to
//!   the engine's empty set. `CountDistinct(x)` = `COUNT(DISTINCT x)`
//!   over that subquery.
//! - Arg-restriction = the join-back template: the distinct subquery as
//!   `WITH d AS (...)`, joined against its per-group key extreme, with
//!   `SELECT DISTINCT` on the outer — ties survive set-honestly on both
//!   sides. The global variant omits the group columns.
//! - A zero-binding atom (nonemptiness gate) becomes `EXISTS (SELECT 1
//!   FROM t)`; negated, `NOT EXISTS` (the relation must be empty).
//! - Never-interned strings/bytes need no special case: SQL compares
//!   values, which is exactly the sentinel semantics.

use std::collections::BTreeMap;

use bumbledb::schema::StatementDescriptor;
use bumbledb::{ParamId, Query, Schema, Value, VarId};

mod builder;
mod query;
#[cfg(test)]
mod tests;
mod types;

pub use query::translate;

/// One positional SQL placeholder's source (index `i` maps to placeholder
/// `?i + 1`): a scalar param's whole value, or one endpoint of an
/// interval-typed param — the placeholder side of the two-column interval
/// mapping. The endpoints are the raw typed values, never the engine's
/// sign-flipped words. Set params never appear here: their element lists
/// render as literals ([`translate`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParamSlot {
    Whole(ParamId),
    Start(ParamId),
    End(ParamId),
}

/// A translated query: positional SQL plus the [`ParamSlot`] bound to
/// each `?N`. For a query with set params the SQL embeds the set elements
/// as literals and is re-rendered per execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    pub sql: String,
    pub params: Vec<ParamSlot>,
}

/// A variable's column reference(s) in the positive join: one column for
/// a scalar variable, the two half columns for an interval-typed one.
#[derive(Debug, Clone)]
enum VarCols {
    Scalar(String),
    Interval { start: String, end: String },
}

struct Builder<'q> {
    schema: &'q Schema,
    /// Interval-vs-scalar resolution per term ([`types`]).
    types: types::TermTypes,
    /// The bound element lists of the query's set params.
    sets: &'q [(ParamId, Vec<Value>)],
    /// FROM entries: `"Table" AS tN`.
    from: Vec<String>,
    /// WHERE conjuncts.
    predicates: Vec<String>,
    /// Membership tests (interval half columns × point variable) deferred
    /// until every positive atom is walked — the variable's scalar anchor
    /// may be bound by a later atom.
    deferred: Vec<(String, String, VarId)>,
    /// Var → its first binding's column reference(s).
    columns: BTreeMap<VarId, VarCols>,
    /// [`ParamSlot`] → positional index (params may repeat; one `?N` each).
    param_index: BTreeMap<ParamSlot, usize>,
    params: Vec<ParamSlot>,
}

/// One case the differential harness may route to the `SQLite` lane: a
/// query execution, or a dependency-statement verdict over a write.
#[derive(Debug, Clone, Copy)]
pub enum LaneCase<'a> {
    Query(&'a Query),
    Judgment(&'a StatementDescriptor),
}

/// What the `SQLite` lane cannot express — exactly the dependency
/// judgments (`docs/architecture/60-validation.md` § the two oracles:
/// "`SQLite` cannot express the judgments"). The naive model is the oracle
/// for every listed case; the verify harness consumes this enumeration so
/// nothing is ever *silently* skipped. Trigger emulation is refused by
/// decision, not deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inexpressible {
    /// A functionality verdict, scalar or pointwise: SQL has no
    /// accept/abort judgment with a pinned statement id, and the pointwise
    /// overlap rule is not an index shape.
    FunctionalityJudgment,
    /// A containment verdict, either direction (source-unsatisfied or
    /// target-required), σ-conditional sides and interval coverage
    /// included.
    ContainmentJudgment,
}

/// The `SQLite` lane's expressibility gate. Every query construct
/// translates — negation, membership, param sets, `CountDistinct`,
/// Arg-restriction included — so the `Query` arm is unconditionally
/// expressible; only the dependency judgments are the naive lane's alone.
///
/// # Errors
///
/// The [`Inexpressible`] judgment kind for a write-side statement verdict.
pub fn sqlite_expressible(case: &LaneCase<'_>) -> Result<(), Inexpressible> {
    match case {
        LaneCase::Query(_) => Ok(()),
        LaneCase::Judgment(StatementDescriptor::Functionality { .. }) => {
            Err(Inexpressible::FunctionalityJudgment)
        }
        LaneCase::Judgment(StatementDescriptor::Containment { .. }) => {
            Err(Inexpressible::ContainmentJudgment)
        }
    }
}

/// The hand-written golden SQL per translation form — the 3-way
/// arbitration anchor (`docs/architecture/60-validation.md`): when the
/// engine and `SQLite` disagree, compare the translator's output against
/// these; golden ≠ translator ⇒ translator bug, golden == translator ⇒ a
/// human reads the semantics docs and rules which engine is wrong.
/// Written BY HAND, never regenerated from the translator.
pub mod goldens;
