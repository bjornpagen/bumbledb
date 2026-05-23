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

