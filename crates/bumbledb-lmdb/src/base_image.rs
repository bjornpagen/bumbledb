#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use bumbledb_core::schema::RelationDescriptor;

use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::storage_format::{
    FactHandle, column_prefix_key, decode_column_key_handle, live_row_key,
};
use crate::{Error, ReadTxn, Result, StorageSchema};

/// Snapshot-local immutable relation image for GHT/COLT sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelationBaseImage {
    /// Relation descriptor ID.
    pub(crate) relation_id: u32,
    /// Relation name.
    pub(crate) name: String,
    /// Live fact handles in deterministic snapshot order.
    pub(crate) row_handles: Vec<FactHandle>,
    /// Loaded columns by field ID.
    pub(crate) columns: BTreeMap<usize, ColumnImage>,
    /// Lightweight relation stats.
    pub(crate) stats: RelationStats,
}

pub(crate) type RelationBaseImageRef = Rc<RelationBaseImage>;

/// One fixed-width encoded field column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ColumnImage {
    /// Field descriptor ID.
    pub(crate) field_id: usize,
    /// Fixed encoded width of one cell.
    pub(crate) width: usize,
    /// Contiguous encoded values aligned with `RelationBaseImage::row_handles`.
    pub(crate) values: Vec<u8>,
}

impl ColumnImage {
    /// Returns a zero-copy cell slice by row offset.
    pub(crate) fn value_at(&self, offset: usize) -> Option<&[u8]> {
        let start = offset.checked_mul(self.width)?;
        let end = start.checked_add(self.width)?;
        self.values.get(start..end)
    }

    /// Returns the number of loaded rows in this column.
    pub(crate) fn row_count(&self) -> usize {
        if self.width == 0 {
            return 0;
        }
        self.values.len() / self.width
    }
}

/// Relation statistics visible to planning/execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelationStats {
    /// Number of live rows in this image.
    pub(crate) row_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BaseImageCacheKey {
    schema: [u8; 32],
    storage_tx_id: u64,
    relation_id: u32,
    field_ids: FieldScope,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FieldScope {
    words: [u64; 4],
}

impl FieldScope {
    pub(crate) fn insert(&mut self, field_id: usize) {
        let word = field_id / 64;
        if let Some(slot) = self.words.get_mut(word) {
            *slot |= 1u64 << (field_id % 64);
        }
    }

    pub(crate) fn extend(&mut self, field_ids: impl IntoIterator<Item = usize>) {
        for field_id in field_ids {
            self.insert(field_id);
        }
    }

    pub(crate) fn iter(self) -> FieldScopeIter {
        FieldScopeIter {
            scope: self,
            next: 0,
        }
    }
}

pub(crate) struct FieldScopeIter {
    scope: FieldScope,
    next: usize,
}

impl Iterator for FieldScopeIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.scope.words.len() * 64 {
            let field_id = self.next;
            self.next += 1;
            if self.scope.words[field_id / 64] & (1u64 << (field_id % 64)) != 0 {
                return Some(field_id);
            }
        }
        None
    }
}

/// Process-local cache for immutable base images keyed by LMDB snapshot metadata.
#[derive(Default)]
pub(crate) struct BaseImageCache {
    images: RefCell<Vec<BaseImageCacheEntry>>,
}

struct BaseImageCacheEntry {
    key: BaseImageCacheKey,
    image: Rc<RelationBaseImage>,
}

/// Builds or retrieves a relation base image for the current read snapshot.
pub(crate) fn relation_base_image(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    relation_name: &str,
    field_ids: impl IntoIterator<Item = usize>,
) -> Result<RelationBaseImageRef> {
    relation_base_image_inner(txn, schema, relation_name, field_ids, None)
}

pub(crate) fn relation_base_image_with_trace(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    relation_name: &str,
    field_ids: impl IntoIterator<Item = usize>,
    trace: &mut QueryTrace,
) -> Result<RelationBaseImageRef> {
    relation_base_image_inner(txn, schema, relation_name, field_ids, Some(trace))
}

fn relation_base_image_inner(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    relation_name: &str,
    field_ids: impl IntoIterator<Item = usize>,
    mut trace: Option<&mut QueryTrace>,
) -> Result<RelationBaseImageRef> {
    let (relation_id, relation) = find_relation(schema, relation_name)?;
    let mut field_scope = FieldScope::default();
    field_scope.extend(field_ids);
    validate_fields(relation, field_scope)?;
    let key = BaseImageCacheKey {
        schema: schema.descriptor().fingerprint().0,
        storage_tx_id: txn.storage_tx_id()?,
        relation_id,
        field_ids: field_scope,
    };
    let lookup_span = trace.as_deref_mut().and_then(|trace| {
        crate::query_trace_span!(
            trace,
            TracePhase::BaseImageCacheLookup,
            "relation={} fields={:?}",
            relation_name,
            key.field_ids
        )
    });
    if let Some(image) = txn
        .base_images
        .images
        .borrow()
        .iter()
        .find_map(|entry| (entry.key == key).then_some(&entry.image))
    {
        if let (Some(trace), Some(span)) = (trace.as_deref_mut(), lookup_span) {
            trace.finish_span(
                span,
                TraceCounters {
                    base_image_cache_hits: 1,
                    ..TraceCounters::default()
                },
            );
        }
        return Ok(Rc::clone(image));
    }
    if let (Some(trace), Some(span)) = (trace.as_deref_mut(), lookup_span) {
        trace.finish_span(
            span,
            TraceCounters {
                base_image_cache_misses: 1,
                ..TraceCounters::default()
            },
        );
    }

    let load_span = trace.as_deref_mut().and_then(|trace| {
        crate::query_trace_span!(
            trace,
            TracePhase::BaseImageLoad,
            "relation={} fields={:?}",
            relation_name,
            key.field_ids
        )
    });
    let image = Rc::new(load_relation_base_image(
        txn,
        relation_id,
        relation,
        key.field_ids,
    )?);
    if let (Some(trace), Some(span)) = (trace, load_span) {
        trace.finish_span(span, base_image_counters(&image));
    }
    txn.base_images
        .images
        .borrow_mut()
        .push(BaseImageCacheEntry {
            key,
            image: Rc::clone(&image),
        });
    Ok(image)
}

