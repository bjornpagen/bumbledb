use super::*;
use crate::query_image::{QueryImageBuilder, QueryImageScope};
use crate::{AggregateError, Environment, ExecuteError, Fact, QueryError};
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
    let allocations = QueryAllocationStats::default();
    assert!(!allocations.enabled);
    assert_eq!(allocations.alloc_calls, 0);
    assert_eq!(allocations.net_bytes, 0);

    let counters = PlanCounters::default();
    assert_eq!(counters.sink_emit_calls, 0);
    assert_eq!(counters.encoded_project_facts_seen, 0);
    assert_eq!(counters.lftj_next_calls, 0);
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
fn query_result_set_sorts_and_deduplicates_facts() {
    let set = QueryResultSet::new(
        vec![ResultColumn::Variable("id".to_owned())],
        vec![
            vec![Value::U64(2)],
            vec![Value::U64(1)],
            vec![Value::U64(1)],
        ],
    );

    assert_eq!(set.cardinality(), 2);
    assert_eq!(set.facts, vec![vec![Value::U64(1)], vec![Value::U64(2)]]);
}

#[test]
fn encoded_width_comparisons_match_byte_order() {
    assert_eq!(compare_encoded_bytes(&[1], &[2]), std::cmp::Ordering::Less);
    assert_eq!(
        compare_encoded_bytes(&1u64.to_be_bytes(), &2u64.to_be_bytes()),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        compare_encoded_bytes(&[0; 16], &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
        std::cmp::Ordering::Less
    );
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
        output.result.facts,
        vec![vec![Value::Serial(1)], vec![Value::Serial(2)]]
    );
    assert!(output.plan.timings.total_micros > 0);
    assert!(output.plan.timings.execute_micros <= output.plan.timings.total_micros);
    assert!(!output.plan.allocations.enabled);
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

    assert_same_facts(
        &output.result.facts,
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
fn static_lookup_uses_planned_lftj_after_storage_bypass_deletion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(optimizer_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, item_fact(1, 1))?;
        txn.insert(&schema, item_fact(2, 1))?;
        txn.insert(&schema, item_fact(3, 2))?;
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
    assert_eq!(output.plan.optimizer.chosen, "free_join_sorted_leapfrog");
    assert_eq!(output.plan.query_image_cache.builds, 1);
    assert_same_facts(
        &output.result.facts,
        &[vec![Value::Serial(1)], vec![Value::Serial(2)]],
    );
    Ok(())
}

#[test]
fn lftj_empty_checks_static_existence_atoms() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, b_fact(1, 99))?;
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

    assert!(output.result.facts.is_empty());
    assert_eq!(output.plan.counters.trie_open, 0);
    Ok(())
}

#[test]
fn partial_probe_shape_falls_back_to_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_fact(1))?;
        txn.insert(&schema, chain_b_fact(10, 1))?;
        txn.insert(&schema, chain_c_fact(20, 10))?;
        txn.insert(&schema, chain_c_fact(21, 10))?;
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
    assert!(
        output
            .plan
            .free_join
            .nodes
            .iter()
            .any(|node| node.implementation == NodeImpl::SortedLeapfrog)
    );
    assert!(output.plan.counters.trie_next > 0);
    assert_same_facts(
        &output.result.facts,
        &[vec![Value::U64(20)], vec![Value::U64(21)]],
    );
    Ok(())
}

#[test]
fn prefix_range_filter_uses_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(reserve_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, reserve_fact(1, 10, 5))?;
        txn.insert(&schema, reserve_fact(1, 11, 15))?;
        txn.insert(&schema, reserve_fact(2, 12, 5))?;
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
    assert_same_facts(
        &output.result.facts,
        &[vec![Value::U64(10), Value::Timestamp(TimestampMicros(5))]],
    );
    assert_eq!(output.plan.query_image_cache.builds, 1);
    assert!(output.plan.counters.trie_open > 0);
    Ok(())
}

#[test]
fn no_prefix_range_filter_uses_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(reserve_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, reserve_fact(1, 10, 5))?;
        txn.insert(&schema, reserve_fact(1, 11, 15))?;
        txn.insert(&schema, reserve_fact(2, 12, 25))?;
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
    assert_eq!(output.plan.query_image_cache.builds, 1);
    assert_same_facts(
        &output.result.facts,
        &[
            vec![Value::U64(1), Value::U64(11)],
            vec![Value::U64(2), Value::U64(12)],
        ],
    );
    Ok(())
}

#[test]
fn prefix_range_empty_prefix_returns_zero_facts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(reserve_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, reserve_fact(1, 10, 5))?;
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
    assert!(output.result.facts.is_empty());
    assert_eq!(output.plan.counters.trie_open, 0);
    Ok(())
}

