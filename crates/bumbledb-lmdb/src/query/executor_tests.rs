use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb_core::encoding::encode_u64;
use bumbledb_core::query_ir::{
    TypedClause, TypedFieldBinding, TypedFindTerm, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::{count_bindings_for_test, execute_plan_for_test, execute_plan_with_policy_for_test};
use crate::base_image::{ColumnImage, RelationBaseImage, RelationStats};
use crate::colt::ColtSource;
use crate::query::cover::{CoverPolicy, ExecutionStats, choose_cover};
use crate::query::free_join::{FjNode, FjPlan, FjSubatom, NodeList};
use crate::query::model::AtomOccurrenceId;
use crate::query::normalize::normalize_query;
use crate::query::runtime_frame::SourceStore;
use crate::query::trace::QueryTrace;
use crate::storage_format::FactHandle;
use crate::tuple::{EncodedTuple, GhtSource, KeyCountEstimate, TupleField, TupleSchema};
use crate::{Environment, Fact, InputBindings, QueryResultSet, Result, StorageSchema, Value};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn free_join_executor_clover_binary_plan_exact_output() -> Result<()> {
    let (env, schema) = env_and_schema("clover-binary")?;
    insert_clover(&env, &schema)?;
    let query = clover_query(["x", "a", "b", "c"], &[0, 1, 2, 3]);
    let plan = FjPlan {
        query_variables: 4,
        nodes: plan_nodes([
            node(0, [sub(0, [0, 1], [0, 1]), sub(1, [0], [0])]),
            node(1, [sub(1, [2], [1]), sub(2, [0], [0])]),
            node(2, [sub(2, [3], [1])]),
        ]),
    };

    let result = env.read(|txn| execute_plan_for_test(txn, &schema, &query, &plan))?;

    assert_eq!(result.facts, vec![row([0, 10, 20, 30])]);
    Ok(())
}

#[test]
fn free_join_executor_clover_factorized_plan_exact_output() -> Result<()> {
    let (env, schema) = env_and_schema("clover-factorized")?;
    insert_clover(&env, &schema)?;
    let query = clover_query(["x", "a", "b", "c"], &[0, 1, 2, 3]);
    let plan = FjPlan {
        query_variables: 4,
        nodes: plan_nodes([
            node(
                0,
                [sub(0, [0, 1], [0, 1]), sub(1, [0], [0]), sub(2, [0], [0])],
            ),
            node(1, [sub(1, [2], [1])]),
            node(2, [sub(2, [3], [1])]),
        ]),
    };

    let result = env.read(|txn| execute_plan_for_test(txn, &schema, &query, &plan))?;

    assert_eq!(result.facts, vec![row([0, 10, 20, 30])]);
    Ok(())
}

#[test]
fn free_join_executor_triangle_singleton_plan_exact_output() -> Result<()> {
    let (env, schema) = env_and_schema("triangle-singleton")?;
    env.write(|txn| {
        for fact in [
            pair("R", 1, 2),
            pair("R", 1, 3),
            pair("S", 2, 4),
            pair("S", 3, 4),
            pair("T", 4, 1),
            pair("T", 4, 9),
        ] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "y", "z"],
        &[0, 1, 2],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 1), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 2), (1, "right", 0)]),
        ],
    );
    let plan = FjPlan {
        query_variables: 3,
        nodes: plan_nodes([
            node(0, [sub(0, [0], [0]), sub(2, [0], [1])]),
            node(1, [sub(0, [1], [1]), sub(1, [1], [0])]),
            node(2, [sub(1, [2], [1]), sub(2, [2], [0])]),
        ]),
    };

    let result = env.read(|txn| execute_plan_for_test(txn, &schema, &query, &plan))?;

    assert_eq!(result.facts, vec![row([1, 2, 4]), row([1, 3, 4])]);
    Ok(())
}

#[test]
fn free_join_executor_chain_and_star_default_plans() -> Result<()> {
    let (env, schema) = env_and_schema("chain-star")?;
    env.write(|txn| {
        for fact in [
            pair("R", 1, 2),
            pair("R", 1, 9),
            pair("S", 2, 3),
            pair("S", 1, 5),
            pair("S", 9, 8),
            pair("T", 3, 4),
            pair("T", 1, 6),
            pair("T", 7, 1),
        ] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let chain = typed_query(
        &["x", "y", "z", "w"],
        &[0, 3],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 1), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 2), (1, "right", 3)]),
        ],
    );
    let star = clover_query(["x", "a", "b", "c"], &[0]);

    let chain_result = env.read(|txn| txn.execute_query(&schema, &chain, &InputBindings::new()))?;
    let star_result = env.read(|txn| txn.execute_query(&schema, &star, &InputBindings::new()))?;

    assert_eq!(chain_result.facts, vec![row([1, 4])]);
    assert_eq!(star_result.facts, vec![row([1])]);
    Ok(())
}

