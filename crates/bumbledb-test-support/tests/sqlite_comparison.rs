#![allow(clippy::result_large_err)]

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuilder};
use bumbledb_core::query_ir::ComparisonOperator;
use bumbledb_lmdb::{Environment, InputBindings, StorageSchema, Value};
use bumbledb_test_support::assertions::{assert_same_rows, execute_sorted};
use bumbledb_test_support::rows::seeded_ledger_rows;
use bumbledb_test_support::schemas::ledger_schema;
use bumbledb_test_support::sqlite::{load_ledger, query_i64_rows};

#[test]
fn sqlite_comparison_queries_match_bumbledb() -> Result<(), Box<dyn std::error::Error>> {
    let rows = seeded_ledger_rows();
    let sqlite = load_ledger(&rows)?;
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.bulk_load(&schema, rows)?;

    let query = QueryBuilder::new(schema.descriptor())
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("amount", "amount")?
        .var("at", "t")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("posting")?
        .find_var("amount")?
        .finish()?;
    let bumbledb_rows = execute_sorted(&env, &schema, &query, &inputs())?;
    let sqlite_rows = query_i64_rows(
        &sqlite,
        r#"
        SELECT DISTINCT p.id, p.amount
        FROM posting p JOIN account a ON a.id = p.account
        WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
        "#,
        &[1, 0, 1_000_000],
    )?;
    let sqlite_rows = sqlite_rows
        .into_iter()
        .map(|row| {
            vec![
                Value::Serial(row[0] as u64),
                Value::Decimal(DecimalRaw(row[1] as i128)),
            ]
        })
        .collect();

    assert_same_rows(bumbledb_rows, sqlite_rows);
    Ok(())
}

fn inputs() -> InputBindings {
    InputBindings::from_values([
        ("holder", Value::Serial(1)),
        ("start", Value::Timestamp(TimestampMicros(0))),
        ("end", Value::Timestamp(TimestampMicros(1_000_000))),
    ])
}
