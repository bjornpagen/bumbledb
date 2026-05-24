use bumbledb_core::query_ir::{
    TypedClause, TypedFieldBinding, TypedFindTerm, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use crate::query::executor::execute_query_with_plan_mode_for_test;
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};
use crate::query::normalize::normalize_query;
use crate::query::planner::{
    BinaryPlan, BinaryPlanError, LeftDeepSource, PlanFamily, PlanMode, deterministic_binary_plan,
    generate_plan_candidates, select_plan,
};
use crate::{Environment, Fact, StorageSchema, Value};

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

#[test]
fn planner_generates_candidates_for_core_shapes() -> crate::Result<()> {
    for query in [
        query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]]),
        query_from_atoms([vec![0, 1], vec![0, 2], vec![0, 3], vec![0, 4]]),
        query_from_atoms([vec![0, 1], vec![1, 2], vec![2, 0], vec![]]),
        query_from_atoms([vec![0, 1], vec![0, 2], vec![0, 3], vec![]]),
    ] {
        let candidates = generate_plan_candidates(&query)?;
        let families: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.family)
            .collect();
        assert!(families.contains(&PlanFamily::BinaryDerived));
        assert!(families.contains(&PlanFamily::FactoredBinary));
        assert!(families.contains(&PlanFamily::Singleton));
        for candidate in candidates {
            candidate
                .plan
                .validate(&query)
                .map_err(|error| crate::Error::invalid_query(error.to_string()))?;
        }
    }
    Ok(())
}

#[test]
fn planner_validates_self_join_alias_candidates() -> crate::Result<()> {
    let query = query_from_atoms([vec![0, 1], vec![1, 2]]);
    let candidates = generate_plan_candidates(&query)?;

    assert_eq!(candidates.len(), 3);
    for candidate in candidates {
        candidate
            .plan
            .validate(&query)
            .map_err(|error| crate::Error::invalid_query(error.to_string()))?;
    }
    Ok(())
}

#[test]
fn planner_forced_modes_return_same_result_set() -> crate::Result<()> {
    let (env, schema) = env_and_schema("forced-modes")?;
    insert_pairs(&env, &schema)?;
    let query = join_query();

    let binary = env.read(|txn| {
        execute_query_with_plan_mode_for_test(txn, &schema, &query, PlanMode::ForceBinaryDerived)
    })?;
    let factored = env.read(|txn| {
        execute_query_with_plan_mode_for_test(txn, &schema, &query, PlanMode::ForceFactoredBinary)
    })?;
    let singleton = env.read(|txn| {
        execute_query_with_plan_mode_for_test(txn, &schema, &query, PlanMode::ForceSingleton)
    })?;

    assert_eq!(binary, factored);
    assert_eq!(binary, singleton);
    Ok(())
}

#[test]
fn planner_skewed_clover_chooses_non_naive_plan() -> crate::Result<()> {
    let (env, schema) = env_and_schema("skewed-clover")?;
    insert_pairs(&env, &schema)?;
    let typed = clover_query();
    let normalized = normalize_query(schema.descriptor(), &typed)?;

    let selection = env.read(|txn| select_plan(txn, &schema, &normalized, PlanMode::Default))?;

    assert_ne!(selection.chosen.family, PlanFamily::BinaryDerived);
    assert!(selection.stats.storage_tx_id > 0);
    assert_eq!(selection.stats.relations.len(), 3);
    Ok(())
}

#[test]
fn planner_tie_break_is_deterministic() -> crate::Result<()> {
    let (env, schema) = env_and_schema("tie-break")?;
    insert_pairs(&env, &schema)?;
    let typed = join_query();
    let normalized = normalize_query(schema.descriptor(), &typed)?;

    let first = env.read(|txn| select_plan(txn, &schema, &normalized, PlanMode::Default))?;
    let second = env.read(|txn| select_plan(txn, &schema, &normalized, PlanMode::Default))?;

    assert_eq!(first.chosen.family, second.chosen.family);
    assert_eq!(first.candidates, second.candidates);
    Ok(())
}

#[test]
fn planner_rejects_invalid_injected_candidate_and_falls_back() -> crate::Result<()> {
    let (env, schema) = env_and_schema("invalid-injected")?;
    insert_pairs(&env, &schema)?;
    let typed = join_query();
    let normalized = normalize_query(schema.descriptor(), &typed)?;
    let invalid = BinaryPlan::leaf(0);

    let selection = env.read(|txn| {
        select_plan(
            txn,
            &schema,
            &normalized,
            PlanMode::InjectedBinary(invalid.clone()),
        )
    })?;

    assert_ne!(selection.chosen.family, PlanFamily::InjectedBinary);
    assert!(selection.candidates.len() >= 3);
    Ok(())
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

fn env_and_schema(name: &str) -> crate::Result<(Environment, StorageSchema)> {
    let path = std::env::temp_dir().join(format!("bumbledb-prd16-{name}-{}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "Planner",
        vec![pair_relation("R"), pair_relation("S"), pair_relation("T")],
    )
}

fn pair_relation(name: &str) -> RelationDescriptor {
    RelationDescriptor::new(
        name,
        vec![
            FieldDescriptor::new("left", ValueType::U64),
            FieldDescriptor::new("right", ValueType::U64),
        ],
    )
}

fn insert_pairs(env: &Environment, schema: &StorageSchema) -> crate::Result<()> {
    env.write(|txn| {
        for fact in [
            pair("R", 1, 10),
            pair("R", 1, 11),
            pair("R", 2, 20),
            pair("S", 1, 30),
            pair("S", 2, 40),
            pair("T", 1, 50),
            pair("T", 3, 60),
        ] {
            txn.insert(schema, fact)?;
        }
        Ok::<(), crate::Error>(())
    })
}

fn pair(relation: &str, left: u64, right: u64) -> Fact {
    Fact::new(
        relation,
        [("left", Value::U64(left)), ("right", Value::U64(right))],
    )
}

fn join_query() -> TypedQuery {
    typed_query(
        &["x", "a", "b"],
        &[0],
        vec![
            typed_atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            typed_atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
        ],
    )
}

fn clover_query() -> TypedQuery {
    typed_query(
        &["x", "a", "b", "c"],
        &[0, 1, 2, 3],
        vec![
            typed_atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            typed_atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
            typed_atom(2, "T", [(0, "left", 0), (1, "right", 3)]),
        ],
    )
}

fn typed_query<const N: usize>(
    variables: &[&str],
    find: &[usize; N],
    atoms: Vec<TypedRelationAtom>,
) -> TypedQuery {
    TypedQuery {
        variables: variables
            .iter()
            .enumerate()
            .map(|(id, name)| TypedVariable {
                id,
                name: (*name).to_owned(),
                value_type: ValueType::U64,
            })
            .collect(),
        inputs: Vec::new(),
        find: find
            .iter()
            .copied()
            .map(|variable| TypedFindTerm::Variable { variable })
            .collect(),
        clauses: atoms.into_iter().map(TypedClause::Relation).collect(),
    }
}

fn typed_atom<const N: usize>(
    relation_id: usize,
    relation: &str,
    fields: [(usize, &str, usize); N],
) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id,
        relation: relation.to_owned(),
        fields: fields
            .into_iter()
            .map(|(field_id, field, variable)| TypedFieldBinding {
                field_id,
                field: field.to_owned(),
                value_type: ValueType::U64,
                term: TypedTerm::Variable(variable),
            })
            .collect(),
    }
}
