use crate::encoding::encode_i64;
use crate::image::testsupport::TestSource;
use crate::image::{ColumnSpan, ColumnWidth};
use crate::ir::Value;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, IntervalElement, RelationDescriptor, RelationId, SchemaDescriptor,
    ValueType,
};

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "T".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::Bool,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const T: RelationId = RelationId(0);

const ROWS: [(u64, i64, i64, bool); 3] = [(0, -100, -7, false), (1, -5, 9, true), (2, 3, 7, false)];

fn w(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

fn populated(schema: &Schema) -> TestSource {
    let facts: Vec<Vec<Value>> = ROWS
        .iter()
        .map(|(id, start, end, kind)| {
            vec![
                Value::U64(*id),
                Value::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
                Value::Bool(*kind),
            ]
        })
        .collect();
    TestSource::new(schema, &[(T, facts)])
}

#[test]
fn interval_field_decodes_into_two_word_columns_with_golden_words() {
    let schema = schema();
    let source = populated(&schema);
    let (_cache, image) = source.image_with_cache(T);
    assert_eq!(image.row_count(), 3);

    // The field→column map: three fields, four columns — the interval

    assert_eq!(
        image.span(FieldId(0)),
        ColumnSpan {
            first_column: 0,
            width: ColumnWidth::Word,
        }
    );
    assert_eq!(
        image.span(FieldId(1)),
        ColumnSpan {
            first_column: 1,
            width: ColumnWidth::WordPair,
        }
    );
    assert_eq!(
        image.span(FieldId(2)),
        ColumnSpan {
            first_column: 3,
            width: ColumnWidth::Byte,
        }
    );

    let ids = image.column_words(0);
    let starts = image.column_words(1);
    let ends = image.column_words(2);
    let kinds = image.column_bytes(3);
    let mut seen = [false; ROWS.len()];
    for position in 0..image.row_count() {
        let row = usize::try_from(ids[position]).expect("fixture ids are 0..3");
        let (id, start, end, kind) = ROWS[row];
        assert_eq!(ids[position], id);
        assert_eq!(starts[position], w(start), "start word of row {id}");
        assert_eq!(ends[position], w(end), "end word of row {id}");
        assert_eq!(kinds[position], u8::from(kind), "bool byte of row {id}");
        assert!(
            starts[position] < ends[position],
            "row {id}: start < end as bare u64 words"
        );
        seen[row] = true;
    }
    assert_eq!(seen, [true; ROWS.len()], "every fixture row decoded");

    let mut start_words = starts.to_vec();
    start_words.sort_unstable();
    assert_eq!(start_words, [w(-100), w(-5), w(3)]);
    let mut end_words = ends.to_vec();
    end_words.sort_unstable();
    assert_eq!(end_words, [w(-7), w(7), w(9)]);
}

#[test]
fn dense_float_intervals_decode_into_order_key_word_pairs() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "D".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "range".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::F64,
                    },
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let interval = |start: f64, end: f64| {
        bumbledb_theory::Interval::new(
            bumbledb_theory::F64::from(start),
            bumbledb_theory::F64::from(end),
        )
        .expect("nonempty dense interval")
    };
    let facts = vec![
        vec![Value::U64(0), Value::IntervalF64(interval(-1.5, 2.25))],
        vec![
            Value::U64(1),
            Value::IntervalF64(
                bumbledb_theory::Interval::new(
                    bumbledb_theory::F64::NEG_INFINITY,
                    bumbledb_theory::F64::from(0.0),
                )
                .expect("left ray"),
            ),
        ],
    ];
    let source = TestSource::new(&schema, &[(T, facts)]);
    let (_cache, image) = source.image_with_cache(T);
    assert_eq!(image.row_count(), 2);
    let ids = image.column_words(0);
    let starts = image.column_words(1);
    let ends = image.column_words(2);
    for position in 0..2 {
        let (expected_start, expected_end) = if ids[position] == 0 {
            (
                bumbledb_theory::F64::from(-1.5).to_order_key(),
                bumbledb_theory::F64::from(2.25).to_order_key(),
            )
        } else {
            (
                bumbledb_theory::F64::NEG_INFINITY.to_order_key(),
                bumbledb_theory::F64::from(0.0).to_order_key(),
            )
        };
        assert_eq!(starts[position], expected_start, "dense start order key");
        assert_eq!(ends[position], expected_end, "dense end order key");
        assert!(starts[position] < ends[position]);
    }
}
