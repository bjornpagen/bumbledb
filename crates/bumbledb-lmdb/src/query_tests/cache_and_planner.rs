use super::*;

#[test]
fn variable_order_is_stable() -> TestResult {
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

    assert_eq!(first.plan.variable_order, second.plan.variable_order);
    assert!(first.explain().contains("planner_stats"));
    assert!(first.explain().contains("free_join_plan"));
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
    assert!(second.plan.planner_stats.hits >= 1);
    assert!(second.plan.counters.lftj_lazy_access_slices >= 1);
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
    let schema_a = StorageSchema::new(variable_order_schema(), env.max_key_size())?;
    let schema_b = StorageSchema::new(triangle_schema(), env.max_key_size())?;
    let item_query = typed_query(&schema_a, |query| {
        query
            .rel("Item")?
            .var("id", "item")?
            .var("kind", "kind")?
            .done();
        query.find_var("item")?;
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
    assert_eq!(edge.plan.query_image_cache.cached_images, 1);
    Ok(())
}

#[test]
fn focused_query_image_scope_loads_fewer_fields_and_accesses() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(
        bumbledb_core::schema::SchemaDescriptor::new(
            "FocusedImageDb",
            vec![
                RelationDescriptor::new(
                    "Item",
                    vec![
                        FieldDescriptor::new("id", ValueType::U64),
                        FieldDescriptor::new("kind", ValueType::U64),
                        FieldDescriptor::new("owner", ValueType::U64),
                        FieldDescriptor::new("payload", ValueType::U64),
                    ],
                )
                .with_unique("id", ["id"])
                .with_index(IndexDescriptor::equality("by_kind", ["kind", "id"]))
                .with_index(IndexDescriptor::equality("by_owner", ["owner", "id"])),
            ],
        ),
        env.max_key_size(),
    )?;
    env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "Item",
                [
                    ("id", Value::U64(1)),
                    ("kind", Value::U64(7)),
                    ("owner", Value::U64(9)),
                    ("payload", Value::U64(11)),
                ],
            ),
        )?;
        Ok::<_, Error>(())
    })?;
    let query = typed_query(&schema, |query| {
        query
            .rel("Item")?
            .var("id", "id")?
            .input("kind", "kind")?
            .done();
        query.find_var("id")?;
        Ok(())
    })?;
    let normalized = env.read(|txn| normalize_query(txn, &schema, &query))?;
    let focused_scope = query_image_scope_for_query(&schema, &normalized);
    let focused = env.read(|txn| {
        crate::query_image::QueryImageBuilder::new(txn, &schema, focused_scope).build()
    })?;
    let full = env.query_image(&schema)?;
    let focused_item = focused
        .relation("Item")
        .ok_or_else(|| Error::internal("missing focused Item image"))?;
    let full_item = full
        .relation("Item")
        .ok_or_else(|| Error::internal("missing full Item image"))?;

    assert!(focused_item.fields.len() < full_item.fields.len());
    assert!(focused_item.indexes().len() < full_item.indexes().len());
    assert!(focused_item.field(FieldId(0)).is_some());
    assert!(focused_item.field(FieldId(3)).is_none());
    assert!(
        focused_item
            .encoded(crate::query_image::FactId(0), FieldId(0))
            .is_some()
    );
    assert!(
        focused_item
            .encoded(crate::query_image::FactId(0), FieldId(3))
            .is_none()
    );
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
