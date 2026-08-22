//! The build path: one sequential scan decodes every column of a relation
//! into structure-of-arrays slabs —
//! and the synthesis path, which fills the same slabs from a closed
//! relation's sealed extension with no catalog anywhere.

use std::ops::Bound;
use std::sync::Arc;

use crate::error::{CorruptionError, Error, Exceeded, Result};
use crate::schema::{Relation, Schema};
use crate::storage::catalog::{Bounds, CatalogMap, CatalogRead, FactCursor, ReadCursor};
use crate::storage::keys;
use bumbledb_theory::schema::RelationId;
use bumbledb_theory::schema::ValueType;

use super::decode::{decode_fact, decode_plan, fill_one};
use super::{
    Column, ColumnSpan, ColumnView, ColumnWidth, LINE, RelationImage, SET_STRIDE, StridePadder,
    column_spans,
};

/// The `S` value is data: overflow in any size computation is typed Corruption
/// before a single byte is allocated.
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

struct Frame {
    spans: Box<[ColumnSpan]>,
    columns: Vec<Column>,
    words: Vec<u64>,
    bytes: Vec<u8>,
}

fn allocate(field_types: &[ValueType], row_count: usize) -> Result<Frame> {
    allocate_with(field_types, row_count, StridePadder::new())
}

fn allocate_with(
    field_types: &[ValueType],
    row_count: usize,
    mut padder: StridePadder,
) -> Result<Frame> {

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

fn seal(
    row_count: usize,
    frame: Frame,
    distincts: Box<[super::distinct::DistinctState]>,
) -> Arc<RelationImage> {
    Arc::new(RelationImage {
        row_count,
        distincts,
        spans: frame.spans,
        columns: frame.columns.into_boxed_slice(),
        words: frame.words,
        bytes: frame.bytes,
    })
}

fn count_frame(row_count: usize, frame: &Frame) -> Box<[super::distinct::DistinctState]> {
    let span = crate::obs::span_args(
        crate::obs::names::IMAGE_DISTINCTS,
        crate::obs::TraceArgs::Pair(frame.columns.len() as u64, row_count as u64),
    );
    let states =
        super::distinct::count_columns(&frame.columns, row_count, &frame.words, &frame.bytes);
    span.end();
    states
}

#[cfg(test)]
pub(super) fn image_with_tolerance(
    field_types: &[ValueType],
    row_count: usize,
    tolerance: usize,
) -> Arc<RelationImage> {
    let frame = allocate_with(
        field_types,
        row_count,
        StridePadder::with_tolerance(tolerance),
    )
    .expect("falsifier row counts sit far below the checked slab ceiling");
    let distincts = count_frame(row_count, &frame);
    seal(row_count, frame, distincts)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
fn fill_scan<C: CatalogRead>(
    catalog: &C,
    rel: RelationId,
    plan: &[super::decode::Decode],
    fact_width: usize,
    from: usize,
    row_count: usize,
    words: &mut [u64],
    bytes: &mut [u8],
) -> Result<usize> {
    let mut facts = catalog.scan_facts(rel)?;
    let mut position = from;
    while let Some(entry) = FactCursor::next(&mut facts)? {
        position = fill_one(
            rel,
            plan,
            fact_width,
            entry.bytes,
            position,
            row_count,
            words,
            bytes,
        )?;
    }
    Ok(position)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
fn fill_from<C: CatalogRead>(
    catalog: &C,
    rel: RelationId,
    from_row_id: u64,
    plan: &[super::decode::Decode],
    fact_width: usize,
    from: usize,
    row_count: usize,
    words: &mut [u64],
    bytes: &mut [u8],
) -> Result<usize> {
    let lo = keys::fact_key(rel, from_row_id);
    let hi = keys::fact_key(rel, u64::MAX);
    let mut range = catalog.range(
        CatalogMap::Data,
        Bounds {
            start: Bound::Included(lo.as_slice()),
            end: Bound::Included(hi.as_slice()),
        },
    )?;
    let mut position = from;
    while let Some(entry) = ReadCursor::next(&mut range)? {
        keys::parse_fact_key(entry.key).ok_or(Error::Corruption(
            CorruptionError::MalformedValue("F key length"),
        ))?;
        position = fill_one(
            rel,
            plan,
            fact_width,
            entry.value,
            position,
            row_count,
            words,
            bytes,
        )?;
    }
    Ok(position)
}

/// # Errors
/// Any scan corruption (wrong fact width) aborts the build; a scan yielding a
/// different number of rows than the stored `S` count is corruption too, and a
/// stored count exceeding the `_data` entry-count witness is
/// [`CorruptionError::CounterDesync`] before any size-derived allocation.
/// # Panics
/// Only on programmer-invariant violations (backing-store capacity computed
/// from the same counters the fill loop trusts; `rel` names a closed relation —
/// closed images synthesize from the theory, and the cache branches before this
/// path).
pub(crate) fn build<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    rel: RelationId,
) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    debug_assert!(
        relation.body().closed_rows().is_none(),
        "closed relations synthesize from the theory, never from a scan"
    );
    let layout = relation.layout();
    let claimed = catalog.row_count(rel)?;

    // The reopen-trust ceiling: the stored `S` count is data, and a

    let witness = catalog.len(CatalogMap::Data)?;
    if claimed > witness {
        return Err(Error::Corruption(CorruptionError::CounterDesync {
            relation: rel,
            exceeded: Exceeded {
                observed: claimed,
                ceiling: witness,
            },
        }));
    }
    let row_count = usize::try_from(claimed).expect("64-bit usize");

    let field_types: Vec<ValueType> = relation.fields().iter().map(|f| f.value_type).collect();
    let mut frame = allocate(&field_types, row_count)?;

    let plan = decode_plan(&field_types, &frame.spans, &frame.columns, layout);
    let decode_span = crate::obs::span_args(
        crate::obs::names::DECODE_BATCH,
        crate::obs::TraceArgs::Pair(row_count as u64, layout.fact_width() as u64),
    );
    let position = fill_scan(
        catalog,
        rel,
        &plan,
        layout.fact_width(),
        0,
        row_count,
        &mut frame.words,
        &mut frame.bytes,
    )?;
    decode_span.end();
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: row_count as u64,
        }));
    }

    let distincts = count_frame(row_count, &frame);
    Ok(seal(row_count, frame, distincts))
}