#[test]
fn free_join_executor_self_join_exact_output() -> Result<()> {
    let (env, schema) = env_and_schema("self-join")?;
    env.write(|txn| {
        for fact in [pair("R", 1, 2), pair("R", 2, 3), pair("R", 1, 4)] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "y", "z"],
        &[0, 2],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(0, "R", [(0, "left", 1), (1, "right", 2)]),
        ],
    );

    let result = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(result.facts, vec![row([1, 3])]);
    Ok(())
}

#[test]
fn free_join_executor_static_atom_existence_success_and_failure() -> Result<()> {
    let (env, schema) = env_and_schema("static-success")?;
    env.write(|txn| {
        txn.insert(&schema, &pair("R", 1, 10))?;
        txn.insert(&schema, &unary("E", 99))?;
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x"],
        &[0],
        vec![atom(0, "R", [(0, "left", 0)]), atom(4, "E", [])],
    );

    let result = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    assert_eq!(result.facts, vec![row([1])]);

    let (empty_env, empty_schema) = env_and_schema("static-failure")?;
    empty_env.write(|txn| txn.insert(&empty_schema, &pair("R", 1, 10)))?;
    let empty =
        empty_env.read(|txn| txn.execute_query(&empty_schema, &query, &InputBindings::new()))?;
    assert!(empty.facts.is_empty());
    Ok(())
}

#[test]
fn free_join_executor_multi_variable_cover_conflict_rejection() -> Result<()> {
    let (env, schema) = env_and_schema("cover-conflict")?;
    env.write(|txn| {
        for fact in [unary("U", 1), pair("S", 1, 10), pair("S", 2, 20)] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "a"],
        &[0, 1],
        vec![
            atom(3, "U", [(0, "left", 0)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 1)]),
        ],
    );
    let plan = FjPlan {
        query_variables: 2,
        nodes: plan_nodes([
            node(0, [sub(0, [0], [0])]),
            node(1, [sub(1, [0, 1], [0, 1])]),
        ]),
    };

    let result = env.read(|txn| execute_plan_for_test(txn, &schema, &query, &plan))?;

    assert_eq!(result.facts, vec![row([1, 10])]);
    Ok(())
}

#[test]
fn free_join_executor_invalid_plan_cannot_bypass_validation() -> Result<()> {
    let (env, schema) = env_and_schema("invalid-plan")?;
    let query = clover_query(["x", "a", "b", "c"], &[0]);
    let invalid = FjPlan {
        query_variables: 4,
        nodes: plan_nodes([node(0, [sub(0, [0], [0])])]),
    };

    let result = env.read(|txn| execute_plan_for_test(txn, &schema, &query, &invalid));

    assert!(result.is_err());
    Ok(())
}

#[test]
fn free_join_executor_binding_sink_counts_full_bindings() -> Result<()> {
    let (env, schema) = env_and_schema("binding-count")?;
    env.write(|txn| {
        for fact in [
            pair("R", 1, 10),
            pair("R", 1, 11),
            pair("S", 1, 20),
            pair("S", 1, 21),
        ] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "a", "b"],
        &[0],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
        ],
    );

    let public = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    let bindings = env.read(|txn| count_bindings_for_test(txn, &schema, &query))?;

    assert_eq!(
        public,
        QueryResultSet::new(
            vec![crate::ResultColumn::Variable("x".to_owned())],
            vec![row([1])]
        )
    );
    assert_eq!(bindings, 4);
    Ok(())
}

#[test]
fn free_join_executor_matches_reference_for_small_query() -> Result<()> {
    let (env, schema) = env_and_schema("reference")?;
    let r = [(1, 2), (1, 3), (2, 4)];
    let s = [(2, 9), (3, 9), (4, 8)];
    env.write(|txn| {
        for (left, right) in r {
            txn.insert(&schema, &pair("R", left, right))?;
        }
        for (left, right) in s {
            txn.insert(&schema, &pair("S", left, right))?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "y", "z"],
        &[0, 2],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 1), (1, "right", 2)]),
        ],
    );

    let result = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(result.facts, vec![row([1, 9]), row([2, 8])]);
    Ok(())
}

