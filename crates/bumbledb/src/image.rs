//! Columnar relation images, the image cache, and filtered views.
//! A relation image is **all columns** of a relation, decoded once from one
//! sequential `F`-prefix scan into structure-of-arrays vectors — the bridge
//! to paper-faithful execution. Immutable once built; `Arc` is the sharing unit.
pub mod cache;
pub mod view;

mod bind;
mod build;
pub(crate) mod canon;
mod decode;
mod distinct;
mod epoch;
pub(crate) mod intern;
mod nonresident;
mod stride;
#[cfg(test)]
pub(crate) mod testsupport;

pub(crate) use bind::{ImageBind, SourceImages};
pub(crate) use build::build_from_source;
pub(crate) use epoch::{CacheGeneration, TextGeneration, ViewEpoch};

pub use build::{TransientImage, synthesize_closed};
/// Production intern/image refusal. L05 execute/spill matches
/// [`ResidentAdmit::BeyondMemory`] and calls
/// [`ResidentTextExhausted::open_nonresident`]. The one equality is
/// [`TextEq::tokens_equal`] / [`TextEq::canonical`] (`Result`, not raw
/// `u64 ==`; resolver failure is `Err`, never inequality).
/// Stamp memos with [`NonresidentTextStore::epoch`] (full owner id, not
/// packed into tokens). Invalidate on mismatch or drop.
pub use intern::{
    is_resident_token, is_scratch_token, ResidentAdmit, ResidentTextExhausted, SCRATCH_TOKEN_TAG,
};
pub use nonresident::{
    scratch_token_epoch, NonresidentTextStore, TextEq, TextStoreEpoch,
};
pub use crate::work::cache::WeakGenerationHandle;

const SET_STRIDE: usize = 16_384;

const LINE: usize = 128;

#[derive(Debug, Clone, Copy)]
enum Column {
    Words { start: usize },

    Bytes { start: usize },
}

/// How many columns a field occupies and what they hold. The image layer
/// has exactly two column *kinds* — there is no 16-byte column: a
/// multi-word field decodes into parallel 8-byte columns and every
/// existing kernel shape applies unchanged.
/// An interval field is two columns with start/end semantics; a
/// `bytes<N>` field with N > 8 is `⌈N/8⌉` plain word columns (the
/// interval two-column precedent, generalized) — and a `bytes<N ≤ 8>`
/// field is ONE word column, exactly like every other 8-byte scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnWidth {
    Byte,

    Word,

    WordPair,

    Words { count: u16 },
}

impl ColumnWidth {
    #[must_use]
    pub const fn column_count(self) -> u16 {
        match self {
            Self::Byte | Self::Word => 1,
            Self::WordPair => 2,
            Self::Words { count } => count,
        }
    }
}

/// One field's columns in the image: the per-relation field→column map's
/// value. The map is the only field→column interface — consumers (the
/// filter evaluator here, the plan witness downstream) dispatch on spans,
/// never on raw field indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnSpan {
    pub first_column: u16,
    pub width: ColumnWidth,
}

/// Builds the per-relation field→column map from the relation's
/// encoding-level field types, once per image (and once per plan witness):
/// an interval field spans two consecutive 8-byte columns, a `bytes<N>`
/// field its `⌈N/8⌉` word columns (one plain word column for N ≤ 8),
/// every other field one column of its width.
#[must_use]
pub fn column_spans(field_types: &[bumbledb_theory::schema::ValueType]) -> Box<[ColumnSpan]> {
    use bumbledb_theory::schema::ValueType;
    let mut next_column = 0u16;
    field_types
        .iter()
        .map(|desc| {
            let width = match desc {
                ValueType::Bool => ColumnWidth::Byte,
                ValueType::U64 | ValueType::I64 | ValueType::F64 | ValueType::String => {
                    ColumnWidth::Word
                }
                // Sixteen exact identity bytes: two big-endian word
                // columns — byte order IS the value's one total order,
                // so two-word lexicographic comparison is exact.
                ValueType::Id128 => ColumnWidth::Words { count: 2 },
                ValueType::FixedBytes { len } => {
                    match u16::try_from(crate::encoding::fixed_bytes_words(*len))
                        .expect("bytes width is at most 8 words")
                    {
                        1 => ColumnWidth::Word,
                        count => ColumnWidth::Words { count },
                    }
                }
                ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
                    ColumnWidth::WordPair
                }
            };
            let span = ColumnSpan {
                first_column: next_column,
                width,
            };
            next_column = next_column
                .checked_add(width.column_count())
                .expect("column count fits u16");
            span
        })
        .collect()
}

