//! The build path: one sequential scan decodes every column of a relation
//! into structure-of-arrays slabs (docs/architecture/40-execution.md D1,
//! `50-storage.md`; the per-fact decode kernel lives in `super::decode`) —
//! and the synthesis path, which fills the same slabs from a closed
//! relation's sealed extension with no LMDB transaction anywhere.

use std::sync::Arc;

use crate::error::{CorruptionError, Error, Result};
use crate::schema::{Relation, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::read;
use bumbledb_theory::TypeDesc;
use bumbledb_theory::schema::RelationId;

use super::decode::{decode_fact, decode_plan, fill_columns};
use super::{
    Column, ColumnSpan, ColumnView, ColumnWidth, LINE, RelationImage, SET_STRIDE, StridePadder,
    column_spans,
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
    allocate_with(field_types, row_count, StridePadder::new())
}

/// [`allocate`] with an explicit padder — the measured falsifier's hook:
/// the shipped stride rule and its twin lay out side by side in one
/// process. The slabs are sized identically either way ([`slab_lengths`]
/// pre-pays one `SET_STRIDE + LINE` of slack per column, the worst-case
/// alignment plus pad), so the tolerance moves column starts within the
/// slack and never the allocation.
fn allocate_with(
    field_types: &[TypeDesc],
    row_count: usize,
    mut padder: StridePadder,
) -> Result<Frame> {
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

/// An empty (all-zero) sealed image laid out under an explicit stride
/// tolerance — the measured falsifier's constructor: identical shape and
/// data either way, only the column starts move. Test-only; production
/// layouts go through [`allocate`] and the one shipped tolerance.
#[cfg(test)]
pub(super) fn image_with_tolerance(
    field_types: &[TypeDesc],
    row_count: usize,
    tolerance: usize,
) -> Arc<RelationImage> {
    let frame = allocate_with(
        field_types,
        row_count,
        StridePadder::with_tolerance(tolerance),
    )
    .expect("falsifier row counts sit far below the checked slab ceiling");
    seal(row_count, frame)
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
        rel,
        read::scan(txn, schema, rel)?,
        &plan,
        layout.fact_width(),
        0,
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

/// [`build`]'s copy-on-append sibling (`docs/architecture/50-storage.md`
/// § the image cache): extends a base image to this snapshot's row count
/// without re-decoding the base's rows. Sound because a delete-free,
/// tail-only lineage makes the base a **logical prefix** of the new
/// image — every row committed after the base has id at or above the
/// base's boundary (the one id allocator, R16: `ImageCache::advance`
/// evicts a base whose relation took a below-boundary insert, so
/// tail-only is ENFORCED, never assumed from counter shape), same
/// ordinals, same column words (fact bytes are immutable). The layout is
/// NOT a physical prefix (column starts and strides are address-dependent,
/// [`StridePadder`]), so the copy unit is the **column**: a fresh frame at
/// the new row count, one `copy_from_slice` per column — the image layer
/// has exactly two column kinds, so the copy is total and safe — then a
/// tail decode of only the new rows through the identical per-fact kernel,
/// scanning from `from_row_id` (the base's build-time boundary — the
/// `Q` next value on a fresh-keyed relation, the `S` high-water
/// otherwise — read in the base's own transaction). The
/// sealed image mints fresh lazy distinct locks — tail rows change exact
/// counts, so distincts re-force on demand (the `TransientImage::refill`
/// precedent), never copy.
///
/// The caller (the cache's append arm) owns the lineage claim; this
/// function still trusts nothing it can check: the stored row count is
/// ceiling-bounded by the `_data` entry witness before any allocation
/// (as [`build`]), a count below the base's rows is typed corruption
/// (only corruption shrinks a delete-free relation), and the tail scan
/// is cross-checked against the claimed count — hard error, never a
/// skip.
///
/// # Errors
///
/// As [`build`]: scan corruption aborts; `CounterDesync` on a count past
/// the entry witness; `RowCountMismatch` when the count shrank below the
/// base or the tail scan disagrees with the claimed count.
///
/// # Panics
///
/// Only on programmer-invariant violations: `rel` names a closed relation,
/// or `base` was built for a different relation shape (the column layouts
/// disagree).
pub fn append(
    txn: &ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    base: &RelationImage,
    from_row_id: u64,
) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    debug_assert!(
        !relation.is_closed(),
        "closed relations synthesize from the theory, never from a scan"
    );
    let layout = relation.layout();
    let claimed = read::row_count(txn, rel)?;

    // The same reopen-trust ceiling as `build`: the stored count is data
    // and must not size an allocation unchecked.
    let witness = read::data_entries(txn)?;
    if claimed > witness {
        return Err(Error::Corruption(CorruptionError::CounterDesync {
            relation: rel,
            claimed,
            witness,
        }));
    }
    let row_count = usize::try_from(claimed).expect("64-bit usize");
    let base_rows = base.row_count();
    // Under a delete-free lineage the count is monotone; a shrink is
    // storage corruption, typed — hard error, never a silent rebuild.
    if row_count < base_rows {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: claimed,
        }));
    }

    let field_types: Vec<TypeDesc> = relation
        .fields()
        .iter()
        .map(|f| f.value_type.type_desc())
        .collect();
    let mut frame = allocate(&field_types, row_count)?;
    assert_eq!(
        frame.columns.len(),
        base.columns.len(),
        "the base image was built from this relation's field→column map"
    );

    // The prefix copy, one column at a time: the base's rows keep their
    // ordinals and words; only the slab addresses move.
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

    // The tail decode: the identical kernel over the suffix scan, filling
    // positions `base_rows..row_count` — the only rows that decode.
    let plan = decode_plan(&field_types, &frame.spans, &frame.columns, layout);
    let position = fill_columns(
        rel,
        read::scan_from(txn, schema, rel, from_row_id)?,
        &plan,
        layout.fact_width(),
        base_rows,
        row_count,
        &mut frame.words,
        &mut frame.bytes,
    )?;
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: claimed,
        }));
    }

    Ok(seal(row_count, frame))
}

