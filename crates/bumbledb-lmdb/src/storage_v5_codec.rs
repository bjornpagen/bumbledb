#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bumbledb_core::encoding::{
    InternId, decode_bool, decode_enum, decode_i64, decode_intern_id, decode_u64, encode_bool,
    encode_enum, encode_i64, encode_intern_id, encode_u64,
};
use bumbledb_core::schema::{
    FieldDescriptor, FieldGeneration, RelationDescriptor, SchemaDescriptor, ValueType,
};

use super::meta::{
    DICT_BYTES, DICT_FWD, DICT_REV, DICT_STRING, META_NEXT_DICT_ID, bytes_to_u64, read_u64,
    relation_id, write_u64,
};
use crate::storage_format::{FactHandle, fact_handle, serial_sequence_key};
use crate::{Error, Fact, RawDatabase, ReadTxn, Result, Value, WriteTxn};

#[derive(Clone, Copy)]
enum InternMode {
    Create,
    Existing,
}

pub(super) struct EncodedFact {
    pub(super) relation_id: u32,
    pub(super) relation: RelationDescriptor,
    pub(super) fields: Vec<Vec<u8>>,
    pub(super) bytes: Vec<u8>,
    pub(super) handle: FactHandle,
}

pub(super) enum EncodeDelete {
    Encoded(EncodedFact),
    MissingDictionary,
}

pub(super) fn encode_insert_fact(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &Fact,
) -> Result<EncodedFact> {
    match encode_fact(txn, schema, fact, InternMode::Create)? {
        EncodeDelete::Encoded(encoded) => Ok(encoded),
        EncodeDelete::MissingDictionary => Err(Error::corrupt(
            "insert encoding returned missing dictionary in create mode",
        )),
    }
}

pub(super) fn encode_delete_fact(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &Fact,
) -> Result<EncodeDelete> {
    encode_fact(txn, schema, fact, InternMode::Existing)
}

fn encode_fact(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &Fact,
    intern_mode: InternMode,
) -> Result<EncodeDelete> {
    let relation_id = relation_id(schema, fact.relation())?;
    let relation = schema.relations[relation_id as usize].clone();
    let field_names: BTreeSet<_> = relation
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    for field in fact.values().keys() {
        if !field_names.contains(field.as_str()) {
            return Err(Error::invalid_fact(format!(
                "unknown field {}.{}",
                relation.name, field
            )));
        }
    }

    let mut encoded_fields = Vec::new();
    let mut fact_bytes = Vec::new();
    for (field_id, field) in relation.fields.iter().enumerate() {
        let value = field_value(
            txn,
            fact,
            &relation,
            relation_id,
            field_id,
            field,
            intern_mode,
        )?;
        if !value.matches_type(&field.value_type) {
            return Err(Error::invalid_fact(format!(
                "field {}.{} has wrong type",
                relation.name, field.name
            )));
        }
        let bytes = match encode_value(txn, schema, field, &value, intern_mode)? {
            Some(bytes) => bytes,
            None => return Ok(EncodeDelete::MissingDictionary),
        };
        if field.generation == FieldGeneration::SerialSequence
            && let Value::Serial(value) = value
            && matches!(intern_mode, InternMode::Create)
        {
            advance_serial_high_water(txn, relation_id, field_id as u32, value)?;
        }
        fact_bytes.extend_from_slice(&bytes);
        encoded_fields.push(bytes);
    }
    let handle = fact_handle(relation_id, &fact_bytes);
    Ok(EncodeDelete::Encoded(EncodedFact {
        relation_id,
        relation,
        fields: encoded_fields,
        bytes: fact_bytes,
        handle,
    }))
}

fn field_value(
    txn: &mut WriteTxn<'_>,
    fact: &Fact,
    relation: &RelationDescriptor,
    relation_id: u32,
    field_id: usize,
    field: &FieldDescriptor,
    intern_mode: InternMode,
) -> Result<Value> {
    match fact.value(&field.name) {
        Some(value) => Ok(value.clone()),
        None if field.generation == FieldGeneration::SerialSequence
            && matches!(intern_mode, InternMode::Create) =>
        {
            Ok(Value::Serial(next_serial(
                txn,
                relation_id,
                field_id as u32,
            )?))
        }
        None => Err(Error::invalid_fact(format!(
            "missing field {}.{}",
            relation.name, field.name
        ))),
    }
}

