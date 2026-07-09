use super::*;
use crate::encoding::{encode_fact, ValueRef};
use crate::image::view::apply;
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use std::collections::HashMap;
use std::sync::Arc;

/// R(k u64, v u64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "k".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);

/// Builds an image over committed (k, v) pairs.
fn view_of(
    dir: &TempDir,
    schema: &Schema,
    rows: &[(u64, u64)],
) -> Arc<crate::image::RelationImage> {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (k, v) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(*k), ValueRef::U64(*v)],
            schema.relation(R).layout(),
            &mut bytes,
        );
        delta.insert(&view, R, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    crate::image::build(&txn, schema, R).expect("build")
}

fn all(image: &Arc<crate::image::RelationImage>) -> View {
    apply(image, &[], &[], Vec::new())
}

/// Scalar selection levels over the given single columns.
fn scalars(columns: &[usize]) -> Vec<SelectionLevel> {
    columns
        .iter()
        .map(|column| SelectionLevel {
            columns: vec![*column],
            set: false,
        })
        .collect()
}

/// One set-bound selection level over a single column.
fn set_level(column: usize) -> Vec<SelectionLevel> {
    vec![SelectionLevel {
        columns: vec![column],
        set: true,
    }]
}

/// Drains every entry at a cursor/level into (key words, child) pairs.
fn drain(colt: &mut Colt, cursor: Cursor, level: usize) -> Vec<(Vec<u64>, Cursor)> {
    let arity = colt.arity(level);
    let mut keys = vec![0u64; 8 * arity.max(1)];
    let mut children = vec![Cursor::Row(0); 8];
    let mut token = BatchToken::default();
    let mut out = Vec::new();
    loop {
        let (n, next) = colt.iter_batch(cursor, level, token, &mut keys, &mut children, 8);
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

mod dense;
mod model;
mod overflow;
mod pins;
mod selection;
mod sizing;
