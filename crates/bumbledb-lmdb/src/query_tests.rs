use super::*;
use crate::query_image::{QueryImageBuilder, QueryImageScope};
use crate::{AggregateError, Environment, ExecuteError, QueryError, Row};
use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, IndexDescriptor, RelationDescriptor,
};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn typed_query(
    schema: &StorageSchema,
    build: impl FnOnce(&mut QueryBuilder<'_>) -> QueryBuildResult<()>,
) -> QueryBuildResult<TypedQuery> {
    let mut builder = QueryBuilder::new(schema.descriptor());
    build(&mut builder)?;
    builder.finish()
}

#[test]
fn query_observability_defaults_are_zero() {
    let timings = QueryTimings::default();
    assert_eq!(timings.total_micros, 0);
    assert_eq!(timings.execute_micros, 0);
    assert_eq!(timings.unaccounted_micros, 0);
    assert_eq!(QueryRuntimeKind::default(), QueryRuntimeKind::Unknown);

    let allocations = QueryAllocationStats::default();
    assert!(!allocations.enabled);
    assert_eq!(allocations.alloc_calls, 0);
    assert_eq!(allocations.net_bytes, 0);
}

#[test]
fn query_timing_unaccounted_saturates_to_zero() {
    let mut timings = QueryTimings {
        total_micros: 5,
        validate_inputs_micros: 4,
        execute_micros: 4,
        ..QueryTimings::default()
    };

    timings.refresh_unaccounted();

    assert_eq!(timings.unaccounted_micros, 0);
}

#[test]
fn executes_single_relation_query() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("holder", Value::Serial(1))]),
        )
    })?;

    assert_eq!(
        output.rows,
        vec![vec![Value::Serial(1)], vec![Value::Serial(2)]]
    );
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert_eq!(output.plan.plan_family, PlanFamily::Direct);
    assert!(
        output
            .plan
            .direct_kernel
            .as_ref()
            .is_some_and(|kernel| kernel.target.contains("Account"))
    );
    assert!(output.plan.timings.total_micros > 0);
    assert!(output.plan.timings.execute_micros <= output.plan.timings.total_micros);
    assert!(!output.plan.allocations.enabled);
    assert!(output.plan.node_timings.is_empty());
    Ok(())
}

#[test]
fn planner_recommends_missing_static_predicate_index() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("currency", "currency")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("currency", Value::Enum(1))]),
        )
    })?;

    assert_same_rows(
        &output.rows,
        &[vec![Value::Serial(1)], vec![Value::Serial(3)]],
    );
    let expected_fields = vec!["currency".to_owned(), "id".to_owned()];
    assert!(output.plan.missing_indexes.iter().any(|missing| {
        missing.relation == "Account"
            && missing.fields == expected_fields
            && missing.reason.contains("StaticPredicate")
    }));
    Ok(())
}

#[test]
fn optimizer_selects_equality_index_and_hash_probe_for_static_lookup() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(optimizer_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, item_row(1, 1))?;
        txn.insert(&schema, item_row(2, 1))?;
        txn.insert(&schema, item_row(3, 2))?;
        Ok::<(), Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Item")?
            .var("id", "item")?
            .input("kind", "kind")?
            .done()
            .find_var("item")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("kind", Value::Enum(1))]),
        )
    })?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert_eq!(output.plan.plan_family, PlanFamily::Direct);
    assert_eq!(output.plan.optimizer.chosen, "direct_storage");
    assert_eq!(output.plan.query_image_cache.builds, 0);
    assert_eq!(output.plan.counters.direct_kernel_rows, 2);
    assert_same_rows(
        &output.rows,
        &[vec![Value::Serial(1)], vec![Value::Serial(2)]],
    );
    Ok(())
}

#[test]
fn hash_probe_runtime_checks_static_existence_atoms() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, b_row(1, 99))?;
        Ok::<(), Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.input("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.input("a", "a")?.done();
        query.cmp(
            OperandRef::var("b"),
            ComparisonOperator::NotEq,
            OperandRef::integer(0),
        )?;
        query.find_var("b")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("a", Value::U64(99))]),
        )
    })?;

    assert!(output.rows.is_empty());
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert_eq!(output.plan.counters.trie_open, 0);
    assert_eq!(output.plan.counters.hash_probe_calls, 0);
    assert!(output.plan.counters.static_empty_atoms_checked > 0);
    Ok(())
}

#[test]
fn mixed_hash_lftj_runtime_executes_hash_nodes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(direct_chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_row(1))?;
        txn.insert(&schema, chain_b_row(10, 1))?;
        txn.insert(&schema, chain_c_row(20, 10))?;
        txn.insert(&schema, chain_c_row(21, 10))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.var("a", "a")?.done();
        query.rel("C")?.var("id", "c")?.var("b", "b")?.done();
        query.cmp(
            OperandRef::var("c"),
            ComparisonOperator::NotEq,
            OperandRef::integer(0),
        )?;
        query.find_var("c")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::Mixed);
    assert!(
        output
            .plan
            .free_join
            .nodes
            .iter()
            .any(|node| node.implementation == NodeImpl::SortedLeapfrog)
    );
    assert!(
        output
            .plan
            .free_join
            .nodes
            .iter()
            .any(|node| node.implementation == NodeImpl::HashProbe)
    );
    assert!(output.plan.counters.trie_next > 0);
    assert!(output.plan.counters.hash_probe_calls > 0);
    assert_same_rows(&output.rows, &[vec![Value::U64(20)], vec![Value::U64(21)]]);
    Ok(())
}

#[test]
fn direct_prefix_range_kernel_selects_and_filters_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(direct_sailors_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, reserve_row(1, 10, 5))?;
        txn.insert(&schema, reserve_row(1, 11, 15))?;
        txn.insert(&schema, reserve_row(2, 12, 5))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Reserve")?
            .input("sailor", "sailor")?
            .var("boat", "boat")?
            .var("day", "day")?
            .done();
        query.cmp(
            OperandRef::var("day"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?;
        query.cmp(
            OperandRef::var("day"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?;
        query.find_var("boat")?.find_var("day")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([
                ("sailor", Value::U64(1)),
                ("start", Value::Timestamp(TimestampMicros(0))),
                ("end", Value::Timestamp(TimestampMicros(10))),
            ]),
        )
    })?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert_eq!(output.plan.plan_family, PlanFamily::Direct);
    assert!(matches!(
        output.plan.direct_kernel.as_ref().map(|direct| direct.kind),
        Some(DirectKernelKind::PrefixRange)
    ));
    assert_same_rows(
        &output.rows,
        &[vec![Value::U64(10), Value::Timestamp(TimestampMicros(5))]],
    );
    assert!(output.plan.counters.direct_kernel_probes > 0);
    assert_eq!(output.plan.counters.direct_kernel_rows, 2);
    assert_eq!(output.plan.counters.direct_kernel_predicates, 4);
    assert_eq!(output.plan.query_image_cache.builds, 0);
    assert_eq!(output.plan.counters.hash_index_builds, 0);
    assert_eq!(output.plan.counters.sorted_trie_builds, 0);
    assert_eq!(output.plan.counters.trie_open, 0);
    assert_eq!(output.plan.counters.hash_probe_calls, 0);
    Ok(())
}

#[test]
fn direct_storage_no_prefix_range_scan_selects_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(direct_sailors_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, reserve_row(1, 10, 5))?;
        txn.insert(&schema, reserve_row(1, 11, 15))?;
        txn.insert(&schema, reserve_row(2, 12, 25))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Reserve")?
            .var("sailor", "sailor")?
            .var("boat", "boat")?
            .var("day", "day")?
            .done();
        query.cmp(
            OperandRef::var("day"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?;
        query.cmp(
            OperandRef::var("day"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?;
        query.find_var("sailor")?.find_var("boat")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([
                ("start", Value::Timestamp(TimestampMicros(10))),
                ("end", Value::Timestamp(TimestampMicros(30))),
            ]),
        )
    })?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert_eq!(output.plan.plan_family, PlanFamily::Direct);
    assert_eq!(output.plan.query_image_cache.builds, 0);
    assert_eq!(output.plan.counters.hash_index_builds, 0);
    assert_eq!(output.plan.counters.sorted_trie_builds, 0);
    assert_same_rows(
        &output.rows,
        &[
            vec![Value::U64(1), Value::U64(11)],
            vec![Value::U64(2), Value::U64(12)],
        ],
    );
    Ok(())
}

#[test]
fn direct_prefix_range_empty_prefix_returns_zero_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(direct_sailors_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, reserve_row(1, 10, 5))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Reserve")?
            .input("sailor", "sailor")?
            .var("boat", "boat")?
            .var("day", "day")?
            .done();
        query.cmp(
            OperandRef::var("day"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?;
        query.cmp(
            OperandRef::var("day"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?;
        query.find_var("boat")?.find_var("day")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([
                ("sailor", Value::U64(99)),
                ("start", Value::Timestamp(TimestampMicros(0))),
                ("end", Value::Timestamp(TimestampMicros(10))),
            ]),
        )
    })?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert!(output.rows.is_empty());
    assert_eq!(output.plan.counters.trie_open, 0);
    assert_eq!(output.plan.counters.hash_probe_calls, 0);
    Ok(())
}

#[test]
fn direct_chain_kernel_selects_and_follows_acyclic_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(direct_chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_row(1))?;
        txn.insert(&schema, chain_b_row(10, 1))?;
        txn.insert(&schema, chain_c_row(20, 10))?;
        txn.insert(&schema, chain_d_row(30, 20))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.input("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.input("a", "a")?.done();
        query.rel("C")?.var("id", "c")?.var("b", "b")?.done();
        query.rel("D")?.var("id", "d")?.var("c", "c")?.done();
        query.find_var("d")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("a", Value::U64(1))]),
        )
    })?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::IndexNestedLoop);
    assert_eq!(output.plan.plan_family, PlanFamily::IndexNestedLoop);
    assert!(matches!(
        output.plan.direct_kernel.as_ref().map(|direct| direct.kind),
        Some(DirectKernelKind::ChainProbe)
    ));
    assert_eq!(output.rows, vec![vec![Value::U64(30)]]);
    assert_eq!(output.plan.counters.direct_kernel_rows, 4);
    assert_eq!(output.plan.counters.hash_index_builds, 0);
    assert_eq!(output.plan.counters.hash_index_build_rows, 0);
    assert_eq!(output.plan.counters.trie_open, 0);
    assert_eq!(output.plan.counters.hash_probe_calls, 0);
    Ok(())
}

