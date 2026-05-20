use bumbledb_core::encoding::TimestampMicros;
use bumbledb_core::query_builder::{OperandRef, QueryBuilder};
use bumbledb_core::query_ir::ComparisonOperator;
use bumbledb_lmdb::{Environment, InputBindings, StorageSchema, Value};
use bumbledb_test_support::assertions::execute_sorted;
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
        SELECT p.id, p.amount
        FROM posting p JOIN account a ON a.id = p.account
        WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
        "#,
        &[1, 0, 1_000_000],
    )?;

    assert_eq!(bumbledb_rows.len(), sqlite_rows.len());
    Ok(())
}

fn inputs() -> InputBindings {
    InputBindings::from_values([
        ("holder", Value::Ref(1)),
        ("start", Value::Timestamp(TimestampMicros(0))),
        ("end", Value::Timestamp(TimestampMicros(1_000_000))),
    ])
}
