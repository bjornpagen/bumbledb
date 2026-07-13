use super::*;
use crate::encoding::{ValueRef, encode_fact, encode_u64};
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    StatementDescriptor, StatementId, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use crate::testutil::TempDir;

/// R(id fresh, amount i64) with a declared key on amount too:
/// statement 0 is the fresh auto-key (materialized first), statement 1
/// the declared `R(amount) -> R`.
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(1)]),
        }],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);

fn fact(schema: &Schema, id: u64, amount: i64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(id), ValueRef::I64(amount)],
        schema.relation(R).layout(),
        &mut b,
    );
    b
}

/// Committed fixture: facts (id, amount) = (0,10), (1,20), (2,30), then
/// (1,20) deleted — leaving a row-id hole at 1.
fn fixture(dir: &TempDir, schema: &Schema) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, amount) in [(0, 10), (1, 20), (2, 30)] {
        delta
            .insert(&view, R, &fact(schema, id, amount))
            .expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    delta
        .delete(&view, R, &fact(schema, 1, 20))
        .expect("delete");
    drop(view);
    commit(delta, &env).expect("commit");
    env
}

#[test]
fn membership_probe_hit_and_miss() {
    let dir = TempDir::new("read-membership");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    // Row ids are assigned in fact-hash order at commit (the delta's
    // deterministic iteration), so tests derive them rather than assume
    // insertion order.
    let row = fact_row(&txn, R, &fact(&schema, 0, 10)).expect("probe");
    assert!(row.is_some());
    // The deleted fact and a never-inserted fact both miss.
    assert_eq!(
        fact_row(&txn, R, &fact(&schema, 1, 20)).expect("probe"),
        None
    );
    assert_eq!(
        fact_row(&txn, R, &fact(&schema, 9, 90)).expect("probe"),
        None
    );
}

#[test]
fn guard_probe_hit_and_miss() {
    let dir = TempDir::new("read-guard");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let row = fact_row(&txn, R, &fact(&schema, 2, 30))
        .expect("probe")
        .expect("present");
    // The fresh auto-key (statement 0) on id and the declared key
    // (statement 1) on amount both resolve to the same row.
    assert_eq!(
        guard_row(&txn, R, StatementId(0), &encode_u64(2)).expect("probe"),
        Some(row)
    );
    assert_eq!(
        guard_row(&txn, R, StatementId(1), &crate::encoding::encode_i64(30)).expect("probe"),
        Some(row)
    );
    // The deleted fact's guard tuples are gone.
    assert_eq!(
        guard_row(&txn, R, StatementId(0), &encode_u64(1)).expect("probe"),
        None
    );
}

#[test]
fn fetch_round_trips_inserted_bytes() {
    let dir = TempDir::new("read-fetch");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let row = fact_row(&txn, R, &fact(&schema, 2, 30))
        .expect("probe")
        .expect("present");
    assert_eq!(
        fetch(&txn, &schema, R, row).expect("fetch"),
        fact(&schema, 2, 30)
    );
    // The deleted fact left a row-id hole: fetching it is corruption
    // (a row id reaching fetch must have come from M/U in-snapshot).
    let live: Vec<u64> = scan(&txn, &schema, R)
        .expect("scan")
        .map(|r| r.expect("ok").0)
        .collect();
    let hole = (0..3).find(|id| !live.contains(id)).expect("one hole");
    let err = fetch(&txn, &schema, R, hole).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MissingFact {
                relation: R,
                row_id
            }) if row_id == hole
        ),
        "{err:?}"
    );
}

#[test]
fn scan_yields_live_facts_in_row_id_order_skipping_holes() {
    let dir = TempDir::new("read-scan");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let rows: Vec<(u64, Vec<u8>)> = scan(&txn, &schema, R)
        .expect("scan")
        .map(|r| r.map(|(id, b)| (id, b.to_vec())))
        .collect::<Result<_>>()
        .expect("no corruption");
    // Exactly the two live facts, in strictly increasing row-id order,
    // with the deleted fact's row id absent.
    assert_eq!(rows.len(), 2);
    assert!(rows[0].0 < rows[1].0);
    let live_bytes: Vec<&[u8]> = rows.iter().map(|(_, b)| b.as_slice()).collect();
    assert!(live_bytes.contains(&fact(&schema, 0, 10).as_slice()));
    assert!(live_bytes.contains(&fact(&schema, 2, 30).as_slice()));
    for (row_id, bytes) in &rows {
        assert_eq!(
            fact_row(&txn, R, bytes).expect("probe"),
            Some(*row_id),
            "scan and membership agree"
        );
    }
}

#[test]
fn corrupted_fact_width_is_an_error_never_a_skip() {
    let dir = TempDir::new("read-corrupt");
    let schema = schema();
    let env = fixture(&dir, &schema);
    // Truncate the *last* live F value behind the schema's back.
    let victim = {
        let txn = env.read_txn().expect("txn");
        scan(&txn, &schema, R)
            .expect("scan")
            .map(|r| r.expect("ok").0)
            .max()
            .expect("nonempty")
    };
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let len = keys::fact_key(&mut key, R, victim);
        env.data()
            .put(wtxn.raw_mut(), &key[..len], &[0xAB, 0xCD])
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let txn = env.read_txn().expect("txn");
    let results: Vec<Result<(u64, &[u8])>> = scan(&txn, &schema, R).expect("scan").collect();
    assert!(results[0].is_ok()); // the first live row is intact
    let err = results[1].as_ref().unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::WrongFactWidth {
                relation: R,
                row_id,
                expected: 16,
                actual: 2
            }) if *row_id == victim
        ),
        "{err:?}"
    );
    // fetch reports the same corruption.
    assert!(fetch(&txn, &schema, R, victim).is_err());
}

/// A 5-byte F key — the bare prefix, the
/// audit's shape — is typed Corruption from `scan`, never a panic.
#[test]
fn a_short_f_key_is_typed_corruption_from_scan() {
    let dir = TempDir::new("read-corrupt-f-key");
    let schema = schema();
    let env = fixture(&dir, &schema);
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: keys::KeyBuf = [0; keys::MAX_KEY];
        let p_len = keys::fact_prefix(&mut key, R);
        assert_eq!(p_len, 5);
        env.data()
            .put(wtxn.raw_mut(), &key[..p_len], [0u8; 16].as_slice())
            .expect("plant");
        wtxn.commit().expect("commit");
    }
    let txn = env.read_txn().expect("txn");
    let err = scan(&txn, &schema, R)
        .expect("cursor opens")
        .find_map(Result::err)
        .expect("the corrupt key is a hard error");
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MalformedValue("F key length"))
        ),
        "{err:?}"
    );
}

#[test]
fn row_count_equals_scan_count_after_mixed_commits() {
    let dir = TempDir::new("read-row-count");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let scanned = scan(&txn, &schema, R).expect("scan").count() as u64;
    assert_eq!(row_count(&txn, R).expect("count"), scanned);
    assert_eq!(scanned, 2);
}
