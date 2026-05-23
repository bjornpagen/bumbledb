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