#[test]
fn chain_query_uses_lftj_and_returns_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_fact(1))?;
        txn.insert(&schema, chain_b_fact(10, 1))?;
        txn.insert(&schema, chain_c_fact(20, 10))?;
        txn.insert(&schema, chain_d_fact(30, 20))?;
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
    assert_eq!(output.result.facts, vec![vec![Value::U64(30)]]);
    assert_eq!(output.plan.counters.materialized_output_values, 1);
    assert_eq!(output.plan.counters.dictionary_reverse_lookups, 0);
    assert!(output.plan.counters.trie_open > 0);
    Ok(())
}

#[test]
fn lazy_access_slice_avoids_temp_trie_builds_and_matches_eager_fallback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Fact::new("A", [("id", Value::U64(1))]))?;
        txn.insert(&schema, Fact::new("A", [("id", Value::U64(2))]))?;
        txn.insert(&schema, b_fact(10, 1))?;
        txn.insert(&schema, b_fact(11, 1))?;
        txn.insert(&schema, b_fact(20, 2))?;
        Ok::<_, Error>(())
    })?;
    let lazy_query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.var("a", "a")?.done();
        query.find_var("a")?.find_var("b")?;
        Ok(())
    })?;
    let eager_equivalent = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.var("a", "a")?.done();
        query.cmp(
            OperandRef::var("a"),
            ComparisonOperator::NotEq,
            OperandRef::integer(999),
        )?;
        query.find_var("a")?.find_var("b")?;
        Ok(())
    })?;

    let lazy = env.read(|txn| txn.execute_query(&schema, &lazy_query, &InputBindings::new()))?;
    let eager =
        env.read(|txn| txn.execute_query(&schema, &eager_equivalent, &InputBindings::new()))?;

    assert_same_facts(&lazy.result.facts, &eager.result.facts);
    assert_eq!(lazy.plan.counters.sorted_trie_builds, 0);
    assert_eq!(lazy.plan.counters.atom_temp_relation_builds, 0);
    assert_eq!(lazy.plan.counters.lftj_atom_bytes_copied, 0);
    assert!(lazy.plan.counters.lftj_eager_builds_avoided >= 2);
    assert!(eager.plan.counters.sorted_trie_builds > lazy.plan.counters.sorted_trie_builds);
    assert!(eager.plan.counters.lftj_atom_bytes_copied > lazy.plan.counters.lftj_atom_bytes_copied);
    Ok(())
}

#[test]
fn chain_existence_filter_after_binding_returns_survivor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_fact(1))?;
        txn.insert(&schema, chain_b_fact(10, 1))?;
        txn.insert(&schema, chain_b_fact(11, 1))?;
        txn.insert(&schema, chain_c_fact(10, 99))?;
        txn.insert(&schema, chain_c_fact(11, 100))?;
        Ok::<_, Error>(())
    })?;
    let query = chain_existence_filter_query(&schema)?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("a", Value::U64(1))]),
        )
    })?;
    assert_eq!(output.result.facts, vec![vec![Value::U64(10)]]);
    assert!(output.plan.counters.trie_open > 0);
    Ok(())
}

#[test]
fn chain_existence_filter_can_remove_all_bindings() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_fact(1))?;
        txn.insert(&schema, chain_b_fact(10, 1))?;
        txn.insert(&schema, chain_c_fact(10, 100))?;
        Ok::<_, Error>(())
    })?;
    let query = chain_existence_filter_query(&schema)?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("a", Value::U64(1))]),
        )
    })?;
    assert!(output.result.facts.is_empty());
    assert_eq!(output.plan.counters.trie_open, 0);
    Ok(())
}

#[test]
fn tag_lookup_like_projection_uses_lftj_after_literal_filter() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_fact(1))?;
        txn.insert(&schema, chain_b_fact(10, 1))?;
        txn.insert(&schema, chain_c_fact(20, 10))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.input("id", "a")?.done();
        query
            .rel("B")?
            .var("id", "posting")?
            .input("a", "a")?
            .done();
        query
            .rel("C")?
            .var("id", "account")?
            .var("b", "posting")?
            .done();
        query.find_var("posting")?.find_var("account")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("a", Value::U64(1))]),
        )
    })?;
    assert_same_facts(
        &output.result.facts,
        &[vec![Value::U64(10), Value::U64(20)]],
    );
    Ok(())
}

#[test]
fn cardinality_matches_materialized_projection_without_decoding_output() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Fact::new("A", [("id", Value::U64(1))]))?;
        txn.insert(&schema, chain_b_fact(10, 1))?;
        txn.insert(&schema, chain_c_fact(20, 10))?;
        txn.insert(&schema, chain_d_fact(30, 20))?;
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
    let cardinality = env.read(|txn| txn.execute_result_cardinality(&schema, &query, &inputs))?;

    assert_eq!(cardinality.cardinality, materialized.result.facts.len());
    assert_eq!(cardinality.plan.counters.materialized_output_values, 0);
    Ok(())
}

