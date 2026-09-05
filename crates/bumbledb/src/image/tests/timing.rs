use super::R;
use crate::image::testsupport::TestSource;
use crate::ir::Value;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
fn image_build_split_evidence() {
    let schema = posting_like_schema();
    let rows: Vec<Vec<Value>> = (0..150_000u64)
        .map(|i| {
            vec![
                Value::U64(i),
                Value::U64(i % 512),
                Value::I64((i % 1000).cast_signed() - 500),
                Value::I64((i * 7 % 100_000).cast_signed()),
                Value::Bool(i % 2 == 0),
            ]
        })
        .collect();
    let fixture = TestSource::new(&schema, &[(R, rows)]);

    let mut sink = 0u64;
    let walk = std::time::Instant::now();
    for _ in 0..5 {
        for bytes in crate::api::prepared::source::HeapRows::rows(&fixture, R) {
            sink = sink
                .wrapping_add(u64::from(bytes[0]))
                .wrapping_add(bytes.len() as u64);
        }
    }
    let walk = walk.elapsed() / 5;

    let full = std::time::Instant::now();
    for _ in 0..5 {
        let (_cache, image) = fixture.image_with_cache(R);
        sink = sink.wrapping_add(image.row_count() as u64);
    }
    let full = full.elapsed() / 5;
    println!(
        "image_build split over 150k rows: walk {walk:?}, full {full:?}, decode+scatter {:?} (sink {sink})",
        full.saturating_sub(walk)
    );
}

fn posting_like_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "P".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "account".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                },
                FieldDescriptor {
                    name: "at".into(),
                    value_type: ValueType::I64,
                },
                FieldDescriptor {
                    name: "flag".into(),
                    value_type: ValueType::Bool,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}
