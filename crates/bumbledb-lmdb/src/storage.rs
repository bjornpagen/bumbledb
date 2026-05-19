use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::encoding::{
    DecimalRaw, InternId, TimestampMicros, UuidBytes, decode_bool, decode_decimal, decode_i64,
    decode_intern_id, decode_timestamp, decode_u64, decode_uuid, encode_bool, encode_decimal,
    encode_i64, encode_intern_id, encode_timestamp, encode_u64, encode_uuid,
};
use bumbledb_core::schema::{
    ConstraintDescriptor, CurrentIndexLayout, FieldDescriptor, IndexComponent, IndexKind,
    RelationDescriptor, SchemaDescriptor, ValueType,
};

use crate::{AccessId, Error, FieldId, ReadTxn, RelationId, Result, WriteTxn};

const NS_CURRENT_TUPLE: u8 = 0x10;
const NS_CURRENT_ROW: u8 = 0x11;
const NS_UNIQUE_GUARD: u8 = 0x20;
const NS_HISTORY: u8 = 0x30;

const SEGMENT_META_PREFIX: &[u8] = b"segment:meta:";
const SEGMENT_COLUMN_PREFIX: &[u8] = b"segment:column:";
const SEGMENT_INDEX_PREFIX: &[u8] = b"segment:index:";
const SEGMENT_VISIBILITY_PREFIX: &[u8] = b"segment:visibility:";
const SEGMENT_NEXT_PREFIX: &[u8] = b"segment:next:";

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

/// Bulk ETL load report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkLoadReport {
    /// Number of logical rows inserted.
    pub rows_inserted: usize,
    /// Storage transaction ID after the bulk load committed.
    pub storage_tx_id: u64,
    /// Number of interned dictionary values after the load committed.
    pub dictionary_entries: usize,
}

/// Durable relation segment metadata visible to query-image builders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentDescriptor {
    /// Relation this segment belongs to.
    pub relation: RelationId,
    /// Monotonic segment ID within the relation.
    pub segment_id: u64,
    /// Inclusive storage transaction ID where this segment becomes visible.
    pub tx_start: u64,
    /// Exclusive storage transaction ID where this segment stops being visible.
    pub tx_end: Option<u64>,
    /// Number of rows represented by this segment.
    pub row_count: usize,
    /// Encoded fixed-width column chunks.
    pub columns: Vec<ColumnSegmentDescriptor>,
    /// Encoded index chunks.
    pub indexes: Vec<IndexSegmentDescriptor>,
}

/// Durable encoded column chunk descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSegmentDescriptor {
    /// Field represented by this column chunk.
    pub field: FieldId,
    /// Logical value type.
    pub value_type: ValueType,
    /// Fixed encoded width.
    pub width: usize,
    /// LMDB key containing contiguous encoded column bytes.
    pub lmdb_key: Vec<u8>,
    /// Stored byte length.
    pub byte_len: usize,
}

/// Durable encoded index chunk descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexSegmentDescriptor {
    /// Access path represented by this index chunk.
    pub access: AccessId,
    /// Leading fields in index order.
    pub fields: Vec<FieldId>,
    /// Index access kind.
    pub kind: IndexKind,
    /// LMDB key containing encoded index bytes.
    pub lmdb_key: Vec<u8>,
    /// Stored byte length.
    pub byte_len: usize,
    /// Lightweight index statistics summary.
    pub stats: IndexStatsSummary,
}

/// Durable index segment statistics summary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IndexStatsSummary {
    /// Number of encoded entries in the index segment.
    pub row_count: usize,
    /// Number of leading fields represented by this index.
    pub depth: usize,
    /// Stored index chunk bytes.
    pub byte_len: usize,
}

impl StorageSchema {
    /// Builds storage metadata and validates generated index key lengths.
    pub fn new(descriptor: SchemaDescriptor, max_key_size: usize) -> Result<Self> {
        descriptor.validate()?;
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

    /// Returns planner-facing access paths for a relation.
    pub fn access_paths(&self, relation_name: &str) -> Result<Vec<AccessPathDescriptor>> {
        let (relation_id, _) = self.relation(relation_name)?;
        Ok(self
            .layouts_for_relation(relation_id)
            .map(AccessPathDescriptor::from_layout)
            .collect())
    }

    pub(crate) fn relation(&self, name: &str) -> Result<(u16, &RelationDescriptor)> {
        self.descriptor
            .relations
            .iter()
            .enumerate()
            .find(|(_, relation)| relation.name == name)
            .map(|(id, relation)| (id as u16, relation))
            .ok_or_else(|| Error::unknown_relation(name))
    }

    fn layouts_for_relation(&self, relation_id: u16) -> impl Iterator<Item = &CurrentIndexLayout> {
        self.layouts
            .iter()
            .filter(move |layout| layout.relation_id == relation_id)
    }

    pub(crate) fn layout(&self, relation: &str, index: &str) -> Option<&CurrentIndexLayout> {
        self.layouts
            .iter()
            .find(|layout| layout.relation_name == relation && layout.index_name == index)
    }
}

/// Planner-facing access path descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPathDescriptor {
    /// Relation name.
    pub relation_name: String,
    /// Index name.
    pub index_name: String,
    /// Index kind.
    pub kind: IndexKind,
    /// Leading fields usable as an index prefix.
    pub leading_fields: Vec<String>,
    /// Full covering components in encoded order.
    pub components: Vec<IndexComponent>,
}

impl AccessPathDescriptor {
    fn from_layout(layout: &CurrentIndexLayout) -> Self {
        Self {
            relation_name: layout.relation_name.clone(),
            index_name: layout.index_name.clone(),
            kind: layout.kind,
            leading_fields: layout.leading_fields.clone(),
            components: layout.components.clone(),
        }
    }
}

/// A logical row for the generic storage layer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

    /// Returns a field value.
    pub fn value(&self, field: &str) -> Option<&Value> {
        self.values.get(field)
    }

    /// Returns all row values keyed by field name.
    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }
}

/// Field values used to build an index prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldValues {
    relation: String,
    values: BTreeMap<String, Value>,
}