#[test]
fn count_only_matches_materialized_projection_without_decoding_output() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(direct_chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Row::new("A", [("id", Value::U64(1))]))?;
        txn.insert(&schema, chain_b_row(10, 1))?;
        txn.insert(&schema, chain_c_row(20, 10))?;
        txn.insert(&schema, chain_d_row(30, 20))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.input("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.input("a", "a")?.done();
        query.rel("C")?.var("id", "c")?.var("b", "b")?.done();
        query.rel("D")?.var("id", "d")?.var("c", "c")?.done();
        query.find_var("d")?;
        Ok(())
    })?;
    let inputs = InputBindings::from_values([("a", Value::U64(1))]);

    let materialized = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;
    let count_only = env.read(|txn| txn.execute_query_count_only(&schema, &query, &inputs))?;

    assert_eq!(count_only.rows, materialized.rows.len());
    assert_eq!(count_only.plan.runtime_kind, materialized.plan.runtime_kind);
    assert_eq!(count_only.plan.counters.materialized_output_values, 0);
    Ok(())
}

#[test]
fn direct_chain_broken_path_returns_zero_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(direct_chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_b_row(10, 1))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.input("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.input("a", "a")?.done();
        query.rel("C")?.var("id", "c")?.var("b", "b")?.done();
        query.rel("D")?.var("id", "d")?.var("c", "c")?.done();
        query.find_var("d")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("a", Value::U64(1))]),
        )
    })?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert_eq!(output.plan.plan_family, PlanFamily::StaticEmpty);
    assert!(output.rows.is_empty());
    assert_eq!(output.plan.counters.trie_open, 0);
    Ok(())
}

#[test]
fn optimizer_keeps_cyclic_triangle_on_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_row(1, 10))?;
        txn.insert(&schema, edge_ac_row(1, 20))?;
        txn.insert(&schema, edge_bc_row(10, 20))?;
        txn.insert(&schema, edge_ab_row(2, 10))?;
        txn.insert(&schema, edge_ac_row(2, 30))?;
        txn.insert(&schema, edge_bc_row(10, 40))?;
        Ok::<(), Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.rel("EdgeBC")?.var("b", "b")?.var("c", "c")?.done();
        query.find_aggregate(AggregateFunction::Count, "a")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(1)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::Lftj);
    assert!(output.plan.direct_kernel.is_none());
    assert!(
        output
            .plan
            .free_join
            .nodes
            .iter()
            .all(|node| node.implementation == NodeImpl::SortedLeapfrog)
    );
    assert!(
        output
            .plan
            .optimizer
            .candidates
            .iter()
            .any(|candidate| candidate.name == "pure_lftj")
    );
    Ok(())
}

#[test]
fn lftj_atom_cache_reuses_equivalent_relation_aliases() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Row::new("A", [("id", Value::U64(1))]))?;
        txn.insert(&schema, Row::new("A", [("id", Value::U64(2))]))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "left")?.done();
        query.rel("A")?.var("id", "right")?.done();
        query.find_var("left")?.find_var("right")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::Lftj);
    assert!(output.plan.counters.sorted_trie_builds <= 1);
    assert_eq!(output.rows.len(), 4);
    Ok(())
}

#[test]
fn lftj_empty_variable_atom_short_circuits_execution() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Row::new("A", [("id", Value::U64(1))]))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.integer("a", 99)?.done();
        query.find_var("a")?.find_var("b")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.rows.is_empty());
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert_eq!(output.plan.optimizer.chosen, "static_empty");
    assert_eq!(output.plan.counters.trie_open, 0);
    assert_eq!(output.plan.counters.variable_candidates, 0);
    Ok(())
}

#[test]
fn static_empty_no_input_query_hits_fast_cache_before_normalize() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Row::new("A", [("id", Value::U64(1))]))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.integer("a", 99)?.done();
        query.find_var("a")?.find_var("b")?;
        Ok(())
    })?;

    let first = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    let second = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(first.rows.is_empty());
    assert!(second.rows.is_empty());
    assert_eq!(first.plan.counters.static_empty_cache_misses, 1);
    assert_eq!(second.plan.counters.static_empty_cache_hits, 1);
    assert!(second.plan.free_join.nodes.is_empty());
    assert!(second.explain().contains("static_empty cache_hits=1"));
    assert_eq!(second.plan.timings.validate_inputs_micros, 0);
    assert_eq!(second.plan.timings.normalize_micros, 0);
    assert_eq!(second.plan.timings.encode_inputs_micros, 0);
    assert_eq!(second.plan.timings.query_image_micros, 0);
    Ok(())
}

#[test]
fn direct_count_plan_has_no_free_join_nodes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_row(1, 10))?;
        txn.insert(&schema, edge_ab_row(1, 11))?;
        txn.insert(
            &schema,
            Row::new("EdgeAC", [("a", Value::U64(1)), ("c", Value::U64(20))]),
        )?;
        txn.insert(
            &schema,
            Row::new("EdgeAC", [("a", Value::U64(2)), ("c", Value::U64(30))]),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.find_aggregate(AggregateFunction::Count, "a")?;
        Ok(())
    })?;
    let prepared = env.prepare_query(&schema, &query)?;

    let output =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(2)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert_eq!(output.plan.plan_family, PlanFamily::Direct);
    assert!(output.plan.free_join.nodes.is_empty());
    assert_eq!(output.plan.optimizer.chosen, "direct_count");
    assert!(
        output
            .explain()
            .contains("direct_kernel kind=CountOnly target=factorized_count")
    );
    Ok(())
}

#[test]
fn factorized_count_supports_serial_literal_filter() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, owner_group_row(1, 10))?;
        txn.insert(&schema, owner_group_row(2, 20))?;
        txn.insert(&schema, owned_fact_row(9, 10, 100))?;
        txn.insert(&schema, owned_fact_row(9, 10, 101))?;
        txn.insert(&schema, owned_fact_row(9, 20, 200))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("OwnerGroup")?
            .integer("owner", 1)?
            .var("group", "group")?
            .done();
        query
            .rel("OwnedFact")?
            .var("group", "group")?
            .var("item", "item")?
            .done();
        query.find_aggregate(AggregateFunction::Count, "item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(2)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert!(output.explain().contains("target=factorized_count"));
    Ok(())
}

#[test]
fn factorized_count_supports_enum_literal_filter() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_row(1, 1))?;
        txn.insert(&schema, dim_row(2, 2))?;
        txn.insert(&schema, fact_row(1, 10))?;
        txn.insert(&schema, fact_row(1, 11))?;
        txn.insert(&schema, fact_row(2, 20))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "dim")?
            .integer("kind", 1)?
            .done();
        query
            .rel("Fact")?
            .var("dim", "dim")?
            .var("item", "item")?
            .done();
        query.find_aggregate(AggregateFunction::Count, "item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(2)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert!(output.explain().contains("target=factorized_count"));
    Ok(())
}

#[test]
fn factorized_count_supports_range_filter() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(
            &schema,
            Row::new("Title", [("id", Value::U64(1)), ("year", Value::I64(2004))]),
        )?;
        txn.insert(
            &schema,
            Row::new("Title", [("id", Value::U64(2)), ("year", Value::I64(2005))]),
        )?;
        txn.insert(
            &schema,
            Row::new("Title", [("id", Value::U64(3)), ("year", Value::I64(2015))]),
        )?;
        txn.insert(
            &schema,
            Row::new("Title", [("id", Value::U64(4)), ("year", Value::I64(2016))]),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(1)), ("company", Value::U64(10))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(2)), ("company", Value::U64(20))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(3)), ("company", Value::U64(30))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(4)), ("company", Value::U64(40))],
            ),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("WorkCompany")?
            .var("work", "work")?
            .var("company", "company")?
            .done();
        query
            .rel("Title")?
            .var("id", "work")?
            .var("year", "year")?
            .done();
        query.cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gte,
            OperandRef::integer(2005),
        )?;
        query.cmp(
            OperandRef::var("year"),
            ComparisonOperator::Lte,
            OperandRef::integer(2015),
        )?;
        query.find_aggregate(AggregateFunction::Count, "company")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(2)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert!(output.explain().contains("target=factorized_count"));
    Ok(())
}

#[test]
fn factorized_count_supports_mixed_literal_and_range_filters() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(
            &schema,
            Row::new(
                "Company",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::String("[us]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Company",
                [
                    ("id", Value::U64(2)),
                    ("country", Value::String("[de]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Title",
                [("id", Value::U64(10)), ("year", Value::I64(2010))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Title",
                [("id", Value::U64(20)), ("year", Value::I64(2010))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Title",
                [("id", Value::U64(30)), ("year", Value::I64(2020))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(10)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(20)), ("company", Value::U64(2))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(30)), ("company", Value::U64(1))],
            ),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("WorkCompany")?
            .var("work", "work")?
            .var("company", "company")?
            .done();
        query
            .rel("Company")?
            .var("id", "company")?
            .string("country", "[us]")?
            .done();
        query
            .rel("Title")?
            .var("id", "work")?
            .var("year", "year")?
            .done();
        query.cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gte,
            OperandRef::integer(2005),
        )?;
        query.cmp(
            OperandRef::var("year"),
            ComparisonOperator::Lte,
            OperandRef::integer(2015),
        )?;
        query.find_aggregate(AggregateFunction::Count, "work")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(1)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::DirectKernel);
    assert!(output.explain().contains("target=factorized_count"));
    Ok(())
}

#[test]
fn factorized_count_rejects_unsafe_cycle() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_row(1, 10))?;
        txn.insert(
            &schema,
            Row::new("EdgeAC", [("a", Value::U64(1)), ("c", Value::U64(20))]),
        )?;
        txn.insert(
            &schema,
            Row::new("EdgeBC", [("b", Value::U64(10)), ("c", Value::U64(20))]),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.rel("EdgeBC")?.var("b", "b")?.var("c", "c")?.done();
        query.find_aggregate(AggregateFunction::Count, "a")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(1)]]);
    assert!(!output.explain().contains("target=factorized_count"));
    Ok(())
}

