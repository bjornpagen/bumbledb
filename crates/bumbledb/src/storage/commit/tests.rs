use super::plan::CommitPlan;
use super::*;
use crate::encoding::{ValueRef, encode_fact};
use crate::error::{Admission, Result};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::key;
use bumbledb_theory::Value;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId,
    SchemaDescriptor, Side, StatementDescriptor, StatementId, ValueType,
};

use std::collections::BTreeSet;

mod apply;
mod closed;
mod commit;
mod functionality;
mod host;
mod judgment;
mod marks;
mod plan;
mod sealed_checks;
mod target;

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn interval() -> ValueType {
    ValueType::Interval {
        element: IntervalElement::U64,
    }
}

fn side(relation: RelationId, projection: &[u16]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|&f| FieldId(f)).collect(),
        selection: Box::new([]),
    }
}

fn selected(relation: RelationId, projection: &[u16], selection: &[(u16, Value)]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|&f| FieldId(f)).collect(),
        selection: selection
            .iter()
            .map(|(f, literal)| {
                (
                    FieldId(*f),
                    bumbledb_theory::schema::LiteralSet::One(literal.clone()),
                )
            })
            .collect(),
    }
}

fn fact(schema: &Schema, rel: RelationId, values: &[ValueRef]) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(values, schema.relation(rel).layout(), &mut b);
    b
}

fn apply_delta(
    env: &Environment,
    schema: &Schema,
    deletes: &[(RelationId, Vec<u8>)],
    inserts: &[(RelationId, Vec<u8>)],
) -> Result<Admission<()>> {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (rel, fact) in deletes {
        delta.delete(&view, *rel, fact).expect("record delete");
    }
    for (rel, fact) in inserts {
        delta.insert(&view, *rel, fact).expect("record insert");
    }
    drop(view);
    super::commit(delta, env).map(|admission| admission.map(|_| ()))
}

fn plan_for<'d>(delta: &'d WriteDelta<'_>, env: &Environment) -> CommitPlan<'d> {
    let view = env.read_txn().expect("txn");
    let selections = super::judgment::Selections::encode(delta, &view).expect("encode selections");
    super::plan::plan_commit(delta, selections).expect("derive plan")
}

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                }],
            },
            RelationDescriptor {
                extension: None,
                name: "Keyed".into(),
                fields: vec![field("x", ValueType::U64), field("y", ValueType::I64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field("during", interval()),
                    field("tag", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
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
                source: side(CLAIM, &[0]),
                target: side(TARGET, &[0]),
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

const KEYED_KEY: StatementId = StatementId(1);
const BOOKING_KEY: StatementId = StatementId(2);
const CLAIM_TARGET: StatementId = StatementId(3);

fn target_fact(schema: &Schema, id: u64) -> Vec<u8> {
    fact(schema, TARGET, &[ValueRef::U64(id)])
}

fn keyed_fact(schema: &Schema, x: u64, y: i64) -> Vec<u8> {
    fact(schema, KEYED, &[ValueRef::U64(x), ValueRef::I64(y)])
}

fn claim_fact(schema: &Schema, holder: u64) -> Vec<u8> {
    fact(schema, CLAIM, &[ValueRef::U64(holder)])
}

fn booking_fact(schema: &Schema, room: u64, start: u64, end: u64, tag: u64) -> Vec<u8> {
    fact(
        schema,
        BOOKING,
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
            ValueRef::U64(tag),
        ],
    )
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
