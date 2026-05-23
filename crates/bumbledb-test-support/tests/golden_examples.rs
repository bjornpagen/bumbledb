#![allow(clippy::result_large_err)]

use std::collections::BTreeSet;

use bumbledb_core::encoding::DecimalRaw;
use bumbledb_core::query_builder::{OperandRef, QueryBuilder};
use bumbledb_core::query_ir::ComparisonOperator;
use bumbledb_core::schema::SchemaDescriptor;
use bumbledb_lmdb::{DeleteOutcome, Environment, Fact, InputBindings, StorageSchema, Value};
use bumbledb_test_support::assertions::assert_same_facts;
use bumbledb_test_support::golden::{GOLDEN_FAMILIES, GoldenFamily};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "golden_examples/facts.rs"]
mod facts;
#[path = "golden_examples/schemas.rs"]
mod schemas;

use facts::*;
use schemas::*;

#[test]
fn golden_manifest_lists_all_required_families() {
    let names = GOLDEN_FAMILIES
        .iter()
        .map(|family| family.name())
        .collect::<BTreeSet<_>>();
    assert_eq!(GOLDEN_FAMILIES.len(), 7);
    assert!(names.contains(GoldenFamily::Ledger.name()));
    assert!(names.contains(GoldenFamily::Sailors.name()));
    assert!(names.contains(GoldenFamily::Joinstress.name()));
    assert!(names.contains(GoldenFamily::TpchSubset.name()));
    assert!(names.contains(GoldenFamily::ImdbJobSubset.name()));
    assert!(names.contains(GoldenFamily::LahmanSubset.name()));
    assert!(names.contains(GoldenFamily::LdbcSubset.name()));
}

#[test]
fn ledger_golden_preserves_set_projection_and_restrict() -> TestResult {
    let schema = bumbledb_lmdb::benchmark::benchmark_schema();
    let facts = bumbledb_lmdb::benchmark::benchmark_facts(2);
    let (env, storage) = load(schema, facts)?;

    let duplicate = env.write(|txn| {
        txn.insert(
            &storage,
            Fact::new(
                "Holder",
                [
                    ("id", Value::Serial(1)),
                    ("name", Value::String("holder-1".to_owned())),
                ],
            ),
        )
    })?;
    assert_eq!(duplicate, bumbledb_lmdb::InsertOutcome::AlreadyPresent);

    let absent = env.write(|txn| {
        txn.delete(
            &storage,
            Fact::new(
                "PostingTag",
                [("posting", Value::Serial(99)), ("tag", Value::Enum(1))],
            ),
        )
    })?;
    assert_eq!(absent, DeleteOutcome::Absent);

    let restricted = env.write(|txn| {
        txn.delete(
            &storage,
            Fact::new(
                "Holder",
                [
                    ("id", Value::Serial(1)),
                    ("name", Value::String("holder-1".to_owned())),
                ],
            ),
        )
    });
    assert!(restricted.is_err());

    let account_projection = QueryBuilder::new(storage.descriptor())
        .rel("Posting")?
        .var("account", "account")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .find_var("account")?
        .finish()?;
    assert_same_facts(
        execute(
            &env,
            &storage,
            &account_projection,
            inputs([("holder", Value::Serial(1))]),
        )?,
        vec![vec![Value::Serial(1)]],
    );

    let balances = QueryBuilder::new(storage.descriptor())
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("instrument", "instrument")?
        .var("amount", "amount")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .find_var("instrument")?
        .find_var("amount")?
        .finish()?;
    assert_same_facts(
        execute(
            &env,
            &storage,
            &balances,
            inputs([("holder", Value::Serial(1))]),
        )?,
        vec![
            vec![Value::Serial(1), Value::Decimal(DecimalRaw(100))],
            vec![Value::Serial(2), Value::Decimal(DecimalRaw(200))],
            vec![Value::Serial(3), Value::Decimal(DecimalRaw(300))],
        ],
    );
    Ok(())
}

