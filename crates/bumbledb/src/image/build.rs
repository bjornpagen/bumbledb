//! The build path: one sequential canonical-row scan decodes every column
//! of a relation into structure-of-arrays slabs — and the synthesis path,
//! which fills the same slabs from a closed relation's sealed extension
//! with no storage anywhere.
use std::sync::Arc;

use crate::api::prepared::source::QuerySource;
use crate::error::{CorruptionError, Error, Result};
use crate::image::canon::{TextWords, row_words};
use crate::schema::{Relation, Schema};
use crate::work::GenerationHandle;
use bumbledb_theory::schema::RelationId;
use bumbledb_theory::schema::ValueType;

use super::decode::{decode_fact, decode_plan};
use super::{
    Column, ColumnSpan, ColumnWidth, LINE, PAD_MIN_STRIDE, RelationImage, SET_STRIDE, StridePadder,
    column_spans,
};

/// The `S` value is data: overflow in any size computation is typed Corruption
/// before a single byte is allocated.
fn slab_lengths(row_count: usize, word_cols: usize, byte_cols: usize) -> Result<(usize, usize)> {
    let corrupt = || Error::Corruption(CorruptionError::MalformedValue("S row count"));
    let length = |element_size: usize, columns: usize| {
        // Alignment adds less than LINE bytes. If even this upper bound
        // stays below PAD_MIN_STRIDE, the padder cannot add a pitch gap.
        // Small images therefore need alignment headroom only, not 16 KiB
        // per column. Large-column placement and its conservative capacity
        // stay unchanged, including experimental padding tolerances.
        let aligned_bound = row_count.checked_add(LINE / element_size)?;
        let per_column = if aligned_bound >= PAD_MIN_STRIDE / element_size {
            aligned_bound.checked_add(SET_STRIDE / element_size)?
        } else {
            aligned_bound
        };
        per_column
            .checked_mul(columns)?
            .checked_mul(element_size)
            .map(|bytes| bytes / element_size)
    };
    Ok((
        length(8, word_cols).ok_or_else(corrupt)?,
        length(1, byte_cols).ok_or_else(corrupt)?,
    ))
}

struct Frame {
    spans: Box<[ColumnSpan]>,
    columns: Vec<Column>,
    words: Vec<u64>,
    bytes: Vec<u8>,
    strings: Box<[bool]>,
    texts: super::TextOwners,
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
        texts: super::TextOwners::default(),
    })
}

fn seal(row_count: usize, frame: Frame, generation: GenerationHandle) -> Arc<RelationImage> {
    Arc::new(RelationImage {
        row_count,
        distincts: (0..frame.columns.len())
            .map(|_| std::sync::OnceLock::new())
            .collect(),
        spans: frame.spans,
        columns: frame.columns.into_boxed_slice(),
        words: frame.words,
        bytes: frame.bytes,
        generation,
        strings: frame.strings,
        texts: frame.texts,
    })
}

/// Bytes the image slabs will retain after allocation, excluding text and metadata.
#[cfg(test)]
pub(crate) fn estimated_slab_bytes(field_types: &[ValueType], row_count: usize) -> Result<usize> {
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
    word_len
        .checked_mul(8)
        .and_then(|words| words.checked_add(byte_len))
        .ok_or(Error::Corruption(CorruptionError::MalformedValue(
            "S row count",
        )))
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
    seal(row_count, frame, test_generation())
}

#[cfg(test)]
pub(crate) fn test_generation() -> GenerationHandle {
    GenerationHandle::new(crate::work::GenerationState::new(
        crate::image::CacheGeneration::initial(),
    ))
}

/// Build one relation's full image from a committed source: one sequential
/// canonical-row scan, decoded through the one walker ([`row_words`]) with
/// the generation's resolver minting text tokens.
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
) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    debug_assert!(
        relation.body().closed_rows().is_none(),
        "closed relations synthesize from the theory, never from a scan"
    );
    let claimed = source.row_count(rel)?;
    build_from_scan(schema, generation, rel, claimed, source.work(), |sink| {
        source.scan(schema, rel, sink)
    })
}

