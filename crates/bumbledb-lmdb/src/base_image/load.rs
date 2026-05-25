use std::collections::BTreeMap;
use std::rc::Rc;

use bumbledb_core::schema::RelationDescriptor;

use super::{
    CacheScope, ColumnImage, FieldScope, PhysicalCacheStats, RelationBaseImage, RelationStats,
    validate_fields,
};
use crate::colt_filter::{SourceFilter, SourceFilterOp};
use crate::storage_format::{
    FactHandle, column_key, column_prefix_key, decode_column_key_handle, live_row_key,
};
use crate::{Error, ReadTxn, Result};

pub(super) fn load_relation_base_image(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    relation: &RelationDescriptor,
    field_ids: FieldScope,
) -> Result<LoadResult> {
    let mut cache_stats = PhysicalCacheStats::default();
    let row_handles = cached_row_handles(txn, scope, &mut cache_stats)?;
    let mut columns = BTreeMap::new();
    for field_id in field_ids.iter() {
        let field = &relation.fields[field_id];
        let width = field.value_type.encoded_width();
        let column =
            cached_full_column(txn, scope, field_id, width, &row_handles, &mut cache_stats)?;
        columns.insert(field_id, column);
    }

    Ok(LoadResult {
        image: RelationBaseImage {
            relation_id: scope.relation_id,
            name: relation.name.clone(),
            stats: RelationStats {
                row_count: row_handles.len(),
            },
            row_handles,
            columns,
        },
        rows_tested: 0,
        cache_stats,
    })
}

pub(super) struct LoadResult {
    pub(super) image: RelationBaseImage,
    pub(super) rows_tested: usize,
    pub(super) cache_stats: PhysicalCacheStats,
}

pub(super) fn load_filtered_relation_base_image(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    relation: &RelationDescriptor,
    field_ids: FieldScope,
    filters: &[SourceFilter],
) -> Result<LoadResult> {
    let mut cache_stats = PhysicalCacheStats::default();
    if filters
        .iter()
        .any(|filter| matches!(filter, SourceFilter::False))
    {
        return Ok(LoadResult {
            image: RelationBaseImage {
                relation_id: scope.relation_id,
                name: relation.name.clone(),
                stats: RelationStats { row_count: 0 },
                row_handles: Rc::new(Vec::new()),
                columns: BTreeMap::new(),
            },
            rows_tested: 0,
            cache_stats,
        });
    }
    let mut filter_scope = FieldScope::default();
    filter_scope.extend(filters.iter().filter_map(SourceFilter::field_id));
    validate_fields(relation, filter_scope)?;
    let primary_filter_field = filters
        .iter()
        .find_map(SourceFilter::field_id)
        .ok_or_else(|| Error::corrupt("filtered base image without filter field"))?;

    let mut filter_columns = BTreeMap::new();
    let primary_width = relation.fields[primary_filter_field]
        .value_type
        .encoded_width();
    let (all_handles, primary_column) = cached_primary_filter_column(
        txn,
        scope,
        primary_filter_field,
        primary_width,
        &mut cache_stats,
    )?;
    filter_columns.insert(primary_filter_field, primary_column);
    for field_id in filter_scope.iter() {
        if field_id == primary_filter_field {
            continue;
        }
        let width = relation.fields[field_id].value_type.encoded_width();
        let column =
            cached_full_column(txn, scope, field_id, width, &all_handles, &mut cache_stats)?;
        filter_columns.insert(field_id, column);
    }

    let compiled_filters = compile_filters(&filter_columns, filters)?;
    let survivor_offsets = (0..all_handles.len())
        .filter(|offset| {
            compiled_filters
                .iter()
                .all(|filter| filter.matches(*offset))
        })
        .collect::<Vec<_>>();
    let survivor_handles = survivor_offsets
        .iter()
        .map(|offset| all_handles[*offset])
        .collect::<Vec<_>>();
    let survivor_handles = Rc::new(survivor_handles);

    let mut columns = BTreeMap::new();
    for field_id in field_ids.iter() {
        let width = relation.fields[field_id].value_type.encoded_width();
        let values = if let Some(filter_column) = filter_columns.get(&field_id) {
            selected_column_values(filter_column, &survivor_offsets)?
        } else {
            load_column_values_for_selection(
                txn,
                scope.relation_id,
                field_id,
                width,
                &all_handles,
                &survivor_handles,
            )?
        };
        columns.insert(
            field_id,
            ColumnImage {
                field_id,
                width,
                values: Rc::new(values),
            },
        );
    }

    Ok(LoadResult {
        image: RelationBaseImage {
            relation_id: scope.relation_id,
            name: relation.name.clone(),
            stats: RelationStats {
                row_count: survivor_handles.len(),
            },
            row_handles: Rc::clone(&survivor_handles),
            columns,
        },
        rows_tested: all_handles.len(),
        cache_stats,
    })
}

