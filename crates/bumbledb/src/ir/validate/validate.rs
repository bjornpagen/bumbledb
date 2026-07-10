use super::{Context, ParamKind, TypeSlot, ValidatedQuery};
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
        return Err(ValidationError::NoPositiveAtoms);
    }
    // The planner caps are roster items: rejected here, at the boundary,
    // so nothing downstream (normalize's u16 occurrence ids, the DP's
    // bitmask table, the 128-bit variable bitsets) ever sees an
    // over-limit query. Negated atoms are occurrences too — each one is
    // an anti-probe the DP places — so they count.
    let occurrences = query.atoms.len() + query.negated.len();
    if occurrences > crate::plan::planner::MAX_OCCURRENCES {
        return Err(ValidationError::TooManyAtoms { count: occurrences });
    }
    for (index, term) in query.finds.iter().enumerate() {
        if query.finds[..index].contains(term) {
            return Err(ValidationError::DuplicateFindTerm { index });
        }
    }

    let mut ctx = Context::default();
    ctx.check_atoms(schema, query)?;
    ctx.check_comparisons(query)?;
    ctx.check_membership_domains()?;
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
    if ctx.var_slots.len() > crate::plan::planner::MAX_DISTINCT_VARS {
        return Err(ValidationError::TooManyVariables {
            count: ctx.var_slots.len(),
        });
    }

    // Point-position params (the point-domain law): anchored at an
    // interval position and resolved element-typed — their bound values
    // are points, so bind rejects the domain ceiling.
    let point_params: BTreeSet<_> = ctx
        .interval_position_params
        .iter()
        .filter(|param| {
            matches!(
                ctx.param_slots.get(param),
                Some(TypeSlot::Mono(
                    crate::schema::ValueType::U64 | crate::schema::ValueType::I64
                ))
            )
        })
        .copied()
        .collect();

    // Every slot is monovalent past `resolve_bivalents`, and every param
    // position anchored its param — the witness carries plain types.
    let var_types = ctx
        .var_slots
        .into_iter()
        .map(|(var, slot)| match slot {
            TypeSlot::Mono(value_type) => (var, value_type),
            TypeSlot::Bivalent(_) => unreachable!("resolve_bivalents ran"),
        })
        .collect();
    let param_types = ctx
        .param_slots
        .into_iter()
        .map(|(param, slot)| match slot {
            TypeSlot::Mono(value_type) => (param, value_type),
            TypeSlot::Bivalent(_) => unreachable!("resolve_bivalents ran"),
        })
        .collect();
    let set_params = ctx
        .param_kinds
        .into_iter()
        .filter_map(|(param, kind)| matches!(kind, ParamKind::Set).then_some(param))
        .collect();

    Ok(ValidatedQuery {
        query: query.clone(),
        var_types,
        param_types,
        set_params,
        point_params,
        group_key,
    })
}