#[test]
fn optimizer_trace_and_cost_tiebreak_are_stable() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query
            .rel("Holder")?
            .var("id", "holder")?
            .var("name", "holder_name")?
            .done();
        query.find_var("account")?.find_var("holder_name")?;
        Ok(())
    })?;

    let first = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    let second = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(first.plan.optimizer, second.plan.optimizer);
    assert!(first.explain().contains("plan_family"));
    assert!(first.explain().contains("setup_micros"));
    assert!(first.explain().contains("candidate_plan"));
    assert!(first.explain().contains("free_join_estimates"));
    assert!(first.explain().contains("reason=stats"));
    Ok(())
}

#[test]
fn prepared_plan_cache_reuses_parameterized_shape() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query
            .rel("Holder")?
            .input("id", "holder")?
            .var("name", "holder_name")?
            .done();
        query.find_var("account")?.find_var("holder_name")?;
        Ok(())
    })?;
    let inputs = InputBindings::from_values([("holder", Value::Serial(1))]);

    let first = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;
    let second = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;

    assert_eq!(first.rows, second.rows);
    assert_eq!(first.plan.prepared_plan_cache.misses, 1);
    assert_eq!(first.plan.prepared_plan_cache.builds, 1);
    assert_eq!(second.plan.prepared_plan_cache.hits, 1);
    assert_ne!(first.plan.plan_family, PlanFamily::Unknown);
    Ok(())
}

#[test]
fn prepared_plan_cache_reuses_no_input_physical_plan() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query.find_var("account")?.find_var("holder")?;
        Ok(())
    })?;

    let first = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    let second = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(first.rows, second.rows);
    assert_eq!(first.plan.prepared_plan_cache.cached_plans, 1);
    assert_eq!(first.plan.prepared_plan_cache.misses, 1);
    assert_eq!(first.plan.prepared_plan_cache.builds, 1);
    assert_eq!(first.plan.prepared_plan_cache.hits, 0);
    assert_eq!(second.plan.prepared_plan_cache.cached_plans, 1);
    assert_eq!(second.plan.prepared_plan_cache.misses, 1);
    assert_eq!(second.plan.prepared_plan_cache.builds, 1);
    assert_eq!(second.plan.prepared_plan_cache.hits, 1);
    assert_eq!(first.plan.optimizer, second.plan.optimizer);
    assert_eq!(first.plan.free_join, second.plan.free_join);
    assert!(second.plan.timings.plan_micros <= first.plan.timings.plan_micros);
    assert!(second.explain().contains("prepared_plan_cache"));
    Ok(())
}

#[test]
fn prepared_plan_cache_is_snapshot_scoped() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query.find_var("account")?.find_var("holder")?;
        Ok(())
    })?;

    let before = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    env.write(|txn| {
        txn.insert(&schema, account_row(4, 2, 2))?;
        Ok::<_, Error>(())
    })?;
    let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(before.plan.prepared_plan_cache.misses, 1);
    assert_eq!(before.plan.prepared_plan_cache.builds, 1);
    assert_eq!(after.plan.prepared_plan_cache.misses, 1);
    assert_eq!(after.plan.prepared_plan_cache.builds, 1);
    assert_eq!(after.plan.prepared_plan_cache.hits, 0);
    assert_eq!(after.rows.len(), before.rows.len() + 1);
    Ok(())
}

#[test]
fn planner_stats_are_cached_per_query_image() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;
    let inputs = InputBindings::new();

    let first = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;
    let second = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;

    assert_eq!(first.rows, second.rows);
    assert_eq!(first.plan.planner_stats.builds, 1);
    assert_eq!(first.plan.planner_stats.misses, 1);
    assert_eq!(second.plan.planner_stats.builds, 1);
    assert_eq!(second.plan.planner_stats.misses, 1);
    assert!(second.plan.planner_stats.hits >= 1 || second.plan.prepared_plan_cache.hits >= 1);
    if second
        .plan
        .free_join
        .nodes
        .iter()
        .all(|node| node.implementation == NodeImpl::HashProbe)
    {
        assert!(second.plan.counters.hash_probe_calls > 0);
        assert_eq!(second.plan.counters.trie_open, 0);
    } else {
        assert_eq!(second.plan.counters.sorted_trie_builds, 0);
        assert_eq!(second.plan.counters.atom_temp_relation_builds, 0);
        assert!(second.plan.counters.sorted_trie_cache_hits >= 1);
    }
    Ok(())
}

#[test]
fn execute_query_uses_warmed_query_image_cache() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;
    let inputs = InputBindings::new();

    let warm = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;
    let before = env.query_image_cache_diagnostics();
    let output = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;
    let after = env.query_image_cache_diagnostics();

    assert_eq!(before.builds, 1);
    assert_eq!(after.builds, 1);
    assert_eq!(output.plan.query_image_cache.builds, 1);
    assert!(output.plan.query_image_cache.hits > before.hits);
    assert_eq!(warm.rows.len(), 3);
    assert_eq!(output.rows.len(), 3);
    Ok(())
}

#[test]
fn execute_query_cache_misses_after_write_commit() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;

    let before = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    env.write(|txn| {
        txn.insert(&schema, account_row(4, 2, 2))?;
        Ok::<_, Error>(())
    })?;
    let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(before.plan.query_image_cache.builds, 1);
    assert_eq!(after.plan.query_image_cache.builds, 2);
    assert_eq!(after.rows.len(), before.rows.len() + 1);
    Ok(())
}

#[test]
fn execute_query_cache_is_schema_fingerprint_scoped() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema_a = StorageSchema::new(optimizer_schema(), env.max_key_size())?;
    let schema_b = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    let item_query = typed_query(&schema_a, |query| {
        query
            .rel("Item")?
            .var("id", "item")?
            .var("kind", "kind")?
            .done();
        query.find_var("item")?;
        Ok(())
    })?;
    let edge_query = typed_query(&schema_b, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.find_var("a")?;
        Ok(())
    })?;

    let item = env.read(|txn| txn.execute_query(&schema_a, &item_query, &InputBindings::new()))?;
    let edge = env.read(|txn| txn.execute_query(&schema_b, &edge_query, &InputBindings::new()))?;

    assert_eq!(item.plan.query_image_cache.builds, 1);
    assert_eq!(edge.plan.query_image_cache.builds, 2);
    assert_eq!(edge.plan.query_image_cache.cached_images, 2);
    Ok(())
}

#[test]
fn planner_stats_reuse_shared_relations_across_queries() -> TestResult {
    let (env, schema) = seeded_db()?;
    let first_query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .done();
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query.find_var("posting")?;
        Ok(())
    })?;
    let second_query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("at", "t")?
            .done();
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::integer(0),
        )?;
        query.find_var("posting")?;
        Ok(())
    })?;

    let inputs = InputBindings::new();

    let first = env.read(|txn| txn.execute_query(&schema, &first_query, &inputs))?;
    let second = env.read(|txn| txn.execute_query(&schema, &second_query, &inputs))?;

    assert_eq!(first.plan.planner_stats.builds, 2);
    assert_eq!(second.plan.planner_stats.builds, 1);
    assert_eq!(second.rows.len(), 3);
    Ok(())
}

#[test]
fn planner_stats_cache_is_snapshot_scoped() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;

    let before = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    env.write(|txn| {
        txn.insert(&schema, account_row(4, 2, 2))?;
        Ok::<_, Error>(())
    })?;
    let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(before.plan.planner_stats.builds, 1);
    assert_eq!(after.plan.planner_stats.builds, 1);
    assert_eq!(after.rows.len(), before.rows.len() + 1);
    Ok(())
}

#[test]
fn normalized_query_preserves_typed_query_shape() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("amount", "amount")?
            .var("at", "t")?
            .done();
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done();
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?;
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?;
        query.find_var("posting")?.find_var("amount")?;
        Ok(())
    })?;

    let normalized = env.read(|txn| normalize_query(txn, &schema, &query))?;

    assert_eq!(normalized.vars.len(), query.variables.len());
    assert_eq!(normalized.inputs.len(), query.inputs.len());
    assert_eq!(normalized.atoms.len(), 2);
    assert_eq!(normalized.predicates.len(), 2);
    assert!(matches!(normalized.output, OutputPlan::Project(_)));
    assert!(matches!(
        normalized.atoms[0].fields[0].term,
        NormTerm::Var(_)
    ));
    Ok(())
}

#[test]
fn query_shape_key_is_structural_and_stable() -> TestResult {
    let (env, schema) = seeded_db()?;
    let posting_amount_before = |limit, operator| {
        typed_query(&schema, |query| {
            query
                .rel("Posting")?
                .var("id", "posting")?
                .var("amount", "amount")?
                .var("at", "t")?
                .done();
            query.cmp(OperandRef::var("t"), operator, OperandRef::integer(limit))?;
            query.find_var("posting")?.find_var("amount")?;
            Ok(())
        })
    };
    let base = posting_amount_before(30, ComparisonOperator::Lt)?;
    let same = posting_amount_before(30, ComparisonOperator::Lt)?;
    let different_literal = posting_amount_before(31, ComparisonOperator::Lt)?;
    let different_operator = posting_amount_before(30, ComparisonOperator::Lte)?;
    let different_output = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("amount", "amount")?
            .var("at", "t")?
            .done();
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::integer(30),
        )?;
        query.find_var("amount")?.find_var("posting")?;
        Ok(())
    })?;

    let keys = env.read(|txn| {
        let base = normalize_query(txn, &schema, &base)?;
        let same = normalize_query(txn, &schema, &same)?;
        let different_literal = normalize_query(txn, &schema, &different_literal)?;
        let different_operator = normalize_query(txn, &schema, &different_operator)?;
        let different_output = normalize_query(txn, &schema, &different_output)?;
        Ok::<_, Error>((
            query_shape_key(&schema, &base),
            query_shape_key(&schema, &same),
            query_shape_key(&schema, &different_literal),
            query_shape_key(&schema, &different_operator),
            query_shape_key(&schema, &different_output),
        ))
    })?;

    assert_eq!(keys.0, keys.1);
    assert_ne!(keys.0, keys.2);
    assert_ne!(keys.0, keys.3);
    assert_ne!(keys.0, keys.4);
    Ok(())
}