fn cached_row_handles(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    stats: &mut PhysicalCacheStats,
) -> Result<Rc<Vec<FactHandle>>> {
    if let Some(row_handles) = txn.base_images.row_handles(scope) {
        stats.hits += 1;
        return Ok(row_handles);
    }
    stats.misses += 1;
    let row_handles = Rc::new(live_row_handles(txn, scope.relation_id)?);
    txn.base_images
        .insert_row_handles(scope, Rc::clone(&row_handles));
    Ok(row_handles)
}

fn cached_full_column(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    field_id: usize,
    width: usize,
    row_handles: &[FactHandle],
    stats: &mut PhysicalCacheStats,
) -> Result<ColumnImage> {
    if let Some(column) = txn.base_images.column(scope, field_id) {
        stats.hits += 1;
        return Ok(column);
    }
    stats.misses += 1;
    let values = load_column_values(txn, scope.relation_id, field_id, width, row_handles)?;
    let column = ColumnImage {
        field_id,
        width,
        values: Rc::new(values),
    };
    txn.base_images
        .insert_column(scope, field_id, column.clone());
    Ok(column)
}

fn cached_primary_filter_column(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    field_id: usize,
    width: usize,
    stats: &mut PhysicalCacheStats,
) -> Result<(Rc<Vec<FactHandle>>, ColumnImage)> {
    let cached_rows = txn.base_images.row_handles(scope);
    let cached_column = txn.base_images.column(scope, field_id);
    match (cached_rows, cached_column) {
        (Some(row_handles), Some(column)) => {
            stats.hits += 2;
            Ok((row_handles, column))
        }
        _ => {
            stats.misses += 2;
            let (row_handles, values) =
                load_column_handles_and_values(txn, scope.relation_id, field_id, width)?;
            let column = ColumnImage {
                field_id,
                width,
                values: Rc::new(values),
            };
            txn.base_images
                .insert_row_handles(scope, Rc::clone(&row_handles));
            txn.base_images
                .insert_column(scope, field_id, column.clone());
            Ok((row_handles, column))
        }
    }
}

fn load_column_handles_and_values(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
) -> Result<(Rc<Vec<FactHandle>>, Vec<u8>)> {
    let prefix_key = column_prefix_key(relation_id, field_id as u32);
    let prefix = prefix_key.as_bytes();
    let mut handles = Vec::new();
    let mut values = Vec::new();
    for item in txn.dbs.data.prefix_iter(&txn.txn, prefix)? {
        let (key, value) = item?;
        let handle = decode_column_key_handle(key)
            .ok_or_else(|| Error::corrupt("column key handle width invalid"))?;
        if value.len() != width {
            return Err(Error::corrupt(format!(
                "column entry width mismatch for field {field_id}"
            )));
        }
        handles.push(handle);
        values.extend_from_slice(value);
    }
    Ok((Rc::new(handles), values))
}

