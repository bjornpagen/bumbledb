//! The build path: one sequential canonical-row scan decodes every column
//! of a relation into structure-of-arrays slabs — and the synthesis path,
//! which fills the same slabs from a closed relation's sealed extension
//! with no storage anywhere.
use std::sync::Arc;

use crate::api::prepared::source::QuerySource;
use crate::error::{CorruptionError, Error, Result};
use crate::image::canon::{TextWords, row_words};
use crate::schema::{Relation, Schema};
use crate::work::{ByteKind, ChargedImage, GenerationHandle};
use bumbledb_theory::schema::RelationId;
use bumbledb_theory::schema::ValueType;

use super::decode::{decode_fact, decode_plan};
use super::{
    Column, ColumnSpan, ColumnWidth, LINE, RelationImage, SET_STRIDE, StridePadder, column_spans,
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
    strings: Box<[bool]>,
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
        strings: field_types
            .iter()
            .map(|ty| matches!(ty, ValueType::String))
            .collect(),
    })
}

fn seal(
    row_count: usize,
    frame: Frame,
    distincts: Box<[super::distinct::DistinctState]>,
    generation: GenerationHandle,
    charge: Option<ChargedImage>,
) -> Arc<RelationImage> {
    Arc::new(RelationImage {
        row_count,
        distincts,
        spans: frame.spans,
        columns: frame.columns.into_boxed_slice(),
        words: frame.words,
        bytes: frame.bytes,
        generation,
        charge,
        strings: frame.strings,
    })
}

/// Bytes the image slabs will retain after allocate. Admission uses this
/// before growth so a cache refusal never leaves an uncharged allocation.
pub(crate) fn estimated_slab_bytes(
    field_types: &[ValueType],
    row_count: usize,
) -> Result<usize> {
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
    Ok(word_len
        .checked_mul(8)
        .and_then(|words| words.checked_add(byte_len))
        .ok_or_else(|| Error::Corruption(CorruptionError::MalformedValue("S row count")))?)
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
    seal(
        row_count,
        frame,
        distincts,
        test_generation(),
        None,
    )
}

#[cfg(test)]
fn test_generation() -> GenerationHandle {
    GenerationHandle::new(crate::work::GenerationState::new(
        crate::image::CacheGeneration::initial(),
        crate::work::CacheLedger::unbounded(),
    ))
}