#[test]
fn prepared_query_reuses_normalized_snapshot_shape() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("amount", "amount")?
            .done();
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done();
        query.find_var("posting")?.find_var("amount")?;
        Ok(())
    })?;
    let prepared = env.prepare_query(&schema, &query)?;
    let inputs = InputBindings::from_values([("holder", Value::Serial(1))]);

    let first = env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &inputs))?;
    let second = env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &inputs))?;

    assert_eq!(first.rows, second.rows);
    assert!(first.plan.timings.normalize_micros > 0);
    assert_eq!(second.plan.timings.normalize_micros, 0);
    Ok(())
}

#[test]
fn lftj_atom_key_includes_encoded_inputs() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;
    let first_inputs = InputBindings::from_values([("holder", Value::Serial(1))]);
    let second_inputs = InputBindings::from_values([("holder", Value::Serial(2))]);

    let (first, same, second) = env.read(|txn| {
        let normalized = normalize_query(txn, &schema, &query)?;
        let first_inputs = encode_inputs(txn, &schema, &normalized, &first_inputs)?;
        let same_inputs = encode_inputs(
            txn,
            &schema,
            &normalized,
            &InputBindings::from_values([("holder", Value::Serial(1))]),
        )?;
        let second_inputs = encode_inputs(txn, &schema, &normalized, &second_inputs)?;
        let atom = &normalized.atoms[0];
        let variables = atom_variables_in_plan_order(atom, &[0]);
        Ok::<_, Error>((
            lftj_atom_cache_key(atom, &variables, &[0], &first_inputs),
            lftj_atom_cache_key(atom, &variables, &[0], &same_inputs),
            lftj_atom_cache_key(atom, &variables, &[0], &second_inputs),
        ))
    })?;

    assert_eq!(first, same);
    assert_ne!(first, second);
    Ok(())
}

#[test]
fn repeated_variable_atom_matches_equal_encoded_fields() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_row(1, 1))?;
        txn.insert(&schema, edge_ab_row(1, 2))?;
        Ok::<(), Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "a")?.done();
        query.find_var("a")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(1)]]);
    Ok(())
}

#[test]
fn predicate_earliest_depth_assignment_is_deterministic() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("at", "t")?
            .done();
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done();
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?;
        query.find_var("posting")?;
        Ok(())
    })?;

    let depths = env.read(|txn| {
        let mut normalized = normalize_query(txn, &schema, &query)?;
        let image = QueryImageBuilder::new(txn, &schema, QueryImageScope::full(&schema)).build()?;
        let plan = plan_query(
            &schema,
            &mut normalized,
            &image,
            QueryImageCacheDiagnostics::default(),
            PreparedPlanCacheDiagnostics::default(),
        )?;
        let t_depth = plan
            .summary
            .variable_order
            .iter()
            .position(|name| name == "t")
            .ok_or_else(|| Error::internal("missing t variable in plan"))?;
        Ok::<_, Error>((normalized.predicates[0].earliest_depth, t_depth))
    })?;

    assert_eq!(depths.0, Some(depths.1));
    Ok(())
}

#[test]
fn specialized_mock_plan_matches_interpreted_sink_output() -> TestResult {
    struct MockSpecializedPlan {
        bindings: Vec<EncodedBinding>,
    }

    impl ExecutablePlan for MockSpecializedPlan {
        fn execute(
            &mut self,
            txn: &ReadTxn<'_>,
            query: &NormalizedQuery,
            _image: &crate::QueryImage,
            _inputs: &EncodedInputs,
            sink: &mut dyn TupleSink,
        ) -> Result<PlanCounters> {
            let mut counters = PlanCounters::default();
            for binding in &self.bindings {
                sink.emit(txn, query, binding, &mut counters)?;
                counters.bindings_yielded += 1;
            }
            Ok(counters)
        }
    }

    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(optimizer_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, item_row(1, 1))?;
        Ok::<(), Error>(())
    })?;
    let typed = typed_query(&schema, |query| {
        query
            .rel("Item")?
            .var("id", "item")?
            .input("kind", "kind")?
            .done()
            .find_var("item")?;
        Ok(())
    })?;
    let inputs = InputBindings::from_values([("kind", Value::Enum(1))]);
    let interpreted = env
        .read(|txn| txn.execute_query(&schema, &typed, &inputs))?
        .rows;

    let specialized = env.read(|txn| {
        let normalized = normalize_query(txn, &schema, &typed)?;
        let encoded_inputs = encode_inputs(txn, &schema, &normalized, &inputs)?;
        let image = QueryImageBuilder::new(txn, &schema, QueryImageScope::full(&schema)).build()?;
        let mut binding = EncodedBinding::new(normalized.vars.len());
        let encoded = txn.encode_query_value(&normalized.vars[0].value_type, &Value::Serial(1))?;
        assert!(binding.bind(
            0,
            encoded_owned_for_width(normalized.vars[0].value_type.encoded_width(), &encoded)?,
        ));
        let mut plan = MockSpecializedPlan {
            bindings: vec![binding],
        };
        let mut sink = OutputSink::new(&normalized.output);
        let _ = plan.execute(txn, &normalized, &image, &encoded_inputs, &mut sink)?;
        sink.finish(txn, &normalized, &mut PlanCounters::default())
    })?;

    assert_same_rows(&specialized, &interpreted);
    Ok(())
}

#[test]
fn executes_two_relation_join() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query
            .rel("Holder")?
            .var("id", "holder")?
            .var("name", "holder_name")?
            .done();
        query.find_var("account")?.find_var("holder_name")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    assert!(output.plan.uses_indexed_multiway_join);
    assert_same_rows(
        &output.rows,
        &[
            vec![Value::Serial(1), Value::String("Alice".to_owned())],
            vec![Value::Serial(2), Value::String("Alice".to_owned())],
            vec![Value::Serial(3), Value::String("Bob".to_owned())],
        ],
    );
    Ok(())
}

#[test]
fn executes_many_relation_join_and_range_filter() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("amount", "amount")?
            .var("at", "t")?
            .done();
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query
            .rel("Holder")?
            .var("id", "holder")?
            .var("name", "holder_name")?
            .done();
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?;
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?;
        query
            .find_var("posting")?
            .find_var("account")?
            .find_var("holder_name")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([
                ("start", Value::Timestamp(TimestampMicros(15))),
                ("end", Value::Timestamp(TimestampMicros(35))),
            ]),
        )
    })?;

    assert!(
        output
            .plan
            .variable_estimates
            .iter()
            .any(|estimate| estimate.access == "Posting.by_at")
    );
    assert_same_rows(
        &output.rows,
        &[
            vec![
                Value::Serial(2),
                Value::Serial(1),
                Value::String("Alice".to_owned()),
            ],
            vec![
                Value::Serial(3),
                Value::Serial(2),
                Value::String("Alice".to_owned()),
            ],
        ],
    );
    Ok(())
}

#[test]
fn projection_deduplicates_results() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .find_var("holder")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    assert_eq!(
        output.rows,
        vec![vec![Value::Serial(1)], vec![Value::Serial(2)]]
    );
    assert_eq!(output.plan.counters.bindings_yielded, 3);
    assert_eq!(output.plan.counters.materialized_output_values, 2);
    Ok(())
}

#[test]
fn count_sink_avoids_decoding_counted_variable() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query.rel("Posting")?.var("id", "posting")?.done();
        query.find_aggregate(AggregateFunction::Count, "posting")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(3)]]);
    assert_eq!(output.plan.counters.bindings_yielded, 0);
    assert_eq!(output.plan.counters.factorized_counted_bindings, 3);
    assert_eq!(output.plan.counters.aggregate_groups, 1);
    assert_eq!(output.plan.counters.decoded_values, 0);
    assert_eq!(output.plan.counters.materialized_output_values, 1);
    assert!(
        output.plan.counters.materialized_output_values
            < output.plan.counters.factorized_counted_bindings
    );
    Ok(())
}

#[test]
fn global_count_over_empty_input_returns_zero_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let query = typed_query(&schema, |query| {
        query
            .rel("A")?
            .var("id", "a")?
            .done()
            .find_aggregate(AggregateFunction::Count, "a")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(0)]]);
    assert_eq!(output.plan.counters.output_rows, 1);
    Ok(())
}

#[test]
fn static_empty_global_count_returns_zero_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        let _ = txn.insert(&schema, Row::new("A", [("id", Value::U64(1))]))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("A")?
            .var("id", "a")?
            .done()
            .rel("B")?
            .var("id", "b")?
            .integer("a", 99)?
            .done()
            .find_aggregate(AggregateFunction::Count, "a")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(0)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    Ok(())
}

#[test]
fn static_semijoin_dimension_row_exists_but_fact_is_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_row(1, 1))?;
        txn.insert(&schema, fact_row(2, 10))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "dim")?
            .integer("kind", 1)?
            .done();
        query
            .rel("Fact")?
            .var("dim", "dim")?
            .var("item", "item")?
            .done();
        query.find_var("item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.rows.is_empty());
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert!(output.plan.counters.static_semijoin_rounds > 0);
    assert!(output.plan.timings.static_semijoin_proof_micros > 0);
    Ok(())
}