#[test]
fn chain_broken_path_returns_zero_facts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_b_fact(10, 1))?;
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
    assert!(output.result.facts.is_empty());
    assert_eq!(output.plan.counters.trie_open, 0);
    Ok(())
}

#[test]
fn optimizer_keeps_cyclic_triangle_on_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_fact(1, 10))?;
        txn.insert(&schema, edge_ac_fact(1, 20))?;
        txn.insert(&schema, edge_bc_fact(10, 20))?;
        txn.insert(&schema, edge_ab_fact(2, 10))?;
        txn.insert(&schema, edge_ac_fact(2, 30))?;
        txn.insert(&schema, edge_bc_fact(10, 40))?;
        Ok::<(), Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.rel("EdgeBC")?.var("b", "b")?.var("c", "c")?.done();
        query.find_count_domain(["a"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(1)]]);
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
            .any(|candidate| candidate.name == "free_join_sorted_leapfrog")
    );
    Ok(())
}

#[test]
fn lftj_atom_cache_reuses_equivalent_relation_aliases() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Fact::new("A", [("id", Value::U64(1))]))?;
        txn.insert(&schema, Fact::new("A", [("id", Value::U64(2))]))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "left")?.done();
        query.rel("A")?.var("id", "right")?.done();
        query.find_var("left")?.find_var("right")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    assert!(output.plan.counters.sorted_trie_builds <= 1);
    assert_eq!(output.result.facts.len(), 4);
    Ok(())
}

#[test]
fn lftj_atom_cache_separates_literal_local_comparison_filters() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_join_schema(), env.max_key_size())?;
    seed_title_company_range_facts(&env, &schema)?;

    let through_2015 = title_company_count_query(&schema, OperandRef::integer(2015))?;
    let through_2020 = title_company_count_query(&schema, OperandRef::integer(2020))?;

    env.read(|txn| {
        let first = txn.execute_query(&schema, &through_2015, &InputBindings::new())?;
        let second = txn.execute_query(&schema, &through_2020, &InputBindings::new())?;

        assert_eq!(first.result.facts, vec![vec![Value::U64(2)]]);
        assert_eq!(second.result.facts, vec![vec![Value::U64(3)]]);
        assert!(second.plan.counters.sorted_trie_cache_hits >= 1);
        assert!(second.plan.counters.sorted_trie_cache_misses >= 1);
        Ok::<_, Error>(())
    })?;

    Ok(())
}

#[test]
fn lftj_atom_cache_reuses_identical_local_comparison_filters() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_join_schema(), env.max_key_size())?;
    seed_title_company_range_facts(&env, &schema)?;

    let query = title_company_count_query(&schema, OperandRef::integer(2015))?;

    env.read(|txn| {
        let first = txn.execute_query(&schema, &query, &InputBindings::new())?;
        let second = txn.execute_query(&schema, &query, &InputBindings::new())?;

        assert_eq!(first.result.facts, vec![vec![Value::U64(2)]]);
        assert_eq!(second.result.facts, vec![vec![Value::U64(2)]]);
        assert!(second.plan.counters.sorted_trie_cache_hits >= 2);
        assert_eq!(second.plan.counters.sorted_trie_cache_misses, 0);
        Ok::<_, Error>(())
    })?;

    Ok(())
}

#[test]
fn lftj_atom_cache_separates_prepared_input_local_comparison_filters() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_join_schema(), env.max_key_size())?;
    seed_title_company_range_facts(&env, &schema)?;

    let query = title_company_count_query(&schema, OperandRef::input("max_year"))?;
    let prepared = env.prepare_query(&schema, &query)?;
    let through_2015 = InputBindings::from_values([("max_year", Value::I64(2015))]);
    let through_2020 = InputBindings::from_values([("max_year", Value::I64(2020))]);

    env.read(|txn| {
        let first = txn.execute_prepared_query(&schema, &prepared, &through_2015)?;
        let second = txn.execute_prepared_query(&schema, &prepared, &through_2020)?;

        assert_eq!(first.result.facts, vec![vec![Value::U64(2)]]);
        assert_eq!(second.result.facts, vec![vec![Value::U64(3)]]);
        assert!(second.plan.prepared_plan_cache.hits >= 1);
        assert!(second.plan.counters.sorted_trie_cache_hits >= 1);
        assert!(second.plan.counters.sorted_trie_cache_misses >= 1);
        Ok::<_, Error>(())
    })?;

    Ok(())
}

