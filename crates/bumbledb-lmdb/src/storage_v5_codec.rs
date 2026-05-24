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
#[cfg(test)]
use crate::Fact;
use crate::colt::KeyOwned;
use crate::storage_format::{FactHandle, fact_handle, serial_sequence_key};
use crate::{Error, FactView, RawDatabase, ReadTxn, Result, Value, ValueRef, WriteTxn};

#[derive(Clone, Copy)]
enum InternMode {
    Create,
    Existing,
}

pub(super) struct EncodedFact<'schema> {
    pub(super) relation_id: u32,
    pub(super) relation: &'schema RelationDescriptor,
    pub(super) fields: Vec<FieldRange>,
    pub(super) bytes: Vec<u8>,
    pub(super) handle: FactHandle,
}

#[derive(Clone, Copy)]
pub(super) struct FieldRange {
    start: usize,
    len: usize,
}

impl EncodedFact<'_> {
    pub(super) fn field_bytes(&self, field_id: usize) -> &[u8] {
        let range = self.fields[field_id];
        &self.bytes[range.start..range.start + range.len]
    }
}

pub(super) enum EncodeDelete<'schema> {
    Encoded(EncodedFact<'schema>),
    MissingDictionary,
}

pub(super) fn encode_insert_fact<'schema, F: FactView>(
    txn: &mut WriteTxn<'_>,
    schema: &'schema SchemaDescriptor,
    fact: &F,
) -> Result<EncodedFact<'schema>> {
    match encode_fact(txn, schema, fact, InternMode::Create)? {
        EncodeDelete::Encoded(encoded) => Ok(encoded),
        EncodeDelete::MissingDictionary => Err(Error::corrupt(
            "insert encoding returned missing dictionary in create mode",
        )),
    }
}

pub(super) fn encode_delete_fact<'schema, F: FactView>(
    txn: &mut WriteTxn<'_>,
    schema: &'schema SchemaDescriptor,
    fact: &F,
) -> Result<EncodeDelete<'schema>> {
    encode_fact(txn, schema, fact, InternMode::Existing)
}

fn encode_fact<'schema, F: FactView>(
    txn: &mut WriteTxn<'_>,
    schema: &'schema SchemaDescriptor,
    fact: &F,
    intern_mode: InternMode,
) -> Result<EncodeDelete<'schema>> {
    let relation_id = relation_id(schema, fact.relation())?;
    let relation = &schema.relations[relation_id as usize];
    let field_names: BTreeSet<_> = relation
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    let mut unknown_field = None;
    fact.for_each_field(|field| {
        if !field_names.contains(field) {
            unknown_field = Some(field.to_owned());
        }
    });
    if let Some(field) = unknown_field {
        return Err(Error::invalid_fact(format!(
            "unknown field {}.{}",
            relation.name, field
        )));
    }

    let mut encoded_fields = Vec::with_capacity(relation.fields.len());
    let mut fact_bytes = Vec::with_capacity(
        relation
            .fields
            .iter()
            .map(|field| field.value_type.encoded_width())
            .sum(),
    );
    for (field_id, field) in relation.fields.iter().enumerate() {
        let value = field_value(
            txn,
            fact,
            relation,
            relation_id,
            field_id,
            field,
            intern_mode,
        )?;
        let value = value.as_value();
        if !value.matches_type(&field.value_type) {
            return Err(Error::invalid_fact(format!(
                "field {}.{} has wrong type",
                relation.name, field.name
            )));
        }
        let start = fact_bytes.len();
        if !encode_value_into(txn, schema, field, value, intern_mode, &mut fact_bytes)? {
            return Ok(EncodeDelete::MissingDictionary);
        }
        if field.generation == FieldGeneration::SerialSequence
            && let ValueRef::Serial(value) = value
            && matches!(intern_mode, InternMode::Create)
        {
            advance_serial_high_water(txn, relation_id, field_id as u32, value)?;
        }
        encoded_fields.push(FieldRange {
            start,
            len: fact_bytes.len() - start,
        });
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

enum FieldValueRef<'fact> {
    Borrowed(ValueRef<'fact>),
    Generated(Value),
}

impl<'fact> FieldValueRef<'fact> {
    fn as_value(&'fact self) -> ValueRef<'fact> {
        match self {
            Self::Borrowed(value) => *value,
            Self::Generated(value) => ValueRef::from(value),
        }
    }
}

fn field_value<'fact, F: FactView>(
    txn: &mut WriteTxn<'_>,
    fact: &'fact F,
    relation: &RelationDescriptor,
    relation_id: u32,
    field_id: usize,
    field: &FieldDescriptor,
    intern_mode: InternMode,
) -> Result<FieldValueRef<'fact>> {
    match fact.value_ref(&field.name) {
        Some(value) => Ok(FieldValueRef::Borrowed(value)),
        None if field.generation == FieldGeneration::SerialSequence
            && matches!(intern_mode, InternMode::Create) =>
        {
            Ok(FieldValueRef::Generated(Value::Serial(next_serial(
                txn,
                relation_id,
                field_id as u32,
            )?)))
        }
        None => Err(Error::invalid_fact(format!(
            "missing field {}.{}",
            relation.name, field.name
        ))),
    }
}

