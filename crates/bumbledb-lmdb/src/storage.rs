use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::encoding::{
    DecimalRaw, InternId, TimestampMicros, decode_bool, decode_decimal, decode_enum, decode_i64,
    decode_intern_id, decode_timestamp, decode_u64, encode_bool, encode_decimal, encode_enum,
    encode_i64, encode_intern_id, encode_timestamp, encode_u64,
};
use bumbledb_core::schema::{
    AccessComponent, AccessLayout, ConstraintDescriptor, FieldDescriptor, RelationDescriptor,
    SchemaDescriptor, ValueType,
};

#[cfg(test)]
use crate::storage_schema::FACT_SET_ACCESS_NAME;
use crate::{Error, ReadTxn, RelationId, Result, StorageSchema, WriteTxn};

const NS_CANONICAL_FACT: u8 = 0x10;
const NS_FACT_ID: u8 = 0x12;
const NS_ACCESS_ENTRY: u8 = 0x11;
const NS_UNIQUE_ENTRY: u8 = 0x13;
const NS_REVERSE_FK_ENTRY: u8 = 0x14;
const FACT_ID_BYTES: usize = 16;
const DICT_FWD: u8 = 0x01;
const DICT_REV: u8 = 0x02;
const DICT_STRING: u8 = 0x01;
const DICT_BYTES: u8 = 0x02;

const NEXT_TX_ID_KEY: &[u8] = b"next_tx_id";

/// A logical fact for the generic storage layer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact {
    relation: String,
    values: BTreeMap<String, Value>,
}

impl Fact {
    /// Creates a fact for `relation`.
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

    /// Returns this fact's relation name.
    pub fn relation(&self) -> &str {
        &self.relation
    }

    /// Returns a field value.
    pub fn value(&self, field: &str) -> Option<&Value> {
        self.values.get(field)
    }

    /// Returns all fact values keyed by field name.
    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }
}

/// Field values used to build an index prefix.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldValues {
    relation: String,
    values: BTreeMap<String, Value>,
}

#[cfg(test)]
impl FieldValues {
    /// Creates index-prefix field values for `relation`.
    fn new(
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// Boolean.
    Bool(bool),
    /// Unsigned 64-bit integer.
    U64(u64),
    /// Signed 64-bit integer.
    I64(i64),
    /// Typed nominal serial.
    Serial(u64),
    /// UTC timestamp micros.
    Timestamp(TimestampMicros),
    /// Fixed-scale decimal raw value.
    Decimal(DecimalRaw),
    /// Closed enum represented as a stable one-byte code.
    Enum(u8),
    /// String to intern.
    String(String),
    /// Bytes to intern.
    Bytes(Vec<u8>),
}

impl Value {
    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "bool",
            Value::U64(_) => "u64",
            Value::I64(_) => "i64",
            Value::Serial(_) => "serial",
            Value::Timestamp(_) => "timestamp",
            Value::Decimal(_) => "decimal",
            Value::Enum(_) => "enum",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
        }
    }
}

/// Encoded component from an access key.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedComponent {
    /// Field name.
    pub field_name: String,
    /// Encoded bytes for this field in the index key.
    pub bytes: Vec<u8>,
}

/// A fact yielded from an index scan.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FactCursorRecord {
    /// Decoded logical fact.
    pub fact: Fact,
    /// Encoded components in index-key order.
    pub encoded_components: Vec<EncodedComponent>,
}

/// Result of inserting a fact into a relation-as-set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    /// The fact was newly inserted.
    Inserted,
    /// The exact fact was already present and no storage state changed.
    AlreadyPresent,
}

/// Result of deleting an exact fact from a relation-as-set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    /// The fact was present and deleted.
    Deleted,
    /// The exact fact was absent and no storage state changed.
    Absent,
}

#[cfg(test)]
impl FactCursorRecord {
    /// Returns an encoded component by field name.
    fn encoded_component(&self, field: &str) -> Option<&[u8]> {
        self.encoded_components
            .iter()
            .find(|component| component.field_name == field)
            .map(|component| component.bytes.as_slice())
    }
}

/// Encoded fact component view yielded from an access scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedAccessItem {
    key: Vec<u8>,
    prefix_len: usize,
}

impl EncodedAccessItem {
    /// Returns the encoded index key bytes.
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// Returns an encoded component by ordinal.
    pub fn component(&self, components: &[AccessComponent], index: usize) -> Option<&[u8]> {
        let mut offset = self.prefix_len;
        for component in components.get(..index)? {
            offset += component.encoded_width;
        }
        let width = components.get(index)?.encoded_width;
        self.key.get(offset..offset + width)
    }
}