impl FieldValues {
    /// Creates index-prefix field values for `relation`.
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    pub(crate) fn kind_name(&self) -> &'static str {
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

/// Encoded component from a covering index key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedComponent {
    /// Field name.
    pub field_name: String,
    /// Encoded bytes for this field in the index key.
    pub bytes: Vec<u8>,
}

/// A row yielded from an index scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanItem {
    /// Decoded logical row.
    pub row: Row,
    /// Encoded components in index-key order.
    pub encoded_components: Vec<EncodedComponent>,
}

impl ScanItem {
    /// Returns an encoded component by field name.
    pub fn encoded_component(&self, field: &str) -> Option<&[u8]> {
        self.encoded_components
            .iter()
            .find(|component| component.field_name == field)
            .map(|component| component.bytes.as_slice())
    }
}

/// Encoded row component view yielded from a covering index scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedIndexItem {
    key: Vec<u8>,
    prefix_len: usize,
}

impl EncodedIndexItem {
    /// Returns an encoded component by ordinal.
    pub fn component(&self, components: &[IndexComponent], index: usize) -> Option<&[u8]> {
        let mut offset = self.prefix_len;
        for component in components.get(..index)? {
            offset += component.encoded_width;
        }
        let width = components.get(index)?.encoded_width;
        self.key.get(offset..offset + width)
    }
}

/// Transaction-scoped scan over one current covering index.
pub struct IndexScan<'borrow, 'env, 'schema> {
    iter: heed::RoPrefix<'borrow, heed::types::Bytes, heed::types::Bytes>,
    txn: &'borrow heed::RoTxn<'env, heed::WithoutTls>,
    dict: crate::RawDatabase,
    relation: &'schema RelationDescriptor,
    layout: &'schema CurrentIndexLayout,
    range: Option<EncodedRange>,
}

/// Transaction-scoped encoded scan over one current covering index.
pub(crate) struct EncodedIndexScan<'borrow, 'env, 'schema> {
    iter: heed::RoPrefix<'borrow, heed::types::Bytes, heed::types::Bytes>,
    layout: &'schema CurrentIndexLayout,
    index_prefix: Vec<u8>,
    _env: std::marker::PhantomData<&'env ()>,
}

#[derive(Clone, Debug)]
struct EncodedRange {
    offset: usize,
    width: usize,
    start: Option<Vec<u8>>,
    end: Option<Vec<u8>>,
}

impl Iterator for IndexScan<'_, '_, '_> {
    type Item = Result<ScanItem>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (key, _) = match self.iter.next()? {
                Ok(item) => item,
                Err(error) => return Some(Err(error.into())),
            };

            if !self.range_matches(key) {
                continue;
            }

            return Some(decode_index_scan_item(
                self.dict,
                self.txn,
                self.relation,
                self.layout,
                key,
            ));
        }
    }
}

impl Iterator for EncodedIndexScan<'_, '_, '_> {
    type Item = Result<EncodedIndexItem>;

    fn next(&mut self) -> Option<Self::Item> {
        let (key, _) = match self.iter.next()? {
            Ok(item) => item,
            Err(error) => return Some(Err(error.into())),
        };
        Some(encoded_index_item(self.layout, &self.index_prefix, key))
    }
}

impl IndexScan<'_, '_, '_> {
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

#[derive(Clone, Debug)]
struct EncodedRow {
    fields: BTreeMap<String, Vec<u8>>,
}

impl EncodedRow {
    fn field(&self, relation: &RelationDescriptor, name: &str) -> Result<&[u8]> {
        self.fields
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::missing_field(&relation.name, name))
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
            return Err(Error::corrupt("row payload width does not match schema"));
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
    #[tracing::instrument(name = "bumbledb.alloc_id", skip_all, fields(relation = relation_name))]
    pub fn alloc_id(&mut self, schema: &StorageSchema, relation_name: &str) -> Result<u64> {
        let (relation_id, relation) = schema.relation(relation_name)?;
        let generated = relation.generated_id.as_ref().ok_or_else(|| {
            Error::internal(format!("relation {relation_name} has no generated ID"))
        })?;
        if relation.field(&generated.field).is_none() {
            return Err(Error::unknown_field(&relation.name, &generated.field));
        }

        let key = next_id_key(relation_id);
        let next = read_u64_meta(self, &key)?.unwrap_or(1);
        write_u64_meta(self, &key, next + 1)?;
        Ok(next)
    }

    /// Bulk-loads rows in deterministic schema relation order.
    ///
    /// This is one write transaction: any constraint failure aborts all current
    /// rows, indexes, stats, history, counters, and dictionary inserts made by
    /// the attempted load.
    pub fn bulk_load(
        &mut self,
        schema: &StorageSchema,
        rows: impl IntoIterator<Item = Row>,
    ) -> Result<usize> {
        let _span = tracing::debug_span!("bumbledb.storage.bulk_load").entered();
        let mut rows = rows.into_iter().collect::<Vec<_>>();
        tracing::debug!(rows = rows.len(), "bulk load rows sorted by relation order");
        rows.sort_by_key(|row| relation_sort_key(schema, row.relation()));

        let previous_defer = self.defer_relation_segments;
        self.defer_relation_segments = true;
        let result = (|| {
            let mut inserted = 0;
            for row in rows {
                self.insert(schema, row)?;
                inserted += 1;
            }
            for relation_id in 0..schema.descriptor.relations.len() {
                self.touched_relation_segments.insert(relation_id as u16);
            }
            self.flush_relation_segments(schema)?;
            Ok(inserted)
        })();
        self.defer_relation_segments = previous_defer;
        result
    }

    /// Inserts a primary-keyed relation row.
    #[tracing::instrument(name = "bumbledb.insert", skip_all, fields(relation = row.relation()))]
    pub fn insert(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        self.insert_inner(schema, row)
    }

    /// Inserts a composite set/edge tuple.
    pub fn insert_tuple(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        self.insert_inner(schema, row)
    }

