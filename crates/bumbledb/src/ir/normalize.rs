//! Normalization (docs/architecture/20-query-ir.md): lowers a [`ValidatedQuery`] into the paper-form
//! conjunctive query execution consumes — distinct-variable atom
//! occurrences, per-atom filters, and residual comparisons
//! (`docs/architecture/20-query-ir.md`, Deviation vs paper §2: the paper's
//! all-distinct-variables / pushed-selections assumption is a WLOG; we own
//! the lowering because there is no external optimizer).
//!
//! Infallible: the witness guarantees every input is lowerable.

use crate::image::view::FilterPredicate;
use crate::ir::{CmpOp, VarId};
use crate::schema::{FieldId, RelationId};

mod lower_literal;
#[allow(clippy::module_inception)]
mod normalize;
mod place_comparisons;

pub use normalize::normalize;

/// Dense atom-occurrence id. Everything downstream (plan validity, trie
/// schemas) quantifies over occurrences, never relation names — self-joins
/// are ordinary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OccId(pub u16);

/// One atom occurrence in paper form: distinct variables only, plus the
/// filters lowered out of its bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub occ_id: OccId,
    pub relation: RelationId,
    /// Distinct variables with the field each is read from (a repeated
    /// variable keeps its first field; later positions became filters).
    pub vars: Vec<(FieldId, VarId)>,
    /// Per-occurrence filters, evaluated at the source (filtered view).
    pub filters: Vec<FilterPredicate>,
}

/// A comparison whose sides are variables — evaluated inside the join at
/// the earliest plan node where both are bound (placement is the 30-execution doc's job).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedComparison {
    pub op: CmpOp,
    pub lhs: VarId,
    pub rhs: VarId,
}

/// The paper-form query: occurrences + residuals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedQuery {
    pub occurrences: Vec<Occurrence>,
    pub residuals: Vec<PlacedComparison>,
}

#[cfg(test)]
mod tests;