/// Build one relation's full image from a committed source: one sequential
/// canonical-row scan, decoded through the one walker ([`row_words`]) with
/// the generation's resolver minting text tokens. Cache admission happens
/// **before** slab allocate; the charge lives inside the sealed image.
/// # Errors
/// A scan yielding a different number of rows than the source's committed
/// count is corruption; malformed stored bytes refuse; stopped work and
/// allocation refusal are typed resource failures.
/// # Panics
/// Only on programmer-invariant violations (`rel` names a closed relation —
/// closed images synthesize from the theory, and the cache branches before
/// this path).
pub(crate) fn build_from_source(
    source: &QuerySource<'_>,
    schema: &Schema,
    generation: &GenerationHandle,
    rel: RelationId,
) -> Result<crate::image::ResidentAdmit<Arc<RelationImage>>> {
    let relation = schema.relation(rel);
    debug_assert!(
        relation.body().closed_rows().is_none(),
        "closed relations synthesize from the theory, never from a scan"
    );
    let claimed = source.row_count(rel)?;
    let row_count = usize::try_from(claimed).expect("64-bit usize");

    let field_types: Vec<ValueType> = relation.fields().iter().map(|f| f.value_type).collect();
    let fields = relation.fields();
    let estimated = estimated_slab_bytes(&field_types, row_count)?;
    let charge = match ChargedImage::admit(generation.ledger(), estimated) {
        Ok(charge) => charge,
        Err(_) => {
            return Ok(crate::image::ResidentAdmit::BeyondMemory(
                crate::image::ResidentTextExhausted::new(generation.clone()),
            ));
        }
    };

    let spans = column_spans(&field_types);
    let column_count = spans
        .last()
        .map_or(0, |s| usize::from(s.first_column + s.width.column_count()));
    let work = source.work();
    let _work_charge = work
        .reserve(ByteKind::Working, estimated as u64)
        .map_err(crate::api::prepared::source::work_error)?;

    let mut frame = allocate(&field_types, row_count)?;
    let decode_span = crate::obs::span_args(
        crate::obs::names::DECODE_BATCH,
        crate::obs::TraceArgs::Pair(row_count as u64, column_count as u64),
    );
    let mut interner = generation.lock_resolver();
    let mut text = TextWords::Intern {
        interner: &mut interner,
        work,
        generation,
    };
    let mut scratch: Vec<u64> = Vec::with_capacity(column_count);
    let mut position = 0usize;
    let mut spilled = None;
    {
        let columns = &frame.columns;
        let words = &mut frame.words;
        let bytes = &mut frame.bytes;
        let scan = source.scan(rel, &mut |row| {
            if position >= row_count {
                return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                    relation: rel,
                    stored: claimed,
                }));
            }
            scratch.clear();
            match row_words(fields, row, &mut text, &mut scratch)? {
                crate::image::ResidentAdmit::Ready(()) => {}
                crate::image::ResidentAdmit::BeyondMemory(exhausted) => {
                    spilled = Some(exhausted);
                    return Err(Error::from_store(
                        crate::storage::store::StoreError::Allocation,
                    ));
                }
            }
            debug_assert_eq!(scratch.len(), columns.len(), "one word per column");
            for (column, &word) in columns.iter().zip(&scratch) {
                match *column {
                    Column::Words { start } => words[start + position] = word,
                    Column::Bytes { start } => {
                        bytes[start + position] = u8::try_from(word).expect("bool word");
                    }
                }
            }
            position += 1;
            Ok(())
        });
        if let Some(exhausted) = spilled {
            return Ok(crate::image::ResidentAdmit::BeyondMemory(exhausted));
        }
        scan?;
    }
    drop(interner);
    decode_span.end();
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: claimed,
        }));
    }

    let distincts = count_frame(row_count, &frame);
    Ok(crate::image::ResidentAdmit::Ready(seal(
        row_count,
        frame,
        distincts,
        generation.clone(),
        Some(charge),
    )))
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
    Empty {
        capacity: usize,
    },
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
    /// layout, or more rows than promised.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "infallible test twin of `refill_drained`")
    )]
    pub fn refill<'r>(
        &mut self,
        field_types: &[ValueType],
        row_count: usize,
        generation: &GenerationHandle,
        mut rows: impl Iterator<Item = &'r [u64]>,
    ) -> Arc<RelationImage> {
        self.fill_drained(
            None,
            field_types,
            0,
            row_count,
            generation,
            CapacityPolicy::Exact,
            |_, write| {
                for row in rows.by_ref() {
                    write(row);
                }
                Ok(())
            },
        )
        .expect("uncharged RAM drains are infallible")
    }

    /// Refill from a fallible drain (the spill-aware seen-set path):
    /// `drain(base, write)` must feed rows `base..row_count` in order. A
    /// `Some` work context reserves the slab bytes BEFORE any allocation
    /// (Q-BUDGET: growth is admitted, never discovered) — a transient
    /// admission charge, matching `build_from_source`.
    /// # Errors
    /// The drain's failure (scratch read, stopped work), or the slab
    /// reservation's typed refusal.
    /// # Panics
    /// As [`Self::refill`]: programmer-invariant violations only.
    pub fn refill_drained(
        &mut self,
        work: Option<&crate::work::WorkContext>,
        field_types: &[ValueType],
        row_count: usize,
        generation: &GenerationHandle,
        drain: impl FnOnce(usize, &mut dyn FnMut(&[u64])) -> crate::error::Result<()>,
    ) -> crate::error::Result<Arc<RelationImage>> {
        self.fill_drained(
            work,
            field_types,
            0,
            row_count,
            generation,
            CapacityPolicy::Exact,
            drain,
        )
    }

    /// Append rows `filled..row_count` under a doubling capacity policy,
    /// from a fallible drain with an optional slab admission charge (see
    /// [`Self::refill_drained`]).
    /// # Errors
    /// As [`Self::refill_drained`].
    /// # Panics
    /// As [`Self::refill`]: programmer-invariant violations only.
    pub fn append_drained(
        &mut self,
        work: Option<&crate::work::WorkContext>,
        field_types: &[ValueType],
        filled: usize,
        row_count: usize,
        generation: &GenerationHandle,
        drain: impl FnOnce(usize, &mut dyn FnMut(&[u64])) -> crate::error::Result<()>,
    ) -> crate::error::Result<Arc<RelationImage>> {
        self.fill_drained(
            work,
            field_types,
            filled,
            row_count,
            generation,
            CapacityPolicy::Doubling,
            drain,
        )
    }

    fn fill_drained(
        &mut self,
        work: Option<&crate::work::WorkContext>,
        field_types: &[ValueType],
        filled: usize,
        row_count: usize,
        generation: &GenerationHandle,
        policy: CapacityPolicy,
        drain: impl FnOnce(usize, &mut dyn FnMut(&[u64])) -> crate::error::Result<()>,
    ) -> crate::error::Result<Arc<RelationImage>> {
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
            // Charge the fresh slabs before allocating them — a transient
            // admission reservation, like `build_from_source`'s (the
            // retained figure is the owner's to report).
            let _slab_charge = match work {
                Some(work) => {
                    let spans = column_spans(field_types);
                    let byte_cols = spans
                        .iter()
                        .filter(|s| s.width == ColumnWidth::Byte)
                        .count();
                    let column_count = spans
                        .last()
                        .map_or(0, |s| usize::from(s.first_column + s.width.column_count()));
                    let (word_len, byte_len) =
                        slab_lengths(capacity, column_count - byte_cols, byte_cols)?;
                    Some(
                        work.reserve(ByteKind::Working, (word_len as u64) * 8 + byte_len as u64)
                            .map_err(crate::api::prepared::source::work_error)?,
                    )
                }
                None => None,
            };
            let frame = allocate(field_types, capacity)
                .expect("seen-set row counts sit far below the checked slab ceiling");

            let distincts = super::distinct::uncounted_columns(&frame.columns);
            *self = Self::Occupied {
                image: seal(row_count, frame, distincts, generation.clone(), None),
                capacity,
            };
        }
        let Self::Occupied { image, .. } = self else {
            unreachable!("fill just occupied the slot");
        };
        let image_mut =
            Arc::get_mut(image).expect("a non-reusable slot was just replaced by a unique Arc");
        image_mut.row_count = row_count;
        let filled_to = drain_encoded_rows(image_mut, base, drain)?;
        debug_assert_eq!(filled_to, row_count, "the caller counted its rows");
        Ok(Arc::clone(image))
    }
}

