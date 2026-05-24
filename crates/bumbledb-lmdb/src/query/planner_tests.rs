use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::ValueType;

use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};
use crate::query::planner::{
    BinaryPlan, BinaryPlanError, LeftDeepSource, deterministic_binary_plan,
};

#[test]
fn planner_builds_two_atom_join() -> Result<(), BinaryPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2]]);
    let plan = deterministic_binary_plan(&query)?;

    plan.validate(&query)?;
    assert_eq!(
        plan.left_deep_sources(),
        vec![atom_source(0), atom_source(1)]
    );
    Ok(())
}

#[test]
fn planner_builds_chain_left_deep_plan() -> Result<(), BinaryPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]]);
    let plan = deterministic_binary_plan(&query)?;

    plan.validate(&query)?;
    assert_eq!(
        plan.left_deep_sources(),
        vec![
            atom_source(0),
            atom_source(1),
            atom_source(2),
            atom_source(3)
        ]
    );
    Ok(())
}

#[test]
fn planner_builds_star_plan_in_atom_order() -> Result<(), BinaryPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![0, 2], vec![0, 3]]);
    let plan = deterministic_binary_plan(&query)?;

    plan.validate(&query)?;
    assert_eq!(
        plan.left_deep_sources(),
        vec![atom_source(0), atom_source(1), atom_source(2)]
    );
    Ok(())
}

#[test]
fn planner_builds_triangle_plan_in_atom_order() -> Result<(), BinaryPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 0]]);
    let plan = deterministic_binary_plan(&query)?;

    plan.validate(&query)?;
    assert_eq!(
        plan.left_deep_sources(),
        vec![atom_source(0), atom_source(1), atom_source(2)]
    );
    Ok(())
}

#[test]
fn planner_self_join_uses_distinct_occurrence_leaves() -> Result<(), BinaryPlanError> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2]]);
    let plan = deterministic_binary_plan(&query)?;

    plan.validate(&query)?;
    assert_eq!(
        plan.left_deep_sources(),
        vec![atom_source(0), atom_source(1)]
    );
    Ok(())
}

#[test]
fn planner_decomposes_bushy_right_child() {
    let left = BinaryPlan::join(BinaryPlan::leaf(0), BinaryPlan::leaf(1), [1], [0, 1, 2]);
    let right = BinaryPlan::join(BinaryPlan::leaf(2), BinaryPlan::leaf(3), [4], [3, 4, 5]);
    let root = BinaryPlan::join(left, right, [2, 3], [0, 1, 2, 3, 4, 5]);

    let decomposed = root.decompose();

    assert_eq!(decomposed.plans.len(), 2);
    assert_eq!(
        decomposed.plans[0].sources,
        vec![atom_source(2), atom_source(3)]
    );
    assert_eq!(
        decomposed.plans[1].sources,
        vec![
            atom_source(0),
            atom_source(1),
            LeftDeepSource::MaterializedSubplan(0)
        ]
    );
}

#[test]
fn planner_rejects_duplicate_leaf() {
    let query = query_from_atoms([vec![0], vec![1]]);
    let plan = BinaryPlan::join(BinaryPlan::leaf(0), BinaryPlan::leaf(0), [], [0]);

    assert!(matches!(
        plan.validate(&query),
        Err(BinaryPlanError::DuplicateLeaf { .. })
    ));
}

#[test]
fn planner_rejects_missing_leaf() {
    let query = query_from_atoms([vec![0], vec![1]]);
    let plan = BinaryPlan::leaf(0);

    assert!(matches!(
        plan.validate(&query),
        Err(BinaryPlanError::MissingLeaf { .. })
    ));
}

#[test]
fn planner_rejects_unknown_leaf() {
    let query = query_from_atoms([vec![0]]);
    let plan = BinaryPlan::leaf(3);

    assert!(matches!(
        plan.validate(&query),
        Err(BinaryPlanError::UnknownLeaf { .. })
    ));
}

#[test]
fn planner_rejects_disconnected_output_variable() {
    let query = query_from_atoms([vec![0], vec![1]]);
    let plan = BinaryPlan::join(BinaryPlan::leaf(0), BinaryPlan::leaf(1), [], [0, 1, 9]);

    assert!(matches!(
        plan.validate(&query),
        Err(BinaryPlanError::DisconnectedOutputVariable { .. })
    ));
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

fn atom_source(atom: usize) -> LeftDeepSource {
    LeftDeepSource::Atom(AtomOccurrenceId(atom))
}
