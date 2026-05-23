#![allow(clippy::result_large_err)]

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuilder};
use bumbledb_core::query_ir::ComparisonOperator;
use bumbledb_lmdb::{Environment, InputBindings, StorageSchema, Value};
use bumbledb_test_support::assertions::{assert_same_facts, execute_sorted_facts};
use bumbledb_test_support::facts::seeded_ledger_facts;
use bumbledb_test_support::schemas::ledger_schema;
use bumbledb_test_support::sqlite::{load_ledger, query_i64_facts};

#[test]
fn sqlite_comparison_queries_match_bumbledb() -> Result<(), Box<dyn std::error::Error>> {
    let facts = seeded_ledger_facts();
    let sqlite = load_ledger(&facts)?;
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.bulk_load(&schema, facts)?;

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
    let bumbledb_facts = execute_sorted_facts(&env, &schema, &query, &inputs())?;
    let sqlite_facts = query_i64_facts(
        &sqlite,
        r#"
        SELECT DISTINCT p.id, p.amount
        FROM posting p JOIN account a ON a.id = p.account
        WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
        "#,
        &[1, 0, 1_000_000],
    )?;
    let sqlite_facts = sqlite_facts
        .into_iter()
        .map(|fact| {
            vec![
                Value::Serial(fact[0] as u64),
                Value::Decimal(DecimalRaw(fact[1] as i128)),
            ]
        })
        .collect();

    assert_same_facts(bumbledb_facts, sqlite_facts);
    Ok(())
}

fn inputs() -> InputBindings {
    InputBindings::from_values([
        ("holder", Value::Serial(1)),
        ("start", Value::Timestamp(TimestampMicros(0))),
        ("end", Value::Timestamp(TimestampMicros(1_000_000))),
    ])
}