#[test]
fn sailors_golden_preserves_duplicate_witness_projection_and_deletes() -> TestResult {
    let (env, schema) = load(
        sailors_schema(),
        vec![
            sailor(1, 9),
            sailor(2, 5),
            boat(1, 1),
            boat(2, 1),
            reserve(1, 1, 10),
            reserve(1, 2, 20),
            reserve(2, 1, 30),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Reserve")?
        .var("sailor", "sailor")?
        .var("boat", "boat")?
        .done()
        .rel("Boat")?
        .var("id", "boat")?
        .input("color", "color")?
        .done()
        .find_var("sailor")?
        .finish()?;
    assert_same_facts(
        execute(&env, &schema, &query, inputs([("color", Value::Enum(1))]))?,
        vec![vec![Value::Serial(1)], vec![Value::Serial(2)]],
    );
    assert_eq!(
        env.write(|txn| txn.delete(&schema, reserve(1, 2, 20)))?,
        DeleteOutcome::Deleted
    );
    assert_eq!(
        env.write(|txn| txn.delete(&schema, reserve(1, 2, 20)))?,
        DeleteOutcome::Absent
    );
    Ok(())
}

#[test]
fn joinstress_golden_preserves_triangle_projection() -> TestResult {
    let (env, schema) = load(
        triangle_schema(),
        vec![
            edge_ab(1, 10),
            edge_ab(1, 11),
            edge_ac(1, 20),
            edge_bc(10, 20),
            edge_bc(11, 20),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("EdgeAB")?
        .var("a", "a")?
        .var("b", "b")?
        .done()
        .rel("EdgeAC")?
        .var("a", "a")?
        .var("c", "c")?
        .done()
        .rel("EdgeBC")?
        .var("b", "b")?
        .var("c", "c")?
        .done()
        .find_var("a")?
        .find_var("b")?
        .find_var("c")?
        .finish()?;
    assert_same_facts(
        execute(&env, &schema, &query, InputBindings::new())?,
        vec![
            vec![Value::U64(1), Value::U64(10), Value::U64(20)],
            vec![Value::U64(1), Value::U64(11), Value::U64(20)],
        ],
    );
    Ok(())
}

#[test]
fn tpch_golden_preserves_lineitem_projection() -> TestResult {
    let (env, schema) = load(
        tpch_schema(),
        vec![
            customer(1, 1),
            orders(1, 1),
            lineitem(1, 1, 100),
            lineitem(2, 1, 100),
            supplier(1, 2),
            supplier(2, 3),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Customer")?
        .var("id", "customer")?
        .input("nation", "nation")?
        .done()
        .rel("Orders")?
        .var("id", "order")?
        .var("customer", "customer")?
        .done()
        .rel("LineItem")?
        .var("id", "line")?
        .var("order", "order")?
        .var("extended_price", "price")?
        .done()
        .find_var("customer")?
        .find_var("line")?
        .find_var("price")?
        .finish()?;
    assert_same_facts(
        execute(&env, &schema, &query, inputs([("nation", Value::U64(1))]))?,
        vec![
            vec![
                Value::Serial(1),
                Value::Serial(1),
                Value::Decimal(DecimalRaw(100)),
            ],
            vec![
                Value::Serial(1),
                Value::Serial(2),
                Value::Decimal(DecimalRaw(100)),
            ],
        ],
    );
    Ok(())
}

#[test]
fn imdb_job_golden_preserves_title_projection_and_empty_projection() -> TestResult {
    let (env, schema) = load(
        imdb_schema(),
        vec![
            title(1, 2020),
            name(1),
            name(2),
            principal(1, 1, 1, 1),
            principal(1, 2, 1, 2),
        ],
    )?;
    let titles = QueryBuilder::new(schema.descriptor())
        .rel("Title")?
        .var("id", "title")?
        .done()
        .rel("Principal")?
        .var("title", "title")?
        .var("name", "name")?
        .input("category", "category")?
        .done()
        .find_var("title")?
        .finish()?;
    assert_same_facts(
        execute(
            &env,
            &schema,
            &titles,
            inputs([("category", Value::Enum(1))]),
        )?,
        vec![vec![Value::Serial(1)]],
    );

    let empty = QueryBuilder::new(schema.descriptor())
        .rel("Title")?
        .var("id", "title")?
        .var("year", "year")?
        .done()
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gt,
            OperandRef::integer(3000),
        )?
        .find_var("title")?
        .finish()?;
    assert_same_facts(
        execute(&env, &schema, &empty, InputBindings::new())?,
        Vec::new(),
    );
    Ok(())
}

#[test]
fn lahman_golden_preserves_compound_year_join() -> TestResult {
    let (env, schema) = load(
        lahman_schema(),
        vec![
            player(1),
            team(1, 2000),
            batting(1, 1, 2000, 7),
            salary(1, 1, 2000, 50),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Salary")?
        .var("player", "player")?
        .var("team", "team")?
        .input("year", "year")?
        .var("salary", "salary")?
        .done()
        .rel("Batting")?
        .var("player", "player")?
        .var("team", "team")?
        .input("year", "year")?
        .var("hits", "hits")?
        .done()
        .find_var("player")?
        .find_var("salary")?
        .find_var("hits")?
        .finish()?;
    assert_same_facts(
        execute(&env, &schema, &query, inputs([("year", Value::I64(2000))]))?,
        vec![vec![Value::Serial(1), Value::I64(50), Value::I64(7)]],
    );
    Ok(())
}

#[test]
fn ldbc_golden_preserves_two_hop_projection_set() -> TestResult {
    let (env, schema) = load(
        ldbc_schema(),
        vec![
            person(1),
            person(2),
            person(3),
            person(4),
            knows(1, 2),
            knows(1, 3),
            knows(2, 4),
            knows(3, 4),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Knows")?
        .input("person1", "person")?
        .var("person2", "friend1")?
        .done()
        .rel("Knows")?
        .var("person1", "friend1")?
        .var("person2", "friend2")?
        .done()
        .find_var("friend2")?
        .finish()?;
    assert_same_facts(
        execute(
            &env,
            &schema,
            &query,
            inputs([("person", Value::Serial(1))]),
        )?,
        vec![vec![Value::Serial(4)]],
    );
    Ok(())
}

fn load(
    schema: SchemaDescriptor,
    facts: Vec<Fact>,
) -> Result<(Environment, StorageSchema), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let storage = StorageSchema::new(schema, env.max_key_size())?;
    env.bulk_load(&storage, facts)?;
    Ok((env, storage))
}

fn execute(
    env: &Environment,
    schema: &StorageSchema,
    query: &bumbledb_core::query_ir::TypedQuery,
    inputs: InputBindings,
) -> bumbledb_lmdb::Result<Vec<Vec<Value>>> {
    Ok(env
        .read(|txn| txn.execute_query(schema, query, &inputs))?
        .result
        .facts)
}

fn inputs(values: impl IntoIterator<Item = (&'static str, Value)>) -> InputBindings {
    InputBindings::from_values(values)
}
