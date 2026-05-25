#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use bumbledb_core::schema::RelationDescriptor;

use crate::colt_filter::SourceFilter;
use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::storage_format::FactHandle;
use crate::{Error, ReadTxn, Result, StorageSchema};

#[path = "base_image/load.rs"]
mod load;

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

pub(crate) fn relation_base_image_filtered_with_trace(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    relation_name: &str,
    field_ids: impl IntoIterator<Item = usize>,
    filters: &[SourceFilter],
    trace: &mut QueryTrace,
) -> Result<RelationBaseImageRef> {
    if filters.is_empty() {
        return relation_base_image_with_trace(txn, schema, relation_name, field_ids, trace);
    }

    let (relation_id, relation) = find_relation(schema, relation_name)?;
    let mut field_scope = FieldScope::default();
    field_scope.extend(field_ids);
    validate_fields(relation, field_scope)?;
    let load_span = crate::query_trace_span!(
        trace,
        TracePhase::BaseImageLoad,
        "relation={} fields={:?} filters=pruned",
        relation_name,
        field_scope
    );
    let (image, rows_tested) =
        load::load_filtered_relation_base_image(txn, relation_id, relation, field_scope, filters)?;
    let image = Rc::new(image);
    if let Some(span) = load_span {
        trace.finish_span(span, filtered_base_image_counters(&image, rows_tested));
    }
    Ok(image)
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
    let image = Rc::new(load::load_relation_base_image(
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

fn filtered_base_image_counters(image: &RelationBaseImage, rows_tested: usize) -> TraceCounters {
    let mut counters = base_image_counters(image);
    counters.source_filter_rows_tested = rows_tested as u64;
    counters.source_filter_survivors = image.row_handles.len() as u64;
    counters
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
