use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{FieldDescriptor, ValueType};
use bumbledb_lmdb::{Fact, Value};

use crate::bench_error;

pub(crate) fn id(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Serial(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "id", other)),
    }
}

pub(crate) fn rf(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Serial(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "ref", other)),
    }
}

pub(crate) fn symbol(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Enum(v) => Ok(i64::from(*v)),
        Value::U64(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "symbol", other)),
    }
}

pub(crate) fn dec(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Decimal(DecimalRaw(v)) => Ok(*v as i64),
        other => Err(unexpected_value(field, "decimal", other)),
    }
}

pub(crate) fn ts(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Timestamp(TimestampMicros(v)) => Ok(*v),
        other => Err(unexpected_value(field, "timestamp", other)),
    }
}

pub(crate) fn u64v(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::U64(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "u64", other)),
    }
}

pub(crate) fn i64v(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::I64(v) => Ok(*v),
        other => Err(unexpected_value(field, "i64", other)),
    }
}

pub(crate) fn text(fact: &Fact, field: &str) -> Result<String, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::String(v) => Ok(v.clone()),
        other => Err(unexpected_value(field, "string", other)),
    }
}

fn required_value<'a>(
    fact: &'a Fact,
    field: &str,
) -> Result<&'a Value, Box<dyn std::error::Error>> {
    fact.value(field)
        .ok_or_else(|| bench_error(format!("missing field {field}")))
}

fn unexpected_value(field: &str, expected: &str, actual: &Value) -> Box<dyn std::error::Error> {
    bench_error(format!("expected {expected} {field}, got {actual:?}"))
}

pub(crate) fn serial_key_field(id_type: &str, relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        "id",
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: relation.to_owned(),
        },
    )
}

pub(crate) fn serial_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: target.to_owned(),
        },
    )
}
