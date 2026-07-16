//! Columnar relation images, the image cache, and filtered views (docs/architecture).
//!
//! A relation image is **all columns** of a relation, decoded once from one
//! sequential `F`-prefix scan into structure-of-arrays vectors — the bridge
//! to paper-faithful execution (`docs/architecture/40-execution.md` D1,
//! `40-storage.md`). Immutable once built; `Arc` is the sharing unit.

pub mod cache;
pub mod view;

mod build;
mod cardinality;
mod decode;
mod stride;

pub use build::{TransientImage, build, synthesize_closed};

/// The 16 KiB granule two hardware structures key on (measured):
/// the L1D's set congruence (256 sets × 64 B lines,
/// index bits 6–13 — a mild ≤1.55× on real lockstep scans) and the
/// stream-prefetch trackers' page-number bits (the severe one: 4–6× on
/// DRAM lockstep scans when strides sit near a multiple). The layout
/// rule pads STRIDES off multiples of this ([`StridePadder`]); the old
/// belief that congruent bases cost "10–20×" is retired — that figure
/// required a fully dependent load chain and never applied to
/// scans.
const SET_STRIDE: usize = 16_384;

/// Column base alignment: 128 B is the L2/SLC/DRAM transfer granule
/// (the L1D manages 64 B lines behind it — both numbers are real,
/// measured); alignment to the outer granule serves both.
const LINE: usize = 128;

/// One decoded column: a range into the image's backing store. Positions
/// are dense scan ordinals `0..row_count`; row ids exist only in LMDB keys
/// and never appear in images.
#[derive(Debug, Clone, Copy)]
enum Column {
    /// 8-byte column: the byte-order-normalized u64 word. For every 8-byte
    /// type the word is `u64::from_be_bytes(canonical bytes)` — for U64 the
    /// numeric value, for I64 the sign-flipped biased word (order-preserving
    /// under u64 compare), for String/Bytes the intern id. An interval
    /// field's start and end halves are each one such column.
    Words { start: usize },
    /// 1-byte column: the validated Bool byte.
    Bytes { start: usize },
}

/// How many columns a field occupies and what they hold. The image layer
/// has exactly two column *kinds* — there is no 16-byte column: a
/// multi-word field decodes into parallel 8-byte columns and every
/// existing kernel shape applies unchanged (`docs/architecture/50-storage.md`).
/// An interval field is two columns with start/end semantics; a
/// `bytes<N>` field with N > 8 is `⌈N/8⌉` plain word columns (the
/// interval two-column precedent, generalized) — and a `bytes<N ≤ 8>`
/// field is ONE word column, exactly like every other 8-byte scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnWidth {
    /// One 1-byte column (Bool).
    Byte,
    /// One 8-byte column (U64/I64/String/bytes<N ≤ 8>).
    Word,
    /// Two consecutive 8-byte columns: the interval's start word at
    /// `first_column`, its end word at `first_column + 1`.
    WordPair,
    /// `count ≥ 2` consecutive 8-byte columns: a `bytes<N > 8>` field's
    /// padded value words, in byte order.
    Words { count: u16 },
}

impl ColumnWidth {
    /// Number of image columns the span covers.
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
    /// Index of the field's first column, in field declaration order with
    /// interval fields counting twice.
    pub first_column: u16,
    pub width: ColumnWidth,
}