#[test]
fn lftj_reuses_lazy_access_across_cross_atom_comparison_filters() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_fact(1, 10))?;
        txn.insert(&schema, edge_ab_fact(1, 30))?;
        txn.insert(&schema, edge_ac_fact(1, 20))?;
        Ok::<_, Error>(())
    })?;
    let less_than = edge_cross_comparison_query(&schema, ComparisonOperator::Lt)?;
    let greater_than = edge_cross_comparison_query(&schema, ComparisonOperator::Gt)?;

    env.read(|txn| {
        let first = txn.execute_query(&schema, &less_than, &InputBindings::new())?;
        let second = txn.execute_query(&schema, &greater_than, &InputBindings::new())?;

        assert_same_facts(&first.result.facts, &[vec![Value::U64(10)]]);
        assert_same_facts(&second.result.facts, &[vec![Value::U64(30)]]);
        assert!(
            second.plan.counters.sorted_trie_cache_hits
                + second.plan.counters.lftj_eager_builds_avoided
                >= 2
        );
        Ok::<_, Error>(())
    })?;

    Ok(())
}

#[test]
fn lftj_empty_variable_atom_short_circuits_execution() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Fact::new("A", [("id", Value::U64(1))]))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.integer("a", 99)?.done();
        query.find_var("a")?.find_var("b")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.result.facts.is_empty());
    assert_eq!(output.plan.optimizer.chosen, "free_join_sorted_leapfrog");
    assert_eq!(output.plan.counters.trie_open, 0);
    assert_eq!(output.plan.counters.variable_candidates, 0);
    Ok(())
}

#[test]
fn domain_count_falls_back_to_lftj_until_fast_paths_are_rebuilt() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_fact(1, 10))?;
        txn.insert(&schema, edge_ab_fact(1, 11))?;
        txn.insert(
            &schema,
            Fact::new("EdgeAC", [("a", Value::U64(1)), ("c", Value::U64(20))]),
        )?;
        txn.insert(
            &schema,
            Fact::new("EdgeAC", [("a", Value::U64(2)), ("c", Value::U64(30))]),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.find_count_domain(["a"])?;
        Ok(())
    })?;
    let prepared = env.prepare_query(&schema, &query)?;

    let output =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(1)]]);
    Ok(())
}

#[test]
fn domain_count_serial_literal_filter_uses_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, owner_group_fact(1, 10))?;
        txn.insert(&schema, owner_group_fact(2, 20))?;
        txn.insert(&schema, owned_fact_fact(9, 10, 100))?;
        txn.insert(&schema, owned_fact_fact(9, 10, 101))?;
        txn.insert(&schema, owned_fact_fact(9, 20, 200))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(2)]]);
    Ok(())
}

#[test]
fn domain_count_enum_literal_filter_uses_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, dim_fact(2, 2))?;
        txn.insert(&schema, fact_fact(1, 10))?;
        txn.insert(&schema, fact_fact(1, 11))?;
        txn.insert(&schema, fact_fact(2, 20))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(2)]]);
    Ok(())
}

#[test]
fn domain_count_range_filter_uses_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_join_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new("Title", [("id", Value::U64(1)), ("year", Value::I64(2004))]),
        )?;
        txn.insert(
            &schema,
            Fact::new("Title", [("id", Value::U64(2)), ("year", Value::I64(2005))]),
        )?;
        txn.insert(
            &schema,
            Fact::new("Title", [("id", Value::U64(3)), ("year", Value::I64(2015))]),
        )?;
        txn.insert(
            &schema,
            Fact::new("Title", [("id", Value::U64(4)), ("year", Value::I64(2016))]),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(1)), ("company", Value::U64(10))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(2)), ("company", Value::U64(20))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(3)), ("company", Value::U64(30))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
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
        query.find_count_domain(["company"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(2)]]);
    Ok(())
}

