//! The IR→SQL translator (`docs/architecture/60-validation.md` § the
//! translation rules, normative; the doc names it infrastructure): total,
//! mechanical `Query` → `SQLite` SQL, faithful to set semantics. Where the
//! translator and the engine disagree, the hand-written goldens
//! ([`goldens`]) decide who is wrong — the 3-way arbitration anchor.
//!
//! Semantics mapping:
//! - Projection = `SELECT DISTINCT` over the find variables.
//! - **Rules = `UNION`**: one `SELECT DISTINCT` per rule joined by
//!   `UNION` (set-semantic union — `SQLite`'s `UNION` is exactly ∪
//!   under `DISTINCT` discipline). A multi-rule aggregate head folds
//!   over the `UNION` of the rules' head-projected distinct rows — the
//!   union-fold template, mirroring the rules-IR definition; the
//!   single-rule fold domain stays the distinct full binding set.
//! - An `Interval(E)` field is two INTEGER columns (`crate::sqlmap`):
//!   a membership binding becomes `f_start <= t AND t < f_end`, interval
//!   value equality compares the halves pairwise, `PointIn`'s point
//!   form is the membership formula, an `Allen` mask is its basics'
//!   endpoint formulas OR'd under the query's `SELECT DISTINCT`, and
//!   `Duration` is `(end - start)` on the two stored columns.
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
//! - **Programs = `WITH RECURSIVE`** ([`program`], the recursive lane):
//!   one CTE per predicate, linear self-recursion under `UNION`; the
//!   non-linear/mutual/fold classes join the enumerated
//!   [`Inexpressible`] set — counted, reported, never silent.

use std::collections::BTreeMap;

use bumbledb::schema::{KeyStatement, StatementDescriptor};
use bumbledb::{ParamId, Query, RelationId, Schema, Value, VarId};

mod builder;
mod program;
mod query;
#[cfg(test)]
mod tests;
mod types;

pub use program::{sqlite_program_expressible, translate_program};
pub use query::translate;

/// The SQL translation is conjunctive-only: it consumes the flat leaf
/// list (the fleet's generators and scenarios emit no trees). The tree
/// grammar's OR shapes are proven against the naive model by the DNF
/// property suite, never round-tripped through SQL.
fn leaf(tree: &bumbledb::ConditionTree) -> &bumbledb::Comparison {
    match tree {
        bumbledb::ConditionTree::Leaf(comparison) => comparison,
        bumbledb::ConditionTree::And(_) | bumbledb::ConditionTree::Or(_) => {
            unreachable!("the SQL translation consumes flat conjunctions only")
        }
    }
}

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

