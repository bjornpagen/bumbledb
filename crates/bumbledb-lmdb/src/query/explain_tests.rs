use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb_core::query_ir::{
    TypedClause, TypedFieldBinding, TypedFindTerm, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::{BenchmarkReport, ExplainConfig, QueryPlan, TraceSpan, summarize_trace_json};
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats, VectorizedStats};
use crate::query::planner::PlanMode;
use crate::query::sink::{OutputMode, OutputStats};
use crate::{Environment, Fact, Result, StorageSchema, Value};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn explain_golden_for_singleton_mode() -> Result<()> {
    let (env, schema) = env_and_schema("singleton")?;
    let query = triangle_query();
    let text = env.read(|txn| {
        QueryPlan::build(
            txn,
            &schema,
            &query,
            ExplainConfig {
                plan_mode: PlanMode::ForceSingleton,
                ..ExplainConfig::default()
            },
        )
        .map(|plan| plan.explain())
    })?;

    assert!(text.contains("formal singleton-subatom Free Join plan"));
    assert!(text.contains("node 0"));
    assert!(text.contains("subatom 0: atom=AtomOccurrenceId"));
    assert!(!text.contains("free_join_node id="));
    assert!(!text.contains("bind_vars"));
    Ok(())
}

#[test]
fn explain_golden_for_binary_and_factored_modes() -> Result<()> {
    let (env, schema) = env_and_schema("binary-factored")?;
    let query = clover_query();
    let binary = explain(&env, &schema, &query, PlanMode::ForceBinaryDerived)?;
    let factored = explain(&env, &schema, &query, PlanMode::ForceFactoredBinary)?;

    assert!(binary.contains("binary-derived Free Join plan"));
    assert!(binary.contains("formal Free Join plan after factorization/selection"));
    assert!(factored.contains("factored Free Join plan"));
    assert!(factored.contains("atom partitions"));
    assert!(factored.contains("GHT schema"));
    Ok(())
}

#[test]
fn explain_golden_for_dynamic_cover_and_modes() -> Result<()> {
    let (env, schema) = env_and_schema("dynamic-cover")?;
    let query = clover_query();
    let text = env.read(|txn| {
        QueryPlan::build(
            txn,
            &schema,
            &query,
            ExplainConfig {
                execution_mode: ExecutionMode::Vectorized { batch_size: 100 },
                cover_policy: CoverPolicy::DynamicMinKeys,
                output_mode: OutputMode::Factorized,
                ..ExplainConfig::default()
            },
        )
        .map(|plan| plan.explain())
    })?;

    assert!(text.contains("cover policy: DynamicMinKeys"));
    assert!(text.contains("execution mode: vectorized batch_size=100"));
    assert!(text.contains("output mode: internal factorized"));
    assert!(text.contains("sink mode: internal factorized projection sink"));
    assert!(text.contains("source kind: COLT"));
    assert!(text.contains("storage_tx_id"));
    assert!(text.contains("no aggregation support"));
    Ok(())
}

#[test]
fn benchmark_renderers_include_required_fields() {
    let report = BenchmarkReport {
        plan_mode: "factored".to_owned(),
        batch_size: 1000,
        cover_mode: "dynamic".to_owned(),
        output_mode: "factorized".to_owned(),
        source_mode: "COLT".to_owned(),
        sink_mode: "internal factorized projection sink".to_owned(),
        execution: ExecutionStats {
            vectorized: VectorizedStats {
                batch_size: 1000,
                batches: 2,
                input_tuples: 10,
                survivor_tuples: 5,
                failed_tuples: 5,
                probe_calls: 10,
            },
            ..ExecutionStats::default()
        },
        output: OutputStats {
            logical_facts_represented: 10,
            materialized_facts: 2,
            duplicate_witnesses_suppressed: 8,
            expansions_avoided: 8,
        },
    };

    let json = report.render_json();
    let markdown = report.render_markdown();

    assert!(json.contains("\"plan_mode\""));
    assert!(json.contains("\"batch_size\":1000"));
    assert!(json.contains("\"cover_mode\""));
    assert!(json.contains("\"output_mode\""));
    assert!(json.contains("\"expansions_saved\":8"));
    assert!(markdown.contains("vectorized counters"));
    assert!(markdown.contains("COLT counters"));
    assert!(markdown.contains("output counters"));
}

#[test]
fn trace_summary_uses_surviving_phase_fields() {
    let summary = summarize_trace_json(&[
        TraceSpan {
            phase: "plan_validation",
        },
        TraceSpan { phase: "binary2fj" },
        TraceSpan {
            phase: "factorization",
        },
        TraceSpan {
            phase: "base_image_build",
        },
        TraceSpan {
            phase: "cover_choice",
        },
        TraceSpan {
            phase: "vectorized_batch_probe",
        },
        TraceSpan {
            phase: "sink_materialization",
        },
        TraceSpan {
            phase: "lmdb_read_transaction",
        },
    ]);

    assert!(summary.contains("trace_phases"));
    assert!(summary.contains("plan_validation"));
    assert!(summary.contains("lmdb_read_transaction"));
    assert!(!summary.contains("trie_intersections"));
}

fn explain(
    env: &Environment,
    schema: &StorageSchema,
    query: &TypedQuery,
    mode: PlanMode,
) -> Result<String> {
    env.read(|txn| {
        QueryPlan::build(
            txn,
            schema,
            query,
            ExplainConfig {
                plan_mode: mode,
                ..ExplainConfig::default()
            },
        )
        .map(|plan| plan.explain())
    })
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("bumbledb-prd18-{name}-{}-{id}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    env.write(|txn| {
        for fact in [pair("R", 1, 2), pair("S", 2, 3), pair("T", 3, 1)] {
            txn.insert(&schema, fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "Explain",
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

fn pair(relation: &str, left: u64, right: u64) -> Fact {
    Fact::new(
        relation,
        [("left", Value::U64(left)), ("right", Value::U64(right))],
    )
}

fn triangle_query() -> TypedQuery {
    typed_query(
        &["x", "y", "z"],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 1), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 2), (1, "right", 0)]),
        ],
    )
}

fn clover_query() -> TypedQuery {
    typed_query(
        &["x", "a", "b", "c"],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 0), (1, "right", 3)]),
        ],
    )
}

fn typed_query(variables: &[&str], atoms: Vec<TypedRelationAtom>) -> TypedQuery {
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
        find: (0..variables.len())
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
