use super::*;
use crate::encoding::{encode_fact, ValueRef};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, Schema,
    SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use crate::storage::env::Environment;
use crate::storage::keys::{KeyBuf, MAX_KEY};

mod apply;
mod commit;
mod functionality;
mod judgment;
mod target;

/// Target(id serial) + Keyed(x u64, y i64; key x) +
/// Booking(room u64, during interval<u64>, tag u64; key (room, during)) +
/// Claim(holder u64; Claim(holder) <= Target(id)) — the containment gives
/// Target's key a dependent, so its guards feed the target-side check.
fn schema() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Target".into(),
                fields: vec![FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Serial,
                }],
            },
            RelationDescriptor {
                name: "Keyed".into(),
                fields: vec![field("x", ValueType::U64), field("y", ValueType::I64)],
            },
            RelationDescriptor {
                name: "Booking".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field(
                        "during",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                    field("tag", ValueType::U64),
                ],
            },
            RelationDescriptor {
                name: "Claim".into(),
                fields: vec![field("holder", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: KEYED,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: BOOKING,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: CLAIM,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: TARGET,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

const TARGET: RelationId = RelationId(0);
const KEYED: RelationId = RelationId(1);
const BOOKING: RelationId = RelationId(2);
const CLAIM: RelationId = RelationId(3);

/// Materialized statement order: Target's serial auto-key first, then the
/// declared statements in declaration order.
const TARGET_KEY: StatementId = StatementId(0);
const KEYED_KEY: StatementId = StatementId(1);
const BOOKING_KEY: StatementId = StatementId(2);
const CLAIM_TARGET: StatementId = StatementId(3);

fn target_fact(schema: &Schema, id: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(id)],
        schema.relation(TARGET).layout(),
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

fn claim_fact(schema: &Schema, holder: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(holder)],
        schema.relation(CLAIM).layout(),
        &mut b,
    );
    b
}

/// A Booking fact: `during = [start, end)`; `tag` distinguishes facts
/// sharing a key guard (an exact-duplicate key on distinct facts).
fn booking_fact(schema: &Schema, room: u64, start: u64, end: u64, tag: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(start, end),
            ValueRef::U64(tag),
        ],
        schema.relation(BOOKING).layout(),
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