/// The keyed-get surface's canonical `SQLite` twin
/// (`crate::scenarios::Surface::KeyedGet`): the prepared point SELECT of
/// the full fact through the key statement's UNIQUE index — every column
/// in field declaration order (an `Interval` field spans its two half
/// columns, `crate::sqlmap::field_columns` — the one naming authority),
/// one equality conjunct per projection field. Placeholder positions
/// follow projection order: one [`ParamSlot::Whole`] per scalar key
/// field, the [`ParamSlot::Start`]/[`ParamSlot::End`] pair per interval
/// one — so [`crate::sqlite_run::bind_params`] binds the same
/// projection-ordered value slice the engine's `get_dyn` consumes.
///
/// # Panics
///
/// Never on a validated schema: projections fit `u16` and every
/// projection field exists on the relation.
#[must_use]
pub fn keyed_get(schema: &Schema, relation: RelationId, statement: &KeyStatement) -> Translated {
    let rel = schema.relation(relation);
    let select: Vec<String> = rel
        .fields()
        .iter()
        .flat_map(|field| {
            crate::sqlmap::field_columns(field)
                .into_iter()
                .map(|(name, _)| format!("\"{name}\""))
        })
        .collect();
    let mut params: Vec<ParamSlot> = Vec::new();
    let mut conjuncts: Vec<String> = Vec::new();
    for (position, &field) in statement.projection.iter().enumerate() {
        let param = ParamId(u16::try_from(position).expect("a projection fits u16"));
        let columns = crate::sqlmap::field_columns(&rel.fields()[usize::from(field.0)]);
        if let [(start, _), (end, _)] = columns.as_slice() {
            conjuncts.push(format!("\"{start}\" = ?{}", params.len() + 1));
            params.push(ParamSlot::Start(param));
            conjuncts.push(format!("\"{end}\" = ?{}", params.len() + 1));
            params.push(ParamSlot::End(param));
        } else {
            let (name, _) = &columns[0];
            conjuncts.push(format!("\"{name}\" = ?{}", params.len() + 1));
            params.push(ParamSlot::Whole(param));
        }
    }
    Translated {
        sql: format!(
            "SELECT {} FROM \"{}\" WHERE {}",
            select.join(", "),
            rel.name(),
            conjuncts.join(" AND ")
        ),
        params,
    }
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
    conditions: Vec<String>,
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

/// What the `SQLite` lane cannot express — the dependency judgments
/// (`docs/architecture/60-validation.md` § the two oracles: "`SQLite`
/// cannot express the judgments") and the `Pack` aggregate. The naive
/// model is the oracle for every listed case; the verify harness consumes
/// this enumeration so nothing is ever *silently* skipped. Trigger
/// emulation is refused by decision, not deferred.
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
    /// A `Pack` head: `SQLite` has no coalescing aggregate — a
    /// relation-shaped GROUP BY (one answer per (group, maximal segment))
    /// is not a SQL fold, and a recursive-CTE emulation would test the
    /// emulation, not the engine. Naive-only by decision; the verify
    /// harness consumes this enumeration to route and report it.
    PackAggregate,
    /// A bind-time Allen mask (`MaskTerm::Param`): the rendered SQL
    /// embeds the mask's basic disjunction as text, so a per-execution
    /// mask has no prepared-statement slot to ride — re-rendering per
    /// draw would test the renderer, not the binding. Naive-only; the
    /// verify harness routes it exactly like `Pack` (finding 086).
    AllenMaskParam,
    /// A cardinality-window verdict: SQL has no per-parent count-window
    /// judgment with a pinned statement id — the same class as the other
    /// two judgment kinds.
    CardinalityJudgment,
    /// A program rule reading its own predicate through two or more
    /// atoms — the non-linear form. `SQLite`'s recursive CTE admits
    /// exactly one reference to the recursive table per arm, and an
    /// emulation would test the emulation, not the translation. Naive+
    /// Lean territory by decision
    /// (`docs/architecture/60-validation.md` § the two oracles).
    NonLinearRecursion,
    /// Two predicates reading each other — mutual recursion. `SQLite`'s
    /// `WITH RECURSIVE` has no mutually recursive CTE form.
    MutualRecursion,
    /// A fold anywhere in a program — aggregation over recursive
    /// strata. The program lane routes every program fold naive-side
    /// whole (the degenerate no-`Idb` aggregate program is the plain
    /// query lane's, so nothing translatable is lost).
    RecursiveFold,
    /// A rule negating its own predicate — the shape the engine's
    /// stratification fence refuses (`NegationThroughCycle`). The gate
    /// mirrors the fence so a raw `Program` handed straight to the
    /// translator lands here as a typed routing, never as a recursive
    /// CTE whose arm names its own table inside `NOT EXISTS` (which
    /// `SQLite` rejects at prepare — a harness panic, not a verdict).
    SelfNegation,
}

/// The `SQLite` lane's expressibility gate. Every other query construct
/// translates — negation, membership, param sets, `CountDistinct`,
/// Arg-restriction included — so a `Query` arm without a `Pack` head
/// or a bind-time mask param is unconditionally expressible; those two
/// and the dependency judgments are the naive lane's alone.
///
/// # Errors
///
/// The [`Inexpressible`] case: a `Pack`-bearing query, or the judgment
/// kind for a write-side statement verdict.
pub fn sqlite_expressible(case: &LaneCase<'_>) -> Result<(), Inexpressible> {
    match case {
        LaneCase::Query(query) => {
            if query
                .head
                .iter()
                .any(|term| matches!(term, bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Pack)))
            {
                Err(Inexpressible::PackAggregate)
            } else if query.rules.iter().any(|rule| {
                rule.conditions.iter().map(leaf).any(|comparison| {
                    matches!(
                        comparison.op,
                        bumbledb::CmpOp::Allen {
                            mask: bumbledb::MaskTerm::Param(_)
                        }
                    )
                })
            }) {
                Err(Inexpressible::AllenMaskParam)
            } else {
                Ok(())
            }
        }
        LaneCase::Judgment(StatementDescriptor::Functionality { .. }) => {
            Err(Inexpressible::FunctionalityJudgment)
        }
        LaneCase::Judgment(StatementDescriptor::Containment { .. }) => {
            Err(Inexpressible::ContainmentJudgment)
        }
        LaneCase::Judgment(StatementDescriptor::Cardinality { .. }) => {
            Err(Inexpressible::CardinalityJudgment)
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
