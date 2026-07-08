use super::{
    check_occurrence_coverage::check_occurrence_coverage, check_selections,
    derive_nodes::derive_nodes, provably_distinct::provably_distinct, split_filters, FjPlan,
    PlanError, PlanOccurrence, ValidatedPlan,
};
use crate::ir::normalize::NormalizedQuery;
use crate::ir::VarId;
use crate::schema::Schema;
use std::collections::BTreeSet;

/// Validates a plan against its normalized query, deriving covers, residual
/// placement, trie schemas, the binding-slot layout, and the
/// distinct-bindings flag.
///
/// # Errors
///
/// [`PlanError`] when the plan does not partition the query, duplicates an
/// occurrence within a node, lacks a cover, or leaves a residual unplaced.
///
/// # Panics
///
/// Only on programmer-invariant violations (more than 256 subatoms in one
/// node — impossible for plans over the planner's occurrence cap).
pub fn validate(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    estimates: Vec<u64>,
    sink_vars: &BTreeSet<VarId>,
) -> Result<ValidatedPlan, PlanError> {
    check_occurrence_coverage(plan, normalized)?;
    // Partition property, per occurrence: subatom vars are disjoint and
    // union to the occurrence's var set.
    for occurrence in &normalized.occurrences {
        let mut seen: BTreeSet<VarId> = BTreeSet::new();
        for node in &plan.nodes {
            for subatom in node.subatoms.iter().filter(|s| s.occ == occurrence.occ_id) {
                for var in &subatom.vars {
                    if !seen.insert(*var) {
                        return Err(PlanError::BrokenPartition {
                            occ: occurrence.occ_id,
                        });
                    }
                }
            }
        }
        let expected: BTreeSet<VarId> = occurrence.vars.iter().map(|(_, v)| *v).collect();
        if seen != expected {
            return Err(PlanError::BrokenPartition {
                occ: occurrence.occ_id,
            });
        }
    }

    let mut nodes = derive_nodes(plan)?;
    for node in &mut nodes {
        node.sink_relevant = node.new_vars.iter().any(|v| sink_vars.contains(v));
    }

    // Residual placement: the earliest node at which both sides are bound.
    for (residual_idx, residual) in normalized.residuals.iter().enumerate() {
        let mut bound: BTreeSet<VarId> = BTreeSet::new();
        let mut placed = false;
        for node in &mut nodes {
            bound.extend(node.new_vars.iter().copied());
            if bound.contains(&residual.lhs) && bound.contains(&residual.rhs) {
                node.residuals.push(*residual);
                placed = true;
                break;
            }
        }
        if !placed {
            return Err(PlanError::UnplacedResidual {
                residual: residual_idx,
            });
        }
    }

    // Trie schemas: each occurrence's subatom var-lists in node order.
    let occurrences: Vec<PlanOccurrence> = normalized
        .occurrences
        .iter()
        .map(|occurrence| {
            let trie_schema: Vec<Vec<VarId>> = plan
                .nodes
                .iter()
                .flat_map(|n| n.subatoms.iter())
                .filter(|s| s.occ == occurrence.occ_id)
                .map(|s| s.vars.clone())
                .collect();
            let (selections, filters) = split_filters(&occurrence.filters);
            PlanOccurrence {
                occ_id: occurrence.occ_id,
                relation: occurrence.relation,
                vars: occurrence.vars.clone(),
                selections,
                filters,
                trie_schema,
            }
        })
        .collect();
    // A tautology at this call site — `split_filters` just constructed
    // these occurrences, so no Eq-constant can sit in `filters`. The real
    // producers `check_selections` guards against are hand-built
    // `PlanOccurrence`s (tests, future callers); the executor-side twin
    // is a debug_assert too.
    debug_assert!(check_selections(&occurrences).is_ok());

    // Binding-slot layout: node order, then `VarId` order within a node
    // (`new_vars` comes off a `BTreeSet`) — dense.
    let mut slots: Vec<VarId> = Vec::new();
    for node in &nodes {
        for var in &node.new_vars {
            if !slots.contains(var) {
                slots.push(*var);
            }
        }
    }

    let distinct_bindings = provably_distinct(normalized, schema);
    let skip_free = nodes.iter().all(|n| n.sink_relevant);

    Ok(ValidatedPlan {
        occurrences,
        nodes,
        slots,
        distinct_bindings,
        skip_free,
        estimates,
    })
}