/// Transaction-scoped scan over one current access path.
#[cfg(test)]
pub(crate) struct FactCursor<'borrow, 'env, 'schema> {
    iter: heed::RoPrefix<'borrow, heed::types::Bytes, heed::types::Bytes>,
    txn: &'borrow heed::RoTxn<'env, heed::WithoutTls>,
    index_db: crate::RawDatabase,
    dict: crate::RawDatabase,
    relation: &'schema RelationDescriptor,
    layout: &'schema AccessLayout,
    range: Option<EncodedRange>,
}

/// Transaction-scoped encoded scan over one current access path.
pub(crate) struct EncodedFactCursor<'borrow, 'env, 'schema> {
    iter: heed::RoPrefix<'borrow, heed::types::Bytes, heed::types::Bytes>,
    layout: &'schema AccessLayout,
    index_prefix: Vec<u8>,
    _env: std::marker::PhantomData<&'env ()>,
}

#[cfg(test)]
#[derive(Clone, Debug)]
struct EncodedRange {
    offset: usize,
    width: usize,
    start: Option<Vec<u8>>,
    end: Option<Vec<u8>>,
}

#[cfg(test)]
impl Iterator for FactCursor<'_, '_, '_> {
    type Item = Result<FactCursorRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (key, _) = match self.iter.next()? {
                Ok(item) => item,
                Err(error) => return Some(Err(error.into())),
            };

            if !self.range_matches(key) {
                continue;
            }

            return Some(decode_access_scan_entry(
                self.dict,
                self.index_db,
                self.txn,
                self.relation,
                self.layout,
                key,
            ));
        }
    }
}

impl Iterator for EncodedFactCursor<'_, '_, '_> {
    type Item = Result<EncodedAccessItem>;

    fn next(&mut self) -> Option<Self::Item> {
        let (key, _) = match self.iter.next()? {
            Ok(item) => item,
            Err(error) => return Some(Err(error.into())),
        };
        Some(encoded_access_item(self.layout, &self.index_prefix, key))
    }
}

