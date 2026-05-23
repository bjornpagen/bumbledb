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

