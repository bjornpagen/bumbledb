use std::collections::BTreeMap;
use std::rc::Rc;

use bumbledb_core::schema::RelationDescriptor;

use super::{
    CacheScope, ColumnImage, FieldScope, PhysicalCacheStats, RelationBaseImage, RelationRows,
    RelationStats, validate_fields,
};
use crate::colt_filter::{SourceFilter, SourceFilterOp};
use crate::storage_format::{
    FactHandle, RowId, accelerator_prefix_key, column_prefix_key, decode_accelerator_key_row_id,
    decode_column_key_row_id, live_row_key,
};
use crate::{Error, ReadTxn, Result};

#[path = "load/storage.rs"]
mod storage;

use storage::{live_rows, load_column_values, load_column_values_by_row_id};

pub(super) fn load_relation_base_image(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    relation: &RelationDescriptor,
    field_ids: FieldScope,
) -> Result<LoadResult> {
    let mut cache_stats = PhysicalCacheStats::default();
    let rows = cached_rows(txn, scope, &mut cache_stats)?;
    let mut columns = BTreeMap::new();
    for field_id in field_ids.iter() {
        let field = &relation.fields[field_id];
        let width = field.value_type.encoded_width();
        let column =
            cached_full_column(txn, scope, field_id, width, &rows.row_ids, &mut cache_stats)?;
        columns.insert(field_id, column);
    }

    Ok(LoadResult {
        image: RelationBaseImage {
            relation_id: scope.relation_id,
            name: relation.name.clone(),
            stats: RelationStats {
                row_count: rows.row_handles.len(),
            },
            row_handles: rows.row_handles,
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
    let accelerated = accelerator_seed(filters);
    let (all_row_ids, primary_column, seeded_by_accelerator) =
        if let Some((field_id, value)) = accelerated {
            let candidate_row_ids = accelerator_row_ids(txn, scope.relation_id, field_id, value)?;
            if candidate_row_ids.is_empty() || candidate_row_ids.len() <= 4096 {
                let values = repeated_value_bytes(value, candidate_row_ids.len());
                (
                    Rc::new(candidate_row_ids),
                    ColumnImage {
                        field_id,
                        width: primary_width,
                        values: Rc::new(values),
                        row_offsets: None,
                    },
                    true,
                )
            } else {
                let (row_ids, column) = load_filter_primary_column(
                    txn,
                    scope,
                    primary_filter_field,
                    primary_width,
                    &mut cache_stats,
                )?;
                (row_ids, column, false)
            }
        } else {
            let (row_ids, column) = load_filter_primary_column(
                txn,
                scope,
                primary_filter_field,
                primary_width,
                &mut cache_stats,
            )?;
            (row_ids, column, false)
        };
    filter_columns.insert(primary_filter_field, primary_column);
    for field_id in filter_scope.iter() {
        if field_id == primary_filter_field {
            continue;
        }
        let width = relation.fields[field_id].value_type.encoded_width();
        let column = if seeded_by_accelerator {
            let values = load_column_values_by_row_id(
                txn,
                scope.relation_id,
                field_id,
                width,
                &all_row_ids,
            )?;
            ColumnImage {
                field_id,
                width,
                values: Rc::new(values),
                row_offsets: None,
            }
        } else {
            cached_full_column(txn, scope, field_id, width, &all_row_ids, &mut cache_stats)?
        };
        filter_columns.insert(field_id, column);
    }

    let compiled_filters = compile_filters(&filter_columns, filters)?;
    let survivor_offsets = (0..all_row_ids.len())
        .filter(|offset| {
            compiled_filters
                .iter()
                .all(|filter| filter.matches(*offset))
        })
        .collect::<Vec<_>>();
    let survivor_row_ids = survivor_offsets
        .iter()
        .map(|offset| all_row_ids[*offset])
        .collect::<Vec<_>>();
    let survivor_handles = load_live_handles_by_row_id(txn, scope.relation_id, &survivor_row_ids)?;
    let survivor_handles = Rc::new(survivor_handles);
    let dense_survivors = !seeded_by_accelerator && survivor_handles.len() * 2 >= all_row_ids.len();
    let survivor_offsets = Rc::new(
        survivor_offsets
            .into_iter()
            .map(|offset| offset as u32)
            .collect::<Vec<_>>(),
    );

    let mut columns = BTreeMap::new();
    for field_id in field_ids.iter() {
        let width = relation.fields[field_id].value_type.encoded_width();
        let column = if let Some(filter_column) = filter_columns.get(&field_id) {
            selected_column(filter_column, &survivor_offsets, dense_survivors)?
        } else {
            load_plan_column_for_selection(
                txn,
                scope,
                field_id,
                width,
                &all_row_ids,
                &survivor_row_ids,
                &survivor_offsets,
                dense_survivors,
                &mut cache_stats,
            )?
        };
        columns.insert(field_id, column);
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
        rows_tested: all_row_ids.len(),
        cache_stats,
    })
}

fn accelerator_seed(filters: &[SourceFilter]) -> Option<(usize, &[u8])> {
    filters.iter().find_map(|filter| match filter {
        SourceFilter::Compare {
            field_id,
            op: SourceFilterOp::Eq,
            value,
        } => Some((*field_id, value.bytes())),
        _ => None,
    })
}

fn accelerator_row_ids(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    encoded_value: &[u8],
) -> Result<Vec<RowId>> {
    let prefix = accelerator_prefix_key(relation_id, field_id as u32, encoded_value);
    let mut row_ids = Vec::new();
    for item in txn.dbs.data.prefix_iter(&txn.txn, &prefix)? {
        let (key, _) = item?;
        let row_id = decode_accelerator_key_row_id(key)
            .ok_or_else(|| Error::corrupt("accelerator key row id width invalid"))?;
        row_ids.push(row_id);
    }
    Ok(row_ids)
}

fn repeated_value_bytes(value: &[u8], rows: usize) -> Vec<u8> {
    let mut values = Vec::with_capacity(value.len() * rows);
    for _ in 0..rows {
        values.extend_from_slice(value);
    }
    values
}

fn load_filter_primary_column(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    field_id: usize,
    width: usize,
    stats: &mut PhysicalCacheStats,
) -> Result<(Rc<Vec<RowId>>, ColumnImage)> {
    stats.misses += 1;
    let prefix_key = column_prefix_key(scope.relation_id, field_id as u32);
    let prefix = prefix_key.as_bytes();
    let mut row_ids = Vec::new();
    let mut values = Vec::new();
    for item in txn.dbs.data.prefix_iter(&txn.txn, prefix)? {
        let (key, value) = item?;
        let row_id = decode_column_key_row_id(key)
            .ok_or_else(|| Error::corrupt("column key row id width invalid"))?;
        if value.len() != width {
            return Err(Error::corrupt(format!(
                "column entry width mismatch for field {field_id}"
            )));
        }
        row_ids.push(row_id);
        values.extend_from_slice(value);
    }
    let column = ColumnImage {
        field_id,
        width,
        values: Rc::new(values),
        row_offsets: None,
    };
    Ok((Rc::new(row_ids), column))
}

fn load_live_handles_by_row_id(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    row_ids: &[RowId],
) -> Result<Vec<FactHandle>> {
    let mut handles = Vec::with_capacity(row_ids.len());
    for row_id in row_ids {
        let bytes = txn
            .dbs
            .data
            .get(&txn.txn, &live_row_key(relation_id, *row_id))?
            .ok_or_else(|| Error::corrupt("live row missing for survivor row id"))?;
        let handle_bytes: [u8; 16] = bytes
            .try_into()
            .map_err(|_| Error::corrupt("live row value handle width invalid"))?;
        handles.push(FactHandle(handle_bytes));
    }
    Ok(handles)
}

fn cached_rows(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    stats: &mut PhysicalCacheStats,
) -> Result<RelationRows> {
    if let Some(rows) = txn.base_images.rows(scope) {
        stats.hits += 1;
        return Ok(rows);
    }
    stats.misses += 1;
    let rows = live_rows(txn, scope.relation_id)?;
    txn.base_images.insert_rows(scope, rows.clone());
    Ok(rows)
}

fn cached_full_column(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    field_id: usize,
    width: usize,
    row_ids: &[RowId],
    stats: &mut PhysicalCacheStats,
) -> Result<ColumnImage> {
    if let Some(column) = txn.base_images.column(scope, field_id) {
        stats.hits += 1;
        return Ok(column);
    }
    stats.misses += 1;
    let values = load_column_values(txn, scope.relation_id, field_id, width, row_ids)?;
    let column = ColumnImage {
        field_id,
        width,
        values: Rc::new(values),
        row_offsets: None,
    };
    txn.base_images
        .insert_column(scope, field_id, column.clone());
    Ok(column)
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

fn selected_column(
    column: &ColumnImage,
    offsets: &Rc<Vec<u32>>,
    dense_survivors: bool,
) -> Result<ColumnImage> {
    if dense_survivors {
        return Ok(ColumnImage {
            field_id: column.field_id,
            width: column.width,
            values: Rc::clone(&column.values),
            row_offsets: Some(Rc::clone(offsets)),
        });
    }
    let mut values = Vec::with_capacity(offsets.len() * column.width);
    for offset in offsets.iter().copied() {
        let value = column
            .value_at(offset as usize)
            .ok_or_else(|| Error::corrupt("filtered column offset missing"))?;
        values.extend_from_slice(value);
    }
    Ok(ColumnImage {
        field_id: column.field_id,
        width: column.width,
        values: Rc::new(values),
        row_offsets: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn load_plan_column_for_selection(
    txn: &ReadTxn<'_>,
    scope: CacheScope,
    field_id: usize,
    width: usize,
    row_ids: &[RowId],
    selected_row_ids: &[RowId],
    survivor_offsets: &Rc<Vec<u32>>,
    dense_survivors: bool,
    stats: &mut PhysicalCacheStats,
) -> Result<ColumnImage> {
    if dense_survivors {
        let column = cached_full_column(txn, scope, field_id, width, row_ids, stats)?;
        return Ok(ColumnImage {
            field_id,
            width,
            values: Rc::clone(&column.values),
            row_offsets: Some(Rc::clone(survivor_offsets)),
        });
    }
    let values = load_selected_column_values_by_key(
        txn,
        scope.relation_id,
        field_id,
        width,
        selected_row_ids,
    )?;
    Ok(ColumnImage {
        field_id,
        width,
        values: Rc::new(values),
        row_offsets: None,
    })
}

fn load_selected_column_values_by_key(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
    selected_row_ids: &[RowId],
) -> Result<Vec<u8>> {
    let mut values = Vec::with_capacity(selected_row_ids.len() * width);
    for row_id in selected_row_ids {
        values.extend_from_slice(&load_column_values_by_row_id(
            txn,
            relation_id,
            field_id,
            width,
            &[*row_id],
        )?);
    }
    Ok(values)
}