#[cfg(test)]
impl FactCursor<'_, '_, '_> {
    fn range_matches(&self, key: &[u8]) -> bool {
        let Some(range) = &self.range else {
            return true;
        };
        let Some(value) = key.get(range.offset..range.offset + range.width) else {
            return false;
        };
        if let Some(start) = &range.start
            && value < start.as_slice()
        {
            return false;
        }
        if let Some(end) = &range.end
            && value >= end.as_slice()
        {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EncodedFact {
    relation: RelationId,
    bytes: Vec<u8>,
}

impl EncodedFact {
    fn field(&self, relation: &RelationDescriptor, name: &str) -> Result<&[u8]> {
        let (offset, width) = field_layout(relation, name)?;
        self.bytes
            .get(offset..offset + width)
            .ok_or_else(|| Error::corrupt("encoded fact width does not match schema"))
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

enum InternMode {
    Create,
    Existing,
}

impl WriteTxn<'_> {
    /// Bulk-loads facts in deterministic schema relation order.
    ///
    /// This is one write transaction: any constraint failure aborts all current
    /// facts, indexes, stats, counters, and dictionary inserts made by
    /// the attempted load.
    pub fn bulk_load(
        &mut self,
        schema: &StorageSchema,
        facts: impl IntoIterator<Item = Fact>,
    ) -> Result<usize> {
        let _span = tracing::debug_span!("bumbledb.storage.bulk_load").entered();
        let mut facts = facts.into_iter().collect::<Vec<_>>();
        tracing::debug!(
            facts = facts.len(),
            "bulk load facts sorted by relation order"
        );
        facts.sort_by_key(|fact| relation_sort_key(schema, fact.relation()));

        let mut inserted = 0;
        for fact in facts {
            if self.insert(schema, fact)? == InsertOutcome::Inserted {
                inserted += 1;
            }
        }
        Ok(inserted)
    }

    /// Inserts a relation fact using set semantics.
    #[tracing::instrument(name = "bumbledb.insert", skip_all, fields(relation = fact.relation()))]
    pub fn insert(&mut self, schema: &StorageSchema, fact: Fact) -> Result<InsertOutcome> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        validate_fact_values(schema.descriptor(), relation, &fact)?;
        let encoded = self.encode_fact(relation_id, relation, &fact, InternMode::Create)?;

        if self.exact_current_fact_exists(relation_id, &encoded)? {
            return Ok(InsertOutcome::AlreadyPresent);
        }

        self.check_foreign_keys(schema, relation, &encoded)?;
        self.check_unique_constraints(schema, relation, &encoded)?;

        self.insert_canonical_fact(relation_id, &encoded)?;
        self.insert_unique_entries(schema, relation_id, relation, &encoded)?;
        self.insert_reverse_fk_entries(schema, relation_id, relation, &encoded)?;
        self.insert_access_entries(schema, relation_id, relation, &encoded)?;
        adjust_relation_fact_count(self, relation_id, 1)?;
        self.ensure_tx_id()?;
        Ok(InsertOutcome::Inserted)
    }

    /// Deletes an exact relation fact using set semantics.
    #[tracing::instrument(name = "bumbledb.delete", skip_all)]
    pub fn delete(&mut self, schema: &StorageSchema, fact: Fact) -> Result<DeleteOutcome> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        validate_fact_values(schema.descriptor(), relation, &fact)?;
        let old_encoded = match self.encode_fact(relation_id, relation, &fact, InternMode::Existing)
        {
            Ok(encoded) => encoded,
            Err(Error::Storage(crate::StorageError::DictionaryValueNotFound { .. })) => {
                return Ok(DeleteOutcome::Absent);
            }
            Err(error) => return Err(error),
        };
        if !self.exact_current_fact_exists(relation_id, &old_encoded)? {
            return Ok(DeleteOutcome::Absent);
        };

        self.check_delete_restrictions(schema, relation, &old_encoded)?;
        self.delete_access_entries(schema, relation_id, relation, &old_encoded)?;
        self.delete_reverse_fk_entries(schema, relation_id, relation, &old_encoded)?;
        self.delete_unique_entries(schema, relation_id, relation, &old_encoded)?;
        self.delete_canonical_fact(relation_id, &old_encoded)?;
        adjust_relation_fact_count(self, relation_id, -1)?;
        self.ensure_tx_id()?;
        Ok(DeleteOutcome::Deleted)
    }

    fn exact_current_fact_exists(&self, relation_id: u16, fact: &EncodedFact) -> Result<bool> {
        let key = canonical_fact_key(relation_id, fact);
        Ok(self.dbs.index.get(&self.txn, key.as_slice())?.is_some())
    }

    fn insert_canonical_fact(&mut self, relation_id: u16, fact: &EncodedFact) -> Result<()> {
        let key = canonical_fact_key(relation_id, fact);
        self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
        let id_key = fact_id_key(relation_id, fact);
        if let Some(existing) = self.dbs.index.get(&self.txn, id_key.as_slice())?
            && existing != fact.bytes()
        {
            return Err(Error::hash_collision("fact id"));
        }
        self.dbs
            .index
            .put(&mut self.txn, id_key.as_slice(), fact.bytes())?;
        crate::failpoints::check(crate::failpoints::Failpoint::AfterCanonicalFactPut)?;
        Ok(())
    }

    fn delete_canonical_fact(&mut self, relation_id: u16, fact: &EncodedFact) -> Result<()> {
        let key = canonical_fact_key(relation_id, fact);
        self.dbs.index.delete(&mut self.txn, key.as_slice())?;
        self.dbs
            .index
            .delete(&mut self.txn, fact_id_key(relation_id, fact).as_slice())?;
        Ok(())
    }

    fn encode_fact(
        &mut self,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &Fact,
        mode: InternMode,
    ) -> Result<EncodedFact> {
        let known_fields = relation
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<BTreeSet<_>>();
        for field in fact.values.keys() {
            if !known_fields.contains(field.as_str()) {
                return Err(Error::unknown_field(&relation.name, field));
            }
        }

        let mut bytes = Vec::with_capacity(fact_width(relation));
        for field in &relation.fields {
            let value = fact
                .values
                .get(&field.name)
                .ok_or_else(|| Error::missing_field(&relation.name, &field.name))?;
            bytes.extend_from_slice(&self.encode_value(relation, field, value, &mode)?);
        }
        Ok(EncodedFact {
            relation: RelationId(relation_id),
            bytes,
        })
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
            InternMode::Existing => self
                .lookup_intern_value(kind, raw)?
                .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind))),
        })
    }

    fn check_foreign_keys(
        &self,
        schema: &StorageSchema,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for constraint in &relation.constraints {
            let ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                ..
            } = constraint
            else {
                continue;
            };
            let (target_relation_id, target) = schema.relation(target_relation)?;
            let key = unique_entry_key_from_source(
                target_relation_id,
                target_constraint,
                relation,
                fact,
                fields,
            )?;
            if self.dbs.index.get(&self.txn, key.as_slice())?.is_none() {
                return Err(Error::foreign_key_violation(
                    &relation.name,
                    name,
                    &target.name,
                ));
            }
        }
        Ok(())
    }

    fn check_unique_constraints(
        &self,
        _schema: &StorageSchema,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for constraint in &relation.constraints {
            let ConstraintDescriptor::Unique { name, fields } = constraint else {
                continue;
            };
            let key = unique_entry_key_from_fact(fact.relation.0, name, relation, fact, fields)?;
            if let Some(existing) = self.dbs.index.get(&self.txn, key.as_slice())? {
                let id = fact_id(fact);
                if existing != id.as_slice() {
                    return Err(Error::unique_violation(&relation.name, name));
                }
            }
        }
        Ok(())
    }

    fn check_delete_restrictions(
        &self,
        schema: &StorageSchema,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let target_relation_id = fact.relation.0;
        for source_relation in schema.descriptor.relations.iter() {
            for constraint in &source_relation.constraints {
                let ConstraintDescriptor::ForeignKey {
                    name,
                    target_relation,
                    target_constraint,
                    ..
                } = constraint
                else {
                    continue;
                };
                if target_relation != &relation.name {
                    continue;
                }
                let Ok((_, target_fields)) = target_unique_constraint(relation, target_constraint)
                else {
                    continue;
                };
                let target_key = target_fields
                    .iter()
                    .map(|field| fact.field(relation, field))
                    .collect::<Result<Vec<_>>>()?
                    .concat();
                let prefix = reverse_fk_prefix(target_relation_id, target_constraint, &target_key);
                let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
                if iter.next().transpose()?.is_some() {
                    return Err(Error::restrict_violation(
                        &relation.name,
                        &source_relation.name,
                        name,
                    ));
                }
            }
        }
        Ok(())
    }

    fn insert_unique_entries(
        &mut self,
        _schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let id = fact_id(fact);
        for constraint in &relation.constraints {
            let ConstraintDescriptor::Unique { name, fields } = constraint else {
                continue;
            };
            let key = unique_entry_key_from_fact(relation_id, name, relation, fact, fields)?;
            self.dbs.index.put(&mut self.txn, key.as_slice(), &id)?;
        }
        Ok(())
    }

    fn delete_unique_entries(
        &mut self,
        _schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for constraint in &relation.constraints {
            let ConstraintDescriptor::Unique { name, fields } = constraint else {
                continue;
            };
            let key = unique_entry_key_from_fact(relation_id, name, relation, fact, fields)?;
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
        }
        Ok(())
    }

    fn insert_reverse_fk_entries(
        &mut self,
        schema: &StorageSchema,
        source_relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let source_id = fact_id(fact);
        for constraint in &relation.constraints {
            let ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                ..
            } = constraint
            else {
                continue;
            };
            let (target_relation_id, _) = schema.relation(target_relation)?;
            let target_key =
                encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
            let key = reverse_fk_entry_key(
                target_relation_id,
                target_constraint,
                &target_key,
                source_relation_id,
                name,
                &source_id,
            );
            self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
        }
        Ok(())
    }

    fn delete_reverse_fk_entries(
        &mut self,
        schema: &StorageSchema,
        source_relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let source_id = fact_id(fact);
        for constraint in &relation.constraints {
            let ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                ..
            } = constraint
            else {
                continue;
            };
            let (target_relation_id, _) = schema.relation(target_relation)?;
            let target_key =
                encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
            let key = reverse_fk_entry_key(
                target_relation_id,
                target_constraint,
                &target_key,
                source_relation_id,
                name,
                &source_id,
            );
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
        }
        Ok(())
    }

    fn insert_access_entries(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for layout in schema.layouts_for_relation(relation_id) {
            tracing::trace!(relation = %relation.name, index = %layout.index_name, "put current index entry");
            let key = access_key(layout, relation, fact)?;
            self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
            crate::failpoints::check(crate::failpoints::Failpoint::AfterCurrentIndexPut)?;
            adjust_access_entry_count(self, relation_id, layout.index_id, 1)?;
        }
        Ok(())
    }

    fn delete_access_entries(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for layout in schema.layouts_for_relation(relation_id) {
            tracing::trace!(relation = %relation.name, index = %layout.index_name, "delete current index entry");
            let key = access_key(layout, relation, fact)?;
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
            adjust_access_entry_count(self, relation_id, layout.index_id, -1)?;
        }
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
        let _span = tracing::trace_span!(
            "bumbledb.dict_intern",
            kind = dict_kind_name(kind),
            bytes = raw.len()
        )
        .entered();
        if let Some(id) = self.lookup_intern_value(kind, raw)? {
            tracing::trace!(id, existing = true, "dictionary value already interned");
            return Ok(id);
        }

        let id_key = next_dict_id_key(kind);
        let id = read_u64_meta(self, &id_key)?.unwrap_or(1);
        write_u64_meta(self, &id_key, id + 1)?;

        let fwd_key = dict_fwd_key(kind, raw);
        crate::failpoints::check(crate::failpoints::Failpoint::BeforeDictionaryPut)?;
        let mut fwd_value = Vec::with_capacity(8 + raw.len());
        push_u64(&mut fwd_value, id);
        fwd_value.extend_from_slice(raw);
        self.dbs
            .dict
            .put(&mut self.txn, fwd_key.as_slice(), fwd_value.as_slice())?;
        self.dbs
            .dict
            .put(&mut self.txn, dict_rev_key(kind, id).as_slice(), raw)?;
        crate::failpoints::check(crate::failpoints::Failpoint::AfterDictionaryPut)?;
        tracing::trace!(id, existing = false, "dictionary value interned");

        Ok(id)
    }

    fn lookup_intern_value(&self, kind: u8, raw: &[u8]) -> Result<Option<u64>> {
        lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)
    }
}

