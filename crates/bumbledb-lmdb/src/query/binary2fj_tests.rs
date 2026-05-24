use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::ValueType;

use crate::query::binary2fj::{PlanRewriteOutcome, binary2fj_from_atoms, factor_plan};
use crate::query::free_join::{FjNode, FjPlan, FjPlanError, FjSubatom};
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};

#[test]
fn binary2fj_generates_clover_shape() -> Result<(), FjPlanError> {
    let query = clover_query();
    let plan = binary2fj_from_atoms(&query, &atoms([0, 1, 2]))?;

    assert_eq!(plan.nodes, clover_binary_nodes());
    Ok(())
}

#[test]
fn binary2fj_generates_chain_shape() -> Result<(), FjPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]]);
    let plan = binary2fj_from_atoms(&query, &atoms([0, 1, 2, 3]))?;

    assert_eq!(
        plan.nodes,
        vec![
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [1], [0])]),
            node(1, [sub(1, [2], [1]), sub(2, [2], [0])]),
            node(2, [sub(2, [3], [1]), sub(3, [3], [0])]),
            node(3, [sub(3, [4], [1])]),
        ]
    );
    Ok(())
}

#[test]
fn binary2fj_generates_triangle_shape_with_static_tail() -> Result<(), FjPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 0]]);
    let plan = binary2fj_from_atoms(&query, &atoms([0, 1, 2]))?;

    assert_eq!(
        plan.nodes,
        vec![
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [1], [0])]),
            node(1, [sub(1, [2], [1]), sub(2, [2, 0], [0, 1])]),
            node(2, [sub(2, [], [])]),
        ]
    );
    Ok(())
}

#[test]
fn binary2fj_generates_self_join_shape_by_occurrence_id() -> Result<(), FjPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2]]);
    let plan = binary2fj_from_atoms(&query, &atoms([0, 1]))?;

    assert_eq!(
        plan.nodes,
        vec![
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [1], [0])]),
            node(1, [sub(1, [2], [1])]),
        ]
    );
    Ok(())
}

#[test]
fn factor_transforms_clover_to_paper_shape() -> Result<(), FjPlanError> {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: clover_binary_nodes(),
    };

    let (factored, trace) = factor_plan(&query, &plan)?;

    assert_eq!(
        factored.nodes,
        vec![
            node(
                0,
                [sub(0, [0, 1], [0, 1]), sub(1, [0], [0]), sub(2, [0], [0])],
            ),
            node(1, [sub(1, [2], [1])]),
            node(2, [sub(2, [3], [1])]),
        ]
    );
    assert!(
        trace
            .steps
            .iter()
            .any(|step| step.outcome == PlanRewriteOutcome::Moved)
    );
    factored.validate(&query)?;
    Ok(())
}

#[test]
fn factor_noops_when_variables_are_unavailable() -> Result<(), FjPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 3]]);
    let plan = FjPlan {
        query_variables: 4,
        nodes: vec![
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [1], [0])]),
            node(1, [sub(1, [2], [1]), sub(2, [2, 3], [0, 1])]),
            node(2, [sub(2, [], [])]),
        ],
    };

    let (factored, trace) = factor_plan(&query, &plan)?;

    assert_eq!(factored, plan);
    assert_eq!(trace.steps[0].outcome, PlanRewriteOutcome::Rejected);
    Ok(())
}

#[test]
fn factor_noops_when_previous_node_has_same_atom() -> Result<(), FjPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![0, 2]]);
    let plan = FjPlan {
        query_variables: 3,
        nodes: vec![
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [0], [0])]),
            node(1, [sub(1, [2], [1]), sub(0, [], [])]),
        ],
    };

    let (factored, trace) = factor_plan(&query, &plan)?;

    assert_eq!(factored, plan);
    assert_eq!(trace.steps[0].outcome, PlanRewriteOutcome::Rejected);
    Ok(())
}

#[test]
fn factor_stops_at_first_unmoved_probe() -> Result<(), FjPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![0, 2], vec![0], vec![0]]);
    let plan = FjPlan {
        query_variables: 3,
        nodes: vec![
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [0], [0])]),
            node(
                1,
                [
                    sub(1, [2], [1]),
                    sub(0, [], []),
                    sub(2, [0], [0]),
                    sub(3, [0], [0]),
                ],
            ),
        ],
    };

    let (factored, trace) = factor_plan(&query, &plan)?;

    assert_eq!(factored, plan);
    assert_eq!(trace.steps.len(), 1);
    assert_eq!(trace.steps[0].outcome, PlanRewriteOutcome::Rejected);
    Ok(())
}

fn clover_binary_nodes() -> Vec<FjNode> {
    vec![
        node(0, [sub(0, [0, 1], [0, 1]), sub(1, [0], [0])]),
        node(1, [sub(1, [2], [1]), sub(2, [0], [0])]),
        node(2, [sub(2, [3], [1])]),
    ]
}

fn clover_query() -> NormalizedQuery {
    query_from_atoms([vec![0, 1], vec![0, 2], vec![0, 3]])
}

fn query_from_atoms<const N: usize>(atom_vars: [Vec<usize>; N]) -> NormalizedQuery {
    let query_variables = atom_vars
        .iter()
        .flat_map(|vars| vars.iter().copied())
        .max()
        .map_or(0, |max| max + 1);
    NormalizedQuery {
        variables: (0..query_variables)
            .map(|id| TypedVariable {
                id,
                name: format!("v{id}"),
                value_type: ValueType::U64,
            })
            .collect(),
        inputs: Vec::new(),
        find: vec![TypedFindTerm::Variable { variable: 0 }],
        atoms: atom_vars
            .into_iter()
            .enumerate()
            .map(|(id, vars)| atom(id, vars))
            .collect(),
        comparisons: Vec::new(),
    }
}

fn atom(id: usize, vars: Vec<usize>) -> AtomOccurrence {
    AtomOccurrence {
        id: AtomOccurrenceId(id),
        relation_id: id,
        relation: format!("R{id}"),
        fields: vars
            .iter()
            .enumerate()
            .map(|(field_id, variable)| NormalizedFieldBinding {
                field_id,
                field: format!("f{field_id}"),
                value_type: ValueType::U64,
                term: NormalizedTerm::Variable(*variable),
            })
            .collect(),
        variable_tuple: vars,
        source_predicates: Vec::new(),
    }
}

fn atoms<const N: usize>(ids: [usize; N]) -> Vec<AtomOccurrenceId> {
    ids.into_iter().map(AtomOccurrenceId).collect()
}

fn node<const N: usize>(id: usize, subatoms: [FjSubatom; N]) -> FjNode {
    FjNode {
        id,
        subatoms: subatoms.into_iter().collect(),
    }
}

fn sub<const V: usize, const F: usize>(
    atom: usize,
    vars: [usize; V],
    field_ids: [usize; F],
) -> FjSubatom {
    FjSubatom {
        atom: AtomOccurrenceId(atom),
        vars: vars.into_iter().collect(),
        field_ids: field_ids.into_iter().collect(),
    }
}