#[test]
fn static_semijoin_disjoint_central_candidates_prove_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_row(1, 1))?;
        txn.insert(&schema, other_dim_row(2, 2))?;
        txn.insert(&schema, fact_row(1, 10))?;
        txn.insert(&schema, fact_row(2, 20))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "left")?
            .integer("kind", 1)?
            .done();
        query
            .rel("OtherDim")?
            .var("id", "right")?
            .integer("kind", 2)?
            .done();
        query
            .rel("Fact")?
            .var("dim", "left")?
            .var("item", "item")?
            .done();
        query
            .rel("Fact")?
            .var("dim", "right")?
            .var("item", "item")?
            .done();
        query.find_var("item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.rows.is_empty());
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert!(output.plan.counters.static_semijoin_candidate_values > 0);
    Ok(())
}

#[test]
fn static_semijoin_enum_literal_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_row(7, 1))?;
        txn.insert(&schema, fact_row(8, 99))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "dim")?
            .integer("kind", 1)?
            .done();
        query
            .rel("Fact")?
            .var("dim", "dim")?
            .var("item", "item")?
            .done();
        query.find_var("item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert!(output.plan.counters.static_semijoin_prefixes_probed > 0);
    Ok(())
}

#[test]
fn static_semijoin_serial_literal_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, owner_group_row(1, 10))?;
        txn.insert(&schema, owned_fact_row(2, 11, 99))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("OwnerGroup")?
            .integer("owner", 1)?
            .var("group", "group")?
            .done();
        query
            .rel("OwnedFact")?
            .var("group", "group")?
            .var("item", "item")?
            .done();
        query.find_var("item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert!(output.plan.counters.static_semijoin_rounds > 0);
    Ok(())
}

#[test]
fn static_semijoin_compound_relation_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_row(1, 1))?;
        txn.insert(&schema, other_dim_row(2, 2))?;
        txn.insert(&schema, pair_row(1, 3))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "left")?
            .integer("kind", 1)?
            .done();
        query
            .rel("OtherDim")?
            .var("id", "right")?
            .integer("kind", 2)?
            .done();
        query
            .rel("Pair")?
            .var("left", "left")?
            .var("right", "right")?
            .done();
        query.find_var("left")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.rows.is_empty());
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    Ok(())
}

#[test]
fn static_semijoin_budget_exhaustion_falls_back_safely() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_budget_schema(), env.max_key_size())?;
    env.write(|txn| {
        for id in 1..=1_001 {
            txn.insert(
                &schema,
                Row::new("Big", [("pad", Value::U64(0)), ("id", Value::U64(id))]),
            )?;
        }
        txn.insert(&schema, Row::new("Link", [("id", Value::U64(999_999))]))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("Big")?.var("id", "id")?.done();
        query.rel("Link")?.var("id", "id")?.done();
        query.cmp(
            OperandRef::var("id"),
            ComparisonOperator::Gt,
            OperandRef::integer(0),
        )?;
        query.find_var("id")?;
        Ok(())
    })?;

    let proof = env.read(|txn| {
        let normalized = normalize_query(txn, &schema, &query)?;
        let encoded_inputs = encode_inputs(txn, &schema, &normalized, &InputBindings::new())?;
        let image = txn.query_images.get_or_build_scoped(
            txn,
            &schema,
            query_image_scope_for_query(&schema, &normalized),
        )?;
        static_semijoin_proves_empty(image.as_ref(), &normalized, &encoded_inputs)
    })?;

    assert!(!proof.empty);
    assert!(proof.rounds > 0);
    Ok(())
}

#[test]
fn static_semijoin_non_empty_query_is_not_proven_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(static_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_row(1, 1))?;
        txn.insert(&schema, fact_row(1, 10))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "dim")?
            .integer("kind", 1)?
            .done();
        query
            .rel("Fact")?
            .var("dim", "dim")?
            .var("item", "item")?
            .done();
        query.find_var("item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_same_rows(&output.rows, &[vec![Value::U64(10)]]);
    assert_ne!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    Ok(())
}

#[test]
fn static_semijoin_q24_like_empty_shape_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Row::new("Alias", [("person", Value::U64(1))]))?;
        txn.insert(&schema, Row::new("Character", [("id", Value::U64(1))]))?;
        txn.insert(
            &schema,
            Row::new(
                "Appearance",
                [
                    ("person", Value::U64(1)),
                    ("work", Value::U64(100)),
                    ("character", Value::U64(1)),
                    ("role", Value::U64(1)),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Company",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::String("[us]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Keyword",
                [
                    ("id", Value::U64(1)),
                    ("word", Value::String("hero".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Person",
                [
                    ("id", Value::U64(1)),
                    ("gender", Value::String("m".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Role",
                [
                    ("id", Value::U64(1)),
                    ("name", Value::String("actor".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Title",
                [("id", Value::U64(100)), ("year", Value::I64(2012))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Title",
                [("id", Value::U64(200)), ("year", Value::I64(2012))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(100)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkKeyword",
                [("work", Value::U64(200)), ("keyword", Value::U64(1))],
            ),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("Alias")?.var("person", "person")?.done();
        query
            .rel("Appearance")?
            .var("person", "person")?
            .var("work", "work")?
            .var("character", "character")?
            .var("role", "role")?
            .done();
        query.rel("Character")?.var("id", "character")?.done();
        query
            .rel("Company")?
            .var("id", "company")?
            .string("country", "[us]")?
            .done();
        query
            .rel("Keyword")?
            .var("id", "keyword")?
            .string("word", "hero")?
            .done();
        query
            .rel("WorkCompany")?
            .var("work", "work")?
            .var("company", "company")?
            .done();
        query
            .rel("WorkKeyword")?
            .var("work", "work")?
            .var("keyword", "keyword")?
            .done();
        query
            .rel("Person")?
            .var("id", "person")?
            .string("gender", "m")?
            .done();
        query
            .rel("Role")?
            .var("id", "role")?
            .string("name", "actor")?
            .done();
        query
            .rel("Title")?
            .var("id", "work")?
            .var("year", "year")?
            .done();
        query.cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gt,
            OperandRef::integer(2010),
        )?;
        query.find_var("work")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.rows.is_empty());
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert!(output.plan.counters.static_semijoin_candidate_values > 0);
    Ok(())
}

#[test]
fn static_semijoin_range_index_q16_like_count_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q16_like_semijoin_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Row::new("Alias", [("person", Value::U64(1))]))?;
        txn.insert(&schema, Row::new("Person", [("id", Value::U64(1))]))?;
        txn.insert(
            &schema,
            Row::new(
                "Cast",
                [("person", Value::U64(1)), ("work", Value::U64(200))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Company",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::String("[us]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Keyword",
                [
                    ("id", Value::U64(1)),
                    ("word", Value::String("character-name-in-title".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(100)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkCompany",
                [("work", Value::U64(200)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "WorkKeyword",
                [("work", Value::U64(200)), ("keyword", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Title",
                [("id", Value::U64(100)), ("episode", Value::I64(60))],
            ),
        )?;
        txn.insert(
            &schema,
            Row::new(
                "Title",
                [("id", Value::U64(200)), ("episode", Value::I64(10))],
            ),
        )?;
        for id in 1_000..2_500 {
            txn.insert(
                &schema,
                Row::new(
                    "Title",
                    [("id", Value::U64(id)), ("episode", Value::I64(10))],
                ),
            )?;
        }
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("Alias")?.var("person", "person")?.done();
        query
            .rel("Cast")?
            .var("person", "person")?
            .var("work", "work")?
            .done();
        query
            .rel("Company")?
            .var("id", "company")?
            .string("country", "[us]")?
            .done();
        query
            .rel("Keyword")?
            .var("id", "keyword")?
            .string("word", "character-name-in-title")?
            .done();
        query
            .rel("WorkCompany")?
            .var("work", "work")?
            .var("company", "company")?
            .done();
        query
            .rel("WorkKeyword")?
            .var("work", "work")?
            .var("keyword", "keyword")?
            .done();
        query.rel("Person")?.var("id", "person")?.done();
        query
            .rel("Title")?
            .var("id", "work")?
            .var("episode", "episode")?
            .done();
        query.cmp(
            OperandRef::var("episode"),
            ComparisonOperator::Gte,
            OperandRef::integer(50),
        )?;
        query.cmp(
            OperandRef::var("episode"),
            ComparisonOperator::Lt,
            OperandRef::integer(100),
        )?;
        query.find_aggregate(AggregateFunction::Count, "work")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.rows, vec![vec![Value::U64(0)]]);
    assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::StaticEmpty);
    assert!(output.plan.counters.static_semijoin_prefixes_probed > 0);
    assert!(output.plan.counters.static_semijoin_candidate_values > 0);
    Ok(())
}

#[test]
fn sum_sink_decodes_only_aggregate_operand_values() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("amount", "amount")?
            .done();
        query
            .find_aggregate(AggregateFunction::Sum, "amount")?
            .find_aggregate(AggregateFunction::Count, "posting")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(
        output.rows,
        vec![vec![Value::Decimal(DecimalRaw(600)), Value::U64(3)]]
    );
    assert_eq!(output.plan.counters.bindings_yielded, 3);
    assert_eq!(output.plan.counters.decoded_values, 3);
    assert_eq!(output.plan.counters.materialized_output_values, 2);
    Ok(())
}

#[test]
fn grouped_count_decodes_dictionary_keys_only_at_final_output() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done();
        query
            .rel("Holder")?
            .var("id", "holder")?
            .var("name", "holder_name")?
            .done();
        query
            .find_var("holder_name")?
            .find_aggregate(AggregateFunction::Count, "account")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_same_rows(
        &output.rows,
        &[
            vec![Value::String("Alice".to_owned()), Value::U64(2)],
            vec![Value::String("Bob".to_owned()), Value::U64(1)],
        ],
    );
    assert_eq!(output.plan.counters.bindings_yielded, 3);
    assert_eq!(output.plan.counters.decoded_values, 2);
    assert_eq!(output.plan.counters.dictionary_reverse_lookups, 2);
    assert_eq!(output.plan.counters.materialized_output_values, 4);
    Ok(())
}

#[test]
fn aggregate_count_range_uses_multiplicity() -> TestResult {
    let mut sink = AggregateSink::new(&AggregatePlan {
        group_vars: Vec::new(),
        aggregates: vec![AggregateTerm {
            function: AggregateFunction::Count,
            var: VarId(0),
            value_type: ValueType::U64,
        }],
    });
    let binding = EncodedBinding::new(0);

    sink.emit_count_range(&binding, 7)?;

    let states = sink
        .groups
        .get(&SmallEncodedRow::new())
        .ok_or_else(|| Error::internal("missing aggregate state group"))?;
    assert!(matches!(states.as_slice(), [AggregateState::Count(7)]));
    Ok(())
}

#[test]
fn aggregation_groups_and_sums_decimal_values() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("amount", "amount")?
            .var("at", "t")?
            .done();
        query
            .find_var("account")?
            .find_aggregate(AggregateFunction::Sum, "amount")?
            .find_aggregate(AggregateFunction::Count, "posting")?
            .find_aggregate(AggregateFunction::Min, "t")?
            .find_aggregate(AggregateFunction::Max, "t")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_same_rows(
        &output.rows,
        &[
            vec![
                Value::Serial(1),
                Value::Decimal(DecimalRaw(300)),
                Value::U64(2),
                Value::Timestamp(TimestampMicros(10)),
                Value::Timestamp(TimestampMicros(20)),
            ],
            vec![
                Value::Serial(2),
                Value::Decimal(DecimalRaw(300)),
                Value::U64(1),
                Value::Timestamp(TimestampMicros(30)),
                Value::Timestamp(TimestampMicros(30)),
            ],
        ],
    );
    Ok(())
}

#[test]
fn detects_integer_and_decimal_aggregation_overflow() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(overflow_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, number_row(1, i64::MAX, i128::MAX))?;
        txn.insert(&schema, number_row(2, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    let int_query = typed_query(&schema, |query| {
        query.rel("Number")?.var("n", "n")?.done();
        query.find_aggregate(AggregateFunction::Sum, "n")?;
        Ok(())
    })?;
    assert!(matches!(
        env.read(|txn| txn.execute_query(&schema, &int_query, &InputBindings::new())),
        Err(Error::Query(QueryError::Aggregate(
            AggregateError::IntegerOverflow { .. }
        )))
    ));

    let decimal_query = typed_query(&schema, |query| {
        query.rel("Number")?.var("d", "d")?.done();
        query.find_aggregate(AggregateFunction::Sum, "d")?;
        Ok(())
    })?;
    assert!(matches!(
        env.read(|txn| txn.execute_query(&schema, &decimal_query, &InputBindings::new())),
        Err(Error::Query(QueryError::Aggregate(
            AggregateError::DecimalOverflow { .. }
        )))
    ));
    Ok(())
}

#[test]
fn input_type_mismatch_is_rejected_at_execution() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;
    let result = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("holder", Value::String("bad".to_owned()))]),
        )
    });
    assert!(matches!(
        result,
        Err(Error::Query(QueryError::Execute(
            ExecuteError::InputTypeMismatch { .. }
        )))
    ));
    Ok(())
}

#[test]
fn serial_input_accepts_serial_value() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("holder", Value::Serial(1))]),
        )
    })?;

    assert!(!output.rows.is_empty());
    Ok(())
}

