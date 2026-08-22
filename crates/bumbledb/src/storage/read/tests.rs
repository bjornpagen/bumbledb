use super::*;
use crate::encoding::{ValueRef, encode_fact, encode_u64};
use crate::error::{CorruptionError, Error, Mismatch, Result};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    StatementDescriptor, StatementId, ValueType,
};

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
    commit(delta, &env).expect("commit").expect("admitted");

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    delta
        .delete(&view, R, &fact(schema, 1, 20))
        .expect("delete");
    drop(view);
    commit(delta, &env).expect("commit").expect("admitted");
    env
}

#[test]
fn membership_probe_hit_and_miss() {
    let dir = TempDir::new("read-membership");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");

    let row = fact_row(&txn, R, &fact(&schema, 0, 10)).expect("probe");
    assert!(row.is_some());

    assert_eq!(
        fact_row(&txn, R, &fact(&schema, 1, 20)).expect("probe"),
        None
    );
    assert_eq!(
        fact_row(&txn, R, &fact(&schema, 9, 90)).expect("probe"),
        None
    );
}

fn probe(
    txn: &crate::storage::env::ReadTxn<'_>,
    statement: StatementId,
    determinant: &[u8],
) -> Option<u64> {
    let mut key = Vec::new();
    begin_determinant_key(&mut key, R, statement);
    key.extend_from_slice(determinant);
    determinant_row_for_key(txn, &key).expect("probe")
}

#[test]
fn key_probe_hit_and_miss() {
    let dir = TempDir::new("read-determinant");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let row = fact_row(&txn, R, &fact(&schema, 2, 30))
        .expect("probe")
        .expect("present");

    assert_eq!(probe(&txn, StatementId(0), &encode_u64(2)), None);
    assert_eq!(row, 2, "the fresh field's value IS the F row id");
    assert_eq!(
        fact_at(&txn, &schema, R, 2)
            .expect("probe")
            .map(crate::encoding::FactView::bytes),
        Some(&fact(&schema, 2, 30)[..])
    );
    assert_eq!(fact_at(&txn, &schema, R, 1).expect("probe"), None);

    assert_eq!(
        probe(&txn, StatementId(1), &crate::encoding::encode_i64(30)),
        Some(row)
    );

    assert_eq!(
        probe(&txn, StatementId(1), &crate::encoding::encode_i64(20)),
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
        fetch(&txn, &schema, R, row).expect("fetch").bytes(),
        fact(&schema, 2, 30).as_slice()
    );

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
        .map(|r| r.map(|(id, b)| (id, b.bytes().to_vec())))
        .collect::<Result<_>>()
        .expect("no corruption");

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
        let key = keys::fact_key(R, victim);
        env.data()
            .put(wtxn.raw_mut(), &key, &[0xAB, 0xCD])
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let txn = env.read_txn().expect("txn");
    let results: Vec<Result<(u64, crate::encoding::FactView<'_, '_>)>> =
        scan(&txn, &schema, R).expect("scan").collect();
    assert!(results[0].is_ok());
    let err = results[1].as_ref().unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::WrongFactWidth {
                relation: R,
                row_id,
                mismatch: Mismatch {
                    witnessed: 2,
                    required: 16,
                },
            }) if *row_id == victim
        ),
        "{err:?}"
    );

    assert!(fetch(&txn, &schema, R, victim).is_err());
}

#[test]
fn a_short_f_key_is_typed_corruption_from_scan() {
    let dir = TempDir::new("read-corrupt-f-key");
    let schema = schema();
    let env = fixture(&dir, &schema);
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: keys::KeyBuf = [0; keys::MAX_KEY];
        let prefix = keys::fact_prefix(&mut key, R);
        assert_eq!(prefix.len(), 5);
        env.data()
            .put(wtxn.raw_mut(), prefix, [0u8; 16].as_slice())
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
fn scan_from_zero_yields_exactly_scan_over_live_facts() {
    let dir = TempDir::new("read-scan-from-zero");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let via_scan: Vec<(u64, Vec<u8>)> = scan(&txn, &schema, R)
        .expect("scan")
        .map(|r| r.map(|(id, b)| (id, b.bytes().to_vec())))
        .collect::<Result<_>>()
        .expect("no corruption");
    let via_scan_from: Vec<(u64, Vec<u8>)> = scan_from(&txn, &schema, R, 0)
        .expect("scan_from")
        .map(|r| r.map(|(id, b)| (id, b.bytes().to_vec())))
        .collect::<Result<_>>()
        .expect("no corruption");
    assert_eq!(
        via_scan, via_scan_from,
        "scan_from(rel, 0) is scan's tail-from-zero"
    );

    let cut = via_scan[1].0;
    let tail: Vec<(u64, Vec<u8>)> = scan_from(&txn, &schema, R, cut)
        .expect("scan_from tail")
        .map(|r| r.map(|(id, b)| (id, b.bytes().to_vec())))
        .collect::<Result<_>>()
        .expect("no corruption");
    assert_eq!(tail, via_scan[1..].to_vec());
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

#[test]
fn composed_determinant_key_matches_the_codec() {
    let determinant = encode_u64(0xAABB_CCDD_EE01_0203);
    let mut composed = Vec::new();
    begin_determinant_key(&mut composed, RelationId(7), StatementId(3));
    assert_eq!(composed.len(), DETERMINANT_KEY_HEADER);
    composed.extend_from_slice(&determinant);
    let codec =
        keys::key(|b| keys::determinant_key(b, RelationId(7), StatementId(3), &determinant));
    assert_eq!(composed, codec);
}

#[test]
fn fact_for_key_chains_the_fetch_and_misses_honestly() {
    let dir = TempDir::new("read-composed-key");
    let schema = schema();
    let env = fixture(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let mut key = Vec::new();
    begin_determinant_key(&mut key, R, StatementId(1));
    key.extend_from_slice(&crate::encoding::encode_i64(30));
    let hit = fact_for_key(&txn, &schema, R, &key)
        .expect("probe")
        .expect("present");
    assert_eq!(hit.bytes(), &fact(&schema, 2, 30)[..]);
    key.truncate(DETERMINANT_KEY_HEADER);
    key.extend_from_slice(&crate::encoding::encode_i64(20));
    assert_eq!(fact_for_key(&txn, &schema, R, &key).expect("probe"), None);
}