/// Sound because a delete-free, tail-only lineage makes the base a **logical
/// prefix** of the new image — every row committed after the base has id at or
/// above the base's boundary (the one id allocator, R16: `ImageCache::advance`
/// evicts a base whose relation took a below-boundary insert, so tail-only is
/// ENFORCED, never assumed from counter shape), same ordinals, same column
/// words (fact bytes are immutable). The caller (the cache's append arm) owns
/// the lineage claim; this function still trusts nothing it can check: the
/// stored row count is ceiling-bounded by the `_data` entry witness before any
/// allocation (as [`build`]), a count below the base's rows is typed corruption
/// (only corruption shrinks a delete-free relation), and the tail scan is
/// cross-checked against the claimed count — hard error, never a skip.
/// # Errors
/// # Panics
/// Only on programmer-invariant violations: `rel` names a closed relation, or
/// `base` was built for a different relation shape (the column layouts
/// disagree).
pub(crate) fn append<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    rel: RelationId,
    base: &RelationImage,
    from_row_id: u64,
) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    debug_assert!(
        relation.body().closed_rows().is_none(),
        "closed relations synthesize from the theory, never from a scan"
    );
    let layout = relation.layout();
    let claimed = catalog.row_count(rel)?;

    // The same reopen-trust ceiling as `build`: the stored count is data
    // and must not size an allocation unchecked.
    let witness = catalog.len(CatalogMap::Data)?;
    if claimed > witness {
        return Err(Error::Corruption(CorruptionError::CounterDesync {
            relation: rel,
            exceeded: Exceeded {
                observed: claimed,
                ceiling: witness,
            },
        }));
    }
    let row_count = usize::try_from(claimed).expect("64-bit usize");
    let base_rows = base.row_count();

    if row_count < base_rows {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: claimed,
        }));
    }

    let field_types: Vec<ValueType> = relation.fields().iter().map(|f| f.value_type).collect();
    let mut frame = allocate(&field_types, row_count)?;
    assert_eq!(
        frame.columns.len(),
        base.columns.len(),
        "the base image was built from this relation's field→column map"
    );

    for (index, column) in frame.columns.iter().enumerate() {
        match (*column, base.column(index)) {
            (Column::Words { start }, ColumnView::Words(prefix)) => {
                frame.words[start..start + base_rows].copy_from_slice(prefix);
            }
            (Column::Bytes { start }, ColumnView::Bytes(prefix)) => {
                frame.bytes[start..start + base_rows].copy_from_slice(prefix);
            }
            _ => unreachable!("one field→column map drives both layouts"),
        }
    }

    let plan = decode_plan(&field_types, &frame.spans, &frame.columns, layout);
    let decode_span = crate::obs::span_args(
        crate::obs::names::DECODE_BATCH,
        crate::obs::TraceArgs::Pair((row_count - base_rows) as u64, layout.fact_width() as u64),
    );
    let position = fill_from(
        catalog,
        rel,
        from_row_id,
        &plan,
        layout.fact_width(),
        base_rows,
        row_count,
        &mut frame.words,
        &mut frame.bytes,
    )?;
    decode_span.end();
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: claimed,
        }));
    }

    let span = crate::obs::span_args(
        crate::obs::names::IMAGE_DISTINCTS,
        crate::obs::TraceArgs::Pair(frame.columns.len() as u64, (row_count - base_rows) as u64),
    );
    let mut distincts = base.distincts.clone();
    super::distinct::extend_columns(
        &mut distincts,
        &frame.columns,
        base_rows,
        row_count,
        &frame.words,
        &frame.bytes,
    );
    span.end();
    Ok(seal(row_count, frame, distincts))
}

