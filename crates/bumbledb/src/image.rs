//! Columnar relation images, the image cache, and filtered views.
//! Images retain **all columns** in structure-of-arrays vectors. A full image
//! comes from one sequential row scan and may enter the shared relation cache;
//! an indexed bucket image is query-local and reused only with matching
//! selections. Both feed the same Free Join machine. Immutable once built;
//! `Arc` is the sharing unit, and the image pins its text generation.
pub mod cache;
pub mod view;

mod bind;
mod build;
#[cfg(test)]
pub(crate) use build::test_generation;
pub(crate) mod canon;
mod decode;
mod distinct;
mod epoch;
pub(crate) mod intern;
mod selection;
mod stride;
#[cfg(test)]
pub(crate) mod testsupport;
mod text_eq;

pub(crate) use bind::{ImageBind, SourceImages};
pub(crate) use build::build_from_source;
pub(crate) use epoch::{CacheGeneration, ViewEpoch};

pub use build::{TransientImage, synthesize_closed};
pub use text_eq::TextEq;

// M2 Max's measured stream-tracker pitch period. Small nonzero residues
// near its multiples are the harmful band; exact multiples are allowed.
const SET_STRIDE: usize = 16_384;

// Outer memory-transfer alignment, NOT L1D line size (64 B on M2 Max).
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
                ValueType::Uuid => ColumnWidth::Words { count: 2 },
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
/// generation. The shared allocation owns its slabs, generation handle,
/// and canonical text. Eviction cannot invalidate a still-held image.
#[derive(Debug)]
pub struct RelationImage {
    row_count: usize,

    /// Exact scalar statistics, initialized only for columns the planner asks for.
    distincts: Box<[std::sync::OnceLock<u64>]>,

    spans: Box<[ColumnSpan]>,
    columns: Box<[Column]>,

    words: Vec<u64>,

    bytes: Vec<u8>,

    generation: crate::work::GenerationHandle,
    strings: Box<[bool]>,

    /// One shared owner per distinct resident text used by these columns.
    /// Holding a resolver namespace alone must not retain its whole history.
    texts: TextOwners,
}

#[derive(Debug, Default)]
pub(crate) struct TextOwners(std::collections::HashMap<u64, TextOwner>);

#[derive(Debug)]
struct TextOwner {
    text: std::sync::Arc<str>,
    observed: bool,
}

impl TextOwners {
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn pin(
        &mut self,
        word: u64,
        resolve: impl FnOnce() -> Option<std::sync::Arc<str>>,
    ) -> crate::error::Result<()> {
        if word == intern::SENTINEL_WORD {
            return Ok(());
        }
        match self.0.entry(word) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().observed = true;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let text = resolve().ok_or(crate::error::Error::Corruption(
                    crate::error::CorruptionError::MalformedValue("retained text token"),
                ))?;
                entry.insert(TextOwner {
                    text,
                    observed: true,
                });
            }
        }
        Ok(())
    }

    fn begin_refill(&mut self) {
        for text in self.0.values_mut() {
            text.observed = false;
        }
    }

    fn finish_refill(&mut self) {
        self.0.retain(|_, text| text.observed);
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn extend_from(&mut self, other: &Self) {
        for (&word, owner) in &other.0 {
            self.0.entry(word).or_insert_with(|| TextOwner {
                text: std::sync::Arc::clone(&owner.text),
                observed: true,
            });
        }
    }

    /// Pin only the string columns of a flat, typed row, without copying payloads.
    pub(crate) fn pin_row(
        &mut self,
        row: &[u64],
        types: &[bumbledb_theory::schema::ValueType],
        generation: &crate::work::GenerationHandle,
    ) -> crate::error::Result<()> {
        let mut slot = 0;
        for ty in types {
            if *ty == bumbledb_theory::schema::ValueType::String {
                let word = row[slot];
                self.pin(word, || generation.resolver().owned_text(word))?;
            }
            slot += crate::ir::normalize::SlotWidth::of(ty).slots();
        }
        Ok(())
    }
}

impl RelationImage {
    pub(crate) fn texts(&self) -> &TextOwners {
        &self.texts
    }

    /// Retained word/byte slab capacity, excluding fixed metadata and text.
    #[cfg(test)]
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>() + self.bytes.capacity()
    }

    #[must_use]
    pub fn generation(&self) -> &crate::work::GenerationHandle {
        &self.generation
    }

    /// True when this field's column words are generation-scoped text tokens.
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
