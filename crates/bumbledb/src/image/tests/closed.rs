use crate::encoding::{encode_bool, encode_u64};
use crate::image::{ColumnWidth, synthesize_closed};
use crate::ir::Value;
use crate::work::{GenerationHandle, GenerationState, WorkContext};
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
                    },
                    FieldDescriptor {
                        name: "sunny".into(),
                        value_type: ValueType::Bool,
                    },
                    FieldDescriptor {
                        name: "rank".into(),
                        value_type: ValueType::U64,
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

fn closed_generation() -> GenerationHandle {
    GenerationHandle::new(GenerationState::new(
        crate::image::CacheGeneration::initial(),
    ))
}

#[test]
fn synthesis_lays_the_id_column_then_every_canonical_encoding() {
    let schema = theory();
    let image = synthesize_closed(&schema, SEASON, &closed_generation(), &WorkContext::new())
        .expect("closed synthesis");

    assert_eq!(image.row_count(), 3);
    let id_span = image.span(bumbledb_theory::schema::FieldId(0));
    assert_eq!(id_span.width, ColumnWidth::Word);
    assert_eq!(
        image.column_words(usize::from(id_span.first_column)),
        &[0, 1, 2]
    );

    let span = image.span(bumbledb_theory::schema::FieldId(1));
    assert_eq!(span.width, ColumnWidth::WordPair);
    let (expected_starts, expected_ends): (Vec<u64>, Vec<u64>) = schema
        .closed_rows(SEASON)
        .expect("closed relation")
        .iter()
        .map(|row| match &row.values[1] {
            Value::IntervalU64(interval) => (interval.start(), interval.end()),
            other => panic!("span field holds {other:?}"),
        })
        .unzip();
    assert_eq!(expected_starts, [1, 172, 265]);
    assert_eq!(expected_ends, [90, 265, 355]);
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

    let work = crate::api::prepared::source::unbounded_work();
    assert!(image.distincts.iter().all(|count| count.get().is_none()));
    assert_eq!(
        image
            .distinct_count(usize::from(id_span.first_column), &work)
            .unwrap(),
        3
    );
    assert_eq!(
        image
            .distinct_count(usize::from(sunny.first_column), &work)
            .unwrap(),
        2
    );
}

#[test]
fn a_columnless_vocabulary_synthesizes_to_its_id_column_alone() {
    let schema = theory();
    let image = synthesize_closed(&schema, STATUS, &closed_generation(), &WorkContext::new())
        .expect("closed synthesis");
    assert_eq!(image.row_count(), 2);
    let id_span = image.span(bumbledb_theory::schema::FieldId(0));
    assert_eq!(
        image.column_words(usize::from(id_span.first_column)),
        &[0, 1]
    );
}
