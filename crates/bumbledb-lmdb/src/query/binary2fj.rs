#![allow(dead_code)]

use std::collections::BTreeSet;

use crate::query::free_join::{FjNode, FjPlan, FjPlanError, FjSubatom};
use crate::query::model::{AtomOccurrence, AtomOccurrenceId, NormalizedQuery, NormalizedTerm};
use crate::query::planner::{BinaryPlan, LeftDeepSource};

/// One conservative factorization attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanRewriteStep {
    /// Source node index.
    pub(crate) from_node: usize,
    /// Destination node index.
    pub(crate) to_node: usize,
    /// Subatom index within the source node at attempt time.
    pub(crate) subatom_index: usize,
    /// Subatom being considered.
    pub(crate) subatom: FjSubatom,
    /// Attempt outcome.
    pub(crate) outcome: PlanRewriteOutcome,
    /// Human-readable reason for tests and future explain output.
    pub(crate) reason: String,
    /// Plan snapshot before the attempt.
    pub(crate) before: String,
    /// Plan snapshot after the attempt.
    pub(crate) after: String,
}

/// Conservative factorization outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlanRewriteOutcome {
    /// The subatom moved to the previous node.
    Moved,
    /// The subatom did not move.
    Rejected,
}

/// Trace of conservative factorization attempts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlanRewriteTrace {
    /// Ordered attempts.
    pub(crate) steps: Vec<PlanRewriteStep>,
}

/// Converts a left-deep binary atom sequence to a formal FJ plan.
pub(crate) fn binary2fj_from_atoms(
    query: &NormalizedQuery,
    atoms: &[AtomOccurrenceId],
) -> Result<FjPlan, FjPlanError> {
    let Some(first) = atoms.first().copied() else {
        return Ok(FjPlan {
            nodes: Vec::new(),
            query_variables: query.variables.len(),
        });
    };

    let mut nodes = Vec::new();
    let mut available = BTreeSet::new();
    let mut current = vec![full_subatom(query, first)?];
    for atom in atoms.iter().copied().skip(1) {
        let current_vars = vars_in_subatoms(&current);
        let visible: BTreeSet<_> = available.union(&current_vars).copied().collect();
        let atom_vars = atom_vars(query, atom)?;
        let probe_vars: Vec<_> = atom_vars
            .iter()
            .copied()
            .filter(|variable| visible.contains(variable))
            .collect();
        current.push(subatom_for_vars(query, atom, &probe_vars)?);
        nodes.push(FjNode {
            id: nodes.len(),
            subatoms: current,
        });
        available = visible;

        let remaining_vars: Vec<_> = atom_vars
            .into_iter()
            .filter(|variable| !available.contains(variable))
            .collect();
        current = vec![subatom_for_vars(query, atom, &remaining_vars)?];
    }
    nodes.push(FjNode {
        id: nodes.len(),
        subatoms: current,
    });

    let plan = FjPlan {
        nodes,
        query_variables: query.variables.len(),
    };
    plan.validate(query)?;
    Ok(plan)
}

/// Converts a left-deep binary plan to a formal FJ plan.
pub(crate) fn binary2fj(
    query: &NormalizedQuery,
    binary_plan: &BinaryPlan,
) -> Result<FjPlan, FjPlanError> {
    let atoms: Vec<_> = binary_plan
        .left_deep_sources()
        .into_iter()
        .filter_map(|source| match source {
            LeftDeepSource::Atom(atom) => Some(atom),
            LeftDeepSource::MaterializedSubplan(_) => None,
        })
        .collect();
    binary2fj_from_atoms(query, &atoms)
}