#[test]
fn dynamic_cover_triangle_with_asymmetric_sizes_chooses_smaller_cover() -> Result<()> {
    let (env, schema) = env_and_schema("dynamic-triangle")?;
    env.write(|txn| {
        for fact in [
            pair("R", 1, 2),
            pair("R", 1, 3),
            pair("R", 9, 9),
            pair("S", 2, 4),
            pair("S", 3, 4),
            pair("T", 4, 1),
        ] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "y", "z"],
        &[0, 1, 2],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 1), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 2), (1, "right", 0)]),
        ],
    );
    let plan = FjPlan {
        query_variables: 3,
        nodes: plan_nodes([
            node(0, [sub(0, [0], [0]), sub(2, [0], [1])]),
            node(1, [sub(0, [1], [1]), sub(1, [1], [0])]),
            node(2, [sub(1, [2], [1]), sub(2, [2], [0])]),
        ]),
    };

    let (result, stats) = env.read(|txn| {
        execute_plan_with_policy_for_test(txn, &schema, &query, &plan, CoverPolicy::DynamicMinKeys)
    })?;

    assert_eq!(result.facts, vec![row([1, 2, 4]), row([1, 3, 4])]);
    assert_eq!(stats.cover_choices[0].chosen_subatom, 1);
    assert!(matches!(
        stats.cover_choices[0].candidates[1].key_count,
        KeyCountEstimate::Estimate(1)
    ));
    Ok(())
}

#[test]
fn dynamic_cover_choice_can_change_by_prefix_subtrie() -> Result<()> {
    let (env, schema) = env_and_schema("dynamic-prefix")?;
    env.write(|txn| {
        for fact in [
            pair("R", 1, 10),
            pair("R", 1, 11),
            pair("R", 1, 12),
            pair("R", 2, 20),
            pair("S", 1, 10),
            pair("S", 2, 20),
            pair("S", 2, 21),
            pair("S", 2, 22),
        ] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "y"],
        &[0, 1],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 1)]),
        ],
    );
    let plan = FjPlan {
        query_variables: 2,
        nodes: plan_nodes([
            node(0, [sub(0, [0], [0]), sub(1, [0], [0])]),
            node(1, [sub(0, [1], [1]), sub(1, [1], [1])]),
        ]),
    };

    let (_result, stats) = env.read(|txn| {
        execute_plan_with_policy_for_test(txn, &schema, &query, &plan, CoverPolicy::DynamicMinKeys)
    })?;
    let node_one_choices: Vec<_> = stats
        .cover_choices
        .iter()
        .filter(|choice| choice.node == 1)
        .map(|choice| choice.chosen_subatom)
        .collect();

    assert!(node_one_choices.contains(&0));
    assert!(node_one_choices.contains(&1));
    Ok(())
}

#[test]
fn dynamic_cover_static_and_dynamic_modes_return_same_set() -> Result<()> {
    let (env, schema) = env_and_schema("dynamic-static-equivalence")?;
    insert_clover(&env, &schema)?;
    let query = clover_query(["x", "a", "b", "c"], &[0, 1, 2, 3]);
    let plan = FjPlan {
        query_variables: 4,
        nodes: plan_nodes([
            node(
                0,
                [sub(0, [0, 1], [0, 1]), sub(1, [0], [0]), sub(2, [0], [0])],
            ),
            node(1, [sub(1, [2], [1])]),
            node(2, [sub(2, [3], [1])]),
        ]),
    };

    let (dynamic, dynamic_stats) = env.read(|txn| {
        execute_plan_with_policy_for_test(txn, &schema, &query, &plan, CoverPolicy::DynamicMinKeys)
    })?;
    let (static_first, static_stats) = env.read(|txn| {
        execute_plan_with_policy_for_test(txn, &schema, &query, &plan, CoverPolicy::StaticFirst)
    })?;

    assert_eq!(dynamic, static_first);
    assert!(!dynamic_stats.cover_choices.is_empty());
    assert!(!static_stats.cover_choices.is_empty());
    Ok(())
}

