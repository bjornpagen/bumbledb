//! The build path: one sequential scan decodes every column of a relation
//! into structure-of-arrays slabs (docs/architecture/40-execution.md D1,
//! `50-storage.md`; the per-fact decode kernel lives in `super::decode`) —
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
/// of the given field shape. Shared by the two fill paths — the catalog
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
fn allocate(field_types: &[ValueType], row_count: usize) -> Result<Frame> {
    allocate_with(field_types, row_count, StridePadder::new())
}

/// [`allocate`] with an explicit padder — the measured falsifier's hook:
/// the shipped stride rule and its twin lay out side by side in one
/// process. The slabs are sized identically either way ([`slab_lengths`]
/// pre-pays one `SET_STRIDE + LINE` of slack per column, the worst-case
/// alignment plus pad), so the tolerance moves column starts within the
/// slack and never the allocation.
fn allocate_with(
    field_types: &[ValueType],
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

/// Seals a filled frame into the shared image with its distinct
/// counting state — computed by the caller: the scan paths count while
/// the slabs are warm ([`super::distinct::count_columns`]), the append
/// path extends the base's persisted state (O(tail), exact), and the
/// transient fixpoint slots stay uncounted (the planner never costs
/// them — the `Interior` floor guard).
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

/// The build-path counting pass over a filled frame, spanned at batch
/// granularity (`image_distincts`): one pass per column while the just-
/// decoded slabs are warm, into state sized to the distincts.
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

/// An empty (all-zero) sealed image laid out under an explicit stride
/// tolerance — the measured falsifier's constructor: identical shape and
/// data either way, only the column starts move. Test-only; production
/// layouts go through [`allocate`] and the one shipped tolerance.
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

/// Prefix `F` scan: the relation's live facts in row-id order.
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

/// Suffix `F` range from `from_row_id` through the relation's last fact
/// key — the image-append tail. Mis-shaped keys are typed corruption.
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
    // corrupt-but-plausible value (2^40 passes every checked size
    // computation) would drive the slab `vec!`s below into a
    // multi-terabyte allocation. Bound it by the `_data` entry count
    // — an over-approximation (the DBI spans F/M/U/R/Q/S, so it counts
    // far more than this relation's F rows), which a ceiling is allowed
    // to be: no real row count can exceed it, and the scan cross-check
    // below stays the exactness guarantee.
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

    // One sequential scan fills every column (positions = scan ordinals),
    // through the hoisted decode plan.
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
/// otherwise — read in the base's own transaction). The distinct
/// counting state persists with the base exactly for this moment: the
/// sealed image clones it and inserts ONLY the tail rows — exact by
/// construction (the image oracle's served-vs-rebuilt equality), at
/// O(tail) instead of a full re-walk.
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
    // Under a delete-free lineage the count is monotone; a shrink is
    // storage corruption, typed — hard error, never a silent rebuild.
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

    // The persisted counting state's whole payoff: clone the base's
    // exact state and insert only the tail — never re-walk the prefix.
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

/// One pooled transient-image slot (40-execution.md § the linear reach driver): the fixpoint
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
#[derive(Debug)]
pub enum TransientImage {
    /// No image yet; `capacity` is the last framed high-water (0 at new).
    Empty { capacity: usize },
    Occupied {
        image: Arc<RelationImage>,
        /// Rows the current allocation was framed for (column strides are
        /// laid out at this count; `row_count` may sit below it).
        capacity: usize,
    },
}

impl Default for TransientImage {
    fn default() -> Self {
        Self::Empty { capacity: 0 }
    }
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
        field_types: &[ValueType],
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

    /// The one fill body behind [`Self::refill`] and [`Self::append`]
    /// (formerly two ~35-line verbatim siblings): rows `[0, filled)`
    /// already sit in the slot; the suffix `[filled, row_count)` is
    /// written in place when the reuse precondition holds — `row_count`
    /// within the framed capacity AND no view still holding the `Arc` —
    /// otherwise the slot rebuilds whole from `rows_since(0)`, framed by
    /// `policy`.
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
            // Transient images stay UNCOUNTED: the planner never costs
            // them (an `Interior` occurrence pins no statistics — the
            // selectivity floor guard), so a per-round counting pass
            // would be pure waste inside the fixpoint's warm loop.
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
/// sealed field list), stride padding, and the build-time distinct
/// counting pass are all the ordinary image machinery, untouched.
///
/// # Panics
///
/// Only on programmer-invariant violations: `relation` is ordinary, or a
/// sealed row fails the canonical decode — both impossible for a
/// validated schema.
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
