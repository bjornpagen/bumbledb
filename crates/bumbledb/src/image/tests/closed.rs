//! Closed-relation synthesis: the image comes from the sealed extension,
//! not from a scan — the fingerprint's preimage IS the storage
//! (`docs/architecture/50-storage.md` § virtual relations). No environment
//! exists anywhere in this module: synthesis is pure.

use crate::encoding::{encode_bool, encode_interval_u64, encode_u64};
use crate::image::{synthesize_closed, ColumnWidth};
use crate::ir::Value;
use crate::schema::{IntervalElement, Row};

use super::*;

/// The three-tier theory (the PRD-02 grammar, hand-sealed): an ordinary
/// relation, a columnless vocabulary (synthetic id only), and a closed
/// relation with intrinsic columns of every span shape — word (u64),
/// word-pair (interval), byte (bool).
fn theory() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![FieldDescriptor {
                    name: "account".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                }],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Open".into(),
                        values: Box::new([]),
                    },
                    Row {
                        handle: "Frozen".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Status".into(),
                fields: vec![],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Winter".into(),
                        values: Box::new([
                            Value::IntervalU64(1, 90),
                            Value::Bool(false),
                            Value::U64(10),
                        ]),
                    },
                    Row {
                        handle: "Summer".into(),
                        values: Box::new([
                            Value::IntervalU64(172, 265),
                            Value::Bool(true),
                            Value::U64(30),
                        ]),
                    },
                    Row {
                        handle: "Autumn".into(),
                        values: Box::new([
                            Value::IntervalU64(265, 355),
                            Value::Bool(false),
                            Value::U64(20),
                        ]),
                    },
                ])),
                name: "Season".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "span".into(),
                        value_type: ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "sunny".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "rank".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const STATUS: RelationId = RelationId(1);
const SEASON: RelationId = RelationId(2);

/// One canonical-encoding word, compared exactly as the image holds it.
fn word(bytes: [u8; 8]) -> u64 {
    u64::from_be_bytes(bytes)
}

#[test]
fn synthesis_lays_the_id_column_then_every_canonical_encoding() {
    let schema = theory();
    let image = synthesize_closed(SEASON, schema.relation(SEASON));

    // rows == extension len; the implicit id column (FieldId 0) is 0..n.
    assert_eq!(image.row_count(), 3);
    let id_span = image.span(crate::schema::FieldId(0));
    assert_eq!(id_span.width, ColumnWidth::Word);
    assert_eq!(
        image.column_words(usize::from(id_span.first_column)),
        &[0, 1, 2]
    );

    // The interval field: two word columns, each half the canonical
    // encoding from validate — compared against `encoding::encode`
    // directly.
    let span = image.span(crate::schema::FieldId(1));
    assert_eq!(span.width, ColumnWidth::WordPair);
    let spans = [(1u64, 90u64), (172, 265), (265, 355)];
    let encoded: Vec<[u8; 16]> = spans
        .iter()
        .map(|(s, e)| encode_interval_u64(*s, *e))
        .collect();
    let expected_starts: Vec<u64> = encoded
        .iter()
        .map(|enc| word(enc[..8].try_into().expect("8-byte half")))
        .collect();
    let expected_ends: Vec<u64> = encoded
        .iter()
        .map(|enc| word(enc[8..].try_into().expect("8-byte half")))
        .collect();
    assert_eq!(
        image.column_words(usize::from(span.first_column)),
        expected_starts.as_slice()
    );
    assert_eq!(
        image.column_words(usize::from(span.first_column) + 1),
        expected_ends.as_slice()
    );

    // The bool field: one byte column of validated encodings.
    let sunny = image.span(crate::schema::FieldId(2));
    assert_eq!(sunny.width, ColumnWidth::Byte);
    assert_eq!(
        image.column_bytes(usize::from(sunny.first_column)),
        &[encode_bool(false), encode_bool(true), encode_bool(false)]
    );

    // The u64 field: canonical `encode_u64` words.
    let rank = image.span(crate::schema::FieldId(3));
    assert_eq!(rank.width, ColumnWidth::Word);
    assert_eq!(
        image.column_words(usize::from(rank.first_column)),
        &[
            word(encode_u64(10)),
            word(encode_u64(30)),
            word(encode_u64(20))
        ]
    );

    // Distinct counters are exact over the synthesized columns.
    assert_eq!(image.distinct(usize::from(id_span.first_column)), 3);
    assert_eq!(image.distinct(usize::from(sunny.first_column)), 2);
}

#[test]
fn a_columnless_vocabulary_synthesizes_to_its_id_column_alone() {
    let schema = theory();
    let image = synthesize_closed(STATUS, schema.relation(STATUS));
    assert_eq!(image.row_count(), 2);
    let id_span = image.span(crate::schema::FieldId(0));
    assert_eq!(
        image.column_words(usize::from(id_span.first_column)),
        &[0, 1]
    );
}