#[test]
fn dynamic_cover_tie_break_is_deterministic() -> Result<()> {
    let (env, schema) = env_and_schema("dynamic-tie")?;
    env.write(|txn| {
        txn.insert(&schema, &pair("R", 1, 10))?;
        txn.insert(&schema, &pair("S", 1, 20))?;
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x"],
        &[0],
        vec![
            atom(0, "R", [(0, "left", 0)]),
            atom(1, "S", [(0, "left", 0)]),
        ],
    );
    let plan = FjPlan {
        query_variables: 1,
        nodes: plan_nodes([node(0, [sub(0, [0], [0]), sub(1, [0], [0])])]),
    };

    let (_result, stats) = env.read(|txn| {
        execute_plan_with_policy_for_test(txn, &schema, &query, &plan, CoverPolicy::DynamicMinKeys)
    })?;

    assert_eq!(stats.cover_choices[0].chosen_subatom, 0);
    assert!(stats.cover_choices[0].tie_break);
    Ok(())
}

#[test]
fn dynamic_cover_prefers_smaller_exact_map_when_available() -> Result<()> {
    let schema = StorageSchema::new(schema(), 511)?;
    let query = typed_query(
        &["x"],
        &[0],
        vec![
            atom(0, "R", [(0, "left", 0)]),
            atom(1, "S", [(0, "left", 0)]),
        ],
    );
    let normalized = normalize_query(schema.descriptor(), &query)?;
    let plan = FjPlan {
        query_variables: 1,
        nodes: plan_nodes([node(0, [sub(0, [0], [0]), sub(1, [0], [0])])]),
    };
    let validated = plan
        .validate(&normalized)
        .map_err(|error| crate::Error::invalid_query(error.to_string()))?;
    let mut sources = SourceStore::with_atom_count(2);
    insert_manual_colt(&mut sources, 0, [1])?;
    insert_manual_colt(&mut sources, 1, [1, 2, 3])?;
    let key = tuple_x(1)?;
    let r = sources
        .source_for_atom(AtomOccurrenceId(0))
        .ok_or_else(|| crate::Error::corrupt("missing manual source"))?;
    assert!(r.get(key.as_ref()).is_some());
    let mut stats = ExecutionStats::default();

    let chosen = choose_cover(
        &validated,
        &validated.nodes[0],
        &sources,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
    )?;

    assert_eq!(chosen, 0);
    assert!(matches!(
        stats.cover_choices[0].candidates[0].key_count,
        KeyCountEstimate::Exact(1)
    ));
    assert!(matches!(
        stats.cover_choices[0].candidates[1].key_count,
        KeyCountEstimate::Estimate(3)
    ));
    Ok(())
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("bumbledb-prd12-{name}-{}-{id}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "Executor",
        vec![
            pair_relation("R"),
            pair_relation("S"),
            pair_relation("T"),
            unary_relation("U"),
            unary_relation("E"),
        ],
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

fn unary_relation(name: &str) -> RelationDescriptor {
    RelationDescriptor::new(name, vec![FieldDescriptor::new("left", ValueType::U64)])
}

fn insert_clover(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.write(|txn| {
        for fact in [
            pair("R", 0, 10),
            pair("R", 1, 11),
            pair("R", 2, 12),
            pair("S", 0, 20),
            pair("S", 2, 21),
            pair("S", 3, 22),
            pair("T", 0, 30),
            pair("T", 3, 31),
            pair("T", 1, 32),
        ] {
            txn.insert(schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })
}

fn clover_query<const N: usize>(vars: [&str; 4], find: &[usize; N]) -> TypedQuery {
    typed_query(
        &vars,
        find,
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 0), (1, "right", 3)]),
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

fn atom<const N: usize>(
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
fn pair(relation: &str, left: u64, right: u64) -> Fact {
    Fact::new(
        relation,
        [("left", Value::U64(left)), ("right", Value::U64(right))],
    )
}
fn unary(relation: &str, value: u64) -> Fact {
    Fact::new(relation, [("left", Value::U64(value))])
}

fn row<const N: usize>(values: [u64; N]) -> Vec<Value> {
    values.into_iter().map(Value::U64).collect()
}

#[rustfmt::skip]
fn insert_manual_colt<const N: usize>(sources: &mut SourceStore, atom: usize, values: [u64; N]) -> Result<()> {
    let image = RelationBaseImage {
        relation_id: atom as u32,
        name: format!("A{atom}"),
        row_handles: Rc::new((0..values.len()).map(|offset| FactHandle([offset as u8; 16])).collect()),
        columns: BTreeMap::from([(0, ColumnImage { field_id: 0, width: 8, values: Rc::new(values.into_iter().flat_map(encode_u64).collect()) })]),
        stats: RelationStats { row_count: N },
    };
    let mut trace = QueryTrace::new();
    sources.insert_filtered_traced_labeled(AtomOccurrenceId(atom), Rc::new(image), vec![TupleSchema::new(vec![TupleField::new(0, Some(0), 8).map_err(|error| crate::Error::corrupt(error.to_string()))?])], ColtSource::build_config(Vec::new(), String::new(), true), &mut trace);
    Ok(())
}

fn tuple_x(value: u64) -> Result<EncodedTuple> {
    let schema = TupleSchema::new(vec![
        TupleField::new(0, Some(0), 8).map_err(|error| crate::Error::corrupt(error.to_string()))?,
    ]);
    EncodedTuple::new(&schema, encode_u64(value).to_vec())
        .map_err(|error| crate::Error::corrupt(error.to_string()))
}

fn node<const N: usize>(id: usize, subatoms: [FjSubatom; N]) -> FjNode {
    FjNode {
        id,
        subatoms: subatoms.into_iter().collect(),
    }
}

fn plan_nodes<const N: usize>(nodes: [FjNode; N]) -> NodeList {
    nodes.into_iter().collect()
}
fn sub<const V: usize, const F: usize>(
    atom: usize,
    vars: [usize; V],
    field_ids: [usize; F],
) -> FjSubatom {
    FjSubatom {
        atom: crate::query::model::AtomOccurrenceId(atom),
        vars: vars.into_iter().collect(),
        field_ids: field_ids.into_iter().collect(),
    }
}