fn base_image_counters(image: &RelationBaseImage) -> TraceCounters {
    let column_values_loaded = image
        .columns
        .values()
        .map(|column| column.row_count() as u64)
        .sum();
    let loaded_bytes = image
        .columns
        .values()
        .map(|column| column.values.len() as u64)
        .sum();
    TraceCounters {
        live_rows_scanned: image.row_handles.len() as u64,
        column_values_loaded,
        loaded_bytes,
        ..TraceCounters::default()
    }
}

/// Computes required field IDs per atom occurrence from a validated FJ plan.
pub(crate) fn field_scope_for_plan(
    plan: &ValidatedFjPlan,
) -> BTreeMap<AtomOccurrenceId, FieldScope> {
    let mut scope = BTreeMap::new();
    for node in &plan.nodes {
        for subatom in plan.node_subatoms(node) {
            scope
                .entry(subatom.atom)
                .or_insert_with(FieldScope::default)
                .extend(plan.subatom_field_ids(subatom).iter().copied());
        }
    }
    scope
}

fn load_relation_base_image(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    relation: &RelationDescriptor,
    field_ids: FieldScope,
) -> Result<RelationBaseImage> {
    let row_handles = live_row_handles(txn, relation_id)?;
    let mut columns = BTreeMap::new();
    for field_id in field_ids.iter() {
        let field = &relation.fields[field_id];
        let width = field.value_type.encoded_width();
        let values = load_column_values(txn, relation_id, field_id, width, &row_handles)?;
        columns.insert(
            field_id,
            ColumnImage {
                field_id,
                width,
                values,
            },
        );
    }

    Ok(RelationBaseImage {
        relation_id,
        name: relation.name.clone(),
        stats: RelationStats {
            row_count: row_handles.len(),
        },
        row_handles,
        columns,
    })
}

fn load_column_values(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
    row_handles: &[FactHandle],
) -> Result<Vec<u8>> {
    let prefix_key = column_prefix_key(relation_id, field_id as u32);
    let prefix = prefix_key.as_bytes();
    let mut values = Vec::with_capacity(row_handles.len() * width);
    let mut live_index = 0usize;

    for item in txn.dbs.data.prefix_iter(&txn.txn, prefix)? {
        let (key, value) = item?;
        let handle = decode_column_key_handle(key)
            .ok_or_else(|| Error::corrupt("column key handle width invalid"))?;
        if value.len() != width {
            return Err(Error::corrupt(format!(
                "column entry width mismatch for field {field_id}"
            )));
        }
        if live_index < row_handles.len() && row_handles[live_index] < handle {
            return Err(Error::corrupt(format!(
                "column entry missing for live row field {field_id}"
            )));
        }
        if live_index == row_handles.len() || row_handles[live_index] > handle {
            return Err(Error::corrupt(format!(
                "column entry without live row for field {field_id}"
            )));
        }
        values.extend_from_slice(value);
        live_index += 1;
    }

    if live_index != row_handles.len() {
        return Err(Error::corrupt(format!(
            "column entry missing for live row field {field_id}"
        )));
    }
    Ok(values)
}

fn live_row_handles(txn: &ReadTxn<'_>, relation_id: u32) -> Result<Vec<FactHandle>> {
    let prefix_key = live_row_key(relation_id, FactHandle([0; 16]));
    let prefix = &prefix_key[..5];
    let mut handles = Vec::new();
    for item in txn.dbs.data.prefix_iter(&txn.txn, prefix)? {
        let (key, _) = item?;
        let handle_bytes: [u8; 16] = key
            .get(5..21)
            .ok_or_else(|| Error::corrupt("live row key too short"))?
            .try_into()
            .map_err(|_| Error::corrupt("live row handle width invalid"))?;
        handles.push(FactHandle(handle_bytes));
    }
    Ok(handles)
}

fn find_relation<'schema>(
    schema: &'schema StorageSchema,
    name: &str,
) -> Result<(u32, &'schema RelationDescriptor)> {
    schema
        .descriptor()
        .relations
        .iter()
        .enumerate()
        .find(|(_, relation)| relation.name == name)
        .map(|(id, relation)| (id as u32, relation))
        .ok_or_else(|| Error::invalid_fact(format!("unknown relation {name}")))
}

fn validate_fields(relation: &RelationDescriptor, field_ids: FieldScope) -> Result<()> {
    for field_id in field_ids.iter() {
        if field_id >= relation.fields.len() {
            return Err(Error::invalid_fact(format!(
                "unknown field id {field_id} in relation {}",
                relation.name
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "base_image_tests.rs"]
mod tests;
