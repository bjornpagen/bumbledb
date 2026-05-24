use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::ValueType;

use crate::query::free_join::{FjNode, FjPlan, FjPlanError, FjSubatom};
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};

#[test]
fn free_join_validates_clover_binary_plan() -> Result<(), FjPlanError> {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [0], [0])]),
            node(1, [sub(1, [2], [1]), sub(2, [0], [0])]),
            node(2, [sub(2, [3], [1])]),
        ]
        .into_iter()
        .collect(),
    };

    let validated = plan.validate(&query)?;

    assert_eq!(validated.node_new_vars(&validated.nodes[0]), &[0, 1]);
    assert_eq!(validated.node_covers(&validated.nodes[0])[0].subatom, 0);
    assert_eq!(validated.atom_partitions.len(), 3);
    Ok(())
}

#[test]
fn free_join_validates_clover_factorized_plan() {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [
            node(
                0,
                [sub(0, [0, 1], [0, 1]), sub(1, [0], [0]), sub(2, [0], [0])],
            ),
            node(1, [sub(1, [2], [1])]),
            node(2, [sub(2, [3], [1])]),
        ]
        .into_iter()
        .collect(),
    };

    assert!(plan.validate(&query).is_ok());
}

#[test]
fn free_join_validates_clover_generic_plan() -> Result<(), FjPlanError> {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [
            node(0, [sub(0, [0], [0]), sub(1, [0], [0]), sub(2, [0], [0])]),
            node(1, [sub(0, [1], [1])]),
            node(2, [sub(1, [2], [1])]),
            node(3, [sub(2, [3], [1])]),
        ]
        .into_iter()
        .collect(),
    };

    let validated = plan.validate(&query)?;

    assert_eq!(validated.node_covers(&validated.nodes[0]).len(), 3);
    Ok(())
}

#[test]
fn free_join_validates_triangle_singleton_plan() {
    let query = query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 0]]);
    let plan = FjPlan {
        query_variables: 3,
        nodes: [
            node(0, [sub(0, [0], [0]), sub(2, [0], [1])]),
            node(1, [sub(0, [1], [1]), sub(1, [1], [0])]),
            node(2, [sub(1, [2], [1]), sub(2, [2], [0])]),
        ]
        .into_iter()
        .collect(),
    };

    assert!(plan.validate(&query).is_ok());
}

#[test]
fn free_join_validates_chain_binary_shape() {
    let query = query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]]);
    let plan = FjPlan {
        query_variables: 5,
        nodes: [
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [1], [0])]),
            node(1, [sub(1, [2], [1]), sub(2, [2], [0])]),
            node(2, [sub(2, [3], [1]), sub(3, [3], [0])]),
            node(3, [sub(3, [4], [1])]),
        ]
        .into_iter()
        .collect(),
    };

    assert!(plan.validate(&query).is_ok());
}

#[test]
fn free_join_rejects_missing_partition() {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [node(0, [sub(0, [0, 1], [0, 1]), sub(1, [0], [0])])]
            .into_iter()
            .collect(),
    };

    assert!(matches!(
        plan.validate(&query),
        Err(FjPlanError::MissingPartition { .. })
    ));
}

#[test]
fn free_join_rejects_duplicate_partition_assignment() {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [
            node(0, [sub(0, [0, 1], [0, 1])]),
            node(1, [sub(0, [0], [0])]),
        ]
        .into_iter()
        .collect(),
    };

    assert!(matches!(
        plan.validate(&query),
        Err(FjPlanError::DuplicatePartitionVariable { .. })
    ));
}

#[test]
fn free_join_rejects_duplicate_atom_in_node() {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [node(0, [sub(0, [0], [0]), sub(0, [1], [1])])]
            .into_iter()
            .collect(),
    };

    assert!(matches!(
        plan.validate(&query),
        Err(FjPlanError::DuplicateAtomInNode { .. })
    ));
}

#[test]
fn free_join_rejects_missing_cover() {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [FjNode {
            id: 0,
            subatoms: Default::default(),
        }]
        .into_iter()
        .collect(),
    };

    assert!(matches!(
        plan.validate(&query),
        Err(FjPlanError::MissingCover { .. })
    ));
}

#[test]
fn free_join_rejects_unavailable_probe_variable() {
    let query = query_from_atoms([vec![0], vec![1]]);
    let plan = FjPlan {
        query_variables: 2,
        nodes: [node(0, [sub(0, [0], [0]), sub(1, [1], [0])])]
            .into_iter()
            .collect(),
    };

    assert!(matches!(
        plan.validate(&query),
        Err(FjPlanError::UnavailableProbeVariable { .. })
    ));
}

#[test]
fn free_join_rejects_variable_outside_atom_occurrence() {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [node(0, [sub(0, [2], [0])])].into_iter().collect(),
    };

    assert!(matches!(
        plan.validate(&query),
        Err(FjPlanError::VariableOutsideAtom { .. })
    ));
}

#[test]
fn free_join_rejects_duplicate_subatom_variable() {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: [node(0, [sub(0, [0, 0], [0, 0])])].into_iter().collect(),
    };

    assert!(matches!(
        plan.validate(&query),
        Err(FjPlanError::DuplicateSubatomVariable { .. })
    ));
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