/// Conservatively factors movable probe subatoms into previous nodes.
pub(crate) fn factor_plan(
    query: &NormalizedQuery,
    plan: &FjPlan,
) -> Result<(FjPlan, PlanRewriteTrace), FjPlanError> {
    plan.validate(query)?;
    let mut rewritten = plan.clone();
    let mut trace = PlanRewriteTrace::default();

    for node_index in (1..rewritten.nodes.len()).rev() {
        let subatom_index = 1;
        loop {
            if subatom_index >= rewritten.nodes[node_index].subatoms.len() {
                break;
            }
            let subatom = rewritten.nodes[node_index].subatoms[subatom_index].clone();
            let before = format!("{rewritten:?}");
            let available = available_before(&rewritten, node_index);
            if !subatom
                .vars
                .iter()
                .all(|variable| available.contains(variable))
            {
                trace_rejected(
                    &mut trace,
                    node_index,
                    subatom_index,
                    subatom,
                    "subatom has variables unavailable before node",
                    before,
                    &rewritten,
                );
                break;
            }
            if rewritten.nodes[node_index - 1]
                .subatoms
                .iter()
                .any(|existing| existing.atom == subatom.atom)
            {
                trace_rejected(
                    &mut trace,
                    node_index,
                    subatom_index,
                    subatom,
                    "previous node already contains atom occurrence",
                    before,
                    &rewritten,
                );
                break;
            }

            let moved = rewritten.nodes[node_index].subatoms.remove(subatom_index);
            rewritten.nodes[node_index - 1].subatoms.push(moved.clone());
            if rewritten.validate(query).is_err() {
                let _restored = rewritten.nodes[node_index - 1].subatoms.pop();
                rewritten.nodes[node_index]
                    .subatoms
                    .insert(subatom_index, moved.clone());
                trace_rejected(
                    &mut trace,
                    node_index,
                    subatom_index,
                    moved,
                    "rewritten plan failed validation",
                    before,
                    &rewritten,
                );
                break;
            }
            trace.steps.push(PlanRewriteStep {
                from_node: node_index,
                to_node: node_index - 1,
                subatom_index,
                subatom: moved,
                outcome: PlanRewriteOutcome::Moved,
                reason: "moved".to_owned(),
                before,
                after: format!("{rewritten:?}"),
            });
        }
    }

    Ok((rewritten, trace))
}

fn full_subatom(query: &NormalizedQuery, atom: AtomOccurrenceId) -> Result<FjSubatom, FjPlanError> {
    let vars = atom_vars(query, atom)?;
    subatom_for_vars(query, atom, &vars)
}

fn subatom_for_vars(
    query: &NormalizedQuery,
    atom: AtomOccurrenceId,
    vars: &[usize],
) -> Result<FjSubatom, FjPlanError> {
    let occurrence = occurrence(query, atom)?;
    let mut field_ids = Vec::new();
    for variable in vars {
        field_ids.push(field_id_for_var(occurrence, *variable)?);
    }
    Ok(FjSubatom {
        atom,
        vars: vars.to_vec(),
        field_ids,
    })
}

fn atom_vars(query: &NormalizedQuery, atom: AtomOccurrenceId) -> Result<Vec<usize>, FjPlanError> {
    Ok(occurrence(query, atom)?.variable_tuple.clone())
}

fn occurrence(
    query: &NormalizedQuery,
    atom: AtomOccurrenceId,
) -> Result<&AtomOccurrence, FjPlanError> {
    query
        .atoms
        .get(atom.0)
        .ok_or(FjPlanError::UnknownAtom { atom })
}

fn field_id_for_var(occurrence: &AtomOccurrence, variable: usize) -> Result<usize, FjPlanError> {
    occurrence
        .fields
        .iter()
        .find_map(|field| match field.term {
            NormalizedTerm::Variable(bound) if bound == variable => Some(field.field_id),
            _ => None,
        })
        .ok_or(FjPlanError::VariableOutsideAtom {
            atom: occurrence.id,
            variable,
        })
}

fn vars_in_subatoms(subatoms: &[FjSubatom]) -> BTreeSet<usize> {
    subatoms
        .iter()
        .flat_map(|subatom| subatom.vars.iter().copied())
        .collect()
}

fn available_before(plan: &FjPlan, node_index: usize) -> BTreeSet<usize> {
    plan.nodes
        .iter()
        .take(node_index)
        .flat_map(|node| node.subatoms.iter())
        .flat_map(|subatom| subatom.vars.iter().copied())
        .collect()
}

fn trace_rejected(
    trace: &mut PlanRewriteTrace,
    from_node: usize,
    subatom_index: usize,
    subatom: FjSubatom,
    reason: &str,
    before: String,
    plan: &FjPlan,
) {
    trace.steps.push(PlanRewriteStep {
        from_node,
        to_node: from_node.saturating_sub(1),
        subatom_index,
        subatom,
        outcome: PlanRewriteOutcome::Rejected,
        reason: reason.to_owned(),
        before,
        after: format!("{plan:?}"),
    });
}

#[cfg(test)]
#[path = "binary2fj_tests.rs"]
mod tests;