#[derive(Clone, Copy)]
enum CapacityPolicy {
    Exact,
    Doubling,
}

fn drain_encoded_rows(
    image: &mut RelationImage,
    base: usize,
    drain: impl FnOnce(usize, &mut dyn FnMut(&[u64])) -> crate::error::Result<()>,
) -> crate::error::Result<usize> {
    let RelationImage {
        columns,
        words,
        bytes,
        ..
    } = image;
    let mut position = base;
    drain(base, &mut |row| {
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
        position += 1;
    })?;
    Ok(position)
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
/// # Errors
/// Cache admission refusal is [`crate::image::ResidentAdmit::BeyondMemory`].
pub fn synthesize_closed(
    rel: RelationId,
    relation: &Relation,
    generation: GenerationHandle,
) -> Result<crate::image::ResidentAdmit<Arc<RelationImage>>> {
    let extension = relation
        .body()
        .closed_rows()
        .expect("synthesize_closed takes a closed relation");
    let layout = relation.layout();
    let row_count = extension.len();
    let field_types: Vec<ValueType> = relation.fields().iter().map(|f| f.value_type).collect();
    let estimated = estimated_slab_bytes(&field_types, row_count)?;
    let charge = match ChargedImage::admit(generation.ledger(), estimated) {
        Ok(charge) => charge,
        Err(_) => {
            return Ok(crate::image::ResidentAdmit::BeyondMemory(
                crate::image::ResidentTextExhausted::new(generation),
            ));
        }
    };
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
    Ok(crate::image::ResidentAdmit::Ready(seal(
        row_count,
        frame,
        distincts,
        generation,
        Some(charge),
    )))
}
