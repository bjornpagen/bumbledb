use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::encoding::InternId;
use bumbledb_core::encoding::{
    DecimalRaw, TimestampMicros, UuidBytes, encode_bool, encode_decimal, encode_i64,
    encode_intern_id, encode_timestamp, encode_u64, encode_uuid,
};
use bumbledb_core::schema::{
    ConstraintDescriptor, CurrentIndexLayout, FieldDescriptor, RelationDescriptor,
    SchemaDescriptor, ValueType,
};

use crate::{Error, ReadTxn, Result, WriteTxn};

const NS_CURRENT_TUPLE: u8 = 0x10;
const NS_CURRENT_ROW: u8 = 0x11;
const NS_UNIQUE_GUARD: u8 = 0x20;
const NS_HISTORY: u8 = 0x30;

const DICT_FWD: u8 = 0x01;
const DICT_REV: u8 = 0x02;
const DICT_STRING: u8 = 0x01;
const DICT_BYTES: u8 = 0x02;

const NEXT_TX_ID_KEY: &[u8] = b"next_tx_id";

/// Compiled storage schema for the LMDB write/read layer.
#[derive(Clone, Debug)]
pub struct StorageSchema {
    descriptor: SchemaDescriptor,
    layouts: Vec<CurrentIndexLayout>,
}

impl StorageSchema {
    /// Builds storage metadata and validates generated index key lengths.
    pub fn new(descriptor: SchemaDescriptor, max_key_size: usize) -> Result<Self> {
        let layouts = descriptor.current_index_layouts(max_key_size)?;
        Ok(Self {
            descriptor,
            layouts,
        })
    }

    /// Returns the underlying schema descriptor.
    pub fn descriptor(&self) -> &SchemaDescriptor {
        &self.descriptor
    }

    /// Returns generated current index layouts.
    pub fn layouts(&self) -> &[CurrentIndexLayout] {
        &self.layouts
    }

    fn relation(&self, name: &str) -> Result<(u16, &RelationDescriptor)> {
        self.descriptor
            .relations
            .iter()
            .enumerate()
            .find(|(_, relation)| relation.name == name)
            .map(|(id, relation)| (id as u16, relation))
            .ok_or_else(|| Error::UnknownRelation {
                relation: name.to_owned(),
            })
    }

    fn layouts_for_relation(&self, relation_id: u16) -> impl Iterator<Item = &CurrentIndexLayout> {
        self.layouts
            .iter()
            .filter(move |layout| layout.relation_id == relation_id)
    }

    fn layout(&self, relation: &str, index: &str) -> Option<&CurrentIndexLayout> {
        self.layouts
            .iter()
            .find(|layout| layout.relation_name == relation && layout.index_name == index)
    }
}

/// A logical row for the generic storage layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    relation: String,
    values: BTreeMap<String, Value>,
}

impl Row {
    /// Creates a row for `relation`.
    pub fn new(
        relation: impl Into<String>,
        values: impl IntoIterator<Item = (impl Into<String>, Value)>,
    ) -> Self {
        Self {
            relation: relation.into(),
            values: values
                .into_iter()
                .map(|(field, value)| (field.into(), value))
                .collect(),
        }
    }

    /// Returns this row's relation name.
    pub fn relation(&self) -> &str {
        &self.relation
    }
}

/// Primary-key values for delete and row lookup operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyValues {
    relation: String,
    values: BTreeMap<String, Value>,
}

impl KeyValues {
    /// Creates primary-key values for `relation`.
    pub fn new(
        relation: impl Into<String>,
        values: impl IntoIterator<Item = (impl Into<String>, Value)>,
    ) -> Self {
        Self {
            relation: relation.into(),
            values: values
                .into_iter()
                .map(|(field, value)| (field.into(), value))
                .collect(),
        }
    }
}

/// Logical storage value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Boolean.
    Bool(bool),
    /// Unsigned 64-bit integer.
    U64(u64),
    /// Signed 64-bit integer.
    I64(i64),
    /// Typed ID represented as `u64`.
    Id(u64),
    /// Typed ref represented as `u64`.
    Ref(u64),
    /// UTC timestamp micros.
    Timestamp(TimestampMicros),
    /// Fixed-scale decimal raw value.
    Decimal(DecimalRaw),
    /// UUID bytes.
    Uuid(UuidBytes),
    /// Symbol represented as `u64`.
    Symbol(u64),
    /// String to intern.
    String(String),
    /// Bytes to intern.
    Bytes(Vec<u8>),
}

impl Value {
    fn kind_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "bool",
            Value::U64(_) => "u64",
            Value::I64(_) => "i64",
            Value::Id(_) => "id",
            Value::Ref(_) => "ref",
            Value::Timestamp(_) => "timestamp",
            Value::Decimal(_) => "decimal",
            Value::Uuid(_) => "uuid",
            Value::Symbol(_) => "symbol",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
        }
    }
}

#[derive(Clone, Debug)]
struct EncodedRow {
    fields: BTreeMap<String, Vec<u8>>,
}