fn relation_sort_key(schema: &StorageSchema, relation_name: &str) -> usize {
    schema
        .descriptor
        .relations
        .iter()
        .position(|relation| relation.name == relation_name)
        .unwrap_or(usize::MAX)
}

impl<'env> ReadTxn<'env> {
    /// Scans a whole relation through the canonical fact-set access path.
    #[cfg(test)]
    pub(crate) fn scan_relation<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        let fact_set_access = schema
            .fact_set_index_name(relation_name)
            .ok_or_else(|| Error::unknown_index(relation_name, FACT_SET_ACCESS_NAME))?;
        self.scan_access_with_prefix(schema, relation_name, fact_set_access, &[], None)
    }

    /// Scans an access path by a leading-field prefix.
    #[cfg(test)]
    fn scan_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        prefix: &FieldValues,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        if prefix.relation != relation_name {
            return Err(Error::internal(format!(
                "prefix relation {} does not match scan relation {relation_name}",
                prefix.relation
            )));
        }
        let (_, relation) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;

        let encoded_prefix = self.encode_index_prefix(relation, layout, &prefix.values)?;
        self.scan_access_with_prefix(schema, relation_name, index_name, &encoded_prefix, None)
    }

    /// Scans a range index. Bounds are inclusive start and exclusive end.
    #[cfg(test)]
    fn scan_range<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        start: Option<Value>,
        end: Option<Value>,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        let (_, relation) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;
        let Some(first_field) = layout.leading_fields.first() else {
            return Err(Error::internal(format!(
                "range index {relation_name}.{index_name} has no leading field"
            )));
        };
        let field = relation
            .field(first_field)
            .ok_or_else(|| Error::unknown_field(&relation.name, first_field))?;

        let start = start
            .as_ref()
            .map(|value| self.encode_read_value(relation, field, value))
            .transpose()?;
        let end = end
            .as_ref()
            .map(|value| self.encode_read_value(relation, field, value))
            .transpose()?;
        let range = EncodedRange {
            offset: access_prefix(layout.relation_id, layout.index_id).len(),
            width: field.value_type.encoded_width(),
            start,
            end,
        };

        self.scan_access_with_prefix(schema, relation_name, index_name, &[], Some(range))
    }

    /// Scans an access path by encoded key prefix without decoding logical facts.
    pub(crate) fn scan_encoded_access_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        encoded_prefix: &[u8],
    ) -> Result<EncodedFactCursor<'borrow, 'env, 'schema>> {
        let (relation_id, _) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;
        let index_prefix = access_prefix(relation_id, layout.index_id);
        let mut scan_prefix = index_prefix.clone();
        scan_prefix.extend_from_slice(encoded_prefix);
        let iter = self
            .dbs
            .index
            .prefix_iter(&self.txn, scan_prefix.as_slice())?;
        Ok(EncodedFactCursor {
            iter,
            layout,
            index_prefix,
            _env: std::marker::PhantomData,
        })
    }

    /// Decodes one encoded query value by logical type.
    pub(crate) fn decode_query_value(&self, value_type: &ValueType, bytes: &[u8]) -> Result<Value> {
        decode_value(self.dbs.dict, &self.txn, value_type, bytes)
    }

    /// Encodes a query value by logical type using existing dictionary entries.
    pub(crate) fn encode_query_value(
        &self,
        value_type: &ValueType,
        value: &Value,
    ) -> Result<Vec<u8>> {
        encode_value_for_type(value_type, value, |kind, raw| {
            lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
        })
    }

    /// Returns the last committed storage transaction ID.
    pub fn last_committed_tx_id(&self) -> Result<u64> {
        Ok(read_u64(&self.dbs.meta, &self.txn, NEXT_TX_ID_KEY)?.unwrap_or(1) - 1)
    }

    /// Returns the stored fact count for a relation.
    pub fn relation_fact_count(&self, schema: &StorageSchema, relation_name: &str) -> Result<u64> {
        let (relation_id, _) = schema.relation(relation_name)?;
        Ok(read_u64(
            &self.dbs.meta,
            &self.txn,
            &relation_fact_count_key(relation_id),
        )?
        .unwrap_or(0))
    }

    /// Returns the stored index-entry count for a current index.
    pub(crate) fn access_entry_count(
        &self,
        schema: &StorageSchema,
        relation_name: &str,
        index_name: &str,
    ) -> Result<u64> {
        let layout = schema.layout(relation_name, index_name).ok_or_else(|| {
            Error::internal(format!("missing index {relation_name}.{index_name}"))
        })?;
        Ok(read_u64(
            &self.dbs.meta,
            &self.txn,
            &access_entry_count_key(layout.relation_id, layout.index_id),
        )?
        .unwrap_or(0))
    }

    /// Counts canonical fact entries for a relation by scanning the canonical namespace.
    #[cfg(test)]
    pub(crate) fn canonical_fact_count(
        &self,
        schema: &StorageSchema,
        relation_name: &str,
    ) -> Result<usize> {
        let (relation_id, _) = schema.relation(relation_name)?;
        let prefix = canonical_fact_prefix(relation_id);
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        let mut count = 0usize;
        while iter.next().transpose()?.is_some() {
            count += 1;
        }
        Ok(count)
    }

    /// Checks whether a current access entry exists for a full fact.
    #[cfg(test)]
    pub(crate) fn access_entry_exists(
        &self,
        schema: &StorageSchema,
        fact: &Fact,
        index_name: &str,
    ) -> Result<bool> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        let layout = schema.layout(&fact.relation, index_name).ok_or_else(|| {
            Error::internal(format!("missing index {}.{index_name}", fact.relation))
        })?;
        let encoded = self.encode_fact_existing(relation_id, relation, fact)?;
        let key = access_key(layout, relation, &encoded)?;
        Ok(self.dbs.index.get(&self.txn, key.as_slice())?.is_some())
    }

    /// Checks whether the exact fact exists in the canonical fact set.
    #[cfg(test)]
    pub(crate) fn exact_fact_exists(&self, schema: &StorageSchema, fact: &Fact) -> Result<bool> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        let encoded = self.encode_fact_existing(relation_id, relation, fact)?;
        let key = canonical_fact_key(relation_id, &encoded);
        Ok(self.dbs.index.get(&self.txn, key.as_slice())?.is_some())
    }

    /// Looks up an interned string ID.
    #[cfg(test)]
    pub(crate) fn dictionary_string_id(&self, value: &str) -> Result<Option<u64>> {
        lookup_intern_value(&self.dbs.dict, &self.txn, DICT_STRING, value.as_bytes())
    }

    /// Counts reverse dictionary entries across all dictionary kinds.
    pub(crate) fn dictionary_entry_count(&self) -> Result<usize> {
        let prefix = [DICT_REV];
        let mut iter = self.dbs.dict.prefix_iter(&self.txn, &prefix[..])?;
        let mut count = 0;
        while iter.next().transpose()?.is_some() {
            count += 1;
        }
        Ok(count)
    }

    #[cfg(test)]
    fn raw_index_value(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.dbs.index.get(&self.txn, key)?.map(ToOwned::to_owned))
    }

    #[cfg(test)]
    fn raw_index_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix)?;
        let mut keys = Vec::new();
        while let Some((key, _)) = iter.next().transpose()? {
            keys.push(key.to_vec());
        }
        Ok(keys)
    }

    #[cfg(test)]
    fn scan_access_with_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        encoded_prefix: &[u8],
        range: Option<EncodedRange>,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        let _span = tracing::trace_span!(
            "bumbledb.query.scan",
            relation = relation_name,
            index = index_name,
            prefix_bytes = encoded_prefix.len(),
            range = range.is_some()
        )
        .entered();
        let (relation_id, relation) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;
        let mut prefix = access_prefix(relation_id, layout.index_id);
        prefix.extend_from_slice(encoded_prefix);
        let iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        Ok(FactCursor {
            iter,
            txn: &self.txn,
            index_db: self.dbs.index,
            dict: self.dbs.dict,
            relation,
            layout,
            range,
        })
    }

    #[cfg(test)]
    fn encode_index_prefix(
        &self,
        relation: &RelationDescriptor,
        layout: &AccessLayout,
        values: &BTreeMap<String, Value>,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut saw_missing = false;

        for field_name in &layout.leading_fields {
            match values.get(field_name) {
                Some(value) if !saw_missing => {
                    let field = relation
                        .field(field_name)
                        .ok_or_else(|| Error::unknown_field(&relation.name, field_name))?;
                    out.extend_from_slice(&self.encode_read_value(relation, field, value)?);
                }
                Some(_) => {
                    return Err(Error::internal(format!(
                        "index prefix for {}.{} is not contiguous",
                        relation.name, layout.index_name
                    )));
                }
                None => saw_missing = true,
            }
        }

        for field_name in values.keys() {
            if !layout
                .leading_fields
                .iter()
                .any(|leading| leading == field_name)
            {
                return Err(Error::unknown_field(&relation.name, field_name));
            }
        }

        Ok(out)
    }

    #[cfg(test)]
    fn encode_read_value(
        &self,
        relation: &RelationDescriptor,
        field: &FieldDescriptor,
        value: &Value,
    ) -> Result<Vec<u8>> {
        encode_value_with(relation, field, value, |kind, raw| {
            lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
        })
    }

    #[cfg(test)]
    fn encode_fact_existing(
        &self,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &Fact,
    ) -> Result<EncodedFact> {
        let mut bytes = Vec::with_capacity(fact_width(relation));
        for field in &relation.fields {
            let value = fact
                .values
                .get(&field.name)
                .ok_or_else(|| Error::missing_field(&relation.name, &field.name))?;
            bytes.extend_from_slice(&encode_value_with(relation, field, value, |kind, raw| {
                lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                    .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
            })?);
        }
        Ok(EncodedFact {
            relation: RelationId(relation_id),
            bytes,
        })
    }
}

