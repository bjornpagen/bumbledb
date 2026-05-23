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

