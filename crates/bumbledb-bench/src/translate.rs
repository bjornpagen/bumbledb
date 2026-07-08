//! The IR→SQL translator (docs/architecture/50-validation.md;
//! `docs/architecture/50-validation.md` names it infrastructure): total,
//! mechanical `Query` → `SQLite` SQL, faithful to set semantics. Where the
//! translator and the engine disagree, the hand-written goldens
//! ([`goldens`]) decide who is wrong — the 3-way arbitration anchor.
//!
//! Semantics mapping:
//! - Projection = `SELECT DISTINCT` over the find variables.
//! - Aggregation = the normative template: fold over a `SELECT DISTINCT`
//!   of **all bound variables** (the distinct full binding set), grouped
//!   by the non-aggregated finds; a *global* aggregate appends
//!   `HAVING COUNT(*) > 0` so SQL's one-NULL-row-over-empty collapses to
//!   the engine's empty set.
//! - A zero-binding atom (nonemptiness gate) becomes `EXISTS (SELECT 1
//!   FROM t)`.
//! - Never-interned strings/bytes need no special case: SQL compares
//!   values, which is exactly the sentinel semantics.

use std::collections::BTreeMap;

use bumbledb::{ParamId, Schema, VarId};

mod builder;
mod query;
#[cfg(test)]
mod tests;

pub use query::translate;

/// A translated query: positional SQL plus the `ParamId` bound to each
/// `?N` (index `i` maps to placeholder `i + 1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    pub sql: String,
    pub params: Vec<ParamId>,
}

struct Builder<'q> {
    schema: &'q Schema,
    /// FROM entries: `"Table" AS tN`.
    from: Vec<String>,
    /// WHERE conjuncts.
    predicates: Vec<String>,
    /// Var → its first binding's column reference (`tN."col"`).
    columns: BTreeMap<VarId, String>,
    /// `ParamId` → positional index (params may repeat; one `?N` each).
    param_index: BTreeMap<ParamId, usize>,
    params: Vec<ParamId>,
}

/// The hand-written golden SQL per read family — the 3-way arbitration
/// anchor (docs/architecture/50-validation.md): when the engine and `SQLite` disagree,
/// compare the translator's output against these; golden ≠ translator ⇒
/// translator bug, golden == translator ⇒ a human reads the semantics
/// docs and rules which engine is wrong. Written BY HAND, never
/// regenerated from the translator.
pub mod goldens;
