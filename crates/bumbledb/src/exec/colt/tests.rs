use super::*;
use crate::image::testsupport::TestSource;
use crate::image::view::apply;
use crate::ir::Value;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};
use std::collections::HashMap;
use std::sync::Arc;

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "k".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);

fn view_of(schema: &Schema, rows: &[(u64, u64)]) -> Arc<crate::image::RelationImage> {
    let facts: Vec<Vec<Value>> = rows
        .iter()
        .map(|(k, v)| vec![Value::U64(*k), Value::U64(*v)])
        .collect();
    let source = TestSource::new(schema, &[(R, facts)]);
    let (_cache, image) = source.image_with_cache(R);
    image
}

fn all(image: &Arc<crate::image::RelationImage>) -> View {
    apply(image, &[], &[], Vec::new(), image.generation().text_eq(None))
}

fn scalars(columns: &[usize]) -> Vec<SelectionLevel> {
    columns
        .iter()
        .map(|column| SelectionLevel::Point {
            columns: vec![*column],
        })
        .collect()
}

fn set_level(column: usize) -> Vec<SelectionLevel> {
    vec![SelectionLevel::Set {
        columns: vec![column],
    }]
}

fn drain(colt: &mut Colt, cursor: Cursor, level: usize) -> Vec<(Vec<u64>, Cursor)> {
    let arity = colt.arity(level);
    let mut keys = vec![0u64; 8 * arity.max(1)];
    let mut children = vec![Cursor::Row(0); 8];
    let mut token = BatchToken::default();
    let mut out = Vec::new();
    loop {
        let (n, next) = colt
            .iter_batch(cursor, level, token, &mut keys, &mut children, 8)
            .expect("iter");
        if n == 0 {
            break;
        }
        for i in 0..n {
            out.push((keys[i * arity..(i + 1) * arity].to_vec(), children[i]));
        }
        token = next;
    }
    out
}

mod admit;
mod dense;
mod model;
mod overflow;
mod pins;
mod selection;
mod sizing;
mod synthetic;