/// A borrowed view of one column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnView<'a> {
    Words(&'a [u64]),
    Bytes(&'a [u8]),
}

/// The immutable full-width columnar image of one relation at one
/// generation. The shared allocation owns the generation handle and,
/// when cache-admitted, the slab charge. Dropping a cache map entry
/// does not refund a still-held image.
#[derive(Debug)]
pub struct RelationImage {
    row_count: usize,

    /// paid by every cold prepare and again by every re-prepare after a
    distincts: Box<[distinct::DistinctState]>,

    spans: Box<[ColumnSpan]>,
    columns: Box<[Column]>,

    words: Vec<u64>,

    bytes: Vec<u8>,

    generation: crate::work::GenerationHandle,
    charge: Option<crate::work::ChargedImage>,
    strings: Box<[bool]>,
}

impl RelationImage {
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>() + self.bytes.capacity()
    }

    #[must_use]
    pub fn generation(&self) -> &crate::work::GenerationHandle {
        &self.generation
    }

    #[must_use]
    pub fn resolver(&self) -> crate::work::ResolverView<'_> {
        self.generation.resolver()
    }

    #[must_use]
    pub fn charged_bytes(&self) -> Option<u64> {
        self.charge.as_ref().map(crate::work::ChargedImage::charged_bytes)
    }

    /// True when this field's column words are intern/scratch text tokens.
    #[must_use]
    pub fn field_is_string(&self, field: bumbledb_theory::schema::FieldId) -> bool {
        self.strings
            .get(usize::from(field.0))
            .copied()
            .unwrap_or(false)
    }

    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }

    #[must_use]
    pub fn span(&self, field: bumbledb_theory::schema::FieldId) -> ColumnSpan {
        self.spans[usize::from(field.0)]
    }

    #[must_use]
    pub fn column(&self, column: usize) -> ColumnView<'_> {
        match self.columns[column] {
            Column::Words { start } => {
                ColumnView::Words(&self.words[start..start + self.row_count])
            }
            Column::Bytes { start } => {
                ColumnView::Bytes(&self.bytes[start..start + self.row_count])
            }
        }
    }

    /// # Panics
    /// On a programmer-invariant violation: `column` is a 1-byte column
    #[cfg(test)]
    #[must_use]
    pub fn column_words(&self, column: usize) -> &[u64] {
        match self.column(column) {
            ColumnView::Words(words) => words,
            ColumnView::Bytes(_) => panic!("column {column} is a 1-byte column"),
        }
    }

    /// # Panics
    /// On a programmer-invariant violation: `column` is an 8-byte column.
    #[cfg(test)]
    #[must_use]
    pub fn column_bytes(&self, column: usize) -> &[u8] {
        match self.column(column) {
            ColumnView::Bytes(bytes) => bytes,
            ColumnView::Words(_) => panic!("column {column} is an 8-byte column"),
        }
    }
}

struct StridePadder {
    tolerance: usize,

    prev_start_by_width: [Option<usize>; 2],
}

const PAD_MIN_STRIDE: usize = 64 * 1024;

const PAD_TOLERANCE: usize = 384;

#[cfg(test)]
mod tests;