fn encode_value(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    field: &FieldDescriptor,
    value: &Value,
    intern_mode: InternMode,
) -> Result<Option<Vec<u8>>> {
    Ok(Some(match (value, &field.value_type) {
        (Value::Bool(value), ValueType::Bool) => encode_bool(*value).to_vec(),
        (Value::U64(value), ValueType::U64) | (Value::Serial(value), ValueType::Serial { .. }) => {
            encode_u64(*value).to_vec()
        }
        (Value::I64(value), ValueType::I64) => encode_i64(*value).to_vec(),
        (Value::Enum(value), ValueType::Enum { name }) => encode_enum_value(schema, name, *value)?,
        (Value::String(value), ValueType::String) => {
            let Some(id) = intern_value(txn, DICT_STRING, value.as_bytes(), intern_mode)? else {
                return Ok(None);
            };
            encode_intern_id(InternId(id)).to_vec()
        }
        (Value::Bytes(value), ValueType::Bytes) => {
            let Some(id) = intern_value(txn, DICT_BYTES, value, intern_mode)? else {
                return Ok(None);
            };
            encode_intern_id(InternId(id)).to_vec()
        }
        _ => return Err(Error::invalid_fact("value/type mismatch")),
    }))
}

fn encode_enum_value(schema: &SchemaDescriptor, name: &str, value: u8) -> Result<Vec<u8>> {
    if !schema.enum_contains_code(name, value) {
        return Err(Error::invalid_fact(format!(
            "enum {name} does not contain code {value}"
        )));
    }
    Ok(encode_enum(value).to_vec())
}

#[cfg(test)]
pub(super) fn decode_fact(
    txn: &ReadTxn<'_>,
    relation: &RelationDescriptor,
    bytes: &[u8],
) -> Result<Fact> {
    let mut offset = 0;
    let mut values = BTreeMap::new();
    for field in &relation.fields {
        let width = field.value_type.encoded_width();
        let Some(value_bytes) = bytes.get(offset..offset + width) else {
            return Err(Error::corrupt("encoded fact has wrong width"));
        };
        values.insert(
            field.name.clone(),
            decode_value(txn, &field.value_type, value_bytes)?,
        );
        offset += width;
    }
    if offset != bytes.len() {
        return Err(Error::corrupt("encoded fact has trailing bytes"));
    }
    Ok(Fact::new(relation.name.clone(), values))
}

pub(super) fn decode_value(
    txn: &ReadTxn<'_>,
    value_type: &ValueType,
    bytes: &[u8],
) -> Result<Value> {
    Ok(match value_type {
        ValueType::Bool => {
            Value::Bool(decode_bool(bytes).map_err(|error| Error::corrupt(error.to_string()))?)
        }
        ValueType::U64 => {
            Value::U64(decode_u64(bytes).map_err(|error| Error::corrupt(error.to_string()))?)
        }
        ValueType::I64 => {
            Value::I64(decode_i64(bytes).map_err(|error| Error::corrupt(error.to_string()))?)
        }
        ValueType::Enum { .. } => {
            Value::Enum(decode_enum(bytes).map_err(|error| Error::corrupt(error.to_string()))?)
        }
        ValueType::Serial { .. } => {
            Value::Serial(decode_u64(bytes).map_err(|error| Error::corrupt(error.to_string()))?)
        }
        ValueType::String => decode_string(txn, bytes)?,
        ValueType::Bytes => decode_bytes(txn, bytes)?,
    })
}

pub(super) fn encode_existing_value(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    value_type: &ValueType,
    value: &Value,
) -> Result<Option<Vec<u8>>> {
    Ok(Some(match (value, value_type) {
        (Value::Bool(value), ValueType::Bool) => encode_bool(*value).to_vec(),
        (Value::U64(value), ValueType::U64) | (Value::Serial(value), ValueType::Serial { .. }) => {
            encode_u64(*value).to_vec()
        }
        (Value::I64(value), ValueType::I64) => encode_i64(*value).to_vec(),
        (Value::Enum(value), ValueType::Enum { name }) => encode_enum_value(schema, name, *value)?,
        (Value::String(value), ValueType::String) => {
            let Some(id) =
                lookup_intern_id(&txn.dbs.dict, &txn.txn, DICT_STRING, value.as_bytes())?
            else {
                return Ok(None);
            };
            encode_intern_id(InternId(id)).to_vec()
        }
        (Value::Bytes(value), ValueType::Bytes) => {
            let Some(id) = lookup_intern_id(&txn.dbs.dict, &txn.txn, DICT_BYTES, value)? else {
                return Ok(None);
            };
            encode_intern_id(InternId(id)).to_vec()
        }
        _ => return Err(Error::invalid_query("input value/type mismatch")),
    }))
}