/// The same admitted columns and decoder for a full relation or a counted
/// physical bucket. The caller owns the coverage proof; this builder never
/// publishes into the shared full-relation cache.
pub(super) fn build_from_scan(
    schema: &Schema,
    generation: &GenerationHandle,
    rel: RelationId,
    claimed: u64,
    work: &crate::work::WorkContext,
    scan: impl FnOnce(&mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()>,
) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    let row_count = usize::try_from(claimed).expect("64-bit usize");

    let field_types: Vec<ValueType> = relation.fields().iter().map(|f| f.value_type).collect();
    let fields = relation.fields();
    work.checkpoint()
        .map_err(crate::api::prepared::source::work_error)?;

    let spans = column_spans(&field_types);
    let column_count = spans
        .last()
        .map_or(0, |s| usize::from(s.first_column + s.width.column_count()));

    let mut frame = allocate(&field_types, row_count)?;
    let mut interner = generation.lock_resolver();
    let mut text = TextWords::Intern {
        interner: &mut interner,
        work,
        texts: &mut frame.texts,
    };
    let mut scratch: Vec<u64> = Vec::with_capacity(column_count);
    let mut position = 0usize;
    {
        let columns = &frame.columns;
        let words = &mut frame.words;
        let bytes = &mut frame.bytes;
        let scan = scan(&mut |row| {
            if position >= row_count {
                return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                    relation: rel,
                    stored: claimed,
                }));
            }
            scratch.clear();
            row_words(fields, row, &mut text, &mut scratch, &mut Vec::new())?;
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
        scan?;
    }
    drop(interner);
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: claimed,
        }));
    }

    Ok(seal(row_count, frame, generation.clone()))
}

/// Reusable columnar storage for derived relations. A published image
/// retains its resolver generation and the text owners behind its tokens;
/// only uniquely owned images can be refilled in place.
#[derive(Debug, Default)]
pub enum TransientImage {
    #[default]
    Empty,
    Occupied {
        image: Arc<RelationImage>,
        capacity: usize,
    },
}

impl TransientImage {
    pub(crate) fn is_uniquely_owned(&mut self) -> bool {
        match self {
            Self::Empty => true,
            Self::Occupied { image, .. } => Arc::get_mut(image).is_some(),
        }
    }

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
        .expect("valid in-memory rows have live text owners")
    }

    /// Refill from a fallible drain (the spill-aware seen-set path):
    /// `drain(0, write)` must feed all `row_count` rows in order.
    /// The optional context provides cooperative cancellation; column
    /// capacity is allocated or reused before the drain begins.
    /// # Errors
    /// The drain's failure, invalid text ownership, or allocation failure.
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
            row_count,
            generation,
            CapacityPolicy::Exact,
            drain,
        )
    }

    /// Finalize at most `row_bound` rows directly into reserved columns.
    /// Pack may coalesce several claims into one row; its input count is
    /// an allocation bound, not the published relation's cardinality.
    pub(crate) fn refill_bounded(
        &mut self,
        work: &crate::work::WorkContext,
        field_types: &[ValueType],
        row_bound: usize,
        generation: &GenerationHandle,
        drain: impl FnOnce(usize, &mut dyn FnMut(&[u64])) -> crate::error::Result<()>,
    ) -> crate::error::Result<Arc<RelationImage>> {
        self.fill_drained(
            Some(work),
            field_types,
            row_bound,
            generation,
            CapacityPolicy::UpperBound,
            drain,
        )
    }

    fn fill_drained(
        &mut self,
        work: Option<&crate::work::WorkContext>,
        field_types: &[ValueType],
        row_count: usize,
        generation: &GenerationHandle,
        policy: CapacityPolicy,
        drain: impl FnOnce(usize, &mut dyn FnMut(&[u64])) -> crate::error::Result<()>,
    ) -> crate::error::Result<Arc<RelationImage>> {
        if let Some(work) = work {
            work.checkpoint()
                .map_err(crate::api::prepared::source::work_error)?;
        }
        let reusable = match self {
            Self::Occupied { image, capacity } if row_count <= *capacity => {
                Arc::get_mut(image).is_some()
            }
            Self::Empty | Self::Occupied { .. } => false,
        };
        if !reusable {
            let capacity = row_count;
            let frame = allocate(field_types, capacity)?;

            *self = Self::Occupied {
                image: seal(row_count, frame, generation.clone()),
                capacity,
            };
        }
        let Self::Occupied { image, .. } = self else {
            unreachable!("fill just occupied the slot");
        };
        let image_mut =
            Arc::get_mut(image).expect("a non-reusable slot was just replaced by a unique Arc");
        if !image_mut.generation.ptr_eq(generation) {
            // Numeric token bits may be reused by a different namespace.
            image_mut.texts.0.clear();
        }
        image_mut.generation = generation.clone();
        // Invalidate before any write, including a drain that fails partway.
        for count in &mut image_mut.distincts {
            count.take();
        }
        image_mut.row_count = row_count;
        image_mut.texts.begin_refill();
        let filled_to = match drain_encoded_rows(image_mut, drain) {
            Ok(filled) => filled,
            Err(error) => {
                // A failed drain cannot leave a reusable image naming
                // partially replaced rows or keep their text owners alive.
                image_mut.row_count = 0;
                image_mut.texts.0.clear();
                return Err(error);
            }
        };
        image_mut.texts.finish_refill();
        match policy {
            CapacityPolicy::UpperBound => image_mut.row_count = filled_to,
            CapacityPolicy::Exact => {
                assert_eq!(filled_to, row_count, "the caller counted its rows");
            }
        }
        Ok(Arc::clone(image))
    }
}

