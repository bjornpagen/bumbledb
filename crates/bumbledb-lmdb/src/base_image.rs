#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use bumbledb_core::schema::RelationDescriptor;

use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::storage_format::{FactHandle, column_key, live_row_key};
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

/// One fixed-width encoded field column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ColumnImage {
    /// Field descriptor ID.
    pub(crate) field_id: usize,
    /// Field name.
    pub(crate) field: String,
    /// Encoded values aligned with `RelationBaseImage::row_handles`.
    pub(crate) values: Vec<Vec<u8>>,
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
    field_ids: Vec<usize>,
}

/// Process-local cache for immutable base images keyed by LMDB snapshot metadata.
#[derive(Default)]
pub(crate) struct BaseImageCache {
    images: Mutex<BTreeMap<BaseImageCacheKey, Arc<RelationBaseImage>>>,
}

/// Builds or retrieves a relation base image for the current read snapshot.
pub(crate) fn relation_base_image(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    relation_name: &str,
    field_ids: impl IntoIterator<Item = usize>,
) -> Result<Arc<RelationBaseImage>> {
    relation_base_image_inner(txn, schema, relation_name, field_ids, None)
}

pub(crate) fn relation_base_image_with_trace(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    relation_name: &str,
    field_ids: impl IntoIterator<Item = usize>,
    trace: &mut QueryTrace,
) -> Result<Arc<RelationBaseImage>> {
    relation_base_image_inner(txn, schema, relation_name, field_ids, Some(trace))
}

fn relation_base_image_inner(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    relation_name: &str,
    field_ids: impl IntoIterator<Item = usize>,
    mut trace: Option<&mut QueryTrace>,
) -> Result<Arc<RelationBaseImage>> {
    let (relation_id, relation) = find_relation(schema, relation_name)?;
    let mut field_ids: Vec<_> = field_ids.into_iter().collect();
    field_ids.sort_unstable();
    field_ids.dedup();
    validate_fields(relation, &field_ids)?;
    let key = BaseImageCacheKey {
        schema: schema.descriptor().fingerprint().0,
        storage_tx_id: txn.storage_tx_id()?,
        relation_id,
        field_ids,
    };
    let lookup_span = trace.as_deref_mut().and_then(|trace| {
        trace.start_span(
            TracePhase::BaseImageCacheLookup,
            format!("relation={relation_name} fields={:?}", key.field_ids),
        )
    });
    if let Some(image) = txn.base_images.images.lock().map_err(lock_error)?.get(&key) {
        if let (Some(trace), Some(span)) = (trace.as_deref_mut(), lookup_span) {
            trace.finish_span(
                span,
                TraceCounters {
                    base_image_cache_hits: 1,
                    ..TraceCounters::default()
                },
            );
        }
        return Ok(Arc::clone(image));
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
        trace.start_span(
            TracePhase::BaseImageLoad,
            format!("relation={relation_name} fields={:?}", key.field_ids),
        )
    });
    let image = Arc::new(load_relation_base_image(
        txn,
        relation_id,
        relation,
        &key.field_ids,
    )?);
    if let (Some(trace), Some(span)) = (trace, load_span) {
        trace.finish_span(span, base_image_counters(&image));
    }
    txn.base_images
        .images
        .lock()
        .map_err(lock_error)?
        .insert(key, Arc::clone(&image));
    Ok(image)
}

fn base_image_counters(image: &RelationBaseImage) -> TraceCounters {
    let column_values_loaded = image
        .columns
        .values()
        .map(|column| column.values.len() as u64)
        .sum();
    let loaded_bytes = image
        .columns
        .values()
        .flat_map(|column| column.values.iter())
        .map(|value| value.len() as u64)
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
) -> BTreeMap<AtomOccurrenceId, BTreeSet<usize>> {
    let mut scope = BTreeMap::new();
    for node in &plan.nodes {
        for subatom in &node.subatoms {
            scope
                .entry(subatom.atom)
                .or_insert_with(BTreeSet::new)
                .extend(subatom.field_ids.iter().copied());
        }
    }
    scope
}

fn load_relation_base_image(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    relation: &RelationDescriptor,
    field_ids: &[usize],
) -> Result<RelationBaseImage> {
    let row_handles = live_row_handles(txn, relation_id)?;
    let mut columns = BTreeMap::new();
    for field_id in field_ids {
        let field = &relation.fields[*field_id];
        let mut values = Vec::with_capacity(row_handles.len());
        for handle in &row_handles {
            let key = column_key(relation_id, *field_id as u32, *handle);
            let value = txn
                .dbs
                .data
                .get(&txn.txn, &key)?
                .ok_or_else(|| Error::corrupt("column entry missing for live row"))?;
            values.push(value.to_vec());
        }
        columns.insert(
            *field_id,
            ColumnImage {
                field_id: *field_id,
                field: field.name.clone(),
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

fn validate_fields(relation: &RelationDescriptor, field_ids: &[usize]) -> Result<()> {
    for field_id in field_ids {
        if *field_id >= relation.fields.len() {
            return Err(Error::invalid_fact(format!(
                "unknown field id {field_id} in relation {}",
                relation.name
            )));
        }
    }
    Ok(())
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> Error {
    Error::corrupt("base image cache lock poisoned")
}

#[cfg(test)]
#[path = "base_image_tests.rs"]
mod tests;
