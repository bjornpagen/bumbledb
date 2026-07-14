use crate::encoding::{ValueRef, encode_fact};
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;

mod closed;
mod corruption;
mod decode;
mod fixed_bytes;
mod interval;
mod stride;
mod timing;

/// R(id u64 fresh, flag bool, kind bool, amount i64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "flag".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
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

fn fact(schema: &Schema, id: u64, flag: bool, kind: bool, amount: i64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(id),
            ValueRef::Bool(flag),
            ValueRef::Bool(kind),
            ValueRef::I64(amount),
        ],
        schema.relation(R).layout(),
        &mut b,
    );
    b
}

fn populated(dir: &TempDir, schema: &Schema) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for i in 0..10u64 {
        let amount = i64::try_from(i).expect("small") * 7 - 30;
        delta
            .insert(&view, R, &fact(schema, i, i % 2 == 0, i % 3 == 0, amount))
            .expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    env
}