fn encode_value_with(
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

fn encode_value_for_type(
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

fn validate_fact_values(
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

fn validate_enum_value(
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

fn storage_value_matches_type(value: &Value, value_type: &ValueType) -> bool {
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
fn decode_access_scan_entry(
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
fn decode_access_key(
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

fn encoded_access_item(
    layout: &AccessLayout,
    index_prefix: &[u8],
    key: &[u8],
) -> Result<EncodedAccessItem> {
    let prefix_len = index_prefix.len();
    if key.len() != layout.encoded_len {
        return Err(Error::corrupt("index key width does not match layout"));
    }
    if key.get(0..prefix_len) != Some(index_prefix) {
        return Err(Error::corrupt("index key prefix does not match layout"));
    }
    Ok(EncodedAccessItem {
        key: key.to_vec(),
        prefix_len,
    })
}

#[cfg(test)]
fn decode_encoded_fact(
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

fn decode_value(
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

fn value_type_name(value_type: &ValueType) -> String {
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

fn fact_width(relation: &RelationDescriptor) -> usize {
    relation
        .fields
        .iter()
        .map(|field| field.value_type.encoded_width())
        .sum()
}

fn field_layout(relation: &RelationDescriptor, name: &str) -> Result<(usize, usize)> {
    let (_, offset, width) = field_layout_with_id(relation, name)?;
    Ok((offset, width))
}

fn field_layout_with_id(
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

fn target_unique_constraint<'a>(
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

fn encoded_key_from_fields<'a>(
    relation: &RelationDescriptor,
    fact: &EncodedFact,
    fields: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<u8>> {
    let mut prefix = Vec::new();
    for field in fields {
        prefix.extend_from_slice(fact.field(relation, field)?);
    }
    Ok(prefix)
}

fn unique_entry_key_from_fact(
    relation_id: u16,
    constraint: &str,
    relation: &RelationDescriptor,
    fact: &EncodedFact,
    fields: &[String],
) -> Result<Vec<u8>> {
    let encoded_key = encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
    Ok(unique_entry_key(relation_id, constraint, &encoded_key))
}

fn unique_entry_key_from_source(
    relation_id: u16,
    constraint: &str,
    relation: &RelationDescriptor,
    fact: &EncodedFact,
    fields: &[String],
) -> Result<Vec<u8>> {
    let encoded_key = encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
    Ok(unique_entry_key(relation_id, constraint, &encoded_key))
}

fn unique_entry_key(relation_id: u16, constraint: &str, encoded_key: &[u8]) -> Vec<u8> {
    let mut key = vec![NS_UNIQUE_ENTRY];
    push_u16(&mut key, relation_id);
    push_name(&mut key, constraint);
    key.extend_from_slice(encoded_key);
    key
}

fn reverse_fk_prefix(relation_id: u16, constraint: &str, encoded_key: &[u8]) -> Vec<u8> {
    let mut key = vec![NS_REVERSE_FK_ENTRY];
    push_u16(&mut key, relation_id);
    push_name(&mut key, constraint);
    key.extend_from_slice(encoded_key);
    key
}

fn reverse_fk_entry_key(
    relation_id: u16,
    constraint: &str,
    encoded_key: &[u8],
    source_relation_id: u16,
    source_constraint: &str,
    source_fact_id: &[u8; FACT_ID_BYTES],
) -> Vec<u8> {
    let mut key = reverse_fk_prefix(relation_id, constraint, encoded_key);
    push_u16(&mut key, source_relation_id);
    push_name(&mut key, source_constraint);
    key.extend_from_slice(source_fact_id);
    key
}

fn access_prefix(relation_id: u16, index_id: u16) -> Vec<u8> {
    let mut key = vec![NS_ACCESS_ENTRY];
    push_u16(&mut key, relation_id);
    push_u16(&mut key, index_id);
    key
}

fn canonical_fact_key(relation_id: u16, fact: &EncodedFact) -> Vec<u8> {
    let mut key = canonical_fact_prefix(relation_id);
    key.extend_from_slice(fact.bytes());
    key
}

fn canonical_fact_prefix(relation_id: u16) -> Vec<u8> {
    let mut key = vec![NS_CANONICAL_FACT];
    push_u16(&mut key, relation_id);
    key
}

fn fact_id(fact: &EncodedFact) -> [u8; FACT_ID_BYTES] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&fact.relation.0.to_be_bytes());
    hasher.update(fact.bytes());
    let hash = hasher.finalize();
    let mut out = [0; FACT_ID_BYTES];
    out.copy_from_slice(&hash.as_bytes()[..FACT_ID_BYTES]);
    out
}

fn fact_id_prefix(relation_id: u16) -> Vec<u8> {
    let mut key = vec![NS_FACT_ID];
    push_u16(&mut key, relation_id);
    key
}

fn fact_id_key(relation_id: u16, fact: &EncodedFact) -> Vec<u8> {
    let mut key = fact_id_prefix(relation_id);
    key.extend_from_slice(&fact_id(fact));
    key
}

#[cfg(test)]
fn lookup_fact_by_id(
    db: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation_id: u16,
    id: &[u8],
) -> Result<EncodedFact> {
    if id.len() != FACT_ID_BYTES {
        return Err(Error::corrupt("fact id width invalid"));
    }
    let mut key = fact_id_prefix(relation_id);
    key.extend_from_slice(id);
    let bytes = db
        .get(txn, key.as_slice())?
        .ok_or_else(|| Error::corrupt("fact id target missing"))?
        .to_vec();
    Ok(EncodedFact {
        relation: RelationId(relation_id),
        bytes,
    })
}

fn access_key(
    layout: &AccessLayout,
    relation: &RelationDescriptor,
    fact: &EncodedFact,
) -> Result<Vec<u8>> {
    if fact.relation.0 != layout.relation_id {
        return Err(Error::corrupt(
            "encoded fact relation does not match index layout",
        ));
    }
    let mut key = access_prefix(layout.relation_id, layout.index_id);
    for component in &layout.components {
        key.extend_from_slice(fact.field(relation, &component.field_name)?);
    }
    key.extend_from_slice(&fact_id(fact));
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
        .map_err(|_| Error::corrupt("u64 metadata must be eight bytes"))?;
    Ok(Some(u64::from_be_bytes(bytes)))
}

fn write_u64(db: &crate::RawDatabase, txn: &mut heed::RwTxn, key: &[u8], value: u64) -> Result<()> {
    let bytes = value.to_be_bytes();
    Ok(db.put(txn, key, &bytes[..])?)
}

fn adjust_relation_fact_count(txn: &mut WriteTxn<'_>, relation_id: u16, delta: i64) -> Result<()> {
    adjust_u64_meta(txn, &relation_fact_count_key(relation_id), delta)
}

fn adjust_access_entry_count(
    txn: &mut WriteTxn<'_>,
    relation_id: u16,
    index_id: u16,
    delta: i64,
) -> Result<()> {
    adjust_u64_meta(txn, &access_entry_count_key(relation_id, index_id), delta)
}

fn adjust_u64_meta(txn: &mut WriteTxn<'_>, key: &[u8], delta: i64) -> Result<()> {
    crate::failpoints::check(crate::failpoints::Failpoint::BeforeStatsUpdate)?;
    let current = read_u64_meta(txn, key)?.unwrap_or(0);
    let next = if delta >= 0 {
        current
            .checked_add(delta as u64)
            .ok_or_else(|| Error::internal("metadata counter overflow"))?
    } else {
        current
            .checked_sub(delta.unsigned_abs())
            .ok_or_else(|| Error::internal("metadata counter underflow"))?
    };
    write_u64_meta(txn, key, next)?;
    crate::failpoints::check(crate::failpoints::Failpoint::AfterStatsUpdate)?;
    Ok(())
}

fn relation_fact_count_key(relation_id: u16) -> Vec<u8> {
    let mut key = b"stats:facts:".to_vec();
    push_u16(&mut key, relation_id);
    key
}

fn access_entry_count_key(relation_id: u16, index_id: u16) -> Vec<u8> {
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
        return Err(Error::corrupt("dictionary forward value too short"));
    }
    let id = u64::from_be_bytes(
        value[..8]
            .try_into()
            .map_err(|_| Error::corrupt("dictionary ID width invalid"))?,
    );
    if &value[8..] != raw {
        return Err(Error::hash_collision(dict_kind_name(kind)));
    }
    Ok(Some(id))
}

fn lookup_intern_raw_by_id(
    db: crate::RawDatabase,
    txn: &heed::RoTxn,
    kind: u8,
    id: u64,
) -> Result<Vec<u8>> {
    db.get(txn, dict_rev_key(kind, id).as_slice())?
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
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

fn push_name(out: &mut Vec<u8>, value: &str) {
    push_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