fn encode_value_into(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    field: &FieldDescriptor,
    value: ValueRef<'_>,
    intern_mode: InternMode,
    out: &mut Vec<u8>,
) -> Result<bool> {
    Ok(match (value, &field.value_type) {
        (ValueRef::Bool(value), ValueType::Bool) => {
            out_extend(out, &encode_bool(value));
            true
        }
        (ValueRef::U64(value), ValueType::U64)
        | (ValueRef::Serial(value), ValueType::Serial { .. }) => {
            out_extend(out, &encode_u64(value));
            true
        }
        (ValueRef::I64(value), ValueType::I64) => {
            out_extend(out, &encode_i64(value));
            true
        }
        (ValueRef::Enum(value), ValueType::Enum { name }) => {
            encode_enum_value_into(schema, name, value, out)?;
            true
        }
        (ValueRef::String(value), ValueType::String) => {
            let Some(id) = intern_value(txn, DICT_STRING, value.as_bytes(), intern_mode)? else {
                return Ok(false);
            };
            out_extend(out, &encode_intern_id(InternId(id)));
            true
        }
        (ValueRef::Bytes(value), ValueType::Bytes) => {
            let Some(id) = intern_value(txn, DICT_BYTES, value, intern_mode)? else {
                return Ok(false);
            };
            out_extend(out, &encode_intern_id(InternId(id)));
            true
        }
        _ => return Err(Error::invalid_fact("value/type mismatch")),
    })
}

fn out_extend(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(bytes);
}

fn encode_enum_value_into(
    schema: &SchemaDescriptor,
    name: &str,
    value: u8,
    out: &mut Vec<u8>,
) -> Result<()> {
    if !schema.enum_contains_code(name, value) {
        return Err(Error::invalid_fact(format!(
            "enum {name} does not contain code {value}"
        )));
    }
    out.extend_from_slice(&encode_enum(value));
    Ok(())
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
) -> Result<Option<KeyOwned>> {
    Ok(Some(match (value, value_type) {
        (Value::Bool(value), ValueType::Bool) => KeyOwned::from_slice(&encode_bool(*value)),
        (Value::U64(value), ValueType::U64) | (Value::Serial(value), ValueType::Serial { .. }) => {
            KeyOwned::from_slice(&encode_u64(*value))
        }
        (Value::I64(value), ValueType::I64) => KeyOwned::from_slice(&encode_i64(*value)),
        (Value::Enum(value), ValueType::Enum { name }) => encode_enum_key(schema, name, *value)?,
        (Value::String(value), ValueType::String) => {
            let Some(id) = lookup_intern_id(txn.dbs.dict, &txn.txn, DICT_STRING, value.as_bytes())?
            else {
                return Ok(None);
            };
            KeyOwned::from_slice(&encode_intern_id(InternId(id)))
        }
        (Value::Bytes(value), ValueType::Bytes) => {
            let Some(id) = lookup_intern_id(txn.dbs.dict, &txn.txn, DICT_BYTES, value)? else {
                return Ok(None);
            };
            KeyOwned::from_slice(&encode_intern_id(InternId(id)))
        }
        _ => return Err(Error::invalid_query("input value/type mismatch")),
    }))
}

fn encode_enum_key(schema: &SchemaDescriptor, name: &str, value: u8) -> Result<KeyOwned> {
    if !schema.enum_contains_code(name, value) {
        return Err(Error::invalid_fact(format!(
            "enum {name} does not contain code {value}"
        )));
    }
    Ok(KeyOwned::from_slice(&encode_enum(value)))
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
        out.extend_from_slice(fact.field_bytes(field_id));
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
    if let Some(id) = lookup_intern_id(txn.dbs.dict, &txn.txn, kind, raw)? {
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
    db: RawDatabase,
    txn: &heed::RoTxn<'_>,
    kind: u8,
    raw: &[u8],
) -> Result<Option<u64>> {
    let Some(bytes) = db.get(txn, &dict_fwd_key(kind, raw))? else {
        return Ok(None);
    };
    let id = bytes_to_u64(bytes)?;
    let existing = lookup_intern_raw_ref(db, txn, kind, id)?;
    if existing != raw {
        return Err(Error::corrupt("dictionary hash collision"));
    }
    Ok(Some(id))
}

fn lookup_intern_raw(db: RawDatabase, txn: &heed::RoTxn<'_>, kind: u8, id: u64) -> Result<Vec<u8>> {
    Ok(lookup_intern_raw_ref(db, txn, kind, id)?.to_owned())
}

fn lookup_intern_raw_ref<'txn>(
    db: RawDatabase,
    txn: &'txn heed::RoTxn<'_>,
    kind: u8,
    id: u64,
) -> Result<&'txn [u8]> {
    db.get(txn, &dict_rev_key(kind, id))?
        .ok_or_else(|| Error::corrupt("dictionary reverse entry missing"))
}

fn dict_fwd_key(kind: u8, raw: &[u8]) -> [u8; 34] {
    let mut key = [0; 34];
    key[0] = DICT_FWD;
    key[1] = kind;
    key[2..].copy_from_slice(blake3::hash(raw).as_bytes());
    key
}

fn dict_rev_key(kind: u8, id: u64) -> [u8; 10] {
    let mut key = [0; 10];
    key[0] = DICT_REV;
    key[1] = kind;
    key[2..].copy_from_slice(&id.to_be_bytes());
    key
}
