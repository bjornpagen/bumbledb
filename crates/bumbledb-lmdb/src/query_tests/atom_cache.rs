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