/// One pooled transient-image slot (40-execution.md § the fixpoint driver): the fixpoint
/// driver's per-round delta and accumulated images, built on the
/// [`synthesize_closed`] precedent — the image machinery is
/// source-agnostic after decode, and here the source is cheaper still:
/// the rows are already encoded column words (a seen-set's dense
/// suffix), so the build is a columnar transpose with no fact-bytes
/// decode at all. **Never cached, never memoized, never pinned**: a
/// transient image is valid for one round of one execution — a lifetime
/// the generation vocabulary cannot express — so it lives entirely
/// outside `image/cache.rs` (whose diff for the recursion campaign is
/// zero lines) and the view memo; the closed carve-out's `OnceLock`
/// slots already proved images can live outside the map.
///
/// The slot is a retained-capacity pool on the prepared query (the
/// allocation contract's iteration-shape axis): a refill whose row
/// count fits the slot's high-water — and whose previous round's views
/// have all been dropped, the driver's ping-pong discipline — rewrites
/// the slabs in place through `Arc::get_mut`, touching the allocator
/// zero times.
#[derive(Debug, Default)]
pub struct TransientImage {
    image: Option<Arc<RelationImage>>,
    /// Rows the current allocation was framed for (column strides are
    /// laid out at this count; `row_count` may sit below it).
    capacity: usize,
}

