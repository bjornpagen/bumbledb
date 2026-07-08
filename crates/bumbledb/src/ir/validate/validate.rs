use super::{Context, ValidatedQuery};
use crate::error::ValidationError;
use crate::ir::{FindTerm, Query, VarId};
use crate::schema::Schema;
use std::collections::BTreeSet;

/// Validates a query against the schema, yielding the sealed witness.
///
/// Duplicate and even statically contradictory predicates (`x < 5,
/// x > 9`) are accepted deliberately: the semantics are exact (an empty
/// result), and the "write the query you mean" roster rejects only
/// shapes with no meaning at all (constant and self comparisons) — it
/// does not extend to statically false conjunctions.
///
/// # Errors
///
/// A distinct [`ValidationError`] per roster item; see the module docs.
pub fn validate(schema: &Schema, query: &Query) -> Result<ValidatedQuery, ValidationError> {
    if query.finds.is_empty() {
        return Err(ValidationError::EmptyFinds);
    }
    if query.atoms.is_empty() {
        return Err(ValidationError::NoAtoms);
    }
    // The planner caps are roster items: rejected here, at the boundary,
    // so nothing downstream (normalize's u16 occurrence ids, the DP's
    // bitmask table, the 128-bit variable bitsets) ever sees an
    // over-limit query.
    if query.atoms.len() > crate::plan::planner::MAX_OCCURRENCES {
        return Err(ValidationError::TooManyAtoms {
            count: query.atoms.len(),
        });
    }
    for (index, term) in query.finds.iter().enumerate() {
        if query.finds[..index].contains(term) {
            return Err(ValidationError::DuplicateFindTerm { index });
        }
    }

    let mut ctx = Context::default();
    ctx.check_atoms(schema, query)?;
    ctx.check_comparisons(query)?;
    // The group key (non-aggregated find variables) is computed once and
    // shared between the find checks and the witness.
    let group_key: BTreeSet<VarId> = query
        .finds
        .iter()
        .filter_map(|term| match term {
            FindTerm::Var(var) => Some(*var),
            FindTerm::Aggregate { .. } => None,
        })
        .collect();
    ctx.check_finds(query, &group_key)?;
    if ctx.var_types.len() > crate::plan::planner::MAX_DISTINCT_VARS {
        return Err(ValidationError::TooManyVariables {
            count: ctx.var_types.len(),
        });
    }

    Ok(ValidatedQuery {
        query: query.clone(),
        var_types: ctx.var_types,
        param_types: ctx.param_types,
        group_key,
    })
}
