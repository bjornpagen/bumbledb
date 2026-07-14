//! The build path: one sequential scan decodes every column of a relation
//! into structure-of-arrays slabs (docs/architecture/40-execution.md D1,
//! `40-storage.md`; the per-fact decode kernel lives in `super::decode`) —
//! and the synthesis path, which fills the same slabs from a closed
//! relation's sealed extension with no LMDB transaction anywhere.

use std::sync::Arc;

use crate::encoding::TypeDesc;
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{Relation, RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::read;

use super::decode::{decode_fact, decode_plan, fill_columns};
use super::{
    Column, ColumnSpan, ColumnWidth, LINE, RelationImage, SET_STRIDE, StridePadder, column_spans,
};

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

/// An image's allocated-but-unfilled frame: the field→column map, the
/// placed columns, and the two backing slabs, sized for `row_count` rows
/// of the given field shape. Shared by the two fill paths — the LMDB
/// scan ([`build`]) and closed-relation synthesis ([`synthesize_closed`]).
struct Frame {
    spans: Box<[ColumnSpan]>,
    columns: Vec<Column>,
    words: Vec<u64>,
    bytes: Vec<u8>,
}

/// Allocates the frame: one up-front allocation per backing store, sized
/// from the row count plus per-column alignment/stride slack, column
/// bases 128-byte aligned with strides padded off 16 KiB multiples (the
/// tracker-aliasing rule, measured). Every slab-size computation is
/// checked; overflow is typed Corruption *before* any allocation is
/// attempted.
fn allocate(field_types: &[TypeDesc], row_count: usize) -> Result<Frame> {
    // The field→column map drives the layout: an interval field spans two
    // consecutive 8-byte columns (start, end), a bytes<N> field its
    // ⌈N/8⌉ word columns, everything else one column of its width — the
    // image layer has no wide column (`docs/architecture/50-storage.md`).
    let spans = column_spans(field_types);
    let byte_cols = spans
        .iter()
        .filter(|s| s.width == ColumnWidth::Byte)
        .count();
    let column_count = spans
        .last()
        .map_or(0, |s| usize::from(s.first_column + s.width.column_count()));
    let word_cols = column_count - byte_cols;
    let (word_len, byte_len) = slab_lengths(row_count, word_cols, byte_cols)?;
    let words = vec![0u64; word_len];
    let bytes = vec![0u8; byte_len];

    let words_addr = words.as_ptr().addr();
    let bytes_addr = bytes.as_ptr().addr();
    let mut padder = StridePadder::new();
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
                let start = padder.place(bytes_addr, 1, byte_cursor);
                byte_cursor = start + row_count;
                columns.push(Column::Bytes { start });
                continue;
            }
            ColumnWidth::Word => 1,
            ColumnWidth::WordPair => 2,
            ColumnWidth::Words { count } => usize::from(count),
        };
        for _ in 0..word_columns {
            let start = padder.place(words_addr, 8, word_cursor);
            word_cursor = start + row_count;
            columns.push(Column::Words { start });
        }
    }

    Ok(Frame {
        spans,
        columns,
        words,
        bytes,
    })
}

/// Seals a filled frame into the shared image. Distinct counts are NOT
/// computed here: the eager pass was the cold path's fixed cost. Each
/// column's count materializes on first planner demand
/// ([`RelationImage::distinct`]).
fn seal(row_count: usize, frame: Frame) -> Arc<RelationImage> {
    let distincts: Vec<std::sync::OnceLock<u64>> = frame
        .columns
        .iter()
        .map(|_| std::sync::OnceLock::new())
        .collect();
    Arc::new(RelationImage {
        row_count,
        distincts: distincts.into_boxed_slice(),
        spans: frame.spans,
        columns: frame.columns.into_boxed_slice(),
        words: frame.words,
        bytes: frame.bytes,
    })
}

