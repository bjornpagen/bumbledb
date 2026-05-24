use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb_core::query_ir::{
    TypedClause, TypedFieldBinding, TypedFindTerm, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use crate::{
    Environment, Fact, InputBindings, QueryExecutionOptions, Result, StorageSchema, TracePhase,
    Value,
};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn profiled_execution_matches_plain_and_emits_top_level_spans() -> Result<()> {
    let (env, schema) = env_and_schema("profiled")?;
    insert_clover(&env, &schema)?;
    let query = clover_query(["x", "a", "b", "c"], &[0, 1, 2, 3]);

    let profiled = env.read(|txn| {
        txn.execute_query_profiled(
            &schema,
            &query,
            &InputBindings::new(),
            QueryExecutionOptions::default(),
        )
    })?;
    let plain = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(plain, profiled.result);
    let phases = profiled
        .trace
        .spans
        .iter()
        .map(|span| span.phase)
        .collect::<Vec<_>>();
    assert!(phases.contains(&TracePhase::Normalize));
    assert!(phases.contains(&TracePhase::PlanSelect));
    assert!(phases.contains(&TracePhase::PlannerStats));
    assert!(phases.contains(&TracePhase::BaseImageCacheLookup));
    assert!(phases.contains(&TracePhase::BaseImageLoad));
    assert!(phases.contains(&TracePhase::SourceFilterEncode));
    assert!(phases.contains(&TracePhase::ColtBuild));
    assert!(phases.contains(&TracePhase::ColtIter));
    assert!(phases.contains(&TracePhase::ColtForce));
    assert!(phases.contains(&TracePhase::ColtGet));
    assert!(phases.contains(&TracePhase::CoverChoice));
    assert!(phases.contains(&TracePhase::ExecuteNode));
    assert!(phases.contains(&TracePhase::ProbeSibling));
    assert!(phases.contains(&TracePhase::BindingExtend));
    assert!(phases.contains(&TracePhase::SinkConsume));
    assert!(phases.contains(&TracePhase::SinkFinish));
    assert!(profiled.trace.counters.live_rows_scanned > 0);
    assert!(profiled.trace.counters.column_values_loaded > 0);
    assert!(profiled.trace.counters.source_filter_rows_tested > 0);
    assert!(profiled.trace.counters.source_filter_survivors > 0);
    assert!(profiled.trace.counters.colt_nodes_created > 0);
    assert!(profiled.trace.counters.colt_offsets_scanned > 0);
    assert!(profiled.trace.counters.tuples_yielded > 0);
    assert!(profiled.trace.counters.cover_choices > 0);
    assert!(profiled.trace.counters.probe_calls > 0);
    assert!(profiled.trace.counters.recursive_node_entries > 0);
    assert!(profiled.trace.counters.binding_copies > 0);
    assert!(profiled.trace.counters.source_frame_changes > 0);
    assert!(profiled.trace.counters.sink_consumes > 0);
    assert!(profiled.trace.counters.decoded_values > 0);
    assert!(!profiled.trace.metadata.selected_plan_family.is_empty());
    assert!(profiled.trace.metadata.node_count > 0);
    assert_eq!(profiled.trace.metadata.cover_policy, "DynamicMinKeys");
    assert_eq!(profiled.trace.metadata.output_mode, "Materialized");
    Ok(())
}

#[test]
fn profiled_execution_rejects_malformed_query() -> Result<()> {
    let (env, schema) = env_and_schema("profiled-invalid")?;
    let mut query = clover_query(["x", "a", "b", "c"], &[0]);
    query.variables[0].id = 99;

    let result = env.read(|txn| {
        txn.execute_query_profiled(
            &schema,
            &query,
            &InputBindings::new(),
            QueryExecutionOptions::default(),
        )
    });

    assert!(result.is_err());
    Ok(())
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "bumbledb-profiled-{name}-{}-{id}",
        std::process::id()
    ));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "ProfiledExecutor",
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