#[test]
fn domain_count_literal_and_range_filters_use_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_join_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "Company",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::String("[us]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Company",
                [
                    ("id", Value::U64(2)),
                    ("country", Value::String("[de]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Title",
                [("id", Value::U64(10)), ("year", Value::I64(2010))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Title",
                [("id", Value::U64(20)), ("year", Value::I64(2010))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Title",
                [("id", Value::U64(30)), ("year", Value::I64(2020))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(10)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(20)), ("company", Value::U64(2))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
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
        query.find_count_domain(["work"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(1)]]);
    Ok(())
}

#[test]
fn domain_count_unsafe_cycle_uses_generic_lftj() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_fact(1, 10))?;
        txn.insert(
            &schema,
            Fact::new("EdgeAC", [("a", Value::U64(1)), ("c", Value::U64(20))]),
        )?;
        txn.insert(
            &schema,
            Fact::new("EdgeBC", [("b", Value::U64(10)), ("c", Value::U64(20))]),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.rel("EdgeBC")?.var("b", "b")?.var("c", "c")?.done();
        query.find_count_domain(["a"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(1)]]);
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

    assert_eq!(first.result.facts, second.result.facts);
    assert_eq!(first.plan.prepared_plan_cache.misses, 1);
    assert_eq!(first.plan.prepared_plan_cache.builds, 1);
    assert_eq!(second.plan.prepared_plan_cache.hits, 1);
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

    assert_eq!(first.result.facts, second.result.facts);
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
        txn.insert(&schema, account_fact(4, 2, 2))?;
        Ok::<_, Error>(())
    })?;
    let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(before.plan.prepared_plan_cache.misses, 1);
    assert_eq!(before.plan.prepared_plan_cache.builds, 1);
    assert_eq!(after.plan.prepared_plan_cache.misses, 1);
    assert_eq!(after.plan.prepared_plan_cache.builds, 1);
    assert_eq!(after.plan.prepared_plan_cache.hits, 0);
    assert_eq!(after.result.facts.len(), before.result.facts.len() + 1);
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

    assert_eq!(first.result.facts, second.result.facts);
    assert_eq!(first.plan.planner_stats.builds, 1);
    assert_eq!(first.plan.planner_stats.misses, 1);
    assert_eq!(second.plan.planner_stats.builds, 1);
    assert_eq!(second.plan.planner_stats.misses, 1);
    assert!(second.plan.planner_stats.hits >= 1 || second.plan.prepared_plan_cache.hits >= 1);
    assert_eq!(second.plan.counters.sorted_trie_builds, 0);
    assert_eq!(second.plan.counters.atom_temp_relation_builds, 0);
    assert!(
        second.plan.counters.sorted_trie_cache_hits
            + second.plan.counters.lftj_eager_builds_avoided
            >= 1
    );
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
    assert_eq!(warm.result.facts.len(), 3);
    assert_eq!(output.result.facts.len(), 3);
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
        txn.insert(&schema, account_fact(4, 2, 2))?;
        Ok::<_, Error>(())
    })?;
    let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(before.plan.query_image_cache.builds, 1);
    assert_eq!(after.plan.query_image_cache.builds, 2);
    assert_eq!(after.result.facts.len(), before.result.facts.len() + 1);
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
        query.find_count_domain(["item"])?;
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
    assert_eq!(second.result.facts.len(), 3);
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
        txn.insert(&schema, account_fact(4, 2, 2))?;
        Ok::<_, Error>(())
    })?;
    let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(before.plan.planner_stats.builds, 1);
    assert_eq!(after.plan.planner_stats.builds, 1);
    assert_eq!(after.result.facts.len(), before.result.facts.len() + 1);
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

    assert_eq!(first.result.facts, second.result.facts);
    assert!(first.plan.timings.normalize_micros > 0);
    assert_eq!(second.plan.timings.normalize_micros, 0);
    Ok(())
}

#[test]
fn cache_options_do_not_cache_aggregate_results() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_fact(1, 10))?;
        txn.insert(&schema, edge_ab_fact(1, 11))?;
        txn.insert(
            &schema,
            Fact::new("EdgeAC", [("a", Value::U64(1)), ("c", Value::U64(20))]),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.find_count_domain(["a"])?;
        Ok(())
    })?;
    let prepared = env.prepare_query(&schema, &query)?;

    let first =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &InputBindings::new()))?;
    let cached =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &InputBindings::new()))?;

    assert_eq!(first.result.facts, vec![vec![Value::U64(1)]]);
    assert_eq!(cached.result.facts, first.result.facts);

    env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new("EdgeAC", [("a", Value::U64(1)), ("c", Value::U64(21))]),
        )?;
        Ok::<_, Error>(())
    })?;
    let after_write =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &InputBindings::new()))?;
    assert_eq!(after_write.result.facts, vec![vec![Value::U64(1)]]);
    Ok(())
}

#[test]
fn aggregate_domain_results_differ_for_different_inputs() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done();
        query.find_count_domain(["account"])?;
        Ok(())
    })?;
    let prepared = env.prepare_query(&schema, &query)?;
    let holder_one = InputBindings::from_values([("holder", Value::Serial(1))]);
    let holder_two = InputBindings::from_values([("holder", Value::Serial(2))]);

    let first = env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &holder_one))?;
    let different_input =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &holder_two))?;

    assert_eq!(first.result.facts, vec![vec![Value::U64(2)]]);
    assert_eq!(different_input.result.facts, vec![vec![Value::U64(1)]]);
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
        let local_comparisons = atom_local_comparison_predicates(&normalized, atom);
        Ok::<_, Error>((
            lftj_atom_cache_key(atom, &variables, &first_inputs, &local_comparisons),
            lftj_atom_cache_key(atom, &variables, &same_inputs, &local_comparisons),
            lftj_atom_cache_key(atom, &variables, &second_inputs, &local_comparisons),
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
        txn.insert(&schema, edge_ab_fact(1, 1))?;
        txn.insert(&schema, edge_ab_fact(1, 2))?;
        Ok::<(), Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "a")?.done();
        query.find_var("a")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(1)]]);
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
    assert!(output.plan.counters.lftj_next_calls > 0);
    assert!(output.plan.counters.lftj_key_reads > 0);
    assert!(output.plan.counters.lftj_completed_bindings > 0);
    assert_eq!(output.plan.counters.sink_emit_calls, 0);
    assert_eq!(
        output.plan.counters.encoded_project_facts_seen,
        output.plan.counters.bindings_yielded
    );
    assert_same_facts(
        &output.result.facts,
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
    assert_same_facts(
        &output.result.facts,
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
        output.result.facts,
        vec![vec![Value::Serial(1)], vec![Value::Serial(2)]]
    );
    assert_eq!(output.plan.counters.bindings_yielded, 3);
    assert_eq!(output.plan.counters.materialized_output_values, 2);
    assert_eq!(output.plan.counters.encoded_project_facts_seen, 3);
    assert_eq!(output.plan.counters.encoded_project_facts_inserted, 2);
    assert_eq!(output.plan.counters.project_decode_values, 2);
    Ok(())
}

