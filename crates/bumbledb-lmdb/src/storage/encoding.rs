use super::*;

pub(super) fn encode_value_with(
    relation: &RelationDescriptor,
    field: &FieldDescriptor,
    value: &Value,
    mut intern: impl FnMut(u8, &[u8]) -> Result<u64>,
) -> Result<Vec<u8>> {
    if !storage_value_matches_type(value, &field.value_type) {
        return Err(Error::type_mismatch(
            &relation.name,
            &field.name,
            value_type_name(&field.value_type),
            value.kind_name(),
        ));
    }
    encode_value_for_type(&field.value_type, value, &mut intern)
}

pub(super) fn encode_value_for_type(
    value_type: &ValueType,
    value: &Value,
    mut intern: impl FnMut(u8, &[u8]) -> Result<u64>,
) -> Result<Vec<u8>> {
    let bytes = match (value_type, value) {
        (ValueType::Bool, Value::Bool(value)) => encode_bool(*value).to_vec(),
        (ValueType::U64, Value::U64(value)) => encode_u64(*value).to_vec(),
        (ValueType::I64, Value::I64(value)) => encode_i64(*value).to_vec(),
        (ValueType::Serial { .. }, Value::Serial(value)) => encode_u64(*value).to_vec(),
        (ValueType::TimestampMicros, Value::Timestamp(value)) => encode_timestamp(*value).to_vec(),
        (ValueType::Decimal { .. }, Value::Decimal(value)) => encode_decimal(*value).to_vec(),
        (ValueType::Enum { .. }, Value::Enum(value)) => encode_enum(*value).to_vec(),
        (ValueType::String, Value::String(value)) => {
            encode_intern_id(InternId(intern(DICT_STRING, value.as_bytes())?)).to_vec()
        }
        (ValueType::Bytes, Value::Bytes(value)) => {
            encode_intern_id(InternId(intern(DICT_BYTES, value)?)).to_vec()
        }
        _ => {
            return Err(Error::internal(format!(
                "query value type mismatch: expected {}, found {}",
                value_type_name(value_type),
                value.kind_name()
            )));
        }
    };

    Ok(bytes)
}

pub(super) fn validate_fact_values(
    schema: &SchemaDescriptor,
    relation: &RelationDescriptor,
    fact: &Fact,
) -> Result<()> {
    for (field_name, value) in fact.values() {
        let Some(field) = relation.field(field_name) else {
            continue;
        };
        validate_enum_value(schema, relation, field, value)?;
    }
    Ok(())
}

pub(super) fn validate_enum_value(
    schema: &SchemaDescriptor,
    relation: &RelationDescriptor,
    field: &FieldDescriptor,
    value: &Value,
) -> Result<()> {
    let (ValueType::Enum { name }, Value::Enum(code)) = (&field.value_type, value) else {
        return Ok(());
    };
    if schema.enum_contains_code(name, *code) {
        Ok(())
    } else {
        Err(Error::type_mismatch(
            &relation.name,
            &field.name,
            format!("known variant of {name}"),
            value.kind_name(),
        ))
    }
}

