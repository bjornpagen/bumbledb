use std::collections::BTreeMap;

use bumbledb_core::schema::RelationDescriptor;

use super::{ColumnImage, FieldScope, RelationBaseImage, RelationStats, validate_fields};
use crate::colt_filter::{SourceFilter, SourceFilterOp};
use crate::storage_format::{
    FactHandle, column_key, column_prefix_key, decode_column_key_handle, live_row_key,
};
use crate::{Error, ReadTxn, Result};

pub(super) fn load_relation_base_image(
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

pub(super) fn load_filtered_relation_base_image(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    relation: &RelationDescriptor,
    field_ids: FieldScope,
    filters: &[SourceFilter],
) -> Result<(RelationBaseImage, usize)> {
    if filters
        .iter()
        .any(|filter| matches!(filter, SourceFilter::False))
    {
        return Ok((
            RelationBaseImage {
                relation_id,
                name: relation.name.clone(),
                stats: RelationStats { row_count: 0 },
                row_handles: Vec::new(),
                columns: BTreeMap::new(),
            },
            0,
        ));
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
    let (all_handles, primary_values) =
        load_column_handles_and_values(txn, relation_id, primary_filter_field, primary_width)?;
    filter_columns.insert(
        primary_filter_field,
        ColumnImage {
            field_id: primary_filter_field,
            width: primary_width,
            values: primary_values,
        },
    );
    for field_id in filter_scope.iter() {
        if field_id == primary_filter_field {
            continue;
        }
        let width = relation.fields[field_id].value_type.encoded_width();
        let values = load_column_values_for_selection(
            txn,
            relation_id,
            field_id,
            width,
            &all_handles,
            &all_handles,
        )?;
        filter_columns.insert(
            field_id,
            ColumnImage {
                field_id,
                width,
                values,
            },
        );
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

    let mut columns = BTreeMap::new();
    for field_id in field_ids.iter() {
        let width = relation.fields[field_id].value_type.encoded_width();
        let values = if let Some(filter_column) = filter_columns.get(&field_id) {
            selected_column_values(filter_column, &survivor_offsets)?
        } else {
            load_column_values_for_selection(
                txn,
                relation_id,
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
                values,
            },
        );
    }

    Ok((
        RelationBaseImage {
            relation_id,
            name: relation.name.clone(),
            stats: RelationStats {
                row_count: survivor_handles.len(),
            },
            row_handles: survivor_handles,
            columns,
        },
        all_handles.len(),
    ))
}

fn load_column_handles_and_values(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
) -> Result<(Vec<FactHandle>, Vec<u8>)> {
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
    Ok((handles, values))
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