#[test]
fn serial_input_rejects_u64_value() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;

    let result = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("holder", Value::U64(1))]),
        )
    });

    assert!(matches!(
        result,
        Err(Error::Query(QueryError::Execute(
            ExecuteError::InputTypeMismatch { .. }
        )))
    ));
    Ok(())
}

#[test]
fn enum_input_value_must_be_declared_variant() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("currency", "currency")?
            .done()
            .find_var("account")?;
        Ok(())
    })?;
    let result = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("currency", Value::Enum(123))]),
        )
    });
    assert!(matches!(
        result,
        Err(Error::Query(QueryError::Execute(
            ExecuteError::InputTypeMismatch { .. }
        )))
    ));
    Ok(())
}

#[test]
fn explain_and_storage_diagnostics_are_available() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("amount", "amount")?
            .var("at", "t")?
            .done();
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done();
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?;
        query.cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?;
        query.find_var("posting")?.find_var("amount")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([
                ("holder", Value::Serial(1)),
                ("start", Value::Timestamp(TimestampMicros(0))),
                ("end", Value::Timestamp(TimestampMicros(100))),
            ]),
        )
    })?;
    let explain = output.explain();
    assert!(explain.contains("variable_order"));
    assert!(explain.contains("runtime_kind"));
    assert!(explain.contains("timings:"));
    assert!(explain.contains("query_timing"));
    assert!(explain.contains("allocations:"));
    assert!(explain.contains("allocation_summary"));
    assert!(explain.contains("node_timing"));
    assert!(explain.contains("variable_estimate"));
    assert!(explain.contains("free_join_node"));
    assert!(explain.contains("candidate_plan"));
    assert!(explain.contains("free_join_estimates"));
    assert!(explain.contains("node_rows"));
    assert!(explain.contains("free_join_subatom"));
    assert!(!explain.contains("atoms:\n"));
    assert!(!explain.contains("index="));
    assert!(explain.contains("cursor_seeks"));
    assert!(explain.contains("rows_scanned"));
    assert!(explain.contains("bindings_yielded"));
    assert!(explain.contains("decoded_values"));
    assert!(explain.contains("encoded_comparisons_evaluated"));
    assert!(explain.contains("materialized_output_values"));
    assert!(explain.contains("trie_open"));
    assert!(explain.contains("trie_seek"));
    assert!(explain.contains("output_rows"));

    let diagnostics = env.storage_diagnostics(&schema)?;
    assert_eq!(diagnostics.storage_tx_id, 1);
    assert!(diagnostics.lmdb_map_size > 0);
    assert!(diagnostics.dictionary_entries > 0);
    assert!(
        diagnostics
            .relations
            .iter()
            .any(|relation| relation.relation == "Account" && relation.row_count == 3)
    );
    assert_eq!(
        diagnostics.schema_fingerprint,
        schema.descriptor().fingerprint().to_string()
    );
    Ok(())
}

#[test]
fn differential_reference_evaluator_matches_lmdb() -> TestResult {
    let (env, schema) = seeded_db()?;
    let reference = ReferenceDb::from_rows(seeded_rows());
    let cases = [
        (
            typed_query(&schema, |query| {
                query
                    .rel("Account")?
                    .var("id", "account")?
                    .input("holder", "holder")?
                    .done()
                    .find_var("account")?;
                Ok(())
            })?,
            InputBindings::from_values([("holder", Value::Serial(1))]),
        ),
        (
            typed_query(&schema, |query| {
                query
                    .rel("Account")?
                    .var("id", "account")?
                    .var("holder", "holder")?
                    .done();
                query
                    .rel("Holder")?
                    .var("id", "holder")?
                    .var("name", "holder_name")?
                    .done();
                query.find_var("account")?.find_var("holder_name")?;
                Ok(())
            })?,
            InputBindings::new(),
        ),
        (
            typed_query(&schema, |query| {
                query
                    .rel("Posting")?
                    .var("id", "posting")?
                    .var("account", "account")?
                    .var("amount", "amount")?
                    .var("at", "t")?
                    .done();
                query.cmp(
                    OperandRef::var("t"),
                    ComparisonOperator::Gte,
                    OperandRef::input("start"),
                )?;
                query.cmp(
                    OperandRef::var("t"),
                    ComparisonOperator::Lt,
                    OperandRef::input("end"),
                )?;
                query
                    .find_var("account")?
                    .find_aggregate(AggregateFunction::Sum, "amount")?
                    .find_aggregate(AggregateFunction::Count, "posting")?;
                Ok(())
            })?,
            InputBindings::from_values([
                ("start", Value::Timestamp(TimestampMicros(0))),
                ("end", Value::Timestamp(TimestampMicros(100))),
            ]),
        ),
    ];

    for (query, inputs) in cases {
        let lmdb_rows = env
            .read(|txn| txn.execute_query(&schema, &query, &inputs))?
            .rows;
        let reference_rows = reference.execute(&query, &inputs)?;
        assert_same_rows(&lmdb_rows, &reference_rows);
    }
    Ok(())
}

fn seeded_db() -> Result<(Environment, StorageSchema)> {
    let dir = tempfile::tempdir().map_err(|error| Error::io("tempdir", error))?;
    let path = dir.keep();
    let env = Environment::open(&path)?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    let rows = seeded_rows();
    env.write(|txn| {
        for row in &rows {
            txn.insert(&schema, row.clone())?;
        }
        Ok::<(), Error>(())
    })?;
    Ok((env, schema))
}