/// driver's per-round delta and accumulated images, built on the
/// the rows are already encoded column words (a seen-set's dense
/// suffix), so the build is a columnar transpose with no fact-bytes
/// decode at all. **Never cached, never memoized, never pinned**: a
/// outside `image/cache.rs` (whose diff for the recursion campaign is
/// zero lines) and the view memo; the closed carve-out's `OnceLock`
/// slots already proved images can live outside the map.
/// The slot is a retained-capacity pool on the prepared query (the
/// source-agnostic after decode, and here the source is cheaper still:
#[derive(Debug)]
pub enum TransientImage {

    Empty { capacity: usize },
    Occupied {
        image: Arc<RelationImage>,

        capacity: usize,
    },
}

impl Default for TransientImage {
    fn default() -> Self {
        Self::Empty { capacity: 0 }
    }
}

impl TransientImage {

    /// # Panics

    /// Only on programmer-invariant violations: a row narrower than the

    pub fn refill<'r>(
        &mut self,
        field_types: &[ValueType],
        row_count: usize,
        rows: impl Iterator<Item = &'r [u64]>,
    ) -> Arc<RelationImage> {

        self.fill(field_types, 0, row_count, |_| rows, CapacityPolicy::Exact)
    }

    /// # Panics

    /// As [`Self::refill`]: programmer-invariant violations only.
    pub fn append<'r, I>(
        &mut self,
        field_types: &[ValueType],
        filled: usize,
        row_count: usize,
        rows_since: impl FnOnce(usize) -> I,
    ) -> Arc<RelationImage>
    where
        I: Iterator<Item = &'r [u64]>,
    {
        self.fill(
            field_types,
            filled,
            row_count,
            rows_since,
            CapacityPolicy::Doubling,
        )
    }

    fn fill<'r, I>(
        &mut self,
        field_types: &[ValueType],
        filled: usize,
        row_count: usize,
        rows_since: impl FnOnce(usize) -> I,
        policy: CapacityPolicy,
    ) -> Arc<RelationImage>
    where
        I: Iterator<Item = &'r [u64]>,
    {
        debug_assert!(filled <= row_count, "seen-sets never shrink");
        let framed = match self {
            Self::Empty { capacity } | Self::Occupied { capacity, .. } => *capacity,
        };
        let reusable = match self {
            Self::Occupied { image, capacity } if row_count <= *capacity => {
                Arc::get_mut(image).is_some()
            }
            Self::Empty { .. } | Self::Occupied { .. } => false,
        };
        let base = if reusable { filled } else { 0 };
        if !reusable {
            let capacity = match policy {
                CapacityPolicy::Exact => row_count,
                CapacityPolicy::Doubling => framed.max(row_count.saturating_mul(2)),
            };
            let frame = allocate(field_types, capacity)
                .expect("seen-set row counts sit far below the checked slab ceiling");

            let distincts = super::distinct::uncounted_columns(&frame.columns);
            *self = Self::Occupied {
                image: seal(row_count, frame, distincts),
                capacity,
            };
        }
        let Self::Occupied { image, .. } = self else {
            unreachable!("fill just occupied the slot");
        };
        let image_mut =
            Arc::get_mut(image).expect("a non-reusable slot was just replaced by a unique Arc");
        image_mut.row_count = row_count;
        let filled_to = fill_encoded_rows(image_mut, base, rows_since(base));
        debug_assert_eq!(filled_to, row_count, "the caller counted its rows");
        Arc::clone(image)
    }
}