impl EncodedRow {
    fn field(&self, relation: &RelationDescriptor, name: &str) -> Result<&[u8]> {
        self.fields
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::MissingField {
                relation: relation.name.clone(),
                field: name.to_owned(),
            })
    }

    fn payload(&self, relation: &RelationDescriptor) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for field in &relation.fields {
            out.extend_from_slice(self.field(relation, &field.name)?);
        }
        Ok(out)
    }

    fn from_payload(relation: &RelationDescriptor, payload: &[u8]) -> Result<Self> {
        let expected = relation
            .fields
            .iter()
            .map(|field| field.value_type.encoded_width())
            .sum::<usize>();
        if payload.len() != expected {
            return Err(Error::CorruptMetadata(
                "row payload width does not match schema",
            ));
        }

        let mut offset = 0;
        let mut fields = BTreeMap::new();
        for field in &relation.fields {
            let width = field.value_type.encoded_width();
            fields.insert(field.name.clone(), payload[offset..offset + width].to_vec());
            offset += width;
        }

        Ok(Self { fields })
    }
}

enum InternMode {
    Create,
    Existing,
}

impl WriteTxn<'_> {
    /// Allocates a generated ID for a relation and advances its persisted counter.
    pub fn alloc_id(&mut self, schema: &StorageSchema, relation_name: &str) -> Result<u64> {
        let (relation_id, relation) = schema.relation(relation_name)?;
        let generated = relation.generated_id.as_ref().ok_or_else(|| {
            Error::Internal(format!("relation {relation_name} has no generated ID"))
        })?;
        if relation.field(&generated.field).is_none() {
            return Err(Error::UnknownField {
                relation: relation.name.clone(),
                field: generated.field.clone(),
            });
        }

        let key = next_id_key(relation_id);
        let next = read_u64_meta(self, &key)?.unwrap_or(1);
        write_u64_meta(self, &key, next + 1)?;
        Ok(next)
    }

    /// Inserts a primary-keyed relation row.
    pub fn insert(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        self.insert_inner(schema, row)
    }

    /// Inserts a composite set/edge tuple.
    pub fn insert_tuple(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        self.insert_inner(schema, row)
    }

    /// Replaces an existing row by primary key.
    pub fn replace(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        let (relation_id, relation) = schema.relation(&row.relation)?;
        let new_encoded = self.encode_row(relation, &row, InternMode::Create)?;
        let primary = primary_bytes(relation, &new_encoded)?;
        let row_key = current_row_key(relation_id, &primary);
        let Some(old_payload) = self.dbs.index.get(&self.txn, row_key.as_slice())? else {
            return Err(Error::NotFound {
                relation: relation.name.clone(),
            });
        };
        let old_payload = old_payload.to_vec();
        let old_encoded = EncodedRow::from_payload(relation, &old_payload)?;

        self.check_foreign_keys(schema, relation, &new_encoded)?;
        self.check_unique_constraints(relation_id, relation, &new_encoded, &primary)?;

        self.delete_current_indexes(schema, relation_id, relation, &old_encoded)?;
        self.delete_unique_guards(relation_id, relation, &old_encoded)?;
        self.insert_current_indexes(schema, relation_id, relation, &new_encoded)?;
        self.insert_unique_guards(relation_id, relation, &new_encoded, &primary)?;
        self.dbs.index.put(
            &mut self.txn,
            row_key.as_slice(),
            new_encoded.payload(relation)?.as_slice(),
        )?;

        self.append_history(
            b'R',
            relation_id,
            &primary,
            Some(&old_payload),
            Some(&new_encoded.payload(relation)?),
        )?;
        Ok(())
    }

    /// Deletes an existing primary-keyed row.
    pub fn delete(&mut self, schema: &StorageSchema, key: KeyValues) -> Result<()> {
        self.delete_inner(schema, key)
    }

    /// Deletes an existing composite set/edge tuple.
    pub fn delete_tuple(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        let (_, relation) = schema.relation(&row.relation)?;
        let key_values = relation
            .primary_key
            .fields
            .iter()
            .map(|field| {
                row.values
                    .get(field)
                    .cloned()
                    .map(|value| (field.clone(), value))
                    .ok_or_else(|| Error::MissingField {
                        relation: relation.name.clone(),
                        field: field.clone(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        self.delete_inner(schema, KeyValues::new(row.relation, key_values))
    }

    fn insert_inner(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        let (relation_id, relation) = schema.relation(&row.relation)?;
        let encoded = self.encode_row(relation, &row, InternMode::Create)?;
        let primary = primary_bytes(relation, &encoded)?;
        let row_key = current_row_key(relation_id, &primary);

        if self.dbs.index.get(&self.txn, row_key.as_slice())?.is_some() {
            return Err(Error::DuplicateTuple {
                relation: relation.name.clone(),
            });
        }

        self.check_foreign_keys(schema, relation, &encoded)?;
        self.check_unique_constraints(relation_id, relation, &encoded, &primary)?;

        self.dbs.index.put(
            &mut self.txn,
            row_key.as_slice(),
            encoded.payload(relation)?.as_slice(),
        )?;
        self.insert_current_indexes(schema, relation_id, relation, &encoded)?;
        self.insert_unique_guards(relation_id, relation, &encoded, &primary)?;
        adjust_relation_row_count(self, relation_id, 1)?;
        self.append_history(
            b'I',
            relation_id,
            &primary,
            None,
            Some(&encoded.payload(relation)?),
        )?;
        Ok(())
    }

    fn delete_inner(&mut self, schema: &StorageSchema, key: KeyValues) -> Result<()> {
        let (relation_id, relation) = schema.relation(&key.relation)?;
        let primary = self.encode_primary_key(relation, &key.values, InternMode::Existing)?;
        let row_key = current_row_key(relation_id, &primary);
        let Some(old_payload) = self.dbs.index.get(&self.txn, row_key.as_slice())? else {
            return Err(Error::NotFound {
                relation: relation.name.clone(),
            });
        };
        let old_payload = old_payload.to_vec();
        let old_encoded = EncodedRow::from_payload(relation, &old_payload)?;

        self.check_delete_restrictions(schema, relation, &old_encoded)?;
        self.delete_current_indexes(schema, relation_id, relation, &old_encoded)?;
        self.delete_unique_guards(relation_id, relation, &old_encoded)?;
        self.dbs.index.delete(&mut self.txn, row_key.as_slice())?;
        adjust_relation_row_count(self, relation_id, -1)?;
        self.append_history(b'D', relation_id, &primary, Some(&old_payload), None)?;
        Ok(())
    }

    fn encode_row(
        &mut self,
        relation: &RelationDescriptor,
        row: &Row,
        mode: InternMode,
    ) -> Result<EncodedRow> {
        let known_fields = relation
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<BTreeSet<_>>();
        for field in row.values.keys() {
            if !known_fields.contains(field.as_str()) {
                return Err(Error::UnknownField {
                    relation: relation.name.clone(),
                    field: field.clone(),
                });
            }
        }

        let mut fields = BTreeMap::new();
        for field in &relation.fields {
            let value = row
                .values
                .get(&field.name)
                .ok_or_else(|| Error::MissingField {
                    relation: relation.name.clone(),
                    field: field.name.clone(),
                })?;
            fields.insert(
                field.name.clone(),
                self.encode_value(relation, field, value, &mode)?,
            );
        }
        Ok(EncodedRow { fields })
    }

    fn encode_primary_key(
        &mut self,
        relation: &RelationDescriptor,
        values: &BTreeMap<String, Value>,
        mode: InternMode,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for field_name in &relation.primary_key.fields {
            let field = relation
                .field(field_name)
                .ok_or_else(|| Error::UnknownField {
                    relation: relation.name.clone(),
                    field: field_name.clone(),
                })?;
            let value = values.get(field_name).ok_or_else(|| Error::MissingField {
                relation: relation.name.clone(),
                field: field_name.clone(),
            })?;
            out.extend_from_slice(&self.encode_value(relation, field, value, &mode)?);
        }
        Ok(out)
    }

    fn encode_value(
        &mut self,
        relation: &RelationDescriptor,
        field: &FieldDescriptor,
        value: &Value,
        mode: &InternMode,
    ) -> Result<Vec<u8>> {
        encode_value_with(relation, field, value, |kind, raw| match mode {
            InternMode::Create => self.intern_value(kind, raw),
            InternMode::Existing => {
                self.lookup_intern_value(kind, raw)?
                    .ok_or(Error::DictionaryValueNotFound {
                        kind: dict_kind_name(kind),
                    })
            }
        })
    }

    fn check_foreign_keys(
        &self,
        schema: &StorageSchema,
        relation: &RelationDescriptor,
        row: &EncodedRow,
    ) -> Result<()> {
        for field in &relation.fields {
            let ValueType::Ref {
                target_relation, ..
            } = &field.value_type
            else {
                continue;
            };
            let (target_relation_id, target) = schema.relation(target_relation)?;
            if target.primary_key.fields.len() != 1 {
                return Err(Error::UnsupportedCompositeForeignKey {
                    target_relation: target.name.clone(),
                });
            }

            let target_primary = row.field(relation, &field.name)?.to_vec();
            let key = current_row_key(target_relation_id, &target_primary);
            if self.dbs.index.get(&self.txn, key.as_slice())?.is_none() {
                return Err(Error::ForeignKeyViolation {
                    relation: relation.name.clone(),
                    field: field.name.clone(),
                    target_relation: target.name.clone(),
                });
            }
        }
        Ok(())
    }

    fn check_unique_constraints(
        &self,
        relation_id: u16,
        relation: &RelationDescriptor,
        row: &EncodedRow,
        primary: &[u8],
    ) -> Result<()> {
        for (constraint_id, constraint) in relation.constraints.iter().enumerate() {
            let ConstraintDescriptor::Unique { name, fields } = constraint;
            let key = unique_guard_key(relation_id, constraint_id as u16, relation, row, fields)?;
            if let Some(existing_primary) = self.dbs.index.get(&self.txn, key.as_slice())? {
                if existing_primary != primary {
                    return Err(Error::UniqueViolation {
                        relation: relation.name.clone(),
                        constraint: name.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn check_delete_restrictions(
        &self,
        schema: &StorageSchema,
        relation: &RelationDescriptor,
        row: &EncodedRow,
    ) -> Result<()> {
        if relation.primary_key.fields.len() != 1 {
            return Ok(());
        }
        let primary_field = &relation.primary_key.fields[0];
        let target_primary = row.field(relation, primary_field)?.to_vec();

        for (source_relation_id, source_relation) in schema
            .descriptor
            .relations
            .iter()
            .enumerate()
            .map(|(id, relation)| (id as u16, relation))
        {
            for field in &source_relation.fields {
                let ValueType::Ref {
                    target_relation, ..
                } = &field.value_type
                else {
                    continue;
                };
                if target_relation != &relation.name {
                    continue;
                }

                let Some(layout) =
                    schema.layout(&source_relation.name, &format!("by_{}", field.name))
                else {
                    continue;
                };
                let mut prefix = current_index_prefix(source_relation_id, layout.index_id);
                prefix.extend_from_slice(&target_primary);
                let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
                if iter.next().transpose()?.is_some() {
                    return Err(Error::RestrictViolation {
                        relation: relation.name.clone(),
                        referenced_by: source_relation.name.clone(),
                        field: field.name.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn insert_current_indexes(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        row: &EncodedRow,
    ) -> Result<()> {
        for layout in schema.layouts_for_relation(relation_id) {
            let key = current_index_key(layout, relation, row)?;
            self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
            adjust_index_entry_count(self, relation_id, layout.index_id, 1)?;
        }
        Ok(())
    }

    fn delete_current_indexes(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        row: &EncodedRow,
    ) -> Result<()> {
        for layout in schema.layouts_for_relation(relation_id) {
            let key = current_index_key(layout, relation, row)?;
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
            adjust_index_entry_count(self, relation_id, layout.index_id, -1)?;
        }
        Ok(())
    }

    fn insert_unique_guards(
        &mut self,
        relation_id: u16,
        relation: &RelationDescriptor,
        row: &EncodedRow,
        primary: &[u8],
    ) -> Result<()> {
        for (constraint_id, constraint) in relation.constraints.iter().enumerate() {
            let ConstraintDescriptor::Unique { fields, .. } = constraint;
            let key = unique_guard_key(relation_id, constraint_id as u16, relation, row, fields)?;
            self.dbs.index.put(&mut self.txn, key.as_slice(), primary)?;
        }
        Ok(())
    }

    fn delete_unique_guards(
        &mut self,
        relation_id: u16,
        relation: &RelationDescriptor,
        row: &EncodedRow,
    ) -> Result<()> {
        for (constraint_id, constraint) in relation.constraints.iter().enumerate() {
            let ConstraintDescriptor::Unique { fields, .. } = constraint;
            let key = unique_guard_key(relation_id, constraint_id as u16, relation, row, fields)?;
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
        }
        Ok(())
    }

    fn append_history(
        &mut self,
        op: u8,
        relation_id: u16,
        primary: &[u8],
        old: Option<&[u8]>,
        new: Option<&[u8]>,
    ) -> Result<()> {
        let tx_id = self.ensure_tx_id()?;
        let seq = self.history_seq;
        self.history_seq = self.history_seq.checked_add(1).ok_or_else(|| {
            Error::Internal("too many history records in one transaction".to_owned())
        })?;

        let mut key = vec![NS_HISTORY];
        push_u64(&mut key, tx_id);
        push_u32(&mut key, seq);

        let mut value = Vec::new();
        value.push(op);
        push_u16(&mut value, relation_id);
        push_bytes(&mut value, primary);
        push_optional_bytes(&mut value, old);
        push_optional_bytes(&mut value, new);

        self.dbs
            .index
            .put(&mut self.txn, key.as_slice(), value.as_slice())?;
        Ok(())
    }

    fn ensure_tx_id(&mut self) -> Result<u64> {
        if let Some(tx_id) = self.active_tx_id {
            return Ok(tx_id);
        }

        let next = read_u64_meta(self, NEXT_TX_ID_KEY)?.unwrap_or(1);
        write_u64_meta(self, NEXT_TX_ID_KEY, next + 1)?;
        self.active_tx_id = Some(next);
        Ok(next)
    }

    fn intern_value(&mut self, kind: u8, raw: &[u8]) -> Result<u64> {
        if let Some(id) = self.lookup_intern_value(kind, raw)? {
            return Ok(id);
        }

        let id_key = next_dict_id_key(kind);
        let id = read_u64_meta(self, &id_key)?.unwrap_or(1);
        write_u64_meta(self, &id_key, id + 1)?;

        let fwd_key = dict_fwd_key(kind, raw);
        let mut fwd_value = Vec::with_capacity(8 + raw.len());
        push_u64(&mut fwd_value, id);
        fwd_value.extend_from_slice(raw);
        self.dbs
            .dict
            .put(&mut self.txn, fwd_key.as_slice(), fwd_value.as_slice())?;
        self.dbs
            .dict
            .put(&mut self.txn, dict_rev_key(kind, id).as_slice(), raw)?;

        Ok(id)
    }

    fn lookup_intern_value(&self, kind: u8, raw: &[u8]) -> Result<Option<u64>> {
        lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)
    }
}

impl ReadTxn<'_> {
    /// Returns the last committed storage transaction ID.
    pub fn last_committed_tx_id(&self) -> Result<u64> {
        Ok(read_u64(&self.dbs.meta, &self.txn, NEXT_TX_ID_KEY)?.unwrap_or(1) - 1)
    }

    /// Returns the stored row count for a relation.
    pub fn relation_row_count(&self, schema: &StorageSchema, relation_name: &str) -> Result<u64> {
        let (relation_id, _) = schema.relation(relation_name)?;
        Ok(read_u64(
            &self.dbs.meta,
            &self.txn,
            &relation_row_count_key(relation_id),
        )?
        .unwrap_or(0))
    }

    /// Returns the stored index-entry count for a current index.
    pub fn index_entry_count(
        &self,
        schema: &StorageSchema,
        relation_name: &str,
        index_name: &str,
    ) -> Result<u64> {
        let layout = schema.layout(relation_name, index_name).ok_or_else(|| {
            Error::Internal(format!("missing index {relation_name}.{index_name}"))
        })?;
        Ok(read_u64(
            &self.dbs.meta,
            &self.txn,
            &index_entry_count_key(layout.relation_id, layout.index_id),
        )?
        .unwrap_or(0))
    }

    /// Counts history entries by scanning the history namespace.
    pub fn history_entry_count(&self) -> Result<usize> {
        let prefix = [NS_HISTORY];
        let mut iter = self.dbs.index.prefix_iter(&self.txn, &prefix[..])?;
        let mut count = 0;
        while iter.next().transpose()?.is_some() {
            count += 1;
        }
        Ok(count)
    }

    /// Checks whether a current covering index entry exists for a full row.
    pub fn current_index_entry_exists(
        &self,
        schema: &StorageSchema,
        row: &Row,
        index_name: &str,
    ) -> Result<bool> {
        let (_, relation) = schema.relation(&row.relation)?;
        let layout = schema.layout(&row.relation, index_name).ok_or_else(|| {
            Error::Internal(format!("missing index {}.{index_name}", row.relation))
        })?;
        let encoded = self.encode_row_existing(relation, row)?;
        let key = current_index_key(layout, relation, &encoded)?;
        Ok(self.dbs.index.get(&self.txn, key.as_slice())?.is_some())
    }

    /// Checks whether a row exists by primary key.
    pub fn row_exists(&self, schema: &StorageSchema, key: &KeyValues) -> Result<bool> {
        let (relation_id, relation) = schema.relation(&key.relation)?;
        let primary = self.encode_primary_key_existing(relation, &key.values)?;
        Ok(self
            .dbs
            .index
            .get(&self.txn, current_row_key(relation_id, &primary).as_slice())?
            .is_some())
    }

    /// Looks up an interned string ID.
    pub fn dictionary_string_id(&self, value: &str) -> Result<Option<u64>> {
        lookup_intern_value(&self.dbs.dict, &self.txn, DICT_STRING, value.as_bytes())
    }

    fn encode_row_existing(&self, relation: &RelationDescriptor, row: &Row) -> Result<EncodedRow> {
        let mut fields = BTreeMap::new();
        for field in &relation.fields {
            let value = row
                .values
                .get(&field.name)
                .ok_or_else(|| Error::MissingField {
                    relation: relation.name.clone(),
                    field: field.name.clone(),
                })?;
            fields.insert(
                field.name.clone(),
                encode_value_with(relation, field, value, |kind, raw| {
                    lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?.ok_or(
                        Error::DictionaryValueNotFound {
                            kind: dict_kind_name(kind),
                        },
                    )
                })?,
            );
        }
        Ok(EncodedRow { fields })
    }

    fn encode_primary_key_existing(
        &self,
        relation: &RelationDescriptor,
        values: &BTreeMap<String, Value>,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for field_name in &relation.primary_key.fields {
            let field = relation
                .field(field_name)
                .ok_or_else(|| Error::UnknownField {
                    relation: relation.name.clone(),
                    field: field_name.clone(),
                })?;
            let value = values.get(field_name).ok_or_else(|| Error::MissingField {
                relation: relation.name.clone(),
                field: field_name.clone(),
            })?;
            out.extend_from_slice(&encode_value_with(relation, field, value, |kind, raw| {
                lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?.ok_or(
                    Error::DictionaryValueNotFound {
                        kind: dict_kind_name(kind),
                    },
                )
            })?);
        }
        Ok(out)
    }
}

fn encode_value_with(
    relation: &RelationDescriptor,
    field: &FieldDescriptor,
    value: &Value,
    mut intern: impl FnMut(u8, &[u8]) -> Result<u64>,
) -> Result<Vec<u8>> {
    let bytes = match (&field.value_type, value) {
        (ValueType::Bool, Value::Bool(value)) => encode_bool(*value).to_vec(),
        (ValueType::U64, Value::U64(value)) => encode_u64(*value).to_vec(),
        (ValueType::I64, Value::I64(value)) => encode_i64(*value).to_vec(),
        (ValueType::Id { .. }, Value::Id(value)) => encode_u64(*value).to_vec(),
        (ValueType::Ref { .. }, Value::Ref(value)) => encode_u64(*value).to_vec(),
        (ValueType::TimestampMicros, Value::Timestamp(value)) => encode_timestamp(*value).to_vec(),
        (ValueType::Decimal { .. }, Value::Decimal(value)) => encode_decimal(*value).to_vec(),
        (ValueType::Uuid, Value::Uuid(value)) => encode_uuid(*value).to_vec(),
        (ValueType::Symbol { .. }, Value::Symbol(value)) => encode_u64(*value).to_vec(),
        (ValueType::String, Value::String(value)) => {
            encode_intern_id(InternId(intern(DICT_STRING, value.as_bytes())?)).to_vec()
        }
        (ValueType::Bytes, Value::Bytes(value)) => {
            encode_intern_id(InternId(intern(DICT_BYTES, value)?)).to_vec()
        }
        _ => {
            return Err(Error::TypeMismatch {
                relation: relation.name.clone(),
                field: field.name.clone(),
                expected: value_type_name(&field.value_type),
                actual: value.kind_name(),
            });
        }
    };

    Ok(bytes)
}

fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::Id { name, .. } => name.clone(),
        ValueType::Ref { name, .. } => name.clone(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Uuid => "uuid".to_owned(),
        ValueType::Symbol { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
    }
}

fn primary_bytes(relation: &RelationDescriptor, row: &EncodedRow) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for field in &relation.primary_key.fields {
        out.extend_from_slice(row.field(relation, field)?);
    }
    Ok(out)
}

fn current_row_key(relation_id: u16, primary: &[u8]) -> Vec<u8> {
    let mut key = vec![NS_CURRENT_ROW];
    push_u16(&mut key, relation_id);
    key.extend_from_slice(primary);
    key
}

fn current_index_prefix(relation_id: u16, index_id: u16) -> Vec<u8> {
    let mut key = vec![NS_CURRENT_TUPLE];
    push_u16(&mut key, relation_id);
    push_u16(&mut key, index_id);
    key
}

fn current_index_key(
    layout: &CurrentIndexLayout,
    relation: &RelationDescriptor,
    row: &EncodedRow,
) -> Result<Vec<u8>> {
    let mut key = current_index_prefix(layout.relation_id, layout.index_id);
    for component in &layout.components {
        key.extend_from_slice(row.field(relation, &component.field_name)?);
    }
    Ok(key)
}

fn unique_guard_key(
    relation_id: u16,
    constraint_id: u16,
    relation: &RelationDescriptor,
    row: &EncodedRow,
    fields: &[String],
) -> Result<Vec<u8>> {
    let mut key = vec![NS_UNIQUE_GUARD];
    push_u16(&mut key, relation_id);
    push_u16(&mut key, constraint_id);
    for field in fields {
        key.extend_from_slice(row.field(relation, field)?);
    }
    Ok(key)
}

fn read_u64_meta(txn: &WriteTxn<'_>, key: &[u8]) -> Result<Option<u64>> {
    read_u64(&txn.dbs.meta, &txn.txn, key)
}

fn write_u64_meta(txn: &mut WriteTxn<'_>, key: &[u8], value: u64) -> Result<()> {
    write_u64(&txn.dbs.meta, &mut txn.txn, key, value)
}

fn read_u64(db: &crate::RawDatabase, txn: &heed::RoTxn, key: &[u8]) -> Result<Option<u64>> {
    let Some(bytes) = db.get(txn, key)? else {
        return Ok(None);
    };
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| Error::CorruptMetadata("u64 metadata must be eight bytes"))?;
    Ok(Some(u64::from_be_bytes(bytes)))
}

fn write_u64(db: &crate::RawDatabase, txn: &mut heed::RwTxn, key: &[u8], value: u64) -> Result<()> {
    let bytes = value.to_be_bytes();
    Ok(db.put(txn, key, &bytes[..])?)
}

fn adjust_relation_row_count(txn: &mut WriteTxn<'_>, relation_id: u16, delta: i64) -> Result<()> {
    adjust_u64_meta(txn, &relation_row_count_key(relation_id), delta)
}

fn adjust_index_entry_count(
    txn: &mut WriteTxn<'_>,
    relation_id: u16,
    index_id: u16,
    delta: i64,
) -> Result<()> {
    adjust_u64_meta(txn, &index_entry_count_key(relation_id, index_id), delta)
}

fn adjust_u64_meta(txn: &mut WriteTxn<'_>, key: &[u8], delta: i64) -> Result<()> {
    let current = read_u64_meta(txn, key)?.unwrap_or(0);
    let next = if delta >= 0 {
        current
            .checked_add(delta as u64)
            .ok_or_else(|| Error::Internal("metadata counter overflow".to_owned()))?
    } else {
        current
            .checked_sub(delta.unsigned_abs())
            .ok_or_else(|| Error::Internal("metadata counter underflow".to_owned()))?
    };
    write_u64_meta(txn, key, next)
}

fn next_id_key(relation_id: u16) -> Vec<u8> {
    let mut key = b"next_id:".to_vec();
    push_u16(&mut key, relation_id);
    key
}

fn relation_row_count_key(relation_id: u16) -> Vec<u8> {
    let mut key = b"stats:rows:".to_vec();
    push_u16(&mut key, relation_id);
    key
}

fn index_entry_count_key(relation_id: u16, index_id: u16) -> Vec<u8> {
    let mut key = b"stats:index:".to_vec();
    push_u16(&mut key, relation_id);
    push_u16(&mut key, index_id);
    key
}

fn next_dict_id_key(kind: u8) -> Vec<u8> {
    vec![
        b'd', b'i', b'c', b't', b':', b'n', b'e', b'x', b't', b':', kind,
    ]
}

fn dict_fwd_key(kind: u8, raw: &[u8]) -> Vec<u8> {
    let mut key = vec![DICT_FWD, kind];
    key.extend_from_slice(blake3::hash(raw).as_bytes());
    key
}

fn dict_rev_key(kind: u8, id: u64) -> Vec<u8> {
    let mut key = vec![DICT_REV, kind];
    push_u64(&mut key, id);
    key
}

fn lookup_intern_value(
    db: &crate::RawDatabase,
    txn: &heed::RoTxn,
    kind: u8,
    raw: &[u8],
) -> Result<Option<u64>> {
    let Some(value) = db.get(txn, dict_fwd_key(kind, raw).as_slice())? else {
        return Ok(None);
    };
    if value.len() < 8 {
        return Err(Error::CorruptMetadata("dictionary forward value too short"));
    }
    let id = u64::from_be_bytes(
        value[..8]
            .try_into()
            .map_err(|_| Error::CorruptMetadata("dictionary ID width invalid"))?,
    );
    if &value[8..] != raw {
        return Err(Error::HashCollision {
            kind: dict_kind_name(kind),
        });
    }
    Ok(Some(id))
}

fn dict_kind_name(kind: u8) -> &'static str {
    match kind {
        DICT_STRING => "string",
        DICT_BYTES => "bytes",
        _ => "unknown",
    }
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn push_optional_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            push_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;
    use bumbledb_core::schema::{
        ConstraintDescriptor, FieldDescriptor, GeneratedIdDescriptor, PrimaryKeyDescriptor,
        RelationKind,
    };

    #[test]
    fn inserts_rows_indexes_history_stats_and_reopens() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = storage_schema(&env);

        env.write(|txn| {
            let holder = txn.alloc_id(&schema, "Holder")?;
            let account = txn.alloc_id(&schema, "Account")?;
            assert_eq!(holder, 1);
            assert_eq!(account, 1);

            txn.insert(&schema, holder_row(holder, "Alice"))?;
            txn.insert(&schema, account_row(account, holder, 840))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 1);
            assert_eq!(txn.history_entry_count()?, 2);
            assert_eq!(txn.relation_row_count(&schema, "Holder")?, 1);
            assert_eq!(txn.relation_row_count(&schema, "Account")?, 1);
            assert_eq!(txn.index_entry_count(&schema, "Holder", "primary")?, 1);
            assert_eq!(txn.index_entry_count(&schema, "Holder", "unique_name")?, 1);
            assert_eq!(txn.index_entry_count(&schema, "Account", "primary")?, 1);
            assert_eq!(txn.index_entry_count(&schema, "Account", "by_holder")?, 1);
            assert_eq!(
                txn.index_entry_count(&schema, "Account", "unique_holder_currency")?,
                1
            );
            assert!(txn.current_index_entry_exists(&schema, &holder_row(1, "Alice"), "primary")?);
            assert!(txn.current_index_entry_exists(
                &schema,
                &account_row(1, 1, 840),
                "by_holder"
            )?);
            assert!(txn.dictionary_string_id("Alice")?.is_some());
            Ok::<(), Error>(())
        })
        .unwrap();

        drop(env);
        let env = Environment::open(dir.path()).unwrap();
        let schema = storage_schema(&env);
        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 1);
            assert_eq!(txn.relation_row_count(&schema, "Holder")?, 1);
            assert!(txn.row_exists(&schema, &holder_key(1))?);
            assert!(txn.dictionary_string_id("Alice")?.is_some());
            Ok::<(), Error>(())
        })
        .unwrap();

        env.write(|txn| {
            assert_eq!(txn.alloc_id(&schema, "Holder")?, 2);
            Err::<(), Error>(Error::Internal("rollback counter check".to_owned()))
        })
        .unwrap_err();
    }

    #[test]
    fn duplicate_unique_and_foreign_key_failures_abort_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = storage_schema(&env);

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        let duplicate = env.write(|txn| txn.insert(&schema, holder_row(1, "Bob")));
        assert!(matches!(duplicate, Err(Error::DuplicateTuple { .. })));

        let unique = env.write(|txn| txn.insert(&schema, holder_row(2, "Alice")));
        assert!(matches!(unique, Err(Error::UniqueViolation { .. })));

        let fk = env.write(|txn| txn.insert(&schema, account_row(1, 999, 840)));
        assert!(matches!(fk, Err(Error::ForeignKeyViolation { .. })));

        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 1);
            assert_eq!(txn.history_entry_count()?, 1);
            assert_eq!(txn.relation_row_count(&schema, "Holder")?, 1);
            assert_eq!(txn.relation_row_count(&schema, "Account")?, 0);
            assert_eq!(txn.dictionary_string_id("Bob")?, None);
            Ok::<(), Error>(())
        })
        .unwrap();
    }

    #[test]
    fn replace_removes_old_current_entries_and_preserves_counts() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = storage_schema(&env);

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        env.write(|txn| {
            txn.replace(&schema, account_row(1, 1, 978))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 2);
            assert_eq!(txn.history_entry_count()?, 3);
            assert_eq!(txn.relation_row_count(&schema, "Account")?, 1);
            assert_eq!(txn.index_entry_count(&schema, "Account", "primary")?, 1);
            assert!(!txn.current_index_entry_exists(
                &schema,
                &account_row(1, 1, 840),
                "primary"
            )?);
            assert!(txn.current_index_entry_exists(&schema, &account_row(1, 1, 978), "primary")?);
            Ok::<(), Error>(())
        })
        .unwrap();

        env.write(|txn| {
            txn.insert(&schema, account_row(2, 1, 840))?;
            Ok::<(), Error>(())
        })
        .unwrap();
    }

    #[test]
    fn deletes_restrict_then_remove_indexes_and_rows() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = storage_schema(&env);

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        let restricted = env.write(|txn| txn.delete(&schema, holder_key(1)));
        assert!(matches!(restricted, Err(Error::RestrictViolation { .. })));

        env.write(|txn| {
            txn.delete(&schema, account_key(1))?;
            txn.delete(&schema, holder_key(1))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 2);
            assert_eq!(txn.history_entry_count()?, 4);
            assert_eq!(txn.relation_row_count(&schema, "Holder")?, 0);
            assert_eq!(txn.relation_row_count(&schema, "Account")?, 0);
            assert!(!txn.row_exists(&schema, &holder_key(1))?);
            assert_eq!(txn.index_entry_count(&schema, "Account", "by_holder")?, 0);
            Ok::<(), Error>(())
        })
        .unwrap();
    }

    #[test]
    fn composite_tuples_insert_duplicate_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = storage_schema(&env);

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            txn.insert_tuple(&schema, tag_row(1, 7))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        let duplicate = env.write(|txn| txn.insert_tuple(&schema, tag_row(1, 7)));
        assert!(matches!(duplicate, Err(Error::DuplicateTuple { .. })));

        env.write(|txn| {
            txn.delete_tuple(&schema, tag_row(1, 7))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        env.read(|txn| {
            assert_eq!(txn.relation_row_count(&schema, "AccountTag")?, 0);
            assert_eq!(txn.index_entry_count(&schema, "AccountTag", "primary")?, 0);
            assert_eq!(
                txn.index_entry_count(&schema, "AccountTag", "by_account")?,
                0
            );
            Ok::<(), Error>(())
        })
        .unwrap();
    }

    fn storage_schema(env: &Environment) -> StorageSchema {
        StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap()
    }

    fn ledger_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "LedgerDb",
            vec![
                RelationDescriptor::new(
                    "Holder",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "HolderId".to_owned(),
                                relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                    bumbledb_core::schema::PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id"))
                .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
                RelationDescriptor::new(
                    "Account",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "AccountId".to_owned(),
                                relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "holder",
                            ValueType::Ref {
                                name: "HolderId".to_owned(),
                                target_relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Symbol {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id"))
                .with_constraint(ConstraintDescriptor::unique(
                    "holder_currency",
                    ["holder", "currency"],
                )),
                RelationDescriptor::new(
                    "AccountTag",
                    RelationKind::Edge,
                    vec![
                        FieldDescriptor::new(
                            "account",
                            ValueType::Ref {
                                name: "AccountId".to_owned(),
                                target_relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "tag",
                            ValueType::Symbol {
                                name: "Tag".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["account", "tag"]),
                ),
            ],
        )
    }

    fn holder_row(id: u64, name: &str) -> Row {
        Row::new(
            "Holder",
            [
                ("id", Value::Id(id)),
                ("name", Value::String(name.to_owned())),
            ],
        )
    }

    fn account_row(id: u64, holder: u64, currency: u64) -> Row {
        Row::new(
            "Account",
            [
                ("id", Value::Id(id)),
                ("holder", Value::Ref(holder)),
                ("currency", Value::Symbol(currency)),
            ],
        )
    }

    fn tag_row(account: u64, tag: u64) -> Row {
        Row::new(
            "AccountTag",
            [
                ("account", Value::Ref(account)),
                ("tag", Value::Symbol(tag)),
            ],
        )
    }

    fn holder_key(id: u64) -> KeyValues {
        KeyValues::new("Holder", [("id", Value::Id(id))])
    }

    fn account_key(id: u64) -> KeyValues {
        KeyValues::new("Account", [("id", Value::Id(id))])
    }
}
