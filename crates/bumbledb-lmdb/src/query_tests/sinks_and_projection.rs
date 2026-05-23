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

