//! The canonical-scan build path: exact distinct counts, column words that
//! match a per-field decode of the same rows, dense positions, order-word
//! conventions and slab accounting.
use super::{R, default_rows, fact, schema, source_of};
use crate::encoding::encode_i64;
use crate::image::{LINE, SET_STRIDE};
use crate::ir::Value;

#[test]
fn distinct_counts_are_exact() {
    let schema = schema();
    let source = source_of(&schema, default_rows());
    let (_cache, image) = source.image_with_cache(R);

    assert_eq!(image.distinct_count(0), 10, "ids all distinct");
    assert_eq!(image.distinct_count(1), 2, "bools");
    assert_eq!(image.distinct_count(2), 2, "kind bools");
    assert_eq!(image.distinct_count(3), 10, "amounts all distinct");

    let mut rows = default_rows();
    for i in 10..110u64 {
        let amount = i64::try_from(i % 5).expect("small");
        rows.push(fact(i, true, false, amount));
    }
    let source = source_of(&schema, rows);
    let (_cache, image) = source.image_with_cache(R);
    assert_eq!(image.row_count(), 110);
    assert_eq!(image.distinct_count(0), 110);

    assert_eq!(image.distinct_count(3), 15);
}

#[test]
fn columns_equal_per_field_values_of_the_rows() {
    let schema = schema();
    let rows = default_rows();
    let source = source_of(&schema, rows.clone());
    let (_cache, image) = source.image_with_cache(R);
    assert_eq!(image.row_count(), 10);

    // Canonical rows sort by their full bytes; the image scan follows the
    // source order. Compare as sets of decoded per-row words.
    let mut expected: Vec<(u64, u64, u8, u8)> = rows
        .iter()
        .map(|row| {
            let (Value::U64(id), Value::Bool(flag), Value::Bool(kind), Value::I64(amount)) =
                (&row[0], &row[1], &row[2], &row[3])
            else {
                panic!("fixture shape");
            };
            (
                *id,
                u64::from_be_bytes(encode_i64(*amount)),
                u8::from(*flag),
                u8::from(*kind),
            )
        })
        .collect();
    expected.sort_unstable();
    let mut got: Vec<(u64, u64, u8, u8)> = (0..image.row_count())
        .map(|position| {
            (
                image.column_words(0)[position],
                image.column_words(3)[position],
                image.column_bytes(1)[position],
                image.column_bytes(2)[position],
            )
        })
        .collect();
    got.sort_unstable();
    assert_eq!(got, expected, "one word per column, exact conventions");
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
    let schema = schema();
    let source = source_of(&schema, vec![]);
    let (_cache, image) = source.image_with_cache(R);
    assert_eq!(image.row_count(), 0);
    assert!(image.column_words(0).is_empty());
    assert!(image.column_bytes(1).is_empty());
}

#[test]
fn byte_size_covers_rows_and_slab_slack() {
    let schema = schema();
    let source = source_of(&schema, default_rows());
    let (_cache, image) = source.image_with_cache(R);

    let payload = 10 * (2 * 8 + 2);
    assert!(image.byte_size() >= payload, "{}", image.byte_size());
    let slack = 4 * (SET_STRIDE + LINE);
    assert!(
        image.byte_size() <= payload + slack,
        "{}",
        image.byte_size()
    );
}

#[test]
fn text_columns_hold_interner_tokens_with_exact_identity() {
    // Two relations sharing text meet at one token; distinct texts never
    // alias (the interner is the one text→word authority).
    let schema =
        crate::schema::ValidateDescriptor::validate(bumbledb_theory::schema::SchemaDescriptor {
            relations: vec![bumbledb_theory::schema::RelationDescriptor {
                extension: None,
                name: "Doc".into(),
                fields: vec![
                    bumbledb_theory::schema::FieldDescriptor {
                        name: "name".into(),
                        value_type: bumbledb_theory::schema::ValueType::String,
                    },
                    bumbledb_theory::schema::FieldDescriptor {
                        name: "copy".into(),
                        value_type: bumbledb_theory::schema::ValueType::String,
                    },
                ],
            }],
            statements: vec![],
        })
        .expect("valid fixture");
    let rows = vec![
        vec![Value::String("alpha".into()), Value::String("alpha".into())],
        vec![Value::String("beta".into()), Value::String("alpha".into())],
    ];
    let source = crate::image::testsupport::TestSource::new(&schema, &[(super::R, rows)]);
    let (_cache, image) = source.image_with_cache(super::R);
    let names = image.column_words(0);
    let copies = image.column_words(1);
    // Row order is canonical-byte order; identify rows via token equality
    // structure rather than positions.
    let mut equal_pairs = 0;
    for position in 0..image.row_count() {
        if names[position] == copies[position] {
            equal_pairs += 1;
        }
    }
    assert_eq!(equal_pairs, 1, "only the (alpha, alpha) row self-matches");
    assert_eq!(
        names
            .iter()
            .chain(copies)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2,
        "two distinct texts, two tokens, shared across columns"
    );
}