fn static_semijoin_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "StaticSemijoinDb",
        vec![
            RelationDescriptor::new(
                "Dim",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new(
                        "kind",
                        ValueType::Enum {
                            name: "Kind".to_owned(),
                        },
                    ),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_kind", ["kind", "id"])),
            RelationDescriptor::new(
                "OtherDim",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new(
                        "kind",
                        ValueType::Enum {
                            name: "Kind".to_owned(),
                        },
                    ),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_kind", ["kind", "id"])),
            RelationDescriptor::new(
                "Fact",
                vec![
                    FieldDescriptor::new("dim", ValueType::U64),
                    FieldDescriptor::new("item", ValueType::U64),
                ],
            )
            .with_covering_unique("dim_item", ["dim", "item"])
            .with_index(IndexDescriptor::equality("by_item", ["item", "dim"])),
            RelationDescriptor::new(
                "OwnerGroup",
                vec![
                    FieldDescriptor::new(
                        "owner",
                        ValueType::Serial {
                            type_name: "OwnerId".to_owned(),
                            owning_relation: "OwnerGroup".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("group", ValueType::U64),
                ],
            )
            .with_covering_unique("owner_group", ["owner", "group"])
            .with_index(IndexDescriptor::equality("by_group", ["group", "owner"])),
            RelationDescriptor::new(
                "OwnedFact",
                vec![
                    FieldDescriptor::new(
                        "owner",
                        ValueType::Serial {
                            type_name: "OwnerId".to_owned(),
                            owning_relation: "OwnerGroup".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("group", ValueType::U64),
                    FieldDescriptor::new("item", ValueType::U64),
                ],
            )
            .with_covering_unique("owner_group_item", ["owner", "group", "item"])
            .with_index(IndexDescriptor::equality(
                "by_group",
                ["group", "owner", "item"],
            )),
            RelationDescriptor::new(
                "Pair",
                vec![
                    FieldDescriptor::new("left", ValueType::U64),
                    FieldDescriptor::new("right", ValueType::U64),
                ],
            )
            .with_covering_unique("left_right", ["left", "right"])
            .with_index(IndexDescriptor::equality("by_right", ["right", "left"])),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Kind",
        [1, 2, 3],
    ))
}

fn static_semijoin_budget_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "StaticSemijoinBudgetDb",
        vec![
            RelationDescriptor::new(
                "Big",
                vec![
                    FieldDescriptor::new("pad", ValueType::U64),
                    FieldDescriptor::new("id", ValueType::U64),
                ],
            )
            .with_covering_unique("pad_id", ["pad", "id"]),
            RelationDescriptor::new("Link", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_covering_unique("id", ["id"]),
        ],
    )
}

fn q24_like_semijoin_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "StaticSemijoinQ24LikeDb",
        vec![
            RelationDescriptor::new(
                "Alias",
                vec![FieldDescriptor::new("person", ValueType::U64)],
            )
            .with_covering_unique("person", ["person"]),
            RelationDescriptor::new(
                "Character",
                vec![FieldDescriptor::new("id", ValueType::U64)],
            )
            .with_covering_unique("id", ["id"]),
            RelationDescriptor::new(
                "Appearance",
                vec![
                    FieldDescriptor::new("person", ValueType::U64),
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("character", ValueType::U64),
                    FieldDescriptor::new("role", ValueType::U64),
                ],
            )
            .with_covering_unique("person_work_role", ["person", "work", "role", "character"])
            .with_index(IndexDescriptor::equality(
                "by_role_work",
                ["role", "work", "person", "character"],
            )),
            RelationDescriptor::new(
                "Company",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("country", ValueType::String),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_country", ["country", "id"])),
            RelationDescriptor::new(
                "Keyword",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("word", ValueType::String),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_word", ["word", "id"])),
            RelationDescriptor::new(
                "WorkCompany",
                vec![
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("company", ValueType::U64),
                ],
            )
            .with_covering_unique("work_company", ["work", "company"])
            .with_index(IndexDescriptor::equality("by_company", ["company", "work"])),
            RelationDescriptor::new(
                "WorkKeyword",
                vec![
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("keyword", ValueType::U64),
                ],
            )
            .with_covering_unique("work_keyword", ["work", "keyword"])
            .with_index(IndexDescriptor::equality("by_keyword", ["keyword", "work"])),
            RelationDescriptor::new(
                "Person",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("gender", ValueType::String),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_gender", ["gender", "id"])),
            RelationDescriptor::new(
                "Role",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_name", ["name", "id"])),
            RelationDescriptor::new(
                "Title",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("year", ValueType::I64),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_year", ["year", "id"])),
        ],
    )
}

fn q16_like_semijoin_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "StaticSemijoinQ16LikeDb",
        vec![
            RelationDescriptor::new(
                "Alias",
                vec![FieldDescriptor::new("person", ValueType::U64)],
            )
            .with_covering_unique("person", ["person"]),
            RelationDescriptor::new(
                "Cast",
                vec![
                    FieldDescriptor::new("person", ValueType::U64),
                    FieldDescriptor::new("work", ValueType::U64),
                ],
            )
            .with_covering_unique("person_work", ["person", "work"])
            .with_index(IndexDescriptor::equality(
                "by_work_person",
                ["work", "person"],
            )),
            RelationDescriptor::new(
                "Company",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("country", ValueType::String),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_country", ["country", "id"])),
            RelationDescriptor::new(
                "Keyword",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("word", ValueType::String),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_word", ["word", "id"])),
            RelationDescriptor::new(
                "WorkCompany",
                vec![
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("company", ValueType::U64),
                ],
            )
            .with_covering_unique("work_company", ["work", "company"])
            .with_index(IndexDescriptor::equality("by_company", ["company", "work"])),
            RelationDescriptor::new(
                "WorkKeyword",
                vec![
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("keyword", ValueType::U64),
                ],
            )
            .with_covering_unique("work_keyword", ["work", "keyword"])
            .with_index(IndexDescriptor::equality("by_keyword", ["keyword", "work"])),
            RelationDescriptor::new("Person", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_covering_unique("id", ["id"]),
            RelationDescriptor::new(
                "Title",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("episode", ValueType::I64),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_episode", ["episode", "id"])),
        ],
    )
}

fn dim_row(id: u64, kind: u8) -> Row {
    Row::new("Dim", [("id", Value::U64(id)), ("kind", Value::Enum(kind))])
}

fn other_dim_row(id: u64, kind: u8) -> Row {
    Row::new(
        "OtherDim",
        [("id", Value::U64(id)), ("kind", Value::Enum(kind))],
    )
}

fn fact_row(dim: u64, item: u64) -> Row {
    Row::new(
        "Fact",
        [("dim", Value::U64(dim)), ("item", Value::U64(item))],
    )
}

fn owner_group_row(owner: u64, group: u64) -> Row {
    Row::new(
        "OwnerGroup",
        [
            ("owner", Value::Serial(owner)),
            ("group", Value::U64(group)),
        ],
    )
}

fn owned_fact_row(owner: u64, group: u64, item: u64) -> Row {
    Row::new(
        "OwnedFact",
        [
            ("owner", Value::Serial(owner)),
            ("group", Value::U64(group)),
            ("item", Value::U64(item)),
        ],
    )
}

fn pair_row(left: u64, right: u64) -> Row {
    Row::new(
        "Pair",
        [("left", Value::U64(left)), ("right", Value::U64(right))],
    )
}

fn seeded_rows() -> Vec<Row> {
    vec![
        holder_row(1, "Alice"),
        holder_row(2, "Bob"),
        account_row(1, 1, 1),
        account_row(2, 1, 2),
        account_row(3, 2, 1),
        posting_row(1, 1, 100, 10),
        posting_row(2, 1, 200, 20),
        posting_row(3, 2, 300, 30),
    ]
}

fn ledger_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "LedgerDb",
        vec![
            RelationDescriptor::new(
                "Holder",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "HolderId".to_owned(),
                            owning_relation: "Holder".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_covering_unique("id", ["id"]),
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "AccountId".to_owned(),
                            owning_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "holder",
                        ValueType::Serial {
                            type_name: "HolderId".to_owned(),
                            owning_relation: "Holder".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "holder",
                ["holder"],
                "Holder",
                "id",
            )),
            RelationDescriptor::new(
                "Posting",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "PostingId".to_owned(),
                            owning_relation: "Posting".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "account",
                        ValueType::Serial {
                            type_name: "AccountId".to_owned(),
                            owning_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "account",
                ["account"],
                "Account",
                "id",
            )),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Currency",
        [1, 2],
    ))
}

fn overflow_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "OverflowDb",
        vec![
            RelationDescriptor::new(
                "Number",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "NumberId".to_owned(),
                            owning_relation: "Number".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("n", ValueType::I64),
                    FieldDescriptor::new("d", ValueType::Decimal { scale: 0 }),
                ],
            )
            .with_covering_unique("id", ["id"]),
        ],
    )
}

fn optimizer_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "OptimizerDb",
        vec![
            RelationDescriptor::new(
                "Item",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "ItemId".to_owned(),
                            owning_relation: "Item".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "kind",
                        ValueType::Enum {
                            name: "Kind".to_owned(),
                        },
                    ),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_kind", ["kind", "id"])),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes("Kind", [1, 2]))
}

fn triangle_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "TriangleDb",
        vec![
            RelationDescriptor::new(
                "EdgeAB",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("b", ValueType::U64),
                ],
            )
            .with_covering_unique("a_b", ["a", "b"]),
            RelationDescriptor::new(
                "EdgeAC",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_covering_unique("a_c", ["a", "c"]),
            RelationDescriptor::new(
                "EdgeBC",
                vec![
                    FieldDescriptor::new("b", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_covering_unique("b_c", ["b", "c"]),
        ],
    )
}

fn chain_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "ChainDb",
        vec![
            RelationDescriptor::new("A", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_covering_unique("id", ["id"]),
            RelationDescriptor::new(
                "B",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("a", ValueType::U64),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_a", ["a", "id"])),
        ],
    )
}

fn direct_sailors_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "DirectSailorsDb",
        vec![
            RelationDescriptor::new(
                "Reserve",
                vec![
                    FieldDescriptor::new("sailor", ValueType::U64),
                    FieldDescriptor::new("boat", ValueType::U64),
                    FieldDescriptor::new("day", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_covering_unique("sailor_boat_day", ["sailor", "boat", "day"]),
        ],
    )
}

fn direct_chain4_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "DirectChain4Db",
        vec![
            RelationDescriptor::new("A", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_covering_unique("id", ["id"]),
            RelationDescriptor::new(
                "B",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("a", ValueType::U64),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_a", ["a", "id"])),
            RelationDescriptor::new(
                "C",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("b", ValueType::U64),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_b", ["b", "id"])),
            RelationDescriptor::new(
                "D",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_covering_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_c", ["c", "id"])),
        ],
    )
}

fn holder_row(id: u64, name: &str) -> Row {
    Row::new(
        "Holder",
        [
            ("id", Value::Serial(id)),
            ("name", Value::String(name.to_owned())),
        ],
    )
}

fn account_row(id: u64, holder: u64, currency: u8) -> Row {
    Row::new(
        "Account",
        [
            ("id", Value::Serial(id)),
            ("holder", Value::Serial(holder)),
            ("currency", Value::Enum(currency)),
        ],
    )
}

fn posting_row(id: u64, account: u64, amount: i128, at: i64) -> Row {
    Row::new(
        "Posting",
        [
            ("id", Value::Serial(id)),
            ("account", Value::Serial(account)),
            ("amount", Value::Decimal(DecimalRaw(amount))),
            ("at", Value::Timestamp(TimestampMicros(at))),
        ],
    )
}

fn number_row(id: u64, n: i64, d: i128) -> Row {
    Row::new(
        "Number",
        [
            ("id", Value::Serial(id)),
            ("n", Value::I64(n)),
            ("d", Value::Decimal(DecimalRaw(d))),
        ],
    )
}

fn item_row(id: u64, kind: u8) -> Row {
    Row::new(
        "Item",
        [("id", Value::Serial(id)), ("kind", Value::Enum(kind))],
    )
}

fn edge_ab_row(a: u64, b: u64) -> Row {
    Row::new("EdgeAB", [("a", Value::U64(a)), ("b", Value::U64(b))])
}

fn edge_ac_row(a: u64, c: u64) -> Row {
    Row::new("EdgeAC", [("a", Value::U64(a)), ("c", Value::U64(c))])
}

fn edge_bc_row(b: u64, c: u64) -> Row {
    Row::new("EdgeBC", [("b", Value::U64(b)), ("c", Value::U64(c))])
}

fn b_row(id: u64, a: u64) -> Row {
    Row::new("B", [("id", Value::U64(id)), ("a", Value::U64(a))])
}

fn reserve_row(sailor: u64, boat: u64, day: i64) -> Row {
    Row::new(
        "Reserve",
        [
            ("sailor", Value::U64(sailor)),
            ("boat", Value::U64(boat)),
            ("day", Value::Timestamp(TimestampMicros(day))),
        ],
    )
}

fn chain_a_row(id: u64) -> Row {
    Row::new("A", [("id", Value::U64(id))])
}

fn chain_b_row(id: u64, a: u64) -> Row {
    Row::new("B", [("id", Value::U64(id)), ("a", Value::U64(a))])
}

fn chain_c_row(id: u64, b: u64) -> Row {
    Row::new("C", [("id", Value::U64(id)), ("b", Value::U64(b))])
}

fn chain_d_row(id: u64, c: u64) -> Row {
    Row::new("D", [("id", Value::U64(id)), ("c", Value::U64(c))])
}

fn assert_same_rows(actual: &[Vec<Value>], expected: &[Vec<Value>]) {
    let mut actual = actual.to_vec();
    let mut expected = expected.to_vec();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

struct ReferenceDb {
    rows: BTreeMap<String, Vec<Row>>,
}

#[derive(Clone, Debug)]
struct ReferenceBinding {
    values: Vec<Option<Value>>,
}

impl ReferenceBinding {
    fn new(variable_count: usize) -> Self {
        Self {
            values: vec![None; variable_count],
        }
    }

    fn get(&self, variable: usize) -> Option<&Value> {
        self.values[variable].as_ref()
    }

    fn bind(&mut self, variable: usize, value: Value) -> bool {
        match &self.values[variable] {
            Some(existing) => existing == &value,
            None => {
                self.values[variable] = Some(value);
                true
            }
        }
    }
}

impl ReferenceDb {
    fn from_rows(rows: Vec<Row>) -> Self {
        let mut by_relation: BTreeMap<String, Vec<Row>> = BTreeMap::new();
        for row in rows {
            by_relation
                .entry(row.relation().to_owned())
                .or_default()
                .push(row);
        }
        Self { rows: by_relation }
    }

    fn execute(&self, query: &TypedQuery, inputs: &InputBindings) -> Result<Vec<Vec<Value>>> {
        let atoms = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Relation(atom) => Some(atom),
                TypedClause::Comparison(_) => None,
            })
            .collect::<Vec<_>>();
        let comparisons = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Comparison(comparison) => Some(comparison),
                TypedClause::Relation(_) => None,
            })
            .collect::<Vec<_>>();
        let mut output = Vec::new();
        let mut counters = PlanCounters::default();
        self.recurse(
            query,
            inputs,
            &atoms,
            &comparisons,
            0,
            ReferenceBinding::new(query.variables.len()),
            &mut output,
            &mut counters,
        )?;
        reference_project_results(query, &output)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "test reference recursion carries explicit evaluator state"
    )]
    fn recurse(
        &self,
        query: &TypedQuery,
        inputs: &InputBindings,
        atoms: &[&TypedRelationAtom],
        comparisons: &[&TypedComparison],
        depth: usize,
        binding: ReferenceBinding,
        output: &mut Vec<ReferenceBinding>,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if depth == atoms.len() {
            if reference_comparisons_pass(comparisons, query, inputs, &binding, counters)? {
                output.push(binding);
            }
            return Ok(());
        }

        let atom = atoms[depth];
        for row in self.rows.get(&atom.relation).into_iter().flatten() {
            let Some(next) = reference_match_atom(atom, query, inputs, &binding, row)? else {
                continue;
            };
            if reference_comparisons_pass(comparisons, query, inputs, &next, counters)? {
                self.recurse(
                    query,
                    inputs,
                    atoms,
                    comparisons,
                    depth + 1,
                    next,
                    output,
                    counters,
                )?;
            }
        }
        Ok(())
    }
}

fn reference_match_atom(
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
    row: &Row,
) -> Result<Option<ReferenceBinding>> {
    let mut next = binding.clone();
    for field in &atom.fields {
        let Some(row_value) = row.value(&field.field) else {
            return Ok(None);
        };
        match &field.term {
            TypedTerm::Variable(variable) => {
                let normalized =
                    reference_value_for_type(row_value, &query.variables[*variable].value_type);
                if !next.bind(*variable, normalized) {
                    return Ok(None);
                }
            }
            TypedTerm::Input(input) => {
                let input_value = reference_input_value(query, inputs, *input)?;
                let normalized =
                    reference_value_for_type(row_value, &query.inputs[*input].value_type);
                if input_value != &normalized {
                    return Ok(None);
                }
            }
            TypedTerm::Literal(literal) => {
                let normalized = reference_value_for_type(row_value, &literal.value_type);
                if literal_to_value(literal)? != normalized {
                    return Ok(None);
                }
            }
            TypedTerm::Wildcard => {}
        }
    }
    Ok(Some(next))
}

fn reference_comparisons_pass(
    comparisons: &[&TypedComparison],
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
    counters: &mut PlanCounters,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left) = reference_operand_value(&comparison.left, query, inputs, binding)? else {
            continue;
        };
        let Some(right) = reference_operand_value(&comparison.right, query, inputs, binding)?
        else {
            continue;
        };
        counters.comparisons_evaluated += 1;
        let left = reference_value_for_type(&left, &comparison.value_type);
        let right = reference_value_for_type(&right, &comparison.value_type);
        if !compare_values(&left, comparison.operator, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn reference_input_value<'a>(
    query: &'a TypedQuery,
    inputs: &'a InputBindings,
    input: usize,
) -> Result<&'a Value> {
    let input = &query.inputs[input];
    inputs
        .get(&input.name)
        .ok_or_else(|| Error::missing_input(&input.name))
}

fn reference_operand_value(
    operand: &TypedOperand,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
) -> Result<Option<Value>> {
    Ok(match operand {
        TypedOperand::Variable(variable) => binding.get(*variable).cloned(),
        TypedOperand::Input(input) => Some(reference_input_value(query, inputs, *input)?.clone()),
        TypedOperand::Literal(literal) => Some(literal_to_value(literal)?),
    })
}

fn reference_value_for_type(value: &Value, _value_type: &ValueType) -> Value {
    value.clone()
}

fn reference_project_results(
    query: &TypedQuery,
    bindings: &[ReferenceBinding],
) -> Result<Vec<Vec<Value>>> {
    let has_aggregate = query
        .find
        .iter()
        .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }));
    if has_aggregate {
        reference_project_aggregates(query, bindings)
    } else {
        let mut set = BTreeSet::new();
        for binding in bindings {
            let mut row = Vec::new();
            for term in &query.find {
                let TypedFindTerm::Variable { variable } = term else {
                    continue;
                };
                row.push(reference_bound_variable(binding, *variable)?.clone());
            }
            set.insert(row);
        }
        Ok(set.into_iter().collect())
    }
}

