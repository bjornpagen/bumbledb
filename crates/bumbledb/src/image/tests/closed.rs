use crate::encoding::{encode_bool, encode_interval_u64, encode_u64};
use crate::image::{ColumnWidth, synthesize_closed};
use crate::ir::Value;
use bumbledb_theory::schema::{IntervalElement, Row};

use super::*;

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
                            Value::IntervalU64(
                                bumbledb_theory::Interval::<u64>::new(1, 90)
                                    .expect("nonempty interval"),
                            ),
                            Value::Bool(false),
                            Value::U64(10),
                        ]),
                    },
                    Row {
                        handle: "Summer".into(),
                        values: Box::new([
                            Value::IntervalU64(
                                bumbledb_theory::Interval::<u64>::new(172, 265)
                                    .expect("nonempty interval"),
                            ),
                            Value::Bool(true),
                            Value::U64(30),
                        ]),
                    },
                    Row {
                        handle: "Autumn".into(),
                        values: Box::new([
                            Value::IntervalU64(
                                bumbledb_theory::Interval::<u64>::new(265, 355)
                                    .expect("nonempty interval"),
                            ),
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

fn word(bytes: [u8; 8]) -> u64 {
    u64::from_be_bytes(bytes)
}

#[test]
fn synthesis_lays_the_id_column_then_every_canonical_encoding() {
    let schema = theory();
    let image = synthesize_closed(SEASON, schema.relation(SEASON));

    assert_eq!(image.row_count(), 3);
    let id_span = image.span(bumbledb_theory::schema::FieldId(0));
    assert_eq!(id_span.width, ColumnWidth::Word);
    assert_eq!(
        image.column_words(usize::from(id_span.first_column)),
        &[0, 1, 2]
    );

    let span = image.span(bumbledb_theory::schema::FieldId(1));
    assert_eq!(span.width, ColumnWidth::WordPair);
    let spans = [(1u64, 90u64), (172, 265), (265, 355)];
    let encoded: Vec<[u8; 16]> = spans
        .iter()
        .map(|(s, e)| {
            encode_interval_u64(
                bumbledb_theory::Interval::<u64>::new(*s, *e).expect("nonempty interval"),
            )
        })
        .collect();
    let expected_starts: Vec<u64> = encoded
        .iter()
        .map(|enc| word(crate::encoding::split_halves(*enc).0))
        .collect();
    let expected_ends: Vec<u64> = encoded
        .iter()
        .map(|enc| word(crate::encoding::split_halves(*enc).1))
        .collect();
    assert_eq!(
        image.column_words(usize::from(span.first_column)),
        expected_starts.as_slice()
    );
    assert_eq!(
        image.column_words(usize::from(span.first_column) + 1),
        expected_ends.as_slice()
    );

    let sunny = image.span(bumbledb_theory::schema::FieldId(2));
    assert_eq!(sunny.width, ColumnWidth::Byte);
    assert_eq!(
        image.column_bytes(usize::from(sunny.first_column)),
        &[encode_bool(false), encode_bool(true), encode_bool(false)]
    );

    let rank = image.span(bumbledb_theory::schema::FieldId(3));
    assert_eq!(rank.width, ColumnWidth::Word);
    assert_eq!(
        image.column_words(usize::from(rank.first_column)),
        &[
            word(encode_u64(10)),
            word(encode_u64(30)),
            word(encode_u64(20))
        ]
    );

    assert_eq!(image.distinct_count(usize::from(id_span.first_column)), 3);
    assert_eq!(image.distinct_count(usize::from(sunny.first_column)), 2);
}

#[test]
fn a_columnless_vocabulary_synthesizes_to_its_id_column_alone() {
    let schema = theory();
    let image = synthesize_closed(STATUS, schema.relation(STATUS));
    assert_eq!(image.row_count(), 2);
    let id_span = image.span(bumbledb_theory::schema::FieldId(0));
    assert_eq!(
        image.column_words(usize::from(id_span.first_column)),
        &[0, 1]
    );
}
