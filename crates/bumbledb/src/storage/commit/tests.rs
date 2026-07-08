use super::*;
use crate::encoding::{encode_fact, ValueRef};
use crate::schema::{
    ConstraintDescriptor, FieldDescriptor, FieldId, Generation, RelationDescriptor, Schema,
    SchemaDescriptor, ValueType,
};
use crate::storage::env::Environment;
use crate::storage::keys::{KeyBuf, MAX_KEY};

mod apply;
mod commit;

/// Target(id serial) + Source(id serial, t u64 fk -> Target.id) +
/// Keyed(x u64 unique, y i64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Target".into(),
                fields: vec![FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Serial,
                }],
                constraints: vec![],
            },
            RelationDescriptor {
                name: "Source".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "t".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "source_target".into(),
                    fields: Box::new([FieldId(1)]),
                    target_relation: RelationId(0),
                    target_constraint: ConstraintId(0),
                }],
            },
            RelationDescriptor {
                name: "Keyed".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "x".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "y".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "x".into(),
                    fields: Box::new([FieldId(0)]),
                }],
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

const TARGET: RelationId = RelationId(0);
const SOURCE: RelationId = RelationId(1);
const KEYED: RelationId = RelationId(2);
const C0: ConstraintId = ConstraintId(0);

fn target_fact(schema: &Schema, id: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(id)],
        schema.relation(TARGET).layout(),
        &mut b,
    );
    b
}

fn source_fact(schema: &Schema, id: u64, t: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(id), ValueRef::U64(t)],
        schema.relation(SOURCE).layout(),
        &mut b,
    );
    b
}

fn keyed_fact(schema: &Schema, x: u64, y: i64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(x), ValueRef::I64(y)],
        schema.relation(KEYED).layout(),
        &mut b,
    );
    b
}

fn all_data_keys(txn: &WriteTxn<'_>, env: &Environment) -> BTreeSet<Vec<u8>> {
    env.data()
        .iter(txn.raw())
        .expect("iter")
        .map(|kv| kv.expect("kv").0.to_vec())
        .collect()
}

fn committed_data(env: &Environment) -> Vec<(Vec<u8>, Vec<u8>)> {
    let rtxn = env.read_txn().expect("txn");
    env.data()
        .iter(rtxn.raw())
        .expect("iter")
        .map(|kv| {
            let (k, v) = kv.expect("kv");
            (k.to_vec(), v.to_vec())
        })
        .collect()
}

fn key(write: impl FnOnce(&mut KeyBuf) -> usize) -> Vec<u8> {
    let mut buf: KeyBuf = [0; MAX_KEY];
    let len = write(&mut buf);
    buf[..len].to_vec()
}