fn reference_project_aggregates(
    query: &TypedQuery,
    bindings: &[ReferenceBinding],
) -> Result<Vec<Vec<Value>>> {
    let group_terms = query
        .find
        .iter()
        .filter_map(|term| match term {
            TypedFindTerm::Variable { variable } => Some(*variable),
            TypedFindTerm::Aggregate { .. } => None,
        })
        .collect::<Vec<_>>();
    let aggregate_terms = query
        .find
        .iter()
        .filter_map(|term| match term {
            TypedFindTerm::Aggregate {
                function,
                variable,
                value_type,
            } => Some((*function, *variable, value_type.clone())),
            TypedFindTerm::Variable { .. } => None,
        })
        .collect::<Vec<_>>();

    let mut groups: BTreeMap<Vec<Value>, Vec<AggregateState>> = BTreeMap::new();
    for binding in bindings {
        let key = group_terms
            .iter()
            .map(|variable| reference_bound_variable(binding, *variable).cloned())
            .collect::<Result<Vec<_>>>()?;
        let states = groups.entry(key).or_insert_with(|| {
            aggregate_terms
                .iter()
                .map(|(function, _, value_type)| AggregateState::new(*function, value_type.clone()))
                .collect()
        });
        for (state, (_, variable, _)) in states.iter_mut().zip(&aggregate_terms) {
            state.apply(reference_bound_variable(binding, *variable)?)?;
        }
    }

    let mut rows = Vec::new();
    for (key, states) in groups {
        let mut row = Vec::new();
        let mut key_iter = key.into_iter();
        let mut state_iter = states.into_iter();
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { .. } => row.push(
                    key_iter
                        .next()
                        .ok_or_else(|| Error::internal("missing reference aggregate group key"))?,
                ),
                TypedFindTerm::Aggregate { .. } => {
                    let state = state_iter
                        .next()
                        .ok_or_else(|| Error::internal("missing reference aggregate state"))?;
                    row.push(state.finish()?)
                }
            }
        }
        rows.push(row);
    }
    rows.sort();
    Ok(rows)
}

fn reference_bound_variable(binding: &ReferenceBinding, variable: usize) -> Result<&Value> {
    binding
        .get(variable)
        .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
}
