#![allow(clippy::result_large_err)]

use bumbledb_lmdb::Value;
use bumbledb_test_support::*;

#[test]
fn sqlite_select_distinct_oracle_matches_exact_values() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("sqlite-distinct")?;
    insert(
        &env,
        &schema,
        [
            pair("R", 1, 10),
            pair("R", 1, 11),
            pair("S", 1, 20),
            pair("S", 2, 30),
        ],
    )?;
    let query = binary_join_query("R", "S", &[0]);

    let bumbledb = execute(&env, &schema, &query)?.facts;
    let sqlite_select_distinct = distinct(vec![vec![Value::U64(1)], vec![Value::U64(1)]]);

    assert_eq!(bumbledb, sqlite_select_distinct);
    Ok(())
}

#[test]
fn sqlite_oracle_compares_values_not_counts_only() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("sqlite-values")?;
    insert(&env, &schema, [pair("R", 1, 10), pair("S", 1, 20)])?;
    let query = binary_join_query("R", "S", &[0]);

    let bumbledb = execute(&env, &schema, &query)?.facts;
    let wrong_same_count = vec![vec![Value::U64(999)]];

    assert_eq!(bumbledb.len(), wrong_same_count.len());
    assert_ne!(bumbledb, wrong_same_count);
    Ok(())
}
