#![allow(clippy::result_large_err)]

use bumbledb_test_support::*;

#[test]
fn clover_paper_fixture() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("clover")?;
    insert(
        &env,
        &schema,
        [
            pair("R", 0, 10),
            pair("R", 1, 11),
            pair("R", 2, 12),
            pair("S", 0, 20),
            pair("S", 2, 21),
            pair("S", 3, 22),
            pair("T", 0, 30),
            pair("T", 3, 31),
            pair("T", 1, 32),
        ],
    )?;

    let result = execute(&env, &schema, &clover_query(&[0, 1, 2, 3]))?;

    assert_eq!(result.facts, rows([[0, 10, 20, 30]]));
    Ok(())
}

#[test]
fn triangle_chain_star_self_join_and_empty_fixtures() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("goldens")?;
    insert(
        &env,
        &schema,
        [
            pair("R", 1, 2),
            pair("R", 1, 3),
            pair("R", 2, 4),
            pair("S", 2, 4),
            pair("S", 3, 4),
            pair("S", 4, 5),
            pair("T", 4, 1),
            pair("T", 2, 6),
            pair("T", 5, 9),
            pair("Edge", 2, 3),
        ],
    )?;

    assert_eq!(
        execute(&env, &schema, &triangle_query(&[0, 1, 2]))?.facts,
        rows([[1, 2, 4], [1, 3, 4]])
    );
    assert_eq!(
        execute(&env, &schema, &binary_join_query("R", "S", &[0, 2]))?.facts,
        rows([[2, 4]])
    );
    assert_eq!(
        execute(&env, &schema, &clover_query(&[0]))?.facts,
        rows([[2]])
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
    assert!(execute(&env, &schema, &self_join)?.facts.is_empty());

    let no_match = typed_query(
        &["x", "a", "b"],
        &[0],
        vec![
            pair_atom(3, "Edge", [(1, "right", 0), (0, "left", 1)]),
            pair_atom(2, "T", [(0, "left", 0), (1, "right", 2)]),
        ],
        Vec::new(),
        Vec::new(),
    );
    assert!(execute(&env, &schema, &no_match)?.facts.is_empty());
    Ok(())
}

#[test]
fn duplicate_witness_projection_and_no_useful_index_fixture() -> bumbledb_lmdb::Result<()> {
    let (env, schema) = env_and_schema("duplicates")?;
    insert(
        &env,
        &schema,
        [
            pair("R", 1, 10),
            pair("R", 1, 11),
            pair("S", 1, 20),
            pair("S", 1, 21),
        ],
    )?;

    let result = execute(&env, &schema, &binary_join_query("R", "S", &[0]))?;

    assert_eq!(result.facts, rows([[1]]));
    assert_eq!(result.cardinality(), 1);
    Ok(())
}
