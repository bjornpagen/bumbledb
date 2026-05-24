use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb_core::query_ir::{
    ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFieldBinding, TypedFindTerm,
    TypedInput, TypedLiteral, TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::execute_query_with_predicate_mode_for_test;
use crate::query::normalize::normalize_query;
use crate::query::predicate::PredicateMode;
use crate::{Environment, Error, Fact, InputBindings, Result, StorageSchema, Value};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn predicate_literal_equality_filter() -> Result<()> {
    let (env, schema) = env_and_schema("literal-eq")?;
    insert_rows(&env, &schema)?;
    let query = query(
        vars([("id", serial()), ("u", ValueType::U64)]),
        Vec::new(),
        &[0],
        vec![atom([
            field("id", 0, TypedTerm::Variable(0), serial()),
            field(
                "u",
                1,
                TypedTerm::Literal(int_lit(2, ValueType::U64)),
                ValueType::U64,
            ),
        ])],
        Vec::new(),
    );

    let (result, _) = env.read(|txn| {
        txn.execute_query(&schema, &query, &InputBindings::new())
            .map(|result| (result, ()))
    })?;

    assert_eq!(result.facts, vec![vec![Value::Serial(2)]]);
    Ok(())
}

#[test]
fn predicate_runtime_input_equality_filter() -> Result<()> {
    let (env, schema) = env_and_schema("input-eq")?;
    insert_rows(&env, &schema)?;
    let query = query(
        vars([("id", serial())]),
        vec![TypedInput {
            id: 0,
            name: "needle".to_owned(),
            value_type: ValueType::U64,
        }],
        &[0],
        vec![atom([
            field("id", 0, TypedTerm::Variable(0), serial()),
            field("u", 1, TypedTerm::Input(0), ValueType::U64),
        ])],
        Vec::new(),
    );
    let inputs = InputBindings::from_values([("needle", Value::U64(3))]);

    let result = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;

    assert_eq!(result.facts, vec![vec![Value::Serial(3)]]);
    Ok(())
}

#[test]
fn predicate_same_atom_repeated_variable_is_rejected() {
    let schema = schema();
    let query = query(
        vars([("x", ValueType::U64)]),
        Vec::new(),
        &[0],
        vec![atom([
            field("u", 1, TypedTerm::Variable(0), ValueType::U64),
            field("other", 5, TypedTerm::Variable(0), ValueType::U64),
        ])],
        Vec::new(),
    );

    assert!(normalize_query(&schema, &query).is_err());
}

#[test]
fn predicate_cross_atom_comparison_is_residual() -> Result<()> {
    let (env, schema) = env_and_schema("cross-residual")?;
    env.write(|txn| {
        for row in [row(1, 1, 0, "a", b"a", 0), row(2, 3, 0, "b", b"b", 0)] {
            txn.insert(&schema, row)?;
        }
        for fact in [
            Fact::new("Other", [("u", Value::U64(2))]),
            Fact::new("Other", [("u", Value::U64(4))]),
        ] {
            txn.insert(&schema, fact)?;
        }
        Ok::<(), Error>(())
    })?;
    let query = query(
        vars([("x", ValueType::U64), ("y", ValueType::U64)]),
        Vec::new(),
        &[0, 1],
        vec![
            atom([field("u", 1, TypedTerm::Variable(0), ValueType::U64)]),
            other_atom([field("u", 0, TypedTerm::Variable(1), ValueType::U64)]),
        ],
        vec![comparison(
            TypedOperand::Variable(0),
            ComparisonOperator::Lt,
            TypedOperand::Variable(1),
            ValueType::U64,
        )],
    );

    let result = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert_eq!(
        result.facts,
        vec![
            vec![Value::U64(1), Value::U64(2)],
            vec![Value::U64(1), Value::U64(4)],
            vec![Value::U64(3), Value::U64(4)]
        ]
    );
    Ok(())
}

#[test]
fn predicate_range_filters_over_u64_i64_and_serial() -> Result<()> {
    let (env, schema) = env_and_schema("ranges")?;
    insert_rows(&env, &schema)?;

    assert_eq!(
        range_query(
            &env,
            &schema,
            "u",
            ValueType::U64,
            ComparisonOperator::Lt,
            3
        )?,
        vec![Value::U64(1), Value::U64(2)]
    );
    assert_eq!(
        range_query(
            &env,
            &schema,
            "u",
            ValueType::U64,
            ComparisonOperator::Lte,
            2
        )?,
        vec![Value::U64(1), Value::U64(2)]
    );
    assert_eq!(
        range_query(
            &env,
            &schema,
            "i",
            ValueType::I64,
            ComparisonOperator::Gt,
            -2
        )?,
        vec![Value::I64(0), Value::I64(5)]
    );
    assert_eq!(
        range_query(
            &env,
            &schema,
            "i",
            ValueType::I64,
            ComparisonOperator::Gte,
            0
        )?,
        vec![Value::I64(0), Value::I64(5)]
    );
    assert_eq!(
        range_query(&env, &schema, "id", serial(), ComparisonOperator::Gt, 1)?,
        vec![Value::Serial(2), Value::Serial(3)]
    );
    Ok(())
}

#[test]
fn predicate_rejects_non_orderable_string_and_bytes_ranges() {
    let schema = schema();
    let string_query = query(
        vars([("s", ValueType::String)]),
        Vec::new(),
        &[0],
        vec![atom([field(
            "s",
            3,
            TypedTerm::Variable(0),
            ValueType::String,
        )])],
        vec![comparison(
            TypedOperand::Variable(0),
            ComparisonOperator::Gt,
            TypedOperand::Literal(str_lit("a")),
            ValueType::String,
        )],
    );
    let bytes_query = query(
        vars([("b", ValueType::Bytes)]),
        vec![TypedInput {
            id: 0,
            name: "b".to_owned(),
            value_type: ValueType::Bytes,
        }],
        &[0],
        vec![atom([field(
            "b",
            4,
            TypedTerm::Variable(0),
            ValueType::Bytes,
        )])],
        vec![comparison(
            TypedOperand::Variable(0),
            ComparisonOperator::Gt,
            TypedOperand::Input(0),
            ValueType::Bytes,
        )],
    );

    assert!(matches!(
        normalize_query(&schema, &string_query),
        Err(Error::InvalidQuery { .. })
    ));
    assert!(matches!(
        normalize_query(&schema, &bytes_query),
        Err(Error::InvalidQuery { .. })
    ));
}

#[test]
fn predicate_pushdown_and_residual_modes_are_equivalent() -> Result<()> {
    let (env, schema) = env_and_schema("pushdown-equivalence")?;
    insert_rows(&env, &schema)?;
    let query = comparison_query("u", ValueType::U64, ComparisonOperator::Gte, 2, &[0]);

    let (pushdown, _) = env.read(|txn| {
        execute_query_with_predicate_mode_for_test(
            txn,
            &schema,
            &query,
            &InputBindings::new(),
            PredicateMode::Pushdown,
        )
    })?;
    let (residual, _) = env.read(|txn| {
        execute_query_with_predicate_mode_for_test(
            txn,
            &schema,
            &query,
            &InputBindings::new(),
            PredicateMode::ResidualOnly,
        )
    })?;

    assert_eq!(pushdown, residual);
    Ok(())
}

#[test]
fn predicate_empty_result_after_pushed_selection() -> Result<()> {
    let (env, schema) = env_and_schema("empty-selection")?;
    insert_rows(&env, &schema)?;
    let query = query(
        vars([("id", serial())]),
        Vec::new(),
        &[0],
        vec![atom([
            field("id", 0, TypedTerm::Variable(0), serial()),
            field(
                "u",
                1,
                TypedTerm::Literal(int_lit(99, ValueType::U64)),
                ValueType::U64,
            ),
        ])],
        Vec::new(),
    );

    let result = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

    assert!(result.facts.is_empty());
    Ok(())
}

fn range_query(
    env: &Environment,
    schema: &StorageSchema,
    field: &str,
    value_type: ValueType,
    operator: ComparisonOperator,
    literal: i128,
) -> Result<Vec<Value>> {
    let query = comparison_query(field, value_type, operator, literal, &[0]);
    let result = env.read(|txn| txn.execute_query(schema, &query, &InputBindings::new()))?;
    Ok(result
        .facts
        .into_iter()
        .map(|mut fact| fact.remove(0))
        .collect())
}

fn comparison_query<const N: usize>(
    field_name: &str,
    value_type: ValueType,
    operator: ComparisonOperator,
    literal: i128,
    find: &[usize; N],
) -> TypedQuery {
    let field_id = field_id(field_name);
    query(
        vars([("x", value_type.clone())]),
        Vec::new(),
        find,
        vec![atom([field(
            field_name,
            field_id,
            TypedTerm::Variable(0),
            value_type.clone(),
        )])],
        vec![comparison(
            TypedOperand::Variable(0),
            operator,
            TypedOperand::Literal(int_lit(literal, value_type.clone())),
            value_type,
        )],
    )
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("bumbledb-prd15-{name}-{}-{id}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "Predicate",
        vec![
            RelationDescriptor::new(
                "R",
                vec![
                    FieldDescriptor::generated_serial("id", "Rid", "R"),
                    FieldDescriptor::new("u", ValueType::U64),
                    FieldDescriptor::new("i", ValueType::I64),
                    FieldDescriptor::new("s", ValueType::String),
                    FieldDescriptor::new("b", ValueType::Bytes),
                    FieldDescriptor::new("other", ValueType::U64),
                ],
            ),
            RelationDescriptor::new("Other", vec![FieldDescriptor::new("u", ValueType::U64)]),
        ],
    )
}

fn insert_rows(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.write(|txn| {
        for row in [
            row(1, 1, -5, "a", b"a", 1),
            row(2, 2, 0, "b", b"b", 2),
            row(3, 3, 5, "c", b"c", 3),
        ] {
            txn.insert(schema, row)?;
        }
        Ok::<(), Error>(())
    })
}

fn row(id: u64, u: u64, i: i64, s: &str, b: &[u8], other: u64) -> Fact {
    Fact::new(
        "R",
        [
            ("id", Value::Serial(id)),
            ("u", Value::U64(u)),
            ("i", Value::I64(i)),
            ("s", Value::String(s.to_owned())),
            ("b", Value::Bytes(b.to_vec())),
            ("other", Value::U64(other)),
        ],
    )
}

fn query<const N: usize>(
    variables: Vec<TypedVariable>,
    inputs: Vec<TypedInput>,
    find: &[usize; N],
    atoms: Vec<TypedRelationAtom>,
    comparisons: Vec<TypedComparison>,
) -> TypedQuery {
    TypedQuery {
        variables,
        inputs,
        find: find
            .iter()
            .copied()
            .map(|variable| TypedFindTerm::Variable { variable })
            .collect(),
        clauses: atoms
            .into_iter()
            .map(TypedClause::Relation)
            .chain(comparisons.into_iter().map(TypedClause::Comparison))
            .collect(),
    }
}

fn atom<const N: usize>(fields: [TypedFieldBinding; N]) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id: 0,
        relation: "R".to_owned(),
        fields: fields.into_iter().collect(),
    }
}

