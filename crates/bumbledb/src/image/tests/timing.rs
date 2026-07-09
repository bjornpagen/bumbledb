use super::R;
use crate::encoding::{encode_fact, ValueRef};
use crate::image::build;
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;

/// PRD 12's profile split (ignored: timing evidence, run by hand):
/// the LMDB cursor walk alone vs the full build, on a Posting-shaped
/// 150k-row relation.
#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
fn image_build_split_evidence() {
    let dir = TempDir::new("image-split");
    let schema = posting_like_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let txn0 = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let mut bytes = Vec::new();
    for i in 0..150_000u64 {
        bytes.clear();
        encode_fact(
            &[
                ValueRef::U64(i),
                ValueRef::U64(i % 512),
                ValueRef::I64((i % 1000).cast_signed() - 500),
                ValueRef::I64((i * 7 % 100_000).cast_signed()),
                ValueRef::Bool(i % 2 == 0),
            ],
            schema.relation(R).layout(),
            &mut bytes,
        );
        delta.insert(&txn0, R, &bytes).expect("insert");
    }
    drop(txn0);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");

    // Walk floor: drain the cursor, touch every fact byte cheaply.
    let mut sink = 0u64;
    let walk = std::time::Instant::now();
    for _ in 0..5 {
        for entry in crate::storage::read::scan(&txn, &schema, R).expect("scan") {
            let (_, fact) = entry.expect("entry");
            sink = sink
                .wrapping_add(u64::from(fact[0]))
                .wrapping_add(fact.len() as u64);
        }
    }
    let walk = walk.elapsed() / 5;

    let full = std::time::Instant::now();
    for _ in 0..5 {
        let image = build(&txn, &schema, R).expect("build");
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
            name: "P".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "account".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "at".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "flag".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}