#[test]
fn materialized_projection_is_recomputed_without_result_cache() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain4_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, chain_a_fact(1))?;
        txn.insert(&schema, chain_b_fact(10, 1))?;
        txn.insert(&schema, chain_c_fact(20, 10))?;
        txn.insert(&schema, chain_c_fact(21, 10))?;
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
    let prepared = env.prepare_query(&schema, &query)?;

    let first =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &InputBindings::new()))?;
    let second =
        env.read(|txn| txn.execute_prepared_query(&schema, &prepared, &InputBindings::new()))?;

    assert_same_facts(
        &first.result.facts,
        &[vec![Value::U64(20)], vec![Value::U64(21)]],
    );
    assert_eq!(second.result.facts, first.result.facts);
    assert!(second.plan.counters.materialized_output_values <= second.result.facts.len() as u64);
    Ok(())
}

#[test]
fn count_sink_avoids_decoding_counted_variable() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
        query.rel("Posting")?.var("id", "posting")?.done();
        query.find_count_domain(["posting"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(3)]]);
    assert_eq!(output.plan.counters.bindings_yielded, 3);
    assert_eq!(output.plan.counters.aggregate_groups, 1);
    assert_eq!(output.plan.counters.decoded_values, 0);
    assert_eq!(output.plan.counters.materialized_output_values, 1);
    assert_eq!(output.plan.counters.encoded_project_facts_seen, 0);
    assert_eq!(output.plan.counters.encoded_project_facts_inserted, 0);
    assert!(
        output.plan.counters.materialized_output_values < output.plan.counters.bindings_yielded
    );
    Ok(())
}

#[test]
fn global_count_over_empty_input_returns_zero_fact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let query = typed_query(&schema, |query| {
        query
            .rel("A")?
            .var("id", "a")?
            .done()
            .find_count_domain(["a"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
    assert_eq!(output.plan.counters.output_facts, 1);
    Ok(())
}

#[test]
fn grouped_count_over_empty_input_returns_no_facts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.find_var("a")?.find_count_domain(["a"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.result.facts.is_empty());
    Ok(())
}

#[test]
fn count_distinct_ignores_duplicate_existential_witnesses() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, edge_ab_fact(1, 10))?;
        txn.insert(&schema, edge_ab_fact(1, 11))?;
        txn.insert(&schema, edge_ac_fact(1, 20))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.find_count_distinct("a")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(1)]]);
    Ok(())
}

#[test]
fn sum_over_domain_counts_distinct_domain_facts_with_same_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(overflow_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, number_fact(1, 5, 0))?;
        txn.insert(&schema, number_fact(2, 5, 0))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query.rel("Number")?.var("id", "id")?.var("n", "n")?.done();
        query.find_sum_over("n", ["id"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::I64(10)]]);
    Ok(())
}

#[test]
fn lftj_empty_global_count_returns_zero_fact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    env.write(|txn| {
        let _ = txn.insert(&schema, Fact::new("A", [("id", Value::U64(1))]))?;
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
            .find_count_domain(["a"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
    Ok(())
}

#[test]
fn lftj_dimension_fact_exists_but_fact_is_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, fact_fact(2, 10))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
    Ok(())
}

#[test]
fn lftj_disjoint_central_candidates_prove_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, other_dim_fact(2, 2))?;
        txn.insert(&schema, fact_fact(1, 10))?;
        txn.insert(&schema, fact_fact(2, 20))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
    Ok(())
}

#[test]
fn lftj_enum_literal_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(7, 1))?;
        txn.insert(&schema, fact_fact(8, 99))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
    Ok(())
}

#[test]
fn lftj_serial_literal_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, owner_group_fact(1, 10))?;
        txn.insert(&schema, owned_fact_fact(2, 11, 99))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
    Ok(())
}

