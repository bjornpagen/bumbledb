use super::{R, fact, populated, schema};
use crate::encoding::encode_i64;
use crate::image::{LINE, SET_STRIDE, build};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::read;
use crate::testutil::TempDir;

/// Build-time distinct counts are exact per column type
/// (docs/architecture/40-execution.md): fresh ids all-distinct, bools 2, and a
/// skewed i64 column counted through the word set.
#[test]
fn distinct_counts_are_exact() {
    let dir = TempDir::new("image-distincts");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    // populated(): ids 0..10, flag i % 2 == 0, kind i % 3 == 0, amount i*7-30.
    assert_eq!(image.distinct(0), 10, "fresh ids all distinct");
    assert_eq!(image.distinct(1), 2, "bools");
    assert_eq!(image.distinct(2), 2, "kind bools");
    assert_eq!(image.distinct(3), 10, "amounts all distinct");

    // A skewed refresh: 100 more rows sharing 5 amounts.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for i in 10..110u64 {
        let amount = i64::try_from(i % 5).expect("small");
        delta
            .insert(&view, R, &fact(&schema, i, true, false, amount))
            .expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    assert_eq!(image.row_count(), 110);
    assert_eq!(image.distinct(0), 110);
    // Old 10 distinct amounts + {0..5}, minus the overlaps: the old
    // amounts are 7i - 30 (…-30, -23, …, 33); {0..5} intersects at
    // nothing except… 7i-30 ∈ {0,1,2,3,4} ⇔ i has no integer
    // solution except none (7i = 30..34 has none). 10 + 5 = 15.
    assert_eq!(image.distinct(3), 15);
}

#[test]
fn columns_equal_per_field_decode_of_the_scan() {
    let dir = TempDir::new("image-columns");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    assert_eq!(image.row_count(), 10);

    let layout = schema.relation(R).layout();
    for (position, entry) in read::scan(&txn, &schema, R).expect("scan").enumerate() {
        let (_, fact_bytes) = entry.expect("ok");
        // 8-byte columns hold the byte-order-normalized word.
        let id_word = u64::from_be_bytes(fact_bytes[..8].try_into().expect("8"));
        assert_eq!(image.column_words(0)[position], id_word);
        let amount_off = layout.field_offset(3);
        let amount_word = u64::from_be_bytes(
            fact_bytes[amount_off..amount_off + 8]
                .try_into()
                .expect("8"),
        );
        assert_eq!(image.column_words(3)[position], amount_word);
        // 1-byte columns hold the validated byte.
        assert_eq!(image.column_bytes(1)[position], fact_bytes[8]);
        assert_eq!(image.column_bytes(2)[position], fact_bytes[9]);
    }
}

#[test]
fn positions_stay_dense_under_row_id_holes() {
    let dir = TempDir::new("image-holes");
    let schema = schema();
    let env = populated(&dir, &schema);
    // Delete three facts, punching row-id holes.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for i in [2u64, 5, 7] {
        let amount = i64::try_from(i).expect("small") * 7 - 30;
        delta
            .delete(&view, R, &fact(&schema, i, i % 2 == 0, i % 3 == 0, amount))
            .expect("delete");
    }
    drop(view);
    commit(delta, &env).expect("commit");

    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    assert_eq!(image.row_count(), 7);
    // Every position 0..7 is filled, in scan order.
    let scanned: Vec<u64> = read::scan(&txn, &schema, R)
        .expect("scan")
        .map(|e| {
            let (_, bytes) = e.expect("ok");
            u64::from_be_bytes(bytes[..8].try_into().expect("8"))
        })
        .collect();
    assert_eq!(image.column_words(0), &scanned[..]);
}

#[test]
fn i64_word_order_matches_logical_order() {
    let samples = [
        i64::MIN,
        i64::MIN + 1,
        -1_000_000,
        -1,
        0,
        1,
        42,
        1_000_000,
        i64::MAX - 1,
        i64::MAX,
    ];
    let words: Vec<u64> = samples
        .iter()
        .map(|v| u64::from_be_bytes(encode_i64(*v)))
        .collect();
    for pair in words.windows(2) {
        assert!(pair[0] < pair[1], "u64 word compare must match i64 order");
    }
}

#[test]
fn zero_row_relation_builds_an_empty_image() {
    let dir = TempDir::new("image-empty");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    assert_eq!(image.row_count(), 0);
    assert!(image.column_words(0).is_empty());
    assert!(image.column_bytes(1).is_empty());
}

#[test]
fn byte_size_covers_rows_and_slab_slack() {
    let dir = TempDir::new("image-byte-size");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    // The fixture: 10 rows over 2 word columns (id, amount) and 2 byte
    // columns (flag, kind). Lower bound: the raw payload; upper bound:
    // payload plus per-column alignment/stagger slack.
    let payload = 10 * (2 * 8 + 2);
    assert!(image.byte_size() >= payload, "{}", image.byte_size());
    let slack = 4 * (SET_STRIDE + LINE);
    assert!(
        image.byte_size() <= payload + slack,
        "{}",
        image.byte_size()
    );
}
