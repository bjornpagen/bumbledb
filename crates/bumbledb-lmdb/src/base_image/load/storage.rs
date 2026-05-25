use std::rc::Rc;

use super::super::RelationRows;
use crate::storage_format::{
    FactHandle, RowId, column_key, column_prefix_key, decode_column_key_row_id,
    decode_live_row_key_row_id, live_row_key,
};
use crate::{Error, ReadTxn, Result};

pub(super) fn load_column_values(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
    row_ids: &[RowId],
) -> Result<Vec<u8>> {
    let prefix_key = column_prefix_key(relation_id, field_id as u32);
    let prefix = prefix_key.as_bytes();
    let mut values = Vec::with_capacity(row_ids.len() * width);
    let mut row_index = 0usize;

    for item in txn.dbs.data.prefix_iter(&txn.txn, prefix)? {
        let (key, value) = item?;
        let row_id = decode_column_key_row_id(key)
            .ok_or_else(|| Error::corrupt("column key row id width invalid"))?;
        if value.len() != width {
            return Err(Error::corrupt(format!(
                "column entry width mismatch for field {field_id}"
            )));
        }
        if row_index < row_ids.len() && row_ids[row_index] < row_id {
            return Err(Error::corrupt(format!(
                "column entry missing for live row field {field_id}"
            )));
        }
        if row_index == row_ids.len() || row_ids[row_index] > row_id {
            return Err(Error::corrupt(format!(
                "column entry without live row for field {field_id}"
            )));
        }
        values.extend_from_slice(value);
        row_index += 1;
    }

    if row_index != row_ids.len() {
        return Err(Error::corrupt(format!(
            "column entry missing for live row field {field_id}"
        )));
    }
    Ok(values)
}

pub(super) fn load_column_values_by_row_id(
    txn: &ReadTxn<'_>,
    relation_id: u32,
    field_id: usize,
    width: usize,
    row_ids: &[RowId],
) -> Result<Vec<u8>> {
    let mut values = Vec::with_capacity(row_ids.len() * width);
    for row_id in row_ids {
        let key = column_key(relation_id, field_id as u32, *row_id);
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

pub(super) fn live_rows(txn: &ReadTxn<'_>, relation_id: u32) -> Result<RelationRows> {
    let prefix_key = live_row_key(relation_id, RowId(0));
    let prefix = &prefix_key[..5];
    let mut row_ids = Vec::new();
    let mut handles = Vec::new();
    for item in txn.dbs.data.prefix_iter(&txn.txn, prefix)? {
        let (key, value) = item?;
        let row_id = decode_live_row_key_row_id(key)
            .ok_or_else(|| Error::corrupt("live row key row id width invalid"))?;
        let handle_bytes: [u8; 16] = value
            .try_into()
            .map_err(|_| Error::corrupt("live row value handle width invalid"))?;
        row_ids.push(row_id);
        handles.push(FactHandle(handle_bytes));
    }
    Ok(RelationRows {
        row_ids: Rc::new(row_ids),
        row_handles: Rc::new(handles),
    })
}