#[derive(Clone, Copy)]
enum CapacityPolicy {
    Exact,
    Doubling,
}

fn fill_encoded_rows<'r>(
    image: &mut RelationImage,
    base: usize,
    rows: impl Iterator<Item = &'r [u64]>,
) -> usize {
    let RelationImage {
        columns,
        words,
        bytes,
        ..
    } = image;
    let mut filled = base;
    for (offset, row) in rows.enumerate() {
        let position = base + offset;
        debug_assert_eq!(
            row.len(),
            columns.len(),
            "seen-set rows carry one word per image column"
        );
        for (column, &word) in columns.iter().zip(row) {
            match *column {
                Column::Words { start } => words[start + position] = word,
                Column::Bytes { start } => bytes[start + position] = u8::from(word != 0),
            }
        }
        filled = position + 1;
    }
    filled
}

/// Synthesizes a closed relation's image from its sealed extension — the
/// fingerprint's preimage IS the storage
/// . No LMDB
/// transaction parameter exists because synthesis is pure: the sealed
/// rows' canonical fact bytes (encoded ONCE, at validate) decode through
/// exactly the plan a stored fact would, so the column layout, the
/// implicit `id` column (`0..rows`, first — the synthetic field opens the
/// sealed field list), stride padding, and the build-time distinct
/// # Panics
/// validated schema.
/// Only on programmer-invariant violations: `relation` is ordinary, or a
#[must_use]
pub fn synthesize_closed(rel: RelationId, relation: &Relation) -> Arc<RelationImage> {
    let extension = relation
        .body()
        .closed_rows()
        .expect("synthesize_closed takes a closed relation");
    let layout = relation.layout();
    let row_count = extension.len();
    let field_types: Vec<ValueType> = relation.fields().iter().map(|f| f.value_type).collect();
    let mut frame = allocate(&field_types, row_count)
        .expect("the extension-row cap keeps every slab size computation in range");
    let plan = decode_plan(&field_types, &frame.spans, &frame.columns, layout);
    let decode_span = crate::obs::span_args(
        crate::obs::names::DECODE_BATCH,
        crate::obs::TraceArgs::Pair(row_count as u64, layout.fact_width() as u64),
    );
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
    decode_span.end();
    let distincts = count_frame(row_count, &frame);
    seal(row_count, frame, distincts)
}
