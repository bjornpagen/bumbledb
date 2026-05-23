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