pub(super) fn storage_value_matches_type(value: &Value, value_type: &ValueType) -> bool {
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (Value::Serial(_), ValueType::Serial { .. })
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Enum(_), ValueType::Enum { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}

#[cfg(test)]
pub(super) fn decode_access_scan_entry(
    dict: crate::RawDatabase,
    index_db: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation: &RelationDescriptor,
    layout: &AccessLayout,
    key: &[u8],
) -> Result<FactCursorRecord> {
    let (encoded, encoded_components) = decode_access_key(index_db, txn, relation, layout, key)?;
    let fact = decode_encoded_fact(dict, txn, relation, &encoded)?;
    Ok(FactCursorRecord {
        fact,
        encoded_components,
    })
}

#[cfg(test)]
pub(super) fn decode_access_key(
    index_db: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation: &RelationDescriptor,
    layout: &AccessLayout,
    key: &[u8],
) -> Result<(EncodedFact, Vec<EncodedComponent>)> {
    let prefix_len = access_prefix(layout.relation_id, layout.index_id).len();
    if key.len() != layout.encoded_len {
        return Err(Error::corrupt("index key width does not match layout"));
    }
    if key.get(0..prefix_len) != Some(access_prefix(layout.relation_id, layout.index_id).as_slice())
    {
        return Err(Error::corrupt("index key prefix does not match layout"));
    }

    let mut components = Vec::with_capacity(layout.components.len());
    let mut offset = prefix_len;

    for component in &layout.components {
        let end = offset + component.encoded_width;
        let bytes = key
            .get(offset..end)
            .ok_or_else(|| Error::corrupt("index key component is truncated"))?
            .to_vec();
        let (_, _, width) = field_layout_with_id(relation, &component.field_name)?;
        if width != component.encoded_width {
            return Err(Error::corrupt(
                "index key component width does not match field",
            ));
        }
        components.push(EncodedComponent {
            field_name: component.field_name.clone(),
            bytes,
        });
        offset = end;
    }

    let id = key
        .get(offset..offset + FACT_ID_BYTES)
        .ok_or_else(|| Error::corrupt("access key fact id truncated"))?;
    offset += FACT_ID_BYTES;
    if offset != key.len() {
        return Err(Error::corrupt("access key has trailing bytes"));
    }
    let fact = lookup_fact_by_id(index_db, txn, layout.relation_id, id)?;

    Ok((fact, components))
}

#[cfg(test)]
pub(super) fn decode_encoded_fact(
    dict: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation: &RelationDescriptor,
    encoded: &EncodedFact,
) -> Result<Fact> {
    let mut values = BTreeMap::new();
    for field in &relation.fields {
        let bytes = encoded.field(relation, &field.name)?;
        values.insert(
            field.name.clone(),
            decode_value(dict, txn, &field.value_type, bytes)?,
        );
    }
    Ok(Fact {
        relation: relation.name.clone(),
        values,
    })
}

pub(super) fn decode_value(
    dict: crate::RawDatabase,
    txn: &heed::RoTxn,
    value_type: &ValueType,
    bytes: &[u8],
) -> Result<Value> {
    let value = match value_type {
        ValueType::Bool => {
            Value::Bool(decode_bool(bytes).map_err(|_| Error::corrupt("bool width invalid"))?)
        }
        ValueType::U64 => {
            Value::U64(decode_u64(bytes).map_err(|_| Error::corrupt("u64 width invalid"))?)
        }
        ValueType::I64 => {
            Value::I64(decode_i64(bytes).map_err(|_| Error::corrupt("i64 width invalid"))?)
        }
        ValueType::Serial { .. } => {
            Value::Serial(decode_u64(bytes).map_err(|_| Error::corrupt("serial width invalid"))?)
        }
        ValueType::TimestampMicros => Value::Timestamp(
            decode_timestamp(bytes).map_err(|_| Error::corrupt("timestamp width invalid"))?,
        ),
        ValueType::Decimal { .. } => Value::Decimal(
            decode_decimal(bytes).map_err(|_| Error::corrupt("decimal width invalid"))?,
        ),
        ValueType::Enum { .. } => {
            Value::Enum(decode_enum(bytes).map_err(|_| Error::corrupt("enum width invalid"))?)
        }
        ValueType::String => {
            let InternId(id) = decode_intern_id(bytes)
                .map_err(|_| Error::corrupt("string intern ID width invalid"))?;
            let raw = lookup_intern_raw_by_id(dict, txn, DICT_STRING, id)?;
            Value::String(
                String::from_utf8(raw).map_err(|_| Error::invalid_utf8_dictionary_string())?,
            )
        }
        ValueType::Bytes => {
            let InternId(id) = decode_intern_id(bytes)
                .map_err(|_| Error::corrupt("bytes intern ID width invalid"))?;
            Value::Bytes(lookup_intern_raw_by_id(dict, txn, DICT_BYTES, id)?)
        }
    };
    Ok(value)
}

pub(super) fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Enum { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
        ValueType::Serial {
            type_name,
            owning_relation,
        } => format!("{type_name}@{owning_relation}"),
    }
}

pub(super) fn fact_width(relation: &RelationDescriptor) -> usize {
    relation
        .fields
        .iter()
        .map(|field| field.value_type.encoded_width())
        .sum()
}

pub(super) fn field_layout(relation: &RelationDescriptor, name: &str) -> Result<(usize, usize)> {
    let (_, offset, width) = field_layout_with_id(relation, name)?;
    Ok((offset, width))
}

pub(super) fn field_layout_with_id(
    relation: &RelationDescriptor,
    name: &str,
) -> Result<(usize, usize, usize)> {
    let mut offset = 0;
    for (field_id, field) in relation.fields.iter().enumerate() {
        let width = field.value_type.encoded_width();
        if field.name == name {
            return Ok((field_id, offset, width));
        }
        offset += width;
    }
    Err(Error::missing_field(&relation.name, name))
}

pub(super) fn target_unique_constraint<'a>(
    relation: &'a RelationDescriptor,
    name: &str,
) -> Result<(usize, &'a [String])> {
    relation
        .constraints
        .iter()
        .enumerate()
        .find_map(|(index, constraint)| match constraint {
            ConstraintDescriptor::Unique {
                name: constraint_name,
                fields,
                ..
            } if constraint_name == name => Some((index, fields.as_slice())),
            ConstraintDescriptor::Unique { .. } | ConstraintDescriptor::ForeignKey { .. } => None,
        })
        .ok_or_else(|| {
            Error::internal(format!(
                "relation {} has no unique constraint {name}",
                relation.name
            ))
        })
}
