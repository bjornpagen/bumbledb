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
    assert!(explain.contains("free_join_node"));
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