    /// Replaces an existing row by primary key.
    #[tracing::instrument(name = "bumbledb.replace", skip_all, fields(relation = row.relation()))]
    pub fn replace(&mut self, schema: &StorageSchema, row: Row) -> Result<()> {
        let (relation_id, relation) = schema.relation(&row.relation)?;
        let new_encoded = self.encode_row(relation, &row, InternMode::Create)?;
        let primary = primary_bytes(relation, &new_encoded)?;
        let row_key = current_row_key(relation_id, &primary);
        let Some(old_payload) = self.dbs.index.get(&self.txn, row_key.as_slice())? else {
            return Err(Error::not_found(&relation.name));
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
        crate::failpoints::check(crate::failpoints::Failpoint::AfterCurrentRowPut)?;

        self.append_history(
            b'R',
            relation_id,
            &primary,
            Some(&old_payload),
            Some(&new_encoded.payload(relation)?),
        )?;
        self.record_relation_segment_change(schema, relation_id, relation)?;
        Ok(())
    }

    /// Deletes an existing primary-keyed row.
    #[tracing::instrument(name = "bumbledb.delete", skip_all)]
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
                    .ok_or_else(|| Error::missing_field(&relation.name, field))
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
            return Err(Error::duplicate_tuple(&relation.name));
        }

        self.check_foreign_keys(schema, relation, &encoded)?;
        self.check_unique_constraints(relation_id, relation, &encoded, &primary)?;