#[test]
fn lftj_compound_relation_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, other_dim_fact(2, 2))?;
        txn.insert(&schema, pair_fact(1, 3))?;
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
        query.find_count_domain(["left"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
    Ok(())
}

#[test]
fn lftj_large_empty_join_returns_no_facts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_budget_schema(), env.max_key_size())?;
    env.write(|txn| {
        for id in 1..=1_001 {
            txn.insert(
                &schema,
                Fact::new("Big", [("pad", Value::U64(0)), ("id", Value::U64(id))]),
            )?;
        }
        txn.insert(&schema, Fact::new("Link", [("id", Value::U64(999_999))]))?;
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

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(output.result.facts.is_empty());
    Ok(())
}

#[test]
fn lftj_non_empty_query_is_not_proven_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, fact_fact(1, 10))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(1)]]);
    Ok(())
}

#[test]
fn lftj_negative_cache_skips_second_failed_proof() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, fact_fact(1, 10))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let first = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    let second = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(first.result.facts, vec![vec![Value::U64(1)]]);
    assert_eq!(second.result.facts, vec![vec![Value::U64(1)]]);
    Ok(())
}

#[test]
fn lftj_replans_after_write() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, fact_fact(1, 10))?;
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
        query.find_count_domain(["item"])?;
        Ok(())
    })?;

    let first = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    env.write(|txn| {
        txn.insert(&schema, fact_fact(1, 11))?;
        Ok::<_, Error>(())
    })?;
    let after_write = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(first.result.facts, vec![vec![Value::U64(1)]]);
    assert_eq!(after_write.result.facts, vec![vec![Value::U64(2)]]);
    Ok(())
}

#[test]
fn lftj_cache_is_input_scoped_and_reuses_proven_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, dim_fact(2, 2))?;
        txn.insert(&schema, fact_fact(1, 10))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "dim")?
            .input("kind", "kind")?
            .done();
        query
            .rel("Fact")?
            .var("dim", "dim")?
            .var("item", "item")?
            .done();
        query.find_count_domain(["item"])?;
        Ok(())
    })?;
    let kind_one = InputBindings::from_values([("kind", Value::Enum(1))]);
    let kind_two = InputBindings::from_values([("kind", Value::Enum(2))]);

    let non_empty = env.read(|txn| txn.execute_query(&schema, &query, &kind_one))?;
    let empty_first = env.read(|txn| txn.execute_query(&schema, &query, &kind_two))?;
    let empty_cached = env.read(|txn| txn.execute_query(&schema, &query, &kind_two))?;

    assert_eq!(non_empty.result.facts, vec![vec![Value::U64(1)]]);
    assert_eq!(empty_first.result.facts, vec![vec![Value::U64(0)]]);
    assert_eq!(empty_cached.result.facts, vec![vec![Value::U64(0)]]);
    Ok(())
}

#[test]
fn lftj_red_boat_like_wide_projection_skips_and_preserves_facts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, fact_fact(1, 10))?;
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
        query.cmp(
            OperandRef::var("item"),
            ComparisonOperator::NotEq,
            OperandRef::integer(999),
        )?;
        query.find_var("dim")?.find_var("item")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_same_facts(&output.result.facts, &[vec![Value::U64(1), Value::U64(10)]]);
    Ok(())
}

#[test]
fn lftj_tag_lookup_like_chain_projection_skips() -> TestResult {
    let (env, schema) = seeded_db()?;
    let query = typed_query(&schema, |query| {
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
        query.cmp(
            OperandRef::var("account"),
            ComparisonOperator::Eq,
            OperandRef::input("account"),
        )?;
        query.find_var("posting")?.find_var("holder")?;
        Ok(())
    })?;

    let output = env.read(|txn| {
        txn.execute_query(
            &schema,
            &query,
            &InputBindings::from_values([("account", Value::Serial(1))]),
        )
    })?;

    assert_same_facts(
        &output.result.facts,
        &[
            vec![Value::Serial(1), Value::Serial(1)],
            vec![Value::Serial(2), Value::Serial(1)],
        ],
    );
    Ok(())
}

#[test]
fn lftj_tpch_like_non_empty_materialized_projection_skips() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(join_filter_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, dim_fact(1, 1))?;
        txn.insert(&schema, fact_fact(1, 10))?;
        txn.insert(&schema, other_dim_fact(10, 2))?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Dim")?
            .var("id", "supplier")?
            .integer("kind", 1)?
            .done();
        query
            .rel("Fact")?
            .var("dim", "supplier")?
            .var("item", "line")?
            .done();
        query
            .rel("OtherDim")?
            .var("id", "line")?
            .var("kind", "status")?
            .done();
        query.cmp(
            OperandRef::var("line"),
            ComparisonOperator::NotEq,
            OperandRef::integer(999),
        )?;
        query.find_var("line")?.find_var("status")?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_same_facts(
        &output.result.facts,
        &[vec![Value::U64(10), Value::Enum(2)]],
    );
    Ok(())
}