fn decode_string(txn: &ReadTxn<'_>, bytes: &[u8]) -> Result<Value> {
    let id = decode_intern_id(bytes)
        .map_err(|error| Error::corrupt(error.to_string()))?
        .0;
    let raw = lookup_intern_raw(txn.dbs.dict, &txn.txn, DICT_STRING, id)?;
    let value = String::from_utf8(raw).map_err(|error| Error::corrupt(error.to_string()))?;
    Ok(Value::String(value))
}

fn decode_bytes(txn: &ReadTxn<'_>, bytes: &[u8]) -> Result<Value> {
    let id = decode_intern_id(bytes)
        .map_err(|error| Error::corrupt(error.to_string()))?
        .0;
    Ok(Value::Bytes(lookup_intern_raw(
        txn.dbs.dict,
        &txn.txn,
        DICT_BYTES,
        id,
    )?))
}

pub(super) fn encoded_key_from_fields(
    relation: &RelationDescriptor,
    fact: &EncodedFact,
    fields: &[String],
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for field_name in fields {
        let field_id = relation
            .fields
            .iter()
            .position(|field| field.name == *field_name)
            .ok_or_else(|| Error::corrupt("constraint field missing"))?;
        out.extend_from_slice(&fact.fields[field_id]);
    }
    Ok(out)
}

fn next_serial(txn: &WriteTxn<'_>, relation_id: u32, field_id: u32) -> Result<u64> {
    Ok(read_u64(
        &txn.dbs.data,
        &txn.txn,
        &serial_sequence_key(relation_id, field_id),
    )?
    .unwrap_or(1))
}

fn advance_serial_high_water(
    txn: &mut WriteTxn<'_>,
    relation_id: u32,
    field_id: u32,
    value: u64,
) -> Result<()> {
    let next = next_serial(txn, relation_id, field_id)?;
    if value >= next {
        let advanced = value
            .checked_add(1)
            .ok_or_else(|| Error::invalid_fact("serial sequence overflow"))?;
        write_u64(
            &txn.dbs.data,
            &mut txn.txn,
            &serial_sequence_key(relation_id, field_id),
            advanced,
        )?;
    }
    Ok(())
}

fn intern_value(
    txn: &mut WriteTxn<'_>,
    kind: u8,
    raw: &[u8],
    mode: InternMode,
) -> Result<Option<u64>> {
    if let Some(id) = lookup_intern_id(&txn.dbs.dict, &txn.txn, kind, raw)? {
        return Ok(Some(id));
    }
    if matches!(mode, InternMode::Existing) {
        return Ok(None);
    }
    let next = read_u64(&txn.dbs.meta, &txn.txn, META_NEXT_DICT_ID)?.unwrap_or(1);
    write_u64(&txn.dbs.meta, &mut txn.txn, META_NEXT_DICT_ID, next + 1)?;
    txn.dbs
        .dict
        .put(&mut txn.txn, &dict_fwd_key(kind, raw), &next.to_be_bytes())?;
    txn.dbs
        .dict
        .put(&mut txn.txn, &dict_rev_key(kind, next), raw)?;
    Ok(Some(next))
}

fn lookup_intern_id(
    db: &RawDatabase,
    txn: &heed::RoTxn<'_>,
    kind: u8,
    raw: &[u8],
) -> Result<Option<u64>> {
    let Some(bytes) = db.get(txn, &dict_fwd_key(kind, raw))? else {
        return Ok(None);
    };
    bytes_to_u64(bytes).map(Some)
}

fn lookup_intern_raw(db: RawDatabase, txn: &heed::RoTxn<'_>, kind: u8, id: u64) -> Result<Vec<u8>> {
    db.get(txn, &dict_rev_key(kind, id))?
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::corrupt("dictionary reverse entry missing"))
}

fn dict_fwd_key(kind: u8, raw: &[u8]) -> Vec<u8> {
    let mut key = vec![DICT_FWD, kind];
    key.extend_from_slice(raw);
    key
}

fn dict_rev_key(kind: u8, id: u64) -> Vec<u8> {
    let mut key = vec![DICT_REV, kind];
    key.extend_from_slice(&id.to_be_bytes());
    key
}