        self.dbs.index.put(
            &mut self.txn,
            row_key.as_slice(),
            encoded.payload(relation)?.as_slice(),
        )?;
        crate::failpoints::check(crate::failpoints::Failpoint::AfterCurrentRowPut)?;
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
        self.record_relation_segment_change(schema, relation_id, relation)?;
        Ok(())
    }

    fn delete_inner(&mut self, schema: &StorageSchema, key: KeyValues) -> Result<()> {
        let (relation_id, relation) = schema.relation(&key.relation)?;
        let primary = self.encode_primary_key(relation, &key.values, InternMode::Existing)?;
        let row_key = current_row_key(relation_id, &primary);
        let Some(old_payload) = self.dbs.index.get(&self.txn, row_key.as_slice())? else {
            return Err(Error::not_found(&relation.name));
        };
        let old_payload = old_payload.to_vec();
        let old_encoded = EncodedRow::from_payload(relation, &old_payload)?;

        self.check_delete_restrictions(schema, relation, &old_encoded)?;
        self.delete_current_indexes(schema, relation_id, relation, &old_encoded)?;
        self.delete_unique_guards(relation_id, relation, &old_encoded)?;
        self.dbs.index.delete(&mut self.txn, row_key.as_slice())?;
        adjust_relation_row_count(self, relation_id, -1)?;
        self.append_history(b'D', relation_id, &primary, Some(&old_payload), None)?;
        self.record_relation_segment_change(schema, relation_id, relation)?;
        Ok(())
    }

    fn record_relation_segment_change(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
    ) -> Result<()> {
        if self.defer_relation_segments {
            self.touched_relation_segments.insert(relation_id);
            Ok(())
        } else {
            self.append_relation_segment(schema, relation_id, relation)
        }
    }

    fn flush_relation_segments(&mut self, schema: &StorageSchema) -> Result<()> {
        let touched = std::mem::take(&mut self.touched_relation_segments);
        for relation_id in touched {
            let relation = schema
                .descriptor
                .relations
                .get(relation_id as usize)
                .ok_or_else(|| Error::corrupt("touched relation id missing from schema"))?;
            self.append_relation_segment(schema, relation_id, relation)?;
        }
        Ok(())
    }

    fn append_relation_segment(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
    ) -> Result<()> {
        let _span = tracing::trace_span!(
            "bumbledb.storage.segment_publish",
            relation = %relation.name,
        )
        .entered();
        let tx_id = self.ensure_tx_id()?;
        let segment_id = self.next_segment_id(relation_id)?;
        let segment = self.build_relation_segment(schema, relation_id, relation)?;

        self.close_visible_relation_segments(relation_id, tx_id)?;

        for column in &segment.columns {
            self.dbs.index.put(
                &mut self.txn,
                segment_column_key(relation_id, segment_id, column.field.0).as_slice(),
                column.bytes.as_slice(),
            )?;
        }
        for index in &segment.indexes {
            self.dbs.index.put(
                &mut self.txn,
                segment_index_key(relation_id, segment_id, index.index_id).as_slice(),
                index.bytes.as_slice(),
            )?;
        }

        self.dbs.index.put(
            &mut self.txn,
            segment_meta_key(relation_id, segment_id).as_slice(),
            encode_segment_meta(tx_id, None, segment.row_count).as_slice(),
        )?;
        self.dbs.index.put(
            &mut self.txn,
            segment_visibility_key(tx_id, relation_id, segment_id).as_slice(),
            &[],
        )?;
        tracing::trace!(
            relation = %relation.name,
            segment_id,
            tx_id,
            rows = segment.row_count,
            "relation segment published"
        );
        Ok(())
    }

    fn next_segment_id(&mut self, relation_id: u16) -> Result<u64> {
        let key = segment_next_key(relation_id);
        let next = read_u64_meta(self, &key)?.unwrap_or(1);
        write_u64_meta(self, &key, next + 1)?;
        Ok(next)
    }

    fn close_visible_relation_segments(&mut self, relation_id: u16, tx_end: u64) -> Result<()> {
        let mut active = Vec::new();
        let prefix = segment_meta_prefix(relation_id);
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        while let Some((key, value)) = iter.next().transpose()? {
            let segment_id = parse_segment_id_from_meta_key(&prefix, key)?;
            let meta = decode_segment_meta(value)?;
            if meta.tx_end.is_none() {
                active.push((segment_id, meta));
            }
        }
        drop(iter);

        for (segment_id, meta) in active {
            self.dbs.index.put(
                &mut self.txn,
                segment_meta_key(relation_id, segment_id).as_slice(),
                encode_segment_meta(meta.tx_start, Some(tx_end), meta.row_count).as_slice(),
            )?;
        }
        Ok(())
    }

    fn build_relation_segment(
        &self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
    ) -> Result<PendingRelationSegment> {
        let primary_layout = schema
            .layout(&relation.name, "primary")
            .ok_or_else(|| Error::unknown_index(&relation.name, "primary"))?;
        let component_by_field = primary_layout
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| (component.field_name.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let mut columns = relation
            .fields
            .iter()
            .enumerate()
            .map(|(field_id, _)| PendingColumnSegment {
                field: FieldId(field_id as u16),
                bytes: Vec::new(),
            })
            .collect::<Vec<_>>();

        let primary_prefix = current_index_prefix(relation_id, primary_layout.index_id);
        let mut row_count = 0usize;
        let mut iter = self
            .dbs
            .index
            .prefix_iter(&self.txn, primary_prefix.as_slice())?;
        while let Some((key, _)) = iter.next().transpose()? {
            let item = encoded_index_item(primary_layout, &primary_prefix, key)?;
            for (field_id, field) in relation.fields.iter().enumerate() {
                let component_index = *component_by_field
                    .get(field.name.as_str())
                    .ok_or_else(|| Error::corrupt("segment missing primary component"))?;
                let bytes = item
                    .component(&primary_layout.components, component_index)
                    .ok_or_else(|| Error::corrupt("segment primary component truncated"))?;
                columns[field_id].bytes.extend_from_slice(bytes);
            }
            row_count += 1;
        }
        drop(iter);

        let indexes = schema
            .layouts_for_relation(relation_id)
            .map(|layout| self.build_index_segment(layout))
            .collect::<Result<Vec<_>>>()?;

        Ok(PendingRelationSegment {
            row_count,
            columns,
            indexes,
        })
    }

    fn build_index_segment(&self, layout: &CurrentIndexLayout) -> Result<PendingIndexSegment> {
        let prefix = current_index_prefix(layout.relation_id, layout.index_id);
        let mut bytes = Vec::new();
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        while let Some((key, _)) = iter.next().transpose()? {
            bytes.extend_from_slice(key);
        }
        Ok(PendingIndexSegment {
            index_id: layout.index_id,
            bytes,
        })
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
                return Err(Error::unknown_field(&relation.name, field));
            }
        }

        let mut fields = BTreeMap::new();
        for field in &relation.fields {
            let value = row
                .values
                .get(&field.name)
                .ok_or_else(|| Error::missing_field(&relation.name, &field.name))?;
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
                .ok_or_else(|| Error::unknown_field(&relation.name, field_name))?;
            let value = values
                .get(field_name)
                .ok_or_else(|| Error::missing_field(&relation.name, field_name))?;
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
            InternMode::Existing => self
                .lookup_intern_value(kind, raw)?
                .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind))),
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
                return Err(Error::unsupported_composite_foreign_key(&target.name));
            }

            let target_primary = row.field(relation, &field.name)?.to_vec();
            let key = current_row_key(target_relation_id, &target_primary);
            if self.dbs.index.get(&self.txn, key.as_slice())?.is_none() {
                return Err(Error::foreign_key_violation(
                    &relation.name,
                    &field.name,
                    &target.name,
                ));
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
            if let Some(existing_primary) = self.dbs.index.get(&self.txn, key.as_slice())?
                && existing_primary != primary
            {
                return Err(Error::unique_violation(&relation.name, name));
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
                    return Err(Error::restrict_violation(
                        &relation.name,
                        &source_relation.name,
                        &field.name,
                    ));
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
            tracing::trace!(relation = %relation.name, index = %layout.index_name, "put current index entry");
            let key = current_index_key(layout, relation, row)?;
            self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
            crate::failpoints::check(crate::failpoints::Failpoint::AfterCurrentIndexPut)?;
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
            tracing::trace!(relation = %relation.name, index = %layout.index_name, "delete current index entry");
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
            crate::failpoints::check(crate::failpoints::Failpoint::AfterUniqueGuardPut)?;
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
        self.history_seq = self
            .history_seq
            .checked_add(1)
            .ok_or_else(|| Error::internal("too many history records in one transaction"))?;

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
        crate::failpoints::check(crate::failpoints::Failpoint::AfterHistoryAppend)?;
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

struct PendingRelationSegment {
    row_count: usize,
    columns: Vec<PendingColumnSegment>,
    indexes: Vec<PendingIndexSegment>,
}

struct PendingColumnSegment {
    field: FieldId,
    bytes: Vec<u8>,
}

struct PendingIndexSegment {
    index_id: u16,
    bytes: Vec<u8>,
}

#[derive(Clone, Copy)]
struct SegmentMeta {
    tx_start: u64,
    tx_end: Option<u64>,
    row_count: usize,
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
    /// Looks up a row by primary key using the primary covering index.
    pub fn get_row(&self, schema: &StorageSchema, key: &KeyValues) -> Result<Option<Row>> {
        let (relation_id, relation) = schema.relation(&key.relation)?;
        let primary_layout = schema
            .layout(&key.relation, "primary")
            .ok_or_else(|| Error::unknown_index(&key.relation, "primary"))?;
        let primary = self.encode_primary_key_existing(relation, &key.values)?;
        let mut prefix = current_index_prefix(relation_id, primary_layout.index_id);
        prefix.extend_from_slice(&primary);
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;

        let Some((index_key, _)) = iter.next().transpose()? else {
            return Ok(None);
        };
        let item = decode_index_scan_item(
            self.dbs.dict,
            &self.txn,
            relation,
            primary_layout,
            index_key,
        )?;
        Ok(Some(item.row))
    }

    /// Scans a whole relation through the primary covering index.
    pub fn scan_relation<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
    ) -> Result<IndexScan<'borrow, 'env, 'schema>> {
        self.scan_index_with_prefix(schema, relation_name, "primary", &[], None)
    }

    /// Scans a covering index by a leading-field prefix.
    pub fn scan_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        prefix: &FieldValues,
    ) -> Result<IndexScan<'borrow, 'env, 'schema>> {
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
        self.scan_index_with_prefix(schema, relation_name, index_name, &encoded_prefix, None)
    }

    /// Scans a range index. Bounds are inclusive start and exclusive end.
    pub fn scan_range<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        start: Option<Value>,
        end: Option<Value>,
    ) -> Result<IndexScan<'borrow, 'env, 'schema>> {
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
            offset: current_index_prefix(layout.relation_id, layout.index_id).len(),
            width: field.value_type.encoded_width(),
            start,
            end,
        };

        self.scan_index_with_prefix(schema, relation_name, index_name, &[], Some(range))
    }

    /// Scans a covering index by encoded key prefix without decoding logical rows.
    pub(crate) fn scan_encoded_index_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        encoded_prefix: &[u8],
    ) -> Result<EncodedIndexScan<'borrow, 'env, 'schema>> {
        let (relation_id, _) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;
        let index_prefix = current_index_prefix(relation_id, layout.index_id);
        let mut scan_prefix = index_prefix.clone();
        scan_prefix.extend_from_slice(encoded_prefix);
        let iter = self
            .dbs
            .index
            .prefix_iter(&self.txn, scan_prefix.as_slice())?;
        Ok(EncodedIndexScan {
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
            Error::internal(format!("missing index {relation_name}.{index_name}"))
        })?;
        Ok(read_u64(
            &self.dbs.meta,
            &self.txn,
            &index_entry_count_key(layout.relation_id, layout.index_id),
        )?
        .unwrap_or(0))
    }

    /// Returns segment descriptors visible to this read snapshot.
    pub fn visible_segments(&self, schema: &StorageSchema) -> Result<Vec<SegmentDescriptor>> {
        let tx_id = self.last_committed_tx_id()?;
        let mut out = Vec::new();
        for (relation_id, relation) in schema.descriptor.relations.iter().enumerate() {
            if let Some(segment) = self.visible_relation_segment_at(
                schema,
                RelationId(relation_id as u16),
                relation,
                tx_id,
            )? {
                out.push(segment);
            }
        }
        Ok(out)
    }

    pub(crate) fn visible_relation_segment(
        &self,
        schema: &StorageSchema,
        relation: RelationId,
        descriptor: &RelationDescriptor,
    ) -> Result<Option<SegmentDescriptor>> {
        self.visible_relation_segment_at(schema, relation, descriptor, self.last_committed_tx_id()?)
    }

    fn visible_relation_segment_at(
        &self,
        schema: &StorageSchema,
        relation: RelationId,
        descriptor: &RelationDescriptor,
        tx_id: u64,
    ) -> Result<Option<SegmentDescriptor>> {
        let relation_id = relation.0;
        let prefix = segment_meta_prefix(relation_id);
        let mut visible = Vec::new();
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        while let Some((key, value)) = iter.next().transpose()? {
            let segment_id = parse_segment_id_from_meta_key(&prefix, key)?;
            let meta = decode_segment_meta(value)?;
            if meta.tx_start <= tx_id && meta.tx_end.is_none_or(|end| end > tx_id) {
                visible.push((segment_id, meta));
            }
        }
        drop(iter);

        let Some((segment_id, meta)) = visible
            .into_iter()
            .max_by_key(|(segment_id, _)| *segment_id)
        else {
            return Ok(None);
        };
        self.segment_descriptor_from_meta(schema, relation, descriptor, segment_id, meta)
            .map(Some)
    }

    fn segment_descriptor_from_meta(
        &self,
        schema: &StorageSchema,
        relation: RelationId,
        descriptor: &RelationDescriptor,
        segment_id: u64,
        meta: SegmentMeta,
    ) -> Result<SegmentDescriptor> {
        let relation_id = relation.0;
        let columns = descriptor
            .fields
            .iter()
            .enumerate()
            .map(|(field_id, field)| {
                let key = segment_column_key(relation_id, segment_id, field_id as u16);
                let byte_len = self
                    .dbs
                    .index
                    .get(&self.txn, key.as_slice())?
                    .map_or(0, |bytes| bytes.len());
                Ok(ColumnSegmentDescriptor {
                    field: FieldId(field_id as u16),
                    value_type: field.value_type.clone(),
                    width: field.value_type.encoded_width(),
                    lmdb_key: key,
                    byte_len,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let indexes = schema
            .layouts_for_relation(relation_id)
            .map(|layout| {
                let key = segment_index_key(relation_id, segment_id, layout.index_id);
                let byte_len = self
                    .dbs
                    .index
                    .get(&self.txn, key.as_slice())?
                    .map_or(0, |bytes| bytes.len());
                let fields = layout
                    .leading_fields
                    .iter()
                    .map(|field_name| {
                        descriptor
                            .fields
                            .iter()
                            .position(|field| &field.name == field_name)
                            .map(|field_id| FieldId(field_id as u16))
                            .ok_or_else(|| Error::unknown_field(&descriptor.name, field_name))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let row_count = byte_len.checked_div(layout.encoded_len).unwrap_or(0);
                Ok(IndexSegmentDescriptor {
                    access: AccessId(layout.index_id),
                    fields,
                    kind: layout.kind,
                    lmdb_key: key,
                    byte_len,
                    stats: IndexStatsSummary {
                        row_count,
                        depth: layout.leading_fields.len(),
                        byte_len,
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(SegmentDescriptor {
            relation,
            segment_id,
            tx_start: meta.tx_start,
            tx_end: meta.tx_end,
            row_count: meta.row_count,
            columns,
            indexes,
        })
    }

    pub(crate) fn segment_bytes(&self, key: &[u8]) -> Result<Vec<u8>> {
        self.dbs
            .index
            .get(&self.txn, key)?
            .map(ToOwned::to_owned)
            .ok_or_else(|| Error::corrupt("segment bytes missing"))
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
            Error::internal(format!("missing index {}.{index_name}", row.relation))
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

    /// Counts reverse dictionary entries across all dictionary kinds.
    pub fn dictionary_entry_count(&self) -> Result<usize> {
        let prefix = [DICT_REV];
        let mut iter = self.dbs.dict.prefix_iter(&self.txn, &prefix[..])?;
        let mut count = 0;
        while iter.next().transpose()?.is_some() {
            count += 1;
        }
        Ok(count)
    }

    fn scan_index_with_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        encoded_prefix: &[u8],
        range: Option<EncodedRange>,
    ) -> Result<IndexScan<'borrow, 'env, 'schema>> {
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
        let mut prefix = current_index_prefix(relation_id, layout.index_id);
        prefix.extend_from_slice(encoded_prefix);
        let iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        Ok(IndexScan {
            iter,
            txn: &self.txn,
            dict: self.dbs.dict,
            relation,
            layout,
            range,
        })
    }

    fn encode_index_prefix(
        &self,
        relation: &RelationDescriptor,
        layout: &CurrentIndexLayout,
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

    fn encode_row_existing(&self, relation: &RelationDescriptor, row: &Row) -> Result<EncodedRow> {
        let mut fields = BTreeMap::new();
        for field in &relation.fields {
            let value = row
                .values
                .get(&field.name)
                .ok_or_else(|| Error::missing_field(&relation.name, &field.name))?;
            fields.insert(
                field.name.clone(),
                encode_value_with(relation, field, value, |kind, raw| {
                    lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                        .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
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
                .ok_or_else(|| Error::unknown_field(&relation.name, field_name))?;
            let value = values
                .get(field_name)
                .ok_or_else(|| Error::missing_field(&relation.name, field_name))?;
            out.extend_from_slice(&encode_value_with(relation, field, value, |kind, raw| {
                lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                    .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
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
            return Err(Error::internal(format!(
                "query value type mismatch: expected {}, found {}",
                value_type_name(value_type),
                value.kind_name()
            )));
        }
    };

    Ok(bytes)
}

fn storage_value_matches_type(value: &Value, value_type: &ValueType) -> bool {
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (Value::Id(_), ValueType::Id { .. })
            | (Value::Ref(_), ValueType::Ref { .. })
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Uuid(_), ValueType::Uuid)
            | (Value::Symbol(_), ValueType::Symbol { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}

fn decode_index_scan_item(
    dict: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation: &RelationDescriptor,
    layout: &CurrentIndexLayout,
    key: &[u8],
) -> Result<ScanItem> {
    let (encoded, encoded_components) = decode_index_key(relation, layout, key)?;
    let row = decode_encoded_row(dict, txn, relation, &encoded)?;
    Ok(ScanItem {
        row,
        encoded_components,
    })
}

fn decode_index_key(
    relation: &RelationDescriptor,
    layout: &CurrentIndexLayout,
    key: &[u8],
) -> Result<(EncodedRow, Vec<EncodedComponent>)> {
    let prefix_len = current_index_prefix(layout.relation_id, layout.index_id).len();
    if key.len() != layout.encoded_len {
        return Err(Error::corrupt("index key width does not match layout"));
    }
    if key.get(0..prefix_len)
        != Some(current_index_prefix(layout.relation_id, layout.index_id).as_slice())
    {
        return Err(Error::corrupt("index key prefix does not match layout"));
    }

    let mut fields = BTreeMap::new();
    let mut components = Vec::with_capacity(layout.components.len());
    let mut offset = prefix_len;

    for component in &layout.components {
        let end = offset + component.encoded_width;
        let bytes = key
            .get(offset..end)
            .ok_or_else(|| Error::corrupt("index key component is truncated"))?
            .to_vec();
        fields.insert(component.field_name.clone(), bytes.clone());
        components.push(EncodedComponent {
            field_name: component.field_name.clone(),
            bytes,
        });
        offset = end;
    }

    if fields.len() != relation.fields.len() {
        return Err(Error::corrupt(
            "index key does not cover every relation field",
        ));
    }

    Ok((EncodedRow { fields }, components))
}

fn encoded_index_item(
    layout: &CurrentIndexLayout,
    index_prefix: &[u8],
    key: &[u8],
) -> Result<EncodedIndexItem> {
    let prefix_len = index_prefix.len();
    if key.len() != layout.encoded_len {
        return Err(Error::corrupt("index key width does not match layout"));
    }
    if key.get(0..prefix_len) != Some(index_prefix) {
        return Err(Error::corrupt("index key prefix does not match layout"));
    }
    Ok(EncodedIndexItem {
        key: key.to_vec(),
        prefix_len,
    })
}

fn decode_encoded_row(
    dict: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation: &RelationDescriptor,
    encoded: &EncodedRow,
) -> Result<Row> {
    let mut values = BTreeMap::new();
    for field in &relation.fields {
        let bytes = encoded.field(relation, &field.name)?;
        values.insert(
            field.name.clone(),
            decode_value(dict, txn, &field.value_type, bytes)?,
        );
    }
    Ok(Row {
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
        ValueType::Id { .. } => {
            Value::Id(decode_u64(bytes).map_err(|_| Error::corrupt("id width invalid"))?)
        }
        ValueType::Ref { .. } => {
            Value::Ref(decode_u64(bytes).map_err(|_| Error::corrupt("ref width invalid"))?)
        }
        ValueType::TimestampMicros => Value::Timestamp(
            decode_timestamp(bytes).map_err(|_| Error::corrupt("timestamp width invalid"))?,
        ),
        ValueType::Decimal { .. } => Value::Decimal(
            decode_decimal(bytes).map_err(|_| Error::corrupt("decimal width invalid"))?,
        ),
        ValueType::Uuid => {
            Value::Uuid(decode_uuid(bytes).map_err(|_| Error::corrupt("uuid width invalid"))?)
        }
        ValueType::Symbol { .. } => {
            Value::Symbol(decode_u64(bytes).map_err(|_| Error::corrupt("symbol width invalid"))?)
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

fn segment_meta_prefix(relation_id: u16) -> Vec<u8> {
    let mut key = SEGMENT_META_PREFIX.to_vec();
    push_u16(&mut key, relation_id);
    key
}

fn segment_meta_key(relation_id: u16, segment_id: u64) -> Vec<u8> {
    let mut key = segment_meta_prefix(relation_id);
    push_u64(&mut key, segment_id);
    key
}

fn segment_column_key(relation_id: u16, segment_id: u64, field_id: u16) -> Vec<u8> {
    let mut key = SEGMENT_COLUMN_PREFIX.to_vec();
    push_u16(&mut key, relation_id);
    push_u64(&mut key, segment_id);
    push_u16(&mut key, field_id);
    key
}

fn segment_index_key(relation_id: u16, segment_id: u64, index_id: u16) -> Vec<u8> {
    let mut key = SEGMENT_INDEX_PREFIX.to_vec();
    push_u16(&mut key, relation_id);
    push_u64(&mut key, segment_id);
    push_u16(&mut key, index_id);
    key
}

fn segment_visibility_key(tx_id: u64, relation_id: u16, segment_id: u64) -> Vec<u8> {
    let mut key = SEGMENT_VISIBILITY_PREFIX.to_vec();
    push_u64(&mut key, tx_id);
    push_u16(&mut key, relation_id);
    push_u64(&mut key, segment_id);
    key
}

fn segment_next_key(relation_id: u16) -> Vec<u8> {
    let mut key = SEGMENT_NEXT_PREFIX.to_vec();
    push_u16(&mut key, relation_id);
    key
}

fn parse_segment_id_from_meta_key(prefix: &[u8], key: &[u8]) -> Result<u64> {
    let bytes = key
        .get(prefix.len()..prefix.len() + 8)
        .ok_or_else(|| Error::corrupt("segment metadata key is truncated"))?;
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| Error::corrupt("segment id width invalid"))?;
    Ok(u64::from_be_bytes(bytes))
}

fn encode_segment_meta(tx_start: u64, tx_end: Option<u64>, row_count: usize) -> Vec<u8> {
    let mut value = Vec::with_capacity(24);
    push_u64(&mut value, tx_start);
    push_u64(&mut value, tx_end.unwrap_or(0));
    push_u64(&mut value, row_count as u64);
    value
}

fn decode_segment_meta(value: &[u8]) -> Result<SegmentMeta> {
    if value.len() != 24 {
        return Err(Error::corrupt("segment metadata width invalid"));
    }
    let tx_start = u64::from_be_bytes(
        value[0..8]
            .try_into()
            .map_err(|_| Error::corrupt("segment tx_start width invalid"))?,
    );
    let raw_tx_end = u64::from_be_bytes(
        value[8..16]
            .try_into()
            .map_err(|_| Error::corrupt("segment tx_end width invalid"))?,
    );
    let row_count = u64::from_be_bytes(
        value[16..24]
            .try_into()
            .map_err(|_| Error::corrupt("segment row_count width invalid"))?,
    );
    Ok(SegmentMeta {
        tx_start,
        tx_end: (raw_tx_end != 0).then_some(raw_tx_end),
        row_count: row_count
            .try_into()
            .map_err(|_| Error::corrupt("segment row_count too large"))?,
    })
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
        .map_err(|_| Error::corrupt("u64 metadata must be eight bytes"))?;
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
    use crate::{ConstraintError, Environment};
    use bumbledb_core::schema::{
        ConstraintDescriptor, FieldDescriptor, GeneratedIdDescriptor, PrimaryKeyDescriptor,
        RelationKind,
    };

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn inserts_rows_indexes_history_stats_and_reopens() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = storage_schema(&env)?;

        env.write(|txn| {
            let holder = txn.alloc_id(&schema, "Holder")?;
            let account = txn.alloc_id(&schema, "Account")?;
            assert_eq!(holder, 1);
            assert_eq!(account, 1);

            txn.insert(&schema, holder_row(holder, "Alice"))?;
            txn.insert(&schema, account_row(account, holder, 840))?;
            Ok::<(), Error>(())
        })?;

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
        })?;

        drop(env);
        let env = Environment::open(dir.path())?;
        let schema = storage_schema(&env)?;
        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 1);
            assert_eq!(txn.relation_row_count(&schema, "Holder")?, 1);
            assert!(txn.row_exists(&schema, &holder_key(1))?);
            assert!(txn.dictionary_string_id("Alice")?.is_some());
            Ok::<(), Error>(())
        })?;

        assert!(
            env.write(|txn| {
                assert_eq!(txn.alloc_id(&schema, "Holder")?, 2);
                Err::<(), Error>(Error::internal("rollback counter check"))
            })
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn duplicate_unique_and_foreign_key_failures_abort_cleanly() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = storage_schema(&env)?;

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            Ok::<(), Error>(())
        })?;

        let duplicate = env.write(|txn| txn.insert(&schema, holder_row(1, "Bob")));
        assert!(matches!(
            duplicate,
            Err(Error::Constraint(ConstraintError::DuplicateTuple { .. }))
        ));

        let unique = env.write(|txn| txn.insert(&schema, holder_row(2, "Alice")));
        assert!(matches!(
            unique,
            Err(Error::Constraint(ConstraintError::UniqueViolation { .. }))
        ));

        let fk = env.write(|txn| txn.insert(&schema, account_row(1, 999, 840)));
        assert!(matches!(
            fk,
            Err(Error::Constraint(
                ConstraintError::ForeignKeyViolation { .. }
            ))
        ));

        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 1);
            assert_eq!(txn.history_entry_count()?, 1);
            assert_eq!(txn.relation_row_count(&schema, "Holder")?, 1);
            assert_eq!(txn.relation_row_count(&schema, "Account")?, 0);
            assert_eq!(txn.dictionary_string_id("Bob")?, None);
            Ok::<(), Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn replace_removes_old_current_entries_and_preserves_counts() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = storage_schema(&env)?;

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            Ok::<(), Error>(())
        })?;

        env.write(|txn| {
            txn.replace(&schema, account_row(1, 1, 978))?;
            Ok::<(), Error>(())
        })?;

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
        })?;

        env.write(|txn| {
            txn.insert(&schema, account_row(2, 1, 840))?;
            Ok::<(), Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn deletes_restrict_then_remove_indexes_and_rows() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = storage_schema(&env)?;

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            Ok::<(), Error>(())
        })?;

        let restricted = env.write(|txn| txn.delete(&schema, holder_key(1)));
        assert!(matches!(
            restricted,
            Err(Error::Constraint(ConstraintError::RestrictViolation { .. }))
        ));

        env.write(|txn| {
            txn.delete(&schema, account_key(1))?;
            txn.delete(&schema, holder_key(1))?;
            Ok::<(), Error>(())
        })?;

        env.read(|txn| {
            assert_eq!(txn.last_committed_tx_id()?, 2);
            assert_eq!(txn.history_entry_count()?, 4);
            assert_eq!(txn.relation_row_count(&schema, "Holder")?, 0);
            assert_eq!(txn.relation_row_count(&schema, "Account")?, 0);
            assert!(!txn.row_exists(&schema, &holder_key(1))?);
            assert_eq!(txn.index_entry_count(&schema, "Account", "by_holder")?, 0);
            Ok::<(), Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn composite_tuples_insert_duplicate_and_delete() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = storage_schema(&env)?;

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            txn.insert_tuple(&schema, tag_row(1, 7))?;
            Ok::<(), Error>(())
        })?;

        let duplicate = env.write(|txn| txn.insert_tuple(&schema, tag_row(1, 7)));
        assert!(matches!(
            duplicate,
            Err(Error::Constraint(ConstraintError::DuplicateTuple { .. }))
        ));

        env.write(|txn| {
            txn.delete_tuple(&schema, tag_row(1, 7))?;
            Ok::<(), Error>(())
        })?;

        env.read(|txn| {
            assert_eq!(txn.relation_row_count(&schema, "AccountTag")?, 0);
            assert_eq!(txn.index_entry_count(&schema, "AccountTag", "primary")?, 0);
            assert_eq!(
                txn.index_entry_count(&schema, "AccountTag", "by_account")?,
                0
            );
            Ok::<(), Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn read_access_paths_decode_rows_and_preserve_snapshots() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = storage_schema(&env)?;

        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, holder_row(2, "Bob"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            txn.insert(&schema, account_row(2, 1, 978))?;
            Ok::<(), Error>(())
        })?;

        env.read(|txn| {
            assert_eq!(
                txn.get_row(&schema, &holder_key(1))?,
                Some(holder_row(1, "Alice"))
            );
            assert_eq!(
                txn.get_row(&schema, &account_key(1))?,
                Some(account_row(1, 1, 840))
            );

            let access_paths = schema.access_paths("Account")?;
            assert!(access_paths.iter().any(|path| path.index_name == "primary"));
            assert!(
                access_paths
                    .iter()
                    .any(|path| path.index_name == "by_holder")
            );
            assert!(
                access_paths
                    .iter()
                    .any(|path| path.index_name == "by_opened")
            );
            assert!(
                access_paths
                    .iter()
                    .any(|path| path.index_name == "unique_holder_currency")
            );

            let full = collect_rows(txn.scan_relation(&schema, "Account")?)?;
            assert_same_rows(&full, &[account_row(1, 1, 840), account_row(2, 1, 978)])?;

            let by_holder_items = collect_items(txn.scan_prefix(
                &schema,
                "Account",
                "by_holder",
                &FieldValues::new("Account", [("holder", Value::Ref(1))]),
            )?)?;
            assert_same_rows(
                &by_holder_items
                    .iter()
                    .map(|item| item.row.clone())
                    .collect::<Vec<_>>(),
                &[account_row(1, 1, 840), account_row(2, 1, 978)],
            )?;
            assert!(
                by_holder_items
                    .iter()
                    .all(|item| item.encoded_component("holder").is_some())
            );

            let unique_holder = collect_rows(txn.scan_prefix(
                &schema,
                "Holder",
                "unique_name",
                &FieldValues::new("Holder", [("name", Value::String("Alice".to_owned()))]),
            )?)?;
            assert_eq!(unique_holder, [holder_row(1, "Alice")]);

            let ranged = collect_rows(txn.scan_range(
                &schema,
                "Account",
                "by_opened",
                Some(Value::Timestamp(TimestampMicros(15))),
                Some(Value::Timestamp(TimestampMicros(31))),
            )?)?;
            assert_same_rows(&ranged, &[account_row(2, 1, 978)])?;

            for path in access_paths {
                let rows = collect_rows(txn.scan_prefix(
                    &schema,
                    "Account",
                    &path.index_name,
                    &FieldValues::new("Account", std::iter::empty::<(&str, Value)>()),
                )?)?;
                assert_same_rows(&rows, &[account_row(1, 1, 840), account_row(2, 1, 978)])?;
            }

            env.write(|write| {
                write.insert(&schema, account_row(3, 2, 840))?;
                Ok::<(), Error>(())
            })?;

            let still_two = collect_rows(txn.scan_relation(&schema, "Account")?)?;
            assert_same_rows(
                &still_two,
                &[account_row(1, 1, 840), account_row(2, 1, 978)],
            )?;
            Ok::<(), Error>(())
        })?;

        env.read(|txn| {
            let now_three = collect_rows(txn.scan_relation(&schema, "Account")?)?;
            assert_same_rows(
                &now_three,
                &[
                    account_row(1, 1, 840),
                    account_row(2, 1, 978),
                    account_row(3, 2, 840),
                ],
            )?;
            Ok::<(), Error>(())
        })?;
        Ok(())
    }

    fn storage_schema(env: &Environment) -> Result<StorageSchema> {
        StorageSchema::new(ledger_schema(), env.max_key_size())
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
                        FieldDescriptor::new("opened", ValueType::TimestampMicros).range_indexed(),
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
                (
                    "opened",
                    Value::Timestamp(TimestampMicros((id as i64) * 10)),
                ),
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

    fn collect_items(scan: IndexScan<'_, '_, '_>) -> Result<Vec<ScanItem>> {
        scan.collect()
    }

    fn collect_rows(scan: IndexScan<'_, '_, '_>) -> Result<Vec<Row>> {
        scan.map(|item| item.map(|item| item.row)).collect()
    }

    fn assert_same_rows(actual: &[Row], expected: &[Row]) -> Result<()> {
        let mut actual = row_keys(actual)?;
        let mut expected = row_keys(expected)?;
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
        Ok(())
    }

    fn row_keys(rows: &[Row]) -> Result<Vec<(u64, u64, u64, i64)>> {
        rows.iter()
            .map(|row| {
                let id = match required_value(row, "id")? {
                    Value::Id(value) => *value,
                    other => {
                        return Err(Error::internal(format!("unexpected id value: {other:?}")));
                    }
                };
                let holder = match required_value(row, "holder")? {
                    Value::Ref(value) => *value,
                    other => {
                        return Err(Error::internal(format!(
                            "unexpected holder value: {other:?}"
                        )));
                    }
                };
                let currency = match required_value(row, "currency")? {
                    Value::Symbol(value) => *value,
                    other => {
                        return Err(Error::internal(format!(
                            "unexpected currency value: {other:?}"
                        )));
                    }
                };
                let opened = match required_value(row, "opened")? {
                    Value::Timestamp(value) => value.0,
                    other => {
                        return Err(Error::internal(format!(
                            "unexpected opened value: {other:?}"
                        )));
                    }
                };
                Ok((id, holder, currency, opened))
            })
            .collect()
    }

    fn required_value<'a>(row: &'a Row, field: &str) -> Result<&'a Value> {
        row.value(field)
            .ok_or_else(|| Error::internal(format!("missing field {field}")))
    }
}
