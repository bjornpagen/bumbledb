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

    let output = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;

    assert_eq!(output.result.cardinality(), output.result.facts.len());
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
        query.find_var("a")?;
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