fn other_atom<const N: usize>(fields: [TypedFieldBinding; N]) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id: 1,
        relation: "Other".to_owned(),
        fields: fields.into_iter().collect(),
    }
}

fn field(name: &str, field_id: usize, term: TypedTerm, value_type: ValueType) -> TypedFieldBinding {
    TypedFieldBinding {
        field_id,
        field: name.to_owned(),
        value_type,
        term,
    }
}

fn comparison(
    left: TypedOperand,
    operator: ComparisonOperator,
    right: TypedOperand,
    value_type: ValueType,
) -> TypedComparison {
    TypedComparison {
        left,
        operator,
        right,
        value_type,
    }
}

fn vars<const N: usize>(vars: [(&str, ValueType); N]) -> Vec<TypedVariable> {
    vars.into_iter()
        .enumerate()
        .map(|(id, (name, value_type))| TypedVariable {
            id,
            name: name.to_owned(),
            value_type,
        })
        .collect()
}

fn int_lit(value: i128, value_type: ValueType) -> TypedLiteral {
    TypedLiteral {
        literal: Literal::Integer(value),
        value_type,
    }
}

fn str_lit(value: &str) -> TypedLiteral {
    TypedLiteral {
        literal: Literal::String(value.to_owned()),
        value_type: ValueType::String,
    }
}

fn serial() -> ValueType {
    ValueType::Serial {
        type_name: "Rid".to_owned(),
        owning_relation: "R".to_owned(),
    }
}

fn field_id(name: &str) -> usize {
    match name {
        "id" => 0,
        "u" => 1,
        "i" => 2,
        "s" => 3,
        "b" => 4,
        "other" => 5,
        _ => 0,
    }
}