#[derive(Clone, Copy)]
enum CapacityPolicy {
    Exact,
    UpperBound,
}

fn drain_encoded_rows(
    image: &mut RelationImage,
    drain: impl FnOnce(usize, &mut dyn FnMut(&[u64])) -> crate::error::Result<()>,
) -> crate::error::Result<usize> {
    let row_bound = image.row_count;
    let has_text = image.strings.iter().any(|string| *string);
    let RelationImage {
        columns,
        words,
        bytes,
        spans,
        strings,
        texts,
        generation,
        ..
    } = image;
    let mut position = 0;
    let mut text_error = None;
    drain(0, &mut |row| {
        assert!(
            position < row_bound,
            "drain exceeded its admitted row bound"
        );
        debug_assert_eq!(
            row.len(),
            columns.len(),
            "seen-set rows carry one word per image column"
        );
        if has_text && text_error.is_none() {
            for (span, string) in spans.iter().zip(strings.iter()) {
                if *string {
                    let word = row[usize::from(span.first_column)];
                    if let Err(error) = texts.pin(word, || generation.resolver().owned_text(word)) {
                        text_error = Some(error);
                        break;
                    }
                }
            }
        }
        for (column, &word) in columns.iter().zip(row) {
            match *column {
                Column::Words { start } => words[start + position] = word,
                Column::Bytes { start } => bytes[start + position] = u8::from(word != 0),
            }
        }
        position += 1;
    })?;
    if let Some(error) = text_error {
        return Err(error);
    }
    Ok(position)
}

/// Synthesizes a closed relation's image from its sealed extension. Sealed
/// canonical fact bytes use the same decoding and stride-padded columns as
/// stored rows, including the leading implicit `id` column (`0..rows`).
/// # Errors
/// A slab length that cannot be represented.
/// # Panics
/// Only if `relation` is ordinary or its sealed rows violate the validated schema.
pub fn synthesize_closed(
    rel: RelationId,
    relation: &Relation,
    generation: GenerationHandle,
) -> Result<Arc<RelationImage>> {
    let extension = relation
        .body()
        .closed_rows()
        .expect("synthesize_closed takes a closed relation");
    let layout = relation.layout();
    let row_count = extension.len();
    let field_types: Vec<ValueType> = relation.fields().iter().map(|f| f.value_type).collect();
    let mut frame = allocate(&field_types, row_count)?;
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
    Ok(seal(row_count, frame, generation))
}

#[cfg(test)]
mod capacity_tests {
    use super::*;

    #[test]
    fn slab_bound_covers_every_alignment_and_pitch_boundary() {
        for element_size in [1, 8] {
            let boundary = PAD_MIN_STRIDE / element_size;
            for rows in [
                0,
                1,
                LINE / element_size,
                boundary - LINE / element_size - 1,
                boundary - LINE / element_size,
                boundary - 1,
                boundary,
                boundary + 1,
                boundary + 384 / element_size,
                boundary + SET_STRIDE / element_size - 1,
                boundary + SET_STRIDE / element_size,
            ] {
                let (words, bytes) = slab_lengths(rows, 16, 16).unwrap();
                let capacity = if element_size == 8 { words } else { bytes };
                for base in (0..LINE).step_by(element_size) {
                    for tolerance in [0, super::super::PAD_TOLERANCE, 2048] {
                        let mut padder = StridePadder::with_tolerance(tolerance);
                        let mut cursor = 0;
                        for _ in 0..16 {
                            let start = padder.place(base, element_size, cursor);
                            assert_eq!((base + start * element_size) % LINE, 0);
                            assert!(start >= cursor, "columns cannot overlap");
                            cursor = start + rows;
                            assert!(cursor <= capacity, "placement exceeds admitted capacity");
                        }
                    }
                }
            }
        }
        assert!(slab_lengths(usize::MAX, 1, 1).is_err());
        assert!(slab_lengths(usize::MAX / 8, 8, 0).is_err());
    }

    #[test]
    fn tiny_image_retention_is_payload_plus_alignment_not_pitch_slack() {
        let fields = [ValueType::U64; 6];
        let expected = 6 * (32 * 8 + LINE);
        assert_eq!(estimated_slab_bytes(&fields, 32).unwrap(), expected);
        let image = image_with_tolerance(&fields, 32, super::super::PAD_TOLERANCE);
        assert_eq!(image.byte_size(), expected);
        for column in 0..6 {
            assert_eq!(image.column_words(column).len(), 32);
            assert_eq!(image.column_words(column).as_ptr().addr() % LINE, 0);
        }
    }
}
