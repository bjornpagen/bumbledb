//! The build path: one sequential scan decodes every column of a relation
//! into structure-of-arrays slabs (docs/architecture/30-execution.md D1,
//! `40-storage.md`; the per-fact decode kernel lives in `super::decode`).

use std::sync::Arc;

use crate::encoding::TypeDesc;
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::read;

use super::decode::{decode_plan, fill_columns};
use super::{column_spans, Column, ColumnWidth, PitchPadder, RelationImage, LINE, SET_STRIDE};

/// Checked slab lengths (in words and bytes) for the stored row count.
/// The `S` value is data: overflow in any size computation is typed
/// Corruption before a single byte is allocated.
fn slab_lengths(row_count: usize, word_cols: usize, byte_cols: usize) -> Result<(usize, usize)> {
    let corrupt = || Error::Corruption(CorruptionError::MalformedValue("S row count"));
    let word_len = row_count
        .checked_add(SET_STRIDE / 8 + LINE / 8)
        .and_then(|per_col| per_col.checked_mul(word_cols))
        .and_then(|words| words.checked_mul(8))
        .ok_or_else(corrupt)?
        / 8;
    let byte_len = row_count
        .checked_add(SET_STRIDE + LINE)
        .and_then(|per_col| per_col.checked_mul(byte_cols))
        .ok_or_else(corrupt)?;
    Ok((word_len, byte_len))
}

/// Builds the full-width image of `rel` from one sequential scan.
///
/// # Errors
///
/// Any scan corruption (wrong fact width) aborts the build; a scan yielding
/// a different number of rows than the stored `S` count is corruption too.
/// Dangling intern ids are *not* checked here — ids are opaque words at
/// this layer.
///
/// # Panics
///
/// Only on programmer-invariant violations (backing-store capacity computed
/// from the same counters the fill loop trusts).
pub fn build(txn: &ReadTxn<'_>, schema: &Schema, rel: RelationId) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    let layout = relation.layout();
    let row_count = usize::try_from(read::row_count(txn, rel)?).expect("64-bit usize");

    // One up-front allocation per backing store, sized from the row count
    // plus per-column alignment/stagger slack. The stored `S` count is
    // data: every slab-size computation is checked, and overflow is
    // typed Corruption *before* any allocation is attempted (the
    // both-direction scan cross-check below stays the exactness
    // guarantee).
    let field_types: Vec<TypeDesc> = relation
        .fields()
        .iter()
        .map(|f| f.value_type.type_desc())
        .collect();
    // The field→column map drives the layout: an interval field spans two
    // consecutive 8-byte columns (start, end), everything else one column
    // of its width — the image layer has no 16-byte column
    // (`docs/architecture/50-storage.md`).
    let spans = column_spans(&field_types);
    let byte_cols = spans
        .iter()
        .filter(|s| s.width == ColumnWidth::Byte)
        .count();
    let column_count = spans
        .last()
        .map_or(0, |s| usize::from(s.first_column + s.width.column_count()));
    let word_cols = column_count - byte_cols;
    let (word_len, byte_len) = slab_lengths(row_count, word_cols, byte_cols)?;
    let mut words = vec![0u64; word_len];
    let mut bytes = vec![0u8; byte_len];

    // Lay out column bases: 128-byte aligned, pitches padded off 16 KiB
    // multiples (docs/silicon/11 — the tracker-aliasing rule).
    let words_addr = words.as_ptr().addr();
    let bytes_addr = bytes.as_ptr().addr();
    let mut stagger = PitchPadder::new();
    let mut word_cursor = 0usize;
    let mut byte_cursor = 0usize;
    let mut columns: Vec<Column> = Vec::with_capacity(column_count);
    for span in &spans {
        assert_eq!(
            usize::from(span.first_column),
            columns.len(),
            "the field→column map drives the layout"
        );
        let word_columns = match span.width {
            ColumnWidth::Byte => {
                let start = stagger.place(bytes_addr, 1, byte_cursor);
                byte_cursor = start + row_count;
                columns.push(Column::Bytes { start });
                continue;
            }
            ColumnWidth::Word => 1,
            ColumnWidth::WordPair => 2,
        };
        for _ in 0..word_columns {
            let start = stagger.place(words_addr, 8, word_cursor);
            word_cursor = start + row_count;
            columns.push(Column::Words { start });
        }
    }

    // One sequential scan fills every column (positions = scan ordinals),
    // through the hoisted decode plan (docs/perf/ PRD 12).
    let plan = decode_plan(&field_types, &spans, &columns, layout);
    let position = fill_columns(
        txn,
        schema,
        rel,
        &plan,
        layout.fact_width(),
        row_count,
        &mut words,
        &mut bytes,
    )?;
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: row_count as u64,
        }));
    }

    // Distinct counts are NOT computed here (docs/silicon/13): the
    // eager pass was the cold path's fixed cost. Each column's count
    // materializes on first planner demand ([`RelationImage::distinct`]).
    let distincts: Vec<std::sync::OnceLock<u64>> =
        columns.iter().map(|_| std::sync::OnceLock::new()).collect();

    Ok(Arc::new(RelationImage {
        row_count,
        distincts: distincts.into_boxed_slice(),
        spans,
        columns: columns.into_boxed_slice(),
        words,
        bytes,
    }))
}