impl TransientImage {
    /// Rebuilds this slot's image from `row_count` encoded word rows —
    /// one row per answer tuple, in the seen-set's find-word order,
    /// which is exactly the column order `column_spans(field_types)`
    /// lays out (an interval column two words, a `bytes<N>` column its
    /// padded words, a Bool column one 0/1 word written back as the
    /// byte). Reuses the retained allocation when the row count fits
    /// and no view still holds the `Arc`; otherwise allocates a fresh
    /// frame at the new high-water.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations: a row narrower than the
    /// field types' total column count, or a row count past the checked
    /// slab ceiling (seen-set positions are `u32`-bounded, orders of
    /// magnitude below it).
    pub fn refill<'r>(
        &mut self,
        field_types: &[TypeDesc],
        row_count: usize,
        rows: impl Iterator<Item = &'r [u64]>,
    ) -> Arc<RelationImage> {
        // A refill IS an append from row zero — one fill body, two
        // capacity policies (the delta slot is re-framed exactly per
        // round; only the accumulator needs headroom).
        self.fill(field_types, 0, row_count, |_| rows, CapacityPolicy::Exact)
    }

    /// The incremental sibling of [`Self::refill`] — the fixpoint
    /// accumulator's append path. Rows `[0, filled)` already sit in this
    /// slot from its previous call this execution, and a seen-set is
    /// append-only within one, so writing the suffix `[filled,
    /// row_count)` alone reproduces a full refill. When the in-place
    /// precondition fails — a view still holds the `Arc`, or `row_count`
    /// outgrew the framed capacity — the slot rebuilds whole from
    /// `rows_since(0)`, framed with doubling headroom (monotone, never
    /// below the retained high-water) so a growing accumulator
    /// reallocates logarithmically often, never per round.
    ///
    /// # Panics
    ///
    /// As [`Self::refill`]: programmer-invariant violations only.
    pub fn append<'r, I>(
        &mut self,
        field_types: &[TypeDesc],
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

    /// The one fill body behind [`Self::refill`] and [`Self::append`]
    /// (formerly two ~35-line verbatim siblings): rows `[0, filled)`
    /// already sit in the slot; the suffix `[filled, row_count)` is
    /// written in place when the reuse precondition holds — `row_count`
    /// within the framed capacity AND no view still holding the `Arc` —
    /// otherwise the slot rebuilds whole from `rows_since(0)`, framed by
    /// `policy`.
    fn fill<'r, I>(
        &mut self,
        field_types: &[TypeDesc],
        filled: usize,
        row_count: usize,
        rows_since: impl FnOnce(usize) -> I,
        policy: CapacityPolicy,
    ) -> Arc<RelationImage>
    where
        I: Iterator<Item = &'r [u64]>,
    {
        debug_assert!(filled <= row_count, "seen-sets never shrink");
        let reusable = row_count <= self.capacity
            && self
                .image
                .as_mut()
                .is_some_and(|arc| Arc::get_mut(arc).is_some());
        let base = if reusable { filled } else { 0 };
        if !reusable {
            let capacity = match policy {
                CapacityPolicy::Exact => row_count,
                CapacityPolicy::Doubling => self.capacity.max(row_count.saturating_mul(2)),
            };
            let frame = allocate(field_types, capacity)
                .expect("seen-set row counts sit far below the checked slab ceiling");
            self.image = Some(seal(row_count, frame));
            self.capacity = capacity;
        }
        let image = Arc::get_mut(self.image.as_mut().expect("filled above"))
            .expect("a non-reusable slot was just replaced by a unique Arc");
        image.row_count = row_count;
        // The lazy distinct counters restart with the rows (no consumer
        // reads them on the execution path today; staying honest is one
        // assignment per column, allocation-free).
        for lock in &mut image.distincts {
            *lock = std::sync::OnceLock::new();
        }
        let filled_to = fill_encoded_rows(image, base, rows_since(base));
        debug_assert_eq!(filled_to, row_count, "the caller counted its rows");
        Arc::clone(self.image.as_ref().expect("filled above"))
    }
}

/// How a non-reusable slot frames its fresh allocation: the per-round
/// delta refills exactly (each round's delta is independently sized —
/// headroom would be dead slab); the accumulator appends with doubling
/// headroom (monotone growth, never below the retained high-water, so
/// it reallocates logarithmically often, never per round).
#[derive(Clone, Copy)]
enum CapacityPolicy {
    Exact,
    Doubling,
}

/// The shared transpose of both fill paths above: encoded word rows into
/// consecutive positions from `base`; returns one past the last position
/// written.
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