/// Builds the full-width image of `rel` from one sequential scan.
///
/// # Errors
///
/// Any scan corruption (wrong fact width) aborts the build; a scan yielding
/// a different number of rows than the stored `S` count is corruption too,
/// and a stored count exceeding the `_data` entry-count witness is
/// [`CorruptionError::CounterDesync`] before any size-derived allocation.
/// Dangling intern ids are *not* checked here — ids are opaque words at
/// this layer.
///
/// # Panics
///
/// Only on programmer-invariant violations (backing-store capacity computed
/// from the same counters the fill loop trusts; `rel` names a closed
/// relation — closed images synthesize from the theory, and the cache
/// branches before this path).
pub fn build(txn: &ReadTxn<'_>, schema: &Schema, rel: RelationId) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    debug_assert!(
        !relation.is_closed(),
        "closed relations synthesize from the theory, never from a scan"
    );
    let layout = relation.layout();
    let claimed = read::row_count(txn, rel)?;

    // The reopen-trust ceiling: the stored `S` count is data, and a
    // corrupt-but-plausible value (2^40 passes every checked size
    // computation) would drive the slab `vec!`s below into a
    // multi-terabyte allocation. Bound it by the `_data` DBI entry count
    // — an over-approximation (the DBI spans F/M/U/R/Q/S, so it counts
    // far more than this relation's F rows), which a ceiling is allowed
    // to be: no real row count can exceed it, and the scan cross-check
    // below stays the exactness guarantee.
    let witness = read::data_entries(txn)?;
    if claimed > witness {
        return Err(Error::Corruption(CorruptionError::CounterDesync {
            relation: rel,
            claimed,
            witness,
        }));
    }
    let row_count = usize::try_from(claimed).expect("64-bit usize");

    let field_types: Vec<TypeDesc> = relation
        .fields()
        .iter()
        .map(|f| f.value_type.type_desc())
        .collect();
    let mut frame = allocate(&field_types, row_count)?;

    // One sequential scan fills every column (positions = scan ordinals),
    // through the hoisted decode plan.
    let plan = decode_plan(&field_types, &frame.spans, &frame.columns, layout);
    let position = fill_columns(
        txn,
        schema,
        rel,
        &plan,
        layout.fact_width(),
        row_count,
        &mut frame.words,
        &mut frame.bytes,
    )?;
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: row_count as u64,
        }));
    }

    Ok(seal(row_count, frame))
}

/// Synthesizes a closed relation's image from its sealed extension — the
/// fingerprint's preimage IS the storage
/// (`docs/architecture/50-storage.md` § virtual relations). No LMDB
/// transaction parameter exists because synthesis is pure: the sealed
/// rows' canonical fact bytes (encoded ONCE, at validate) decode through
/// exactly the plan a stored fact would, so the column layout, the
/// implicit `id` column (`0..rows`, first — the synthetic field opens the
/// sealed field list), stride padding, and the lazy distinct counters are
/// all the ordinary image machinery, untouched.
///
/// # Panics
///
/// Only on programmer-invariant violations: `relation` is ordinary, or a
/// sealed row fails the canonical decode — both impossible for a
/// validated schema.
#[must_use]
pub fn synthesize_closed(rel: RelationId, relation: &Relation) -> Arc<RelationImage> {
    let extension = relation
        .extension()
        .expect("synthesize_closed takes a closed relation");
    let layout = relation.layout();
    let row_count = extension.len();
    let field_types: Vec<TypeDesc> = relation
        .fields()
        .iter()
        .map(|f| f.value_type.type_desc())
        .collect();
    let mut frame = allocate(&field_types, row_count)
        .expect("the extension-row cap keeps every slab size computation in range");
    let plan = decode_plan(&field_types, &frame.spans, &frame.columns, layout);
    for (position, row) in extension.iter().enumerate() {
        decode_fact(
            rel,
            &plan,
            layout.fact_width(),
            &row.fact,
            position,
            &mut frame.words,
            &mut frame.bytes,
        )
        .expect("sealed rows hold canonical fact bytes, encoded at validate");
    }
    seal(row_count, frame)
}