/// Builds the per-relation field→column map from the relation's
/// encoding-level field types, once per image (and once per plan witness):
/// an interval field spans two consecutive 8-byte columns, a `bytes<N>`
/// field its `⌈N/8⌉` word columns (one plain word column for N ≤ 8),
/// every other field one column of its width.
#[must_use]
pub fn column_spans(field_types: &[crate::encoding::TypeDesc]) -> Box<[ColumnSpan]> {
    use crate::encoding::TypeDesc;
    let mut next_column = 0u16;
    field_types
        .iter()
        .map(|desc| {
            let width = match desc {
                TypeDesc::Bool => ColumnWidth::Byte,
                TypeDesc::U64 | TypeDesc::I64 | TypeDesc::String => ColumnWidth::Word,
                TypeDesc::FixedBytes { len } => {
                    match u16::try_from(crate::encoding::fixed_bytes_words(*len))
                        .expect("bytes width is at most 8 words")
                    {
                        1 => ColumnWidth::Word,
                        count => ColumnWidth::Words { count },
                    }
                }
                TypeDesc::Interval { .. } => ColumnWidth::WordPair,
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
/// generation.
#[derive(Debug)]
pub struct RelationImage {
    row_count: usize,
    /// Per-column exact distinct-value counts, computed LAZILY on first
    /// planner demand: the eager per-column pass was
    /// the cold path's dominant fixed cost (~1.8 ms per 150k rows,
    /// paid before the first query could run — even a key probe that
    /// needs no estimates). The image is generation-keyed by the cache,
    /// so a `OnceLock` per column IS the per-(snapshot, relation,
    /// column) stats cache; the counts themselves are unchanged (same
    /// exact algorithm, same values — laziness moves when, never what).
    distincts: Box<[std::sync::OnceLock<u64>]>,
    /// The field→column map (one span per field, in declaration order).
    spans: Box<[ColumnSpan]>,
    columns: Box<[Column]>,
    /// Backing store for 8-byte columns; column bases are 128-byte aligned
    /// with strides padded off 16 KiB multiples (see [`StridePadder`]).
    words: Vec<u64>,
    /// Backing store for 1-byte columns, same alignment discipline.
    bytes: Vec<u8>,
}

impl RelationImage {
    /// The image's heap footprint: both slab capacities in bytes (a
    /// store-level observability number — the benchmark report and the
    /// `image_build` trace span's byte arg read it).
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>() + self.bytes.capacity()
    }

    /// Number of facts in the image (dense positions `0..row_count`).
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }

    /// The field→column span of field `field` (the per-relation map's
    /// lookup; every field→column translation goes through here).
    #[must_use]
    pub fn span(&self, field: crate::schema::FieldId) -> ColumnSpan {
        self.spans[usize::from(field.0)]
    }

    /// The column at `column` index. Column indices come from
    /// [`ColumnSpan`]s — an interval field's two word columns sit at
    /// `first_column` and `first_column + 1`.
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

    /// The words of an 8-byte column.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: `column` is a 1-byte column
    /// (callers dispatch on the field's [`ColumnSpan`] width).
    #[cfg(test)]
    #[must_use]
    pub fn column_words(&self, column: usize) -> &[u64] {
        match self.column(column) {
            ColumnView::Words(words) => words,
            ColumnView::Bytes(_) => panic!("column {column} is a 1-byte column"),
        }
    }

    /// The bytes of a 1-byte column.
    ///
    /// # Panics
    ///
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

/// Column strides padded away from prefetch-tracker aliasing.
/// The measured law: the L1D's
/// 16 KiB set congruence costs AT MOST 1.55× on real lockstep scans —
/// but stream-prefetch trackers alias on low 16 KiB page-number bits,
/// so power-of-two-ish strides with small (1–3 line) staggers cost
/// 4–6× on DRAM-tier lockstep scans (8.13 vs 1.78 ns/row measured).
/// The old rule here — odd 128 B residues mod 16 KiB, the "stagger" —
/// was built against the first (mild) hazard and CREATED the second.
/// The replacement: when a column-to-column stride is large enough to be
/// scanned from DRAM (≥ [`PAD_MIN_STRIDE`]) and lands a small NONZERO
/// offset (≤ [`PAD_TOLERANCE`]) from a 16 KiB multiple, round it UP to
/// the next exact multiple — exact multiples measured clean (the
/// stagger-16,384 discriminator ran fast); the poison is the small
/// offset. Below [`PAD_MIN_STRIDE`], columns are cache-resident at scan
/// time and no tracker interference was measured — disk is not free.
struct StridePadder {
    /// The band half-width in force ([`PAD_TOLERANCE`] in production;
    /// parameterized so the measured falsifier can lay out the shipped
    /// rule and its widened twin in one process — the interleaved A/B).
    tolerance: usize,
    /// Previous column start per backing slab (element index), so the
    /// stride under test is always between neighbors in the SAME slab —
    /// lockstep scans stride within a slab.
    prev_start_by_width: [Option<usize>; 2],
}

/// Strides below this never pad (the columns are cache-resident when
/// scanned; the pathology is a DRAM-stream phenomenon).
const PAD_MIN_STRIDE: usize = 64 * 1024;

/// How close (bytes) to a 16 KiB multiple a stride must land to count as
/// tracker-aliasing-shaped: the measured discriminators put stagger 8,
/// 32, 64, and 128 in the pathological band and 16,384 out of it.
///
/// A 2 KiB widening was proposed from the ledger's dense residue sweep
/// (poison band ~48 B–2 KiB at small pitches) and REFUTED by the
/// interleaved falsifier (`tests/stride_ab.rs`,
/// `docs/reports/stride-padder-band.md`): at the pitches a real image's
/// DRAM-tier lockstep scans can actually have (≥4 MiB — column stride
/// IS the pitch, and ≥8 streams leave the SLC only at MB-scale spans)
/// the band decays far faster than at the sweep's small pitches —
/// residue 384 costs ~1.4×, 1 KiB ~1.1×, parity by ~1.5 KiB, and
/// padding a 2 KiB residue INVERTS (~0.9×: the pad pessimizes) — all
/// on a synthetic tight kernel; the engine's widest real lockstep
/// reader (the scalar filter conjunction) is retire-bound at ~22 ns/row
/// and measures 1.00× against any of these layouts. 384 stays.
const PAD_TOLERANCE: usize = 384;

#[cfg(test)]
mod tests;
