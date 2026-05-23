fn assert_invalid_typed_query(schema: &StorageSchema, query: &TypedQuery) -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let result = env.read(|txn| txn.execute_query(schema, query, &InputBindings::new()));
    assert!(matches!(
        result,
        Err(Error::Query(QueryError::Execute(
            ExecuteError::InvalidQuery { .. }
        )))
    ));
    Ok(())
}

fn simple_chain_projection(schema: &StorageSchema) -> QueryBuildResult<TypedQuery> {
    typed_query(schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.find_var("a")?;
        Ok(())
    })
}

#[test]
fn execution_rejects_public_ir_bad_projection_variable() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let mut query = simple_chain_projection(&schema)?;
    query.find[0] = TypedFindTerm::Variable { variable: 99 };

    assert_invalid_typed_query(&schema, &query)
}

#[test]
fn execution_rejects_public_ir_bad_relation_variable() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let mut query = simple_chain_projection(&schema)?;
    let TypedClause::Relation(atom) = &mut query.clauses[0] else {
        return Err("expected relation atom".into());
    };
    atom.fields[0].term = TypedTerm::Variable(99);

    assert_invalid_typed_query(&schema, &query)
}

#[test]
fn execution_rejects_public_ir_bad_input_id() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let mut query = typed_query(&schema, |query| {
        query.rel("B")?.var("id", "b")?.input("a", "a")?.done();
        query.find_var("b")?;
        Ok(())
    })?;
    let TypedClause::Relation(atom) = &mut query.clauses[0] else {
        return Err("expected relation atom".into());
    };
    atom.fields[0].term = TypedTerm::Input(99);

    assert_invalid_typed_query(&schema, &query)
}

#[test]
fn execution_rejects_public_ir_relation_id_name_mismatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let mut query = simple_chain_projection(&schema)?;
    let TypedClause::Relation(atom) = &mut query.clauses[0] else {
        return Err("expected relation atom".into());
    };
    atom.relation = "B".to_owned();

    assert_invalid_typed_query(&schema, &query)
}

#[test]
fn execution_rejects_public_ir_field_id_name_mismatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let mut query = simple_chain_projection(&schema)?;
    let TypedClause::Relation(atom) = &mut query.clauses[0] else {
        return Err("expected relation atom".into());
    };
    atom.fields[0].field = "not_id".to_owned();

    assert_invalid_typed_query(&schema, &query)
}

#[test]
fn execution_rejects_public_ir_comparison_type_mismatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
    let mut query = typed_query(&schema, |query| {
        query.rel("A")?.var("id", "a")?.done();
        query.cmp(
            OperandRef::var("a"),
            ComparisonOperator::Eq,
            OperandRef::integer(1),
        )?;
        query.find_var("a")?;
        Ok(())
    })?;
    let TypedClause::Comparison(comparison) = &mut query.clauses[1] else {
        return Err("expected comparison".into());
    };
    comparison.value_type = ValueType::String;

    assert_invalid_typed_query(&schema, &query)
}