#[test]
fn lftj_q24_like_empty_shape_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q24_like_join_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Fact::new("Alias", [("person", Value::U64(1))]))?;
        txn.insert(&schema, Fact::new("Character", [("id", Value::U64(1))]))?;
        txn.insert(
            &schema,
            Fact::new(
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
            Fact::new(
                "Company",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::String("[us]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Keyword",
                [
                    ("id", Value::U64(1)),
                    ("word", Value::String("hero".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Person",
                [
                    ("id", Value::U64(1)),
                    ("gender", Value::String("m".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Role",
                [
                    ("id", Value::U64(1)),
                    ("name", Value::String("actor".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Title",
                [("id", Value::U64(100)), ("year", Value::I64(2012))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Title",
                [("id", Value::U64(200)), ("year", Value::I64(2012))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(100)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
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

    assert!(output.result.facts.is_empty());
    Ok(())
}

#[test]
fn lftj_range_index_q16_like_count_proves_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(q16_like_join_schema(), env.max_key_size())?;
    env.write(|txn| {
        txn.insert(&schema, Fact::new("Alias", [("person", Value::U64(1))]))?;
        txn.insert(&schema, Fact::new("Person", [("id", Value::U64(1))]))?;
        txn.insert(
            &schema,
            Fact::new(
                "Cast",
                [("person", Value::U64(1)), ("work", Value::U64(200))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Company",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::String("[us]".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Keyword",
                [
                    ("id", Value::U64(1)),
                    ("word", Value::String("character-name-in-title".to_owned())),
                ],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(100)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkCompany",
                [("work", Value::U64(200)), ("company", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "WorkKeyword",
                [("work", Value::U64(200)), ("keyword", Value::U64(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Title",
                [("id", Value::U64(100)), ("episode", Value::I64(60))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Title",
                [("id", Value::U64(200)), ("episode", Value::I64(10))],
            ),
        )?;
        for id in 1_000..2_500 {
            txn.insert(
                &schema,
                Fact::new(
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
        query.find_count_domain(["work"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(output.result.facts, vec![vec![Value::U64(0)]]);
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
            .find_sum_over("amount", ["posting"])?
            .find_count_domain(["posting"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(
        output.result.facts,
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
            .find_count_domain(["account"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_same_facts(
        &output.result.facts,
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
            .find_sum_over("amount", ["posting"])?
            .find_count_domain(["posting"])?
            .find_min_over("t", ["posting"])?
            .find_max_over("t", ["posting"])?;
        Ok(())
    })?;

    let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_same_facts(
        &output.result.facts,
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
        txn.insert(&schema, number_fact(1, i64::MAX, i128::MAX))?;
        txn.insert(&schema, number_fact(2, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    let int_query = typed_query(&schema, |query| {
        query.rel("Number")?.var("n", "n")?.done();
        query.find_sum_over("n", ["n"])?;
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
        query.find_sum_over("d", ["d"])?;
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

    assert!(!output.result.facts.is_empty());
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
    assert!(explain.contains("timings:"));
    assert!(explain.contains("query_timing"));
    assert!(explain.contains("allocations:"));
    assert!(explain.contains("allocation_summary"));
    assert!(explain.contains("node_timing"));
    assert!(explain.contains("variable_estimate"));
    assert!(explain.contains("free_join_node"));
    assert!(explain.contains("candidate_plan"));
    assert!(explain.contains("free_join_estimates"));
    assert!(explain.contains("node_facts"));
    assert!(explain.contains("free_join_subatom"));
    assert!(!explain.contains("atoms:\n"));
    assert!(!explain.contains("index="));
    assert!(explain.contains("cursor_seeks"));
    assert!(explain.contains("facts_scanned"));
    assert!(explain.contains("bindings_yielded"));
    assert!(explain.contains("decoded_values"));
    assert!(explain.contains("encoded_comparisons_evaluated"));
    assert!(explain.contains("materialized_output_values"));
    assert!(explain.contains("trie_open"));
    assert!(explain.contains("trie_seek"));
    assert!(explain.contains("output_facts"));

    let diagnostics = env.storage_diagnostics(&schema)?;
    assert_eq!(diagnostics.storage_tx_id, 1);
    assert!(diagnostics.lmdb_map_size > 0);
    assert!(diagnostics.dictionary_entries > 0);
    assert!(
        diagnostics
            .relations
            .iter()
            .any(|relation| relation.relation == "Account" && relation.fact_count == 3)
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
    let reference = ReferenceDb::from_facts(seeded_facts());
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
                    .find_sum_over("amount", ["posting"])?
                    .find_count_domain(["posting"])?;
                Ok(())
            })?,
            InputBindings::from_values([
                ("start", Value::Timestamp(TimestampMicros(0))),
                ("end", Value::Timestamp(TimestampMicros(100))),
            ]),
        ),
    ];

    for (query, inputs) in cases {
        let lmdb_facts = env
            .read(|txn| txn.execute_query(&schema, &query, &inputs))?
            .result
            .facts;
        let reference_facts = reference.execute(&query, &inputs)?;
        assert_same_facts(&lmdb_facts, &reference_facts);
    }
    Ok(())
}

include!("query_test_helpers.rs");