struct CompiledFilter<'a> {
    column: &'a ColumnImage,
    op: SourceFilterOp,
    value: &'a [u8],
}

impl CompiledFilter<'_> {
    fn matches(&self, offset: usize) -> bool {
        self.column
            .value_at(offset)
            .is_some_and(|candidate| compare_encoded(candidate, self.op, self.value))
    }
}

fn compile_filters<'a>(
    columns: &'a BTreeMap<usize, ColumnImage>,
    filters: &'a [SourceFilter],
) -> Result<Vec<CompiledFilter<'a>>> {
    let mut compiled = Vec::with_capacity(filters.len());
    for filter in filters {
        match filter {
            SourceFilter::Compare {
                field_id,
                op,
                value,
            } => {
                let column = columns
                    .get(field_id)
                    .ok_or_else(|| Error::corrupt("filter column missing"))?;
                compiled.push(CompiledFilter {
                    column,
                    op: *op,
                    value: value.bytes(),
                });
            }
            SourceFilter::False => return Ok(Vec::new()),
        }
    }
    Ok(compiled)
}

fn compare_encoded(candidate: &[u8], op: SourceFilterOp, value: &[u8]) -> bool {
    match op {
        SourceFilterOp::Eq => candidate == value,
        SourceFilterOp::NotEq => candidate != value,
        SourceFilterOp::Lt => candidate < value,
        SourceFilterOp::Lte => candidate <= value,
        SourceFilterOp::Gt => candidate > value,
        SourceFilterOp::Gte => candidate >= value,
    }
}

fn selected_column_values(column: &ColumnImage, offsets: &[usize]) -> Result<Vec<u8>> {
    let mut values = Vec::with_capacity(offsets.len() * column.width);
    for offset in offsets {
        let value = column
            .value_at(*offset)
            .ok_or_else(|| Error::corrupt("filtered column offset missing"))?;
        values.extend_from_slice(value);
    }
    Ok(values)
}

fn load_column_values(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
    row_handles: &[FactHandle],
) -> Result<Vec<u8>> {
    load_column_values_for_selection(txn, relation_id, field_id, width, row_handles, row_handles)
}

fn load_column_values_for_selection(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
    row_handles: &[FactHandle],
    selected_handles: &[FactHandle],
) -> Result<Vec<u8>> {
    if !selected_handles.is_empty() && selected_handles.len() * 32 < row_handles.len() {
        return load_selected_column_values_by_key(
            txn,
            relation_id,
            field_id,
            width,
            selected_handles,
        );
    }
    let prefix_key = column_prefix_key(relation_id, field_id as u32);
    let prefix = prefix_key.as_bytes();
    let mut values = Vec::with_capacity(selected_handles.len() * width);
    let mut live_index = 0usize;
    let mut selected_index = 0usize;

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
        if selected_index < selected_handles.len() && selected_handles[selected_index] == handle {
            values.extend_from_slice(value);
            selected_index += 1;
        }
        live_index += 1;
    }

    if live_index != row_handles.len() {
        return Err(Error::corrupt(format!(
            "column entry missing for live row field {field_id}"
        )));
    }
    if selected_index != selected_handles.len() {
        return Err(Error::corrupt(format!(
            "selected column entry missing for field {field_id}"
        )));
    }
    Ok(values)
}

fn load_selected_column_values_by_key(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
    selected_handles: &[FactHandle],
) -> Result<Vec<u8>> {
    let mut values = Vec::with_capacity(selected_handles.len() * width);
    for handle in selected_handles {
        let key = column_key(relation_id, field_id as u32, *handle);
        let value = txn
            .dbs
            .data
            .get(&txn.txn, &key)?
            .ok_or_else(|| Error::corrupt("selected column entry missing"))?;
        if value.len() != width {
            return Err(Error::corrupt(format!(
                "column entry width mismatch for field {field_id}"
            )));
        }
        values.extend_from_slice(value);
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
