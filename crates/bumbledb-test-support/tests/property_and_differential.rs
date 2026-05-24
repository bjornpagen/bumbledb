#![allow(clippy::result_large_err)]

use bumbledb_core::query_ir::{ComparisonOperator, TypedOperand};
use bumbledb_lmdb::{InputBindings, Value};
use bumbledb_test_support::*;

#[test]
fn reference_evaluator_matches_lmdb_for_small_positive_queries() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("reference")?;
    let r = [(1, 2), (1, 3), (2, 4)];
    let s = [(2, 9), (3, 9), (4, 8)];
    insert(
        &env,
        &schema,
        r.into_iter()
            .map(|(l, r)| pair("R", l, r))
            .chain(s.into_iter().map(|(l, r)| pair("S", l, r))),
    )?;
    let query = binary_join_query("R", "S", &[0, 2]);

    let lmdb = execute(&env, &schema, &query)?.facts;
    let reference = distinct(vec![vec![Value::U64(2), Value::U64(9)]]);

    assert_eq!(lmdb, reference);
    Ok(())
}

#[test]
fn differential_covers_inputs_literals_ranges_self_joins_and_empty_sets()
-> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("differential")?;
    insert(
        &env,
        &schema,
        [
            pair("R", 1, 2),
            pair("R", 2, 3),
            pair("R", 3, 4),
            pair("Edge", 1, 2),
            pair("Edge", 2, 3),
        ],
    )?;

    let literal_query = typed_query(
        &["x", "y"],
        &[0],
        vec![pair_atom(0, "R", [(0, "left", 0), (1, "right", 1)])],
        vec![comparison(1, ComparisonOperator::Gt, int_lit(2))],
        Vec::new(),
    );
    assert_eq!(
        execute(&env, &schema, &literal_query)?.facts,
        rows([[2], [3]])
    );

    let input_query = typed_query(
        &["x", "y"],
        &[0],
        vec![pair_atom(0, "R", [(0, "left", 0), (1, "right", 1)])],
        vec![comparison(
            1,
            ComparisonOperator::Gte,
            TypedOperand::Input(0),
        )],
        vec![bumbledb_core::query_ir::TypedInput {
            id: 0,
            name: "min".to_owned(),
            value_type: bumbledb_core::schema::ValueType::U64,
        }],
    );
    let inputs = InputBindings::from_values([("min", Value::U64(3))]);
    assert_eq!(
        execute_inputs(&env, &schema, &input_query, &inputs)?.facts,
        rows([[2], [3]])
    );

    let self_join = typed_query(
        &["x", "y", "z"],
        &[0, 2],
        vec![
            pair_atom(3, "Edge", [(0, "left", 0), (1, "right", 1)]),
            pair_atom(3, "Edge", [(0, "left", 1), (1, "right", 2)]),
        ],
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(execute(&env, &schema, &self_join)?.facts, rows([[1, 3]]));
    Ok(())
}

#[test]
fn storage_sequence_reopen_and_snapshot_regressions() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("storage")?;
    env.write(|txn| {
        txn.insert(&schema, mixed(1, true, 7, -1, 1, "a", b"a"))?;
        txn.insert(&schema, mixed(1, true, 7, -1, 1, "a", b"a"))?;
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    assert_eq!(
        env.read(|txn| txn.relation_fact_count(&schema, "Mixed"))?,
        1
    );
    env.read(|read| {
        assert_eq!(read.relation_fact_count(&schema, "Mixed")?, 1);
        env.write(|write| write.insert(&schema, mixed(2, false, 8, 1, 2, "b", b"b")))?;
        assert_eq!(read.relation_fact_count(&schema, "Mixed")?, 1);
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    assert_eq!(
        env.read(|txn| txn.relation_fact_count(&schema, "Mixed"))?,
        2
    );
    Ok(())
}

#[test]
fn malformed_query_is_rejected_at_execution_boundary() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("malformed")?;
    let mut query = binary_join_query("R", "S", &[0]);
    query.variables[0].id = 99;

    let result = execute(&env, &schema, &query);

    assert!(result.is_err());
    Ok(())
}
