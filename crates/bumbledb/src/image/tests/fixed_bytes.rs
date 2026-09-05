use crate::error::Error;
use crate::image::ColumnWidth;
use crate::image::canon::{TextWords, row_words};
use crate::image::intern::TextInterner;
use crate::image::testsupport::TestSource;
use crate::ir::Value;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};

fn schema() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "D".into(),
            fields: vec![
                field("id", ValueType::U64),
                field("head", ValueType::FixedBytes { len: 9 }),
                field("hash", ValueType::FixedBytes { len: 32 }),
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const D: RelationId = RelationId(0);

fn fact(id: u64) -> Vec<Value> {
    let mut head = [0u8; 9];
    head[8] = u8::try_from(id % 251).expect("byte");
    let mut hash = [0u8; 32];
    hash[24..].copy_from_slice(&id.to_be_bytes());
    vec![
        Value::U64(id),
        Value::FixedBytes(Box::from(&head[..])),
        Value::FixedBytes(Box::from(&hash[..])),
    ]
}

fn populated(schema: &Schema) -> TestSource {
    let rows: Vec<Vec<Value>> = (0..10u64).map(fact).collect();
    TestSource::new(schema, &[(D, rows)])
}

#[test]
fn fixed_bytes_fields_decode_into_padded_word_columns() {
    let schema = schema();
    let source = populated(&schema);
    let (_cache, image) = source.image_with_cache(D);

    // shift accordingly (the field→column map, never raw field indices).
    let head = image.span(bumbledb_theory::schema::FieldId(1));
    assert_eq!(head.width, ColumnWidth::Words { count: 2 });
    let hash = image.span(bumbledb_theory::schema::FieldId(2));
    assert_eq!(hash.first_column, 3);
    assert_eq!(hash.width, ColumnWidth::Words { count: 4 });

    let ids = image.column_words(0);
    let head_tail = image.column_words(usize::from(head.first_column) + 1);
    let hash_lead = image.column_words(usize::from(hash.first_column));
    let hash_tail = image.column_words(usize::from(hash.first_column) + 3);
    let mut seen: Vec<u64> = ids.to_vec();
    seen.sort_unstable();
    assert_eq!(seen, (0..10).collect::<Vec<u64>>());
    for row in 0..10usize {
        let id = ids[row];
        assert_eq!(head_tail[row], (id % 251) << 56, "tail byte, zero pad");
        assert_eq!(hash_lead[row], 0, "the adversarial shared prefix");
        assert_eq!(hash_tail[row], id);
    }
}

#[test]
fn a_wrong_width_stored_blob_refuses_typed() {
    // The canonical codec stores bytes<N> at exact width; a blob whose
    // stored length disagrees with the schema refuses as corruption at
    // the one shared walker (never truncation, never silent padding).
    let schema = schema();
    let work = crate::api::prepared::source::unbounded_work().expect("ledger");
    let healthy =
        crate::canonical::CanonicalRow::encode(schema.relation(D).fields(), &fact(3), &work)
            .expect("canonical")
            .as_bytes()
            .to_vec();

    let walk = |bytes: &[u8]| -> Result<Vec<u64>, Error> {
        let interner = TextInterner::default();
        let mut text = TextWords::Lookup(&interner);
        let mut out = Vec::new();
        row_words(schema.relation(D).fields(), bytes, &mut text, &mut out)?
            .expect_ready("lookup never spills");
        Ok(out)
    };
    let words = walk(&healthy).expect("the healthy row walks");
    assert_eq!(words.len(), 1 + 2 + 4, "id word + padded byte words");

    // Rewrite the head field's stored length prefix from 9 to 8 so the
    // decoded blob width disagrees with the schema's bytes<9>.
    // layout: arity(2) + [tag u64(1) + 8] + [tag bytes(1) + len u64(8) + 9]
    let mut short = healthy.clone();
    short[12..20].copy_from_slice(&8u64.to_be_bytes());
    let err = walk(&short).expect_err("wrong-width blob refuses");
    assert!(matches!(err, Error::Corruption(_)), "{err:?}");
}
