use crate::image::testsupport::TestSource;
use crate::ir::Value;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};

mod closed;
mod corruption;
mod decode;
mod fixed_bytes;
mod interval;
mod stride;
mod stride_ab;
mod timing;

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "flag".into(),
                    value_type: ValueType::Bool,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::Bool,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);

fn fact(id: u64, flag: bool, kind: bool, amount: i64) -> Vec<Value> {
    vec![
        Value::U64(id),
        Value::Bool(flag),
        Value::Bool(kind),
        Value::I64(amount),
    ]
}

fn default_rows() -> Vec<Vec<Value>> {
    (0..10u64)
        .map(|i| {
            let amount = i64::try_from(i).expect("small") * 7 - 30;
            fact(i, i % 2 == 0, i % 3 == 0, amount)
        })
        .collect()
}

fn source_of(schema: &Schema, rows: Vec<Vec<Value>>) -> TestSource {
    TestSource::new(schema, &[(R, rows)])
}
